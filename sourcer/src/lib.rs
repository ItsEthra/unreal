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
    fn begin(&mut self) -> Result<()> {
        Ok(())
    }

    fn add_variant(&mut self, variant: &str, value: i64) -> Result<()>;

    fn end(&mut self) -> Result<()> {
        Ok(())
    }
}

pub trait PackageGenerator<'a> {
    fn add_enum(&'a mut self, name: &str) -> Result<Box<dyn EnumGenerator + 'a>>;

    fn end(&mut self) {}
}

pub trait SdkGenerator {
    fn new(path: &Path) -> Result<Self>
    where
        Self: Sized;

    fn begin_package<'a>(&'a mut self, name: &str) -> Result<Box<dyn PackageGenerator + 'a>>;

    fn end(&mut self) {}
}
