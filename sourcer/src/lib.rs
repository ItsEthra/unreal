use std::{io::Result, path::Path};

pub mod lang;

mod property;
pub use property::*;
mod deps;
pub use deps::*;

pub trait EnumGenerator {
    fn begin(&mut self, _name: &str, _id_name: IdName, _min_max: Option<(i64, i64)>) -> Result<()> {
        Ok(())
    }

    fn add_variant(&mut self, variant: &str, value: i64) -> Result<()>;

    fn end(&mut self) -> Result<()> {
        Ok(())
    }
}

pub trait PackageGenerator {
    fn begin(&mut self) -> Result<()> {
        Ok(())
    }

    fn add_enum<'new>(&'new mut self) -> Result<Box<dyn EnumGenerator + 'new>>;

    fn end(&mut self) -> Result<()> {
        Ok(())
    }
}

pub trait SdkGenerator {
    fn new(path: impl AsRef<Path>) -> Result<Self>
    where
        Self: Sized;

    fn begin_package<'sdk: 'pkg, 'pkg>(
        &'sdk mut self,
        name: &str,
    ) -> Result<Box<dyn PackageGenerator + 'pkg>>;

    fn end(&mut self) -> Result<()> {
        Ok(())
    }
}
