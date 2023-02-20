use crate::{
    ClassData, ClassRegistry, EnumGenerator, IdName, Layout, PackageGenerator, PropertyType,
    SdkGenerator, StructGenerator,
};
use log::warn;
use std::{
    borrow::Cow,
    collections::HashSet,
    fs::{self, File, OpenOptions},
    io::{Result, Write},
    path::{Path, PathBuf},
    rc::Rc,
};

fn pick_enum_size(min: i64, max: i64) -> &'static str {
    if i8::MAX as i64 > max && (i8::MIN as i64) < min {
        "i8"
    } else if i16::MAX as i64 > max && (i16::MIN as i64) < min {
        "i16"
    } else if i32::MAX as i64 > max && (i32::MIN as i64) < min {
        "i32"
    } else {
        "i64"
    }
}

struct EnumGen<'a>(&'a mut Module);
impl<'a> EnumGenerator for EnumGen<'a> {
    fn begin(&mut self, name: &str, id_name: IdName, min_max: Option<(i64, i64)>) -> Result<()> {
        let ty = if let Some((min, max)) = min_max {
            pick_enum_size(min, max)
        } else {
            "i32"
        };

        writeln!(self.0.enums, "// Full name: {id_name}")?;
        writeln!(self.0.enums, "memflex::bitflags! {{")?;
        writeln!(self.0.enums, "\t#[repr(transparent)]")?;
        writeln!(self.0.enums, "\tpub struct {name} : {ty} {{")?;

        Ok(())
    }

    fn append_variant(&mut self, variant: &str, value: i64) -> Result<()> {
        writeln!(self.0.enums, "\t\tconst {variant} = {value};")?;

        Ok(())
    }

    fn end(&mut self) -> Result<()> {
        writeln!(self.0.enums, "\t}}\n}}\n")?;

        Ok(())
    }
}

struct StructGen<'a> {
    module: &'a mut Module,
    registry: &'a ClassRegistry,
    offset: usize,
    layout: Layout,
    name: String,
    field_names: HashSet<String>,
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

        writeln!(self.module.classes, "// {id_name}")?;
        writeln!(
            self.module.classes,
            "// Size: 0x{:X}. Alignment: {:X}.{}",
            layout.align(),
            layout.alignment,
            if let Some(parent) = parent.as_ref().map(|p| self
                .registry
                .lookup(p)
                .expect("Missing import")
                .layout
                .expect("Deriving from enum is not allowed")
                .align())
            {
                Cow::Owned(format!(" (Parent: 0x{parent:X})"))
            } else {
                Cow::Borrowed("")
            }
        )?;
        writeln!(self.module.classes, "memflex::makestruct! {{")?;

        // TODO: Implement zeroed from bytemuck, maybe reexport Zeroed trait in memflex?
        if let Some(parent) = parent {
            let ClassData {
                code_name, layout, ..
            } = self.registry.lookup(&parent).unwrap();
            self.offset = layout
                .as_ref()
                .expect("Deriving from enum is not allowed")
                .size;

            if code_name == name {
                warn!("{parent} is recursive. Skipping parenting");
                writeln!(self.module.classes, "\tpub struct {name} {{",)?;
            } else {
                writeln!(self.module.classes, "\tpub struct {name} : {code_name} {{",)?;
            }
            self.module.imports.insert(parent);
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
        let size = match field_ty {
            PropertyType::Array { ref size, .. } => *size as usize * elem_size,
            _ => elem_size,
        };

        let mut ts = TypeStringifier::new(self.registry);
        let typename = ts.stringify(field_ty);

        self.module.imports.extend(ts.into_imports());

        if offset > self.offset {
            writeln!(
                self.module.classes,
                "\t\t_pad_0x{:X}: [u8; 0x{:X}],",
                self.offset,
                offset - self.offset
            )?;
        }
        self.offset = offset + size;

        // Sometimes field names are duplicated for whatever reason
        let name = if self.field_names.contains(field_name) {
            warn!("Field {field_name} of type {typename} is duplicate");

            format!("{field_name}_{offset:X}")
        } else {
            self.field_names.insert(field_name.to_owned());
            field_name.to_owned()
        };

        writeln!(
            self.module.classes,
            "\t\tpub {name}: {typename}, // Offset: 0x{offset:X}. Size: 0x{size:X}"
        )?;

        Ok(())
    }

    fn end(&mut self) -> Result<()> {
        let expected_size = self.layout.align();
        if self.offset != expected_size {
            writeln!(
                self.module.classes,
                "\t\t_pad_0x{:x}: [u8; 0x{:X}],",
                self.offset,
                expected_size - self.offset
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
    registry: Rc<ClassRegistry>,
}

impl PackageGenerator for PackageGen {
    fn add_enum<'new>(&'new mut self) -> Result<Box<dyn crate::EnumGenerator + 'new>> {
        Ok(Box::new(EnumGen(&mut self.module)))
    }

    fn add_struct<'new>(&'new mut self) -> Result<Box<dyn crate::StructGenerator + 'new>> {
        Ok(Box::new(StructGen {
            module: &mut self.module,
            registry: &self.registry,
            offset: 0,
            layout: Layout::default(),
            field_names: HashSet::new(),
            name: String::new(),
        }))
    }

    fn end(&mut self) -> Result<()> {
        let dependencies = self
            .module
            .imports
            .iter()
            .map(|id| self.registry.lookup(id).expect("Unresolved import"))
            .filter_map(|ClassData { package, .. }| {
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

        self.this.librs.write_all(&self.module.enums)?;
        self.this.librs.write_all(&self.module.classes)?;

        Ok(())
    }
}

pub struct RustSdkGenerator {
    crates: PathBuf,
    workspace: Crate,

    packages: Vec<String>,
}

impl SdkGenerator for RustSdkGenerator {
    fn begin_package<'sdk: 'pkg, 'pkg>(
        &'sdk mut self,
        name: &str,
        registry: &Rc<ClassRegistry>,
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
            ];

            for warn in WARNINGS {
                writeln!(package.librs, "#![allow({warn})]")?;
            }

            writeln!(package.librs)?;
        }

        Ok(Box::new(PackageGen {
            this: package,
            registry: registry.clone(),
            module: Module {
                package_name: name.to_string(),
                ..Default::default()
            },
        }))
    }

    fn new(path: impl AsRef<Path>) -> Result<Self>
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
        })
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
}

struct Crate {
    toml: File,
    librs: File,
}

#[derive(Default)]
struct Module {
    package_name: String,
    imports: HashSet<IdName>,
    enums: Vec<u8>,
    classes: Vec<u8>,
}

struct TypeStringifier<'a> {
    registry: &'a ClassRegistry,
    imports: HashSet<IdName>,
}

impl<'a> TypeStringifier<'a> {
    pub fn new(registry: &'a ClassRegistry) -> Self {
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
                format!("Option<ucore::ClassPtr<{}>>", self.stringify(*ty)).into()
            }
            PropertyType::Name => "ucore::FName".into(),
            PropertyType::String => "ucore::FString".into(),
            PropertyType::Text => "ucore::FText".into(),
            PropertyType::InlineClass(id) => fetch_dep(id).into(),
            PropertyType::InlineEnum(id) => fetch_dep(id).into(),
        }
    }
}
