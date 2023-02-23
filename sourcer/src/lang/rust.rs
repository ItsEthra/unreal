use crate::{
    EnumGenerator, IdName, Layout, PackageGenerator, PackageRegistry, PropertyType,
    RegistrationData, RegistrationExtra, SdkGenerator, StructGenerator,
};
use eyre::{eyre, Result};
use log::warn;
use offsets::Offsets;
use std::{
    borrow::Cow,
    collections::HashSet,
    fmt::Write as WriteFmt,
    fs::{self, File, OpenOptions},
    io::Write as WriteIo,
    path::{Path, PathBuf},
    rc::Rc,
};

fn pick_enum_size(min_max: Option<(i64, i64)>) -> (&'static str, usize) {
    if let Some((min, max)) = min_max {
        if u8::MAX as i64 >= max && (i8::MIN as i64) <= min {
            ("i8", 1)
        } else if u16::MAX as i64 >= max && (i16::MIN as i64) <= min {
            ("i16", 2)
        } else if u32::MAX as i64 >= max && (i32::MIN as i64) <= min {
            ("i32", 4)
        } else {
            ("i64", 8)
        }
    } else {
        ("i32", 4)
    }
}

#[test]
fn test_enum_size() {
    assert_eq!(pick_enum_size(Some((0, 255))), ("i8", 1));
}

struct EnumGen<'a> {
    module: &'a mut Module,
    registry: &'a PackageRegistry,
}

impl<'a> EnumGenerator for EnumGen<'a> {
    fn begin(&mut self, name: &str, id_name: IdName) -> Result<()> {
        let ty = pick_enum_size(
            self.registry
                .lookup(&id_name)
                .and_then(|d| d.extra.unwrap_enum()),
        )
        .0;

        writeln!(self.module.enums, "// Full name: {id_name}")?;
        writeln!(self.module.enums, "memflex::bitflags! {{")?;
        writeln!(self.module.enums, "\t#[repr(transparent)]")?;
        writeln!(self.module.enums, "\tpub struct {name} : {ty} {{")?;

        Ok(())
    }

    fn append_variant(&mut self, variant: &str, value: i64) -> Result<()> {
        writeln!(self.module.enums, "\t\tconst {variant} = {value};")?;

        Ok(())
    }

    fn end(&mut self) -> Result<()> {
        writeln!(self.module.enums, "\t}}\n}}\n")?;

        Ok(())
    }
}

struct StructGen<'a> {
    module: &'a mut Module,
    registry: &'a PackageRegistry,
    offset: usize,
    layout: Layout,
    name: String,
}

impl<'a> StructGenerator for StructGen<'a> {
    fn begin(
        &mut self,
        name: &str,
        id_name: IdName,
        layout: Layout,
        parent: Option<IdName>,
    ) -> Result<()> {
        self.layout = layout;
        self.name = name.to_string();

        let mut parent_code_full_name_and_size = None;

        writeln!(self.module.classes, "// {id_name}")?;
        writeln!(
            self.module.classes,
            "// Size: 0x{:X}. Alignment: {:X}.{}",
            layout.align(),
            layout.alignment,
            if let Some((parent_id, parent_data)) = parent
                .as_ref()
                .map(|p| (p, self.registry.lookup(p).expect("Missing import")))
            {
                let RegistrationExtra::ClassLayout(parent_layout) = parent_data.extra else {
                        return Err(eyre!("Deriving from enum is not allowed"))
                };
                parent_code_full_name_and_size =
                    Some((&parent_data.code_name, parent_id, parent_layout.align()));

                Cow::Owned(format!(" (Parent: 0x{:X})", parent_layout.align()))
            } else {
                Cow::Borrowed("")
            }
        )?;
        writeln!(self.module.classes, "memflex::makestruct! {{")?;

        // TODO: Implement zeroed from bytemuck, maybe reexport Zeroed trait in memflex?
        if let Some((pcode, pfull, psize)) = parent_code_full_name_and_size {
            if pcode == name {
                warn!("{pfull} is recursive. Skipping parenting");
                writeln!(self.module.classes, "\tpub struct {name} {{",)?;
            } else {
                self.offset = psize;
                writeln!(self.module.classes, "\tpub struct {name} : {pcode} {{",)?;
            }
            self.module.imports.insert(pfull.clone());
        } else {
            writeln!(self.module.classes, "\tpub struct {name} {{")?;
        }

        Ok(())
    }

    fn append_field(
        &mut self,
        field_name: &str,
        field_ty: PropertyType,
        elem_size: usize,
        offset: usize,
    ) -> Result<()> {
        let maybe_enum_min_max = match field_ty {
            PropertyType::Inline(ref id) => match self.registry.lookup(id).unwrap().extra {
                RegistrationExtra::EnumMinMax(min_max) => Some(min_max),
                _ => None,
            },
            _ => None,
        };

        let size = match field_ty {
            PropertyType::Array { ref size, .. } => *size as usize * elem_size,
            _ => match maybe_enum_min_max {
                Some(enum_size) => pick_enum_size(enum_size).1.min(elem_size),
                _ => elem_size,
            },
        };

        let mut ts = TypeStringifier::new(self.registry);
        let typename = ts.stringify(field_ty);

        self.module.imports.extend(ts.into_imports());

        if offset > self.offset {
            writeln!(
                self.module.classes,
                "\t\t_pad_0x{:X}: [u8; 0x{:X}], // Offset: 0x{}. Size: 0x{:X}",
                self.offset,
                offset - self.offset,
                self.offset,
                offset - self.offset,
            )?;
        } else if offset < self.offset {
            warn!("{field_name}: {typename} has offset that less than the previous, skipping field to prevent overflows.");
            return Ok(());
        }
        self.offset = offset + size;

        writeln!(
            self.module.classes,
            "\t\tpub {field_name}: {typename}, // Offset: 0x{offset:X}. Size: 0x{size:X}"
        )?;

        Ok(())
    }

    fn end(&mut self) -> Result<()> {
        let expected_size = self.layout.align();
        if self.offset != expected_size {
            writeln!(
                self.module.classes,
                "\t\t_pad_0x{:X}: [u8; 0x{:X}], // Offset: 0x{:X}. Size: 0x{:X}",
                self.offset,
                expected_size - self.offset,
                self.offset,
                expected_size - self.offset,
            )?;
        }

        writeln!(self.module.classes, "\t}}\n}}")?;
        writeln!(
            self.module.classes,
            "ucore::assert_size!({}, 0x{:X});\n",
            self.name,
            self.layout.align()
        )?;

        Ok(())
    }
}

struct PackageGen {
    this: Crate,
    module: Module,
    registry: Rc<PackageRegistry>,
}

impl PackageGenerator for PackageGen {
    fn add_enum<'new>(&'new mut self) -> Result<Box<dyn crate::EnumGenerator + 'new>> {
        Ok(Box::new(EnumGen {
            module: &mut self.module,
            registry: &self.registry,
        }))
    }

    fn add_struct<'new>(&'new mut self) -> Result<Box<dyn crate::StructGenerator + 'new>> {
        Ok(Box::new(StructGen {
            module: &mut self.module,
            registry: &self.registry,
            offset: 0,
            layout: Layout::default(),
            name: String::new(),
        }))
    }

    fn end(&mut self) -> Result<()> {
        let dependencies = self
            .module
            .imports
            .iter()
            .map(|id| self.registry.lookup(id).expect("Unresolved import"))
            .filter_map(|RegistrationData { package, .. }| {
                if package != &self.module.package_name {
                    Some(package)
                } else {
                    None
                }
            })
            .collect::<HashSet<_>>();

        for pkg in dependencies {
            writeln!(self.this.toml, "{pkg}.workspace = true")?;
            writeln!(self.this.librs, "use {pkg}::*;")?;
        }

        writeln!(self.this.librs)?;

        writeln!(self.this.librs, "{}", self.module.bitfields)?;
        writeln!(self.this.librs, "{}", self.module.enums)?;
        writeln!(self.this.librs, "{}", self.module.classes)?;

        Ok(())
    }
}

pub struct RustSdkGenerator {
    crates: PathBuf,
    workspace: Crate,

    packages: Vec<String>,
    offsets: &'static Offsets,
}

impl SdkGenerator for RustSdkGenerator {
    fn new(path: impl AsRef<Path>, offsets: &'static Offsets) -> Result<Self>
    where
        Self: Sized,
    {
        let path = path.as_ref().join("usdk");
        fs::create_dir_all(&path)?;

        let toml = open_file(path.join("Cargo.toml"))?;
        let librs = open_file(path.join("lib.rs"))?;

        Ok(Self {
            crates: path.join("crates"),
            workspace: Crate { toml, librs },
            packages: vec![],
            offsets,
        })
    }

    fn begin_package<'pkg>(
        &mut self,
        name: &str,
        registry: &Rc<PackageRegistry>,
    ) -> Result<Box<dyn PackageGenerator + 'pkg>> {
        self.packages.push(name.to_string());

        // Add to librs
        {
            writeln!(self.workspace.librs, "#[cfg(feature = \"{name}\")]")?;
            writeln!(self.workspace.librs, "pub use {name};")?;
        }

        let crate_path = self.crates.join(name);
        fs::create_dir_all(&crate_path)?;

        let mut package = Crate {
            toml: open_file(crate_path.join("Cargo.toml"))?,
            librs: open_file(crate_path.join("lib.rs"))?,
        };

        // Declare package in Cargo.toml
        {
            writeln!(package.toml, "[package]")?;
            writeln!(package.toml, "name = \"{name}\"")?;
            writeln!(package.toml, "version = \"0.1.0\"")?;
            writeln!(package.toml, "edition = \"2021\"\n")?;
            writeln!(package.toml, "[lib]")?;
            writeln!(package.toml, "path = \"lib.rs\"\n")?;
            writeln!(package.toml, "[dependencies]")?;
            writeln!(package.toml, "ucore.workspace = true")?;
            writeln!(package.toml, "memflex.workspace = true\n")?;
        }

        // Write prelude to lib.rs
        {
            const WARNINGS: &[&str] = &[
                "non_camel_case_types",
                "non_snake_case",
                "non_upper_case_globals",
                "dead_code",
                "overflowing_literals",
                "unused_imports",
            ];

            for warn in WARNINGS {
                writeln!(package.librs, "#![allow({warn})]")?;
            }
            writeln!(
                package.librs,
                "\ntype FName = ucore::FName<{stride}, {size}, {index}, {header}, {data}, {wide_bit}, {len_bit}>;",
                stride = self.offsets.stride,
                index = self.offsets.fname.index,
                size = self.offsets.fname.size,
                header = self.offsets.fnameentry.header,
                data = self.offsets.fnameentry.data,
                wide_bit = self.offsets.fnameentry.wide_bit,
                len_bit = self.offsets.fnameentry.len_bit,
            )?;

            writeln!(package.librs)?;
        }

        Ok(Box::new(PackageGen {
            this: package,
            registry: registry.clone(),
            module: Module {
                package_name: name.to_string(),
                imports: HashSet::new(),
                bitfields: String::new(),
                enums: String::new(),
                classes: String::new(),
            },
        }))
    }

    fn end(&mut self) -> Result<()> {
        const WORKSPACE_DEF: &str = include_str!("../../workspace.toml");
        writeln!(self.workspace.toml, "{WORKSPACE_DEF}")?;

        for pkg in self.packages.iter() {
            writeln!(self.workspace.toml, "{pkg} = {{ path = \"crates/{pkg}\" }}")?;
        }

        writeln!(self.workspace.toml, "\n[dependencies]")?;
        for pkg in self.packages.iter() {
            writeln!(
                self.workspace.toml,
                "{pkg} = {{ workspace = true, optional = true }}"
            )?;
        }

        Ok(())
    }
}

fn open_file(path: impl AsRef<Path>) -> Result<File> {
    OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(path)
        .map_err(Into::into)
}

struct Crate {
    toml: File,
    librs: File,
}

struct Module {
    package_name: String,
    imports: HashSet<IdName>,
    bitfields: String,
    enums: String,
    classes: String,
}

struct TypeStringifier<'a> {
    registry: &'a PackageRegistry,
    imports: HashSet<IdName>,
}

impl<'a> TypeStringifier<'a> {
    pub fn new(registry: &'a PackageRegistry) -> Self {
        Self {
            registry,
            imports: HashSet::new(),
        }
    }

    pub fn into_imports(self) -> impl Iterator<Item = IdName> {
        self.imports.into_iter()
    }

    pub fn stringify(&mut self, ty: PropertyType) -> Cow<'a, str> {
        let mut fetch_dep = |id: IdName| -> &str {
            self.imports.insert(id.clone());
            &self
                .registry
                .lookup(&id)
                .expect("Missing dependency")
                .code_name
        };

        match ty {
            PropertyType::Int8 => "i8".into(),
            PropertyType::Int16 => "i16".into(),
            PropertyType::Int32 => "i32".into(),
            PropertyType::Int64 => "i64".into(),
            PropertyType::UInt8 => "u8".into(),
            PropertyType::UInt16 => "u16".into(),
            PropertyType::UInt32 => "u32".into(),
            PropertyType::UInt64 => "u64".into(),
            PropertyType::Float32 => "f32".into(),
            PropertyType::Float64 => "f64".into(),
            PropertyType::Bool => "bool".into(),
            PropertyType::Array { ty, size } => format!("[{}; {size}]", self.stringify(*ty)).into(),
            PropertyType::Vector(ty) => format!("ucore::TArray<{}>", self.stringify(*ty)).into(),
            PropertyType::Map { key, value } => format!(
                "ucore::TMap<{}, {}>",
                self.stringify(*key),
                self.stringify(*value)
            )
            .into(),
            PropertyType::Set(ty) => format!("ucore::TSet<{}>", self.stringify(*ty)).into(),
            PropertyType::ClassPtr(ty) => {
                format!("Option<ucore::Ptr<{}>>", self.stringify(*ty)).into()
            }
            PropertyType::Name => "crate::FName".into(),
            PropertyType::String => "ucore::FString".into(),
            PropertyType::Text => "ucore::FText".into(),
            PropertyType::Inline(id) => fetch_dep(id).into(),
        }
    }
}
