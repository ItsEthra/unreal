use crate::{EnumGenerator, PackageGenerator, SdkGenerator};
use std::{
    fs::{self, File, OpenOptions},
    io::{Result, Write},
    path::{Path, PathBuf},
};

struct EnumGen<'a> {
    name: String,
    pkg: &'a mut Crate,
}

impl<'a> EnumGenerator for EnumGen<'a> {
    fn begin(&mut self) -> Result<()> {
        writeln!(self.pkg.librs, "memflex::bitflags! {{")?;
        writeln!(self.pkg.librs, "\t#[repr(transparent)]")?;
        writeln!(self.pkg.librs, "\tpub struct {} : u32 {{", self.name)?;

        Ok(())
    }

    fn add_variant(&mut self, variant: &str, value: i64) -> Result<()> {
        writeln!(self.pkg.librs, "\t\tconst {variant} = {value};")?;

        Ok(())
    }

    fn end(&mut self) -> Result<()> {
        writeln!(self.pkg.librs, "\t}}\n}}")?;

        Ok(())
    }
}

struct PackageGen(Crate);
impl PackageGenerator for PackageGen {
    fn add_enum<'new>(&'new mut self, name: &str) -> Result<Box<dyn crate::EnumGenerator + 'new>> {
        Ok(Box::new(EnumGen {
            name: name.to_owned(),
            pkg: &mut self.0,
        }))
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
