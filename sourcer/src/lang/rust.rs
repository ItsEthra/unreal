use crate::{
    ClassData, ClassRegistry, EnumGenerator, IdName, Layout, PackageGenerator, PropertyType,
    SdkGenerator, StructGenerator,
};
use std::{
    borrow::Cow,
    collections::HashSet,
    fs::{self, File, OpenOptions},
    io::{Result, Write},
    path::{Path, PathBuf},
    rc::Rc,
};

struct EnumGen<'a>(&'a mut Module);
impl<'a> EnumGenerator for EnumGen<'a> {
    fn begin(&mut self, name: &str, id_name: IdName, min_max: Option<(i64, i64)>) -> Result<()> {
        let ty = if let Some((min, max)) = min_max {
            if min < i32::MIN as i64 || max > i32::MAX as i64 {
                "i64"
            } else {
                "i32"
            }
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
}

impl<'a> StructGenerator for StructGen<'a> {
    fn begin(
        &mut self,
        name: &str,
        id_name: IdName,
        layout: Layout,
        parent: Option<IdName>,
    ) -> Result<()> {
        writeln!(self.module.classes, "// Full name: {id_name}")?;
        writeln!(
            self.module.classes,
            "// Unaligned size: 0x{:X}",
            layout.size
        )?;
        writeln!(
            self.module.classes,
            "// Alignment: 0x{:X}",
            layout.alignment
        )?;
        writeln!(self.module.classes, "memflex::makestruct! {{")?;

        // TODO: Implement zeroed from bytemuck, maybe reexport Zeroed trait in memflex?
        if let Some(parent) = parent {
            let ClassData {
                code_name, package, ..
            } = self.registry.lookup(&parent).unwrap();

            if &self.module.package_name != package {
                self.module.imports.insert(parent);
            }

            writeln!(self.module.classes, "\tpub struct {name} : {code_name} {{",)?;
        } else {
            writeln!(self.module.classes, "\tpub struct {name} {{")?;
        }

        Ok(())
    }

    fn append_field(
        &mut self,
        _field_name: &str,
        _field_ty: PropertyType,
        _elem_size: usize,
        _offset: usize,
    ) -> Result<()> {
        Ok(())
    }

    fn end(&mut self) -> Result<()> {
        writeln!(self.module.classes, "\t}}\n}}\n")?;

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
        }))
    }

    fn end(&mut self) -> Result<()> {
        let dependencies = self
            .module
            .imports
            .iter()
            .map(|id| self.registry.lookup(id).expect("Unresolved import"))
            .map(|ClassData { package, .. }| package)
            .collect::<HashSet<_>>();

        for pkg in dependencies {
            writeln!(self.this.toml, "{pkg}.workspace = true")?;
            writeln!(self.this.librs, "use {pkg}::*;")?;
        }

        writeln!(self.this.librs, "")?;

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
        // Add to workspace
        {
            self.packages.push(name.to_string());
            // writeln!(self.workspace.toml, "[workspace.dependencies.{name}]")?;
            // writeln!(self.workspace.toml, "path = \"crates/{name}\"")?;
            // writeln!(self.workspace.toml, "[dependencies.{name}]")?;
            // writeln!(self.workspace.toml, "workspace = true\noptional = true")?;
        }

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

            writeln!(package.librs, "")?;
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
    OpenOptions::new().create(true).write(true).open(path)
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
    deps: HashSet<IdName>,
}

impl<'a> TypeStringifier<'a> {
    pub fn new(registry: &'a ClassRegistry) -> Self {
        Self {
            registry,
            deps: HashSet::new(),
        }
    }

    pub fn stringify(&mut self, ty: PropertyType) -> Cow<'a, str> {
        let mut fetch_dep = |id: IdName| -> &str {
            self.deps.insert(id.clone());
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
            PropertyType::Set(ty) => format!("ucore::TArray<{}>", self.stringify(*ty)).into(),
            PropertyType::ClassPtr(ty) => {
                format!("ucore::ClassPtr<{}>", self.stringify(*ty)).into()
            }
            PropertyType::Name => "ucore::FName".into(),
            PropertyType::String => "ucore::FString".into(),
            PropertyType::Text => "ucore::FText".into(),
            PropertyType::InlineClass(id) => fetch_dep(id).into(),
            PropertyType::InlineEnum(id) => fetch_dep(id).into(),
        }
    }
}
