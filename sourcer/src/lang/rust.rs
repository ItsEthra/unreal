use crate::{EnumGenerator, PackageGenerator, SdkGenerator};
use std::{
    fs::{File, OpenOptions},
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
impl<'a> PackageGenerator<'a> for PackageGen {
    fn add_enum(&'a mut self, name: &str) -> Result<Box<dyn crate::EnumGenerator + 'a>> {
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
    fn begin_package<'a>(&'a mut self, name: &str) -> Result<Box<dyn PackageGenerator + 'a>> {
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

        let package = Crate {
            toml: open_file(crate_path.join("Cargo.toml"))?,
            librs: open_file(crate_path.join("Cargo.toml"))?,
        };

        Ok(Box::new(PackageGen(package)))
    }

    fn new(path: &Path) -> Result<Self>
    where
        Self: Sized,
    {
        let path = path.join("usdk");

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
    OpenOptions::new().create(true).open(path)
}

struct Crate {
    toml: File,
    librs: File,
}
