use crate::{
    EnumGenerator, IdName, Layout, PackageGenerator, PropertyType, SdkGenerator, StructGenerator,
};
use std::{
    fs::{self, File, OpenOptions},
    io::{Result, Write},
    path::{Path, PathBuf},
};

struct EnumGen<'a>(&'a mut Crate);
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

        writeln!(self.0.librs, "// Full name: {id_name}")?;
        writeln!(self.0.librs, "memflex::bitflags! {{")?;
        writeln!(self.0.librs, "\t#[repr(transparent)]")?;
        writeln!(self.0.librs, "\tpub struct {name} : {ty} {{")?;

        Ok(())
    }

    fn append_variant(&mut self, variant: &str, value: i64) -> Result<()> {
        writeln!(self.0.librs, "\t\tconst {variant} = {value};")?;

        Ok(())
    }

    fn end(&mut self) -> Result<()> {
        writeln!(self.0.librs, "\t}}\n}}\n")?;

        Ok(())
    }
}

struct StructGen<'a>(&'a mut Crate);
impl<'a> StructGenerator for StructGen<'a> {
    fn begin(
        &mut self,
        name: &str,
        id_name: IdName,
        layout: Layout,
        _parent: Option<IdName>,
    ) -> Result<()> {
        writeln!(self.0.librs, "// Full name: {id_name}")?;
        writeln!(self.0.librs, "// Unaligned size: 0x{:X}", layout.size)?;
        writeln!(self.0.librs, "// Alignment: 0x{:X}", layout.alignment)?;
        writeln!(self.0.librs, "memflex::makestruct! {{")?;
        // TODO: Implement zeroed from bytemuck, maybe reexport Zeroed trait in memflex?
        writeln!(self.0.librs, "\tpub struct {name} {{")?;

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
        writeln!(self.0.librs, "\t}}\n}}\n")?;

        Ok(())
    }
}

struct PackageGen(Crate);
impl PackageGenerator for PackageGen {
    fn add_enum<'new>(&'new mut self) -> Result<Box<dyn crate::EnumGenerator + 'new>> {
        Ok(Box::new(EnumGen(&mut self.0)))
    }

    fn add_struct<'new>(&'new mut self) -> Result<Box<dyn crate::StructGenerator + 'new>> {
        Ok(Box::new(StructGen(&mut self.0)))
    }
}

pub struct RustSdkGenerator {
    crates: PathBuf,
    workspace: Crate,
}

impl SdkGenerator for RustSdkGenerator {
    fn begin_package<'sdk: 'pkg, 'pkg>(
        &'sdk mut self,
        name: &str,
    ) -> Result<Box<dyn PackageGenerator + 'pkg>> {
        // Add to workspace
        {
            writeln!(self.workspace.toml, "[workspace.dependencies.{name}]")?;
            writeln!(self.workspace.toml, "path = \"crates/{name}\"")?;
            writeln!(self.workspace.toml, "[dependencies.{name}]")?;
            writeln!(self.workspace.toml, "workspace = true\noptional = true")?;
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
            writeln!(package.toml, "memflex.workspace = true")?;
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

        Ok(Box::new(PackageGen(package)))
    }

    fn new(path: impl AsRef<Path>) -> Result<Self>
    where
        Self: Sized,
    {
        let path = path.as_ref().join("usdk");
        fs::create_dir_all(&path)?;

        let mut toml = open_file(path.join("Cargo.toml"))?;
        let librs = open_file(path.join("lib.rs"))?;

        const WORKSPACE_DEF: &str = include_str!("../../workspace.toml");
        writeln!(toml, "{WORKSPACE_DEF}")?;

        Ok(Self {
            crates: path.join("crates"),
            workspace: Crate { toml, librs },
        })
    }
}

fn open_file(path: impl AsRef<Path>) -> Result<File> {
    OpenOptions::new().create(true).write(true).open(path)
}

struct Crate {
    toml: File,
    librs: File,
}
