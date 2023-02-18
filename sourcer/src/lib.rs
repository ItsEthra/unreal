use std::{io::Result, path::Path};

pub mod lang;

#[rustfmt::skip]
pub enum PropertyType {
    I8, I16, I32, I64,
    U8, U16, U32, U64,
    F32, F64,
    Bool,    
}

pub trait EnumGenerator {
    fn begin(&mut self, _name: &str, _full_name: &str) -> Result<()> {
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
