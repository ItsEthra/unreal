use eyre::Result;
use offsets::Offsets;
use std::{path::Path, rc::Rc};

pub mod lang;

mod property;
pub use property::*;
mod deps;
pub use deps::*;

pub trait StructGenerator {
    fn begin(
        &mut self,
        name: &str,
        id_name: IdName,
        layout: Layout,
        parent: Option<IdName>,
    ) -> Result<()>;

    fn append_field(
        &mut self,
        field_name: &str,
        field_ty: PropertyType,
        elem_size: usize,
        offset: usize,
    ) -> Result<()>;

    fn end(&mut self) -> Result<()> {
        Ok(())
    }
}

pub trait EnumGenerator {
    fn begin(&mut self, name: &str, id_name: IdName, min_max: Option<(i64, i64)>) -> Result<()>;

    fn append_variant(&mut self, variant: &str, value: i64) -> Result<()>;

    fn end(&mut self) -> Result<()> {
        Ok(())
    }
}

pub trait PackageGenerator {
    fn add_enum<'new>(&'new mut self) -> Result<Box<dyn EnumGenerator + 'new>>;
    fn add_struct<'new>(&'new mut self) -> Result<Box<dyn StructGenerator + 'new>>;

    fn end(&mut self) -> Result<()> {
        Ok(())
    }
}

pub trait SdkGenerator {
    fn new(path: impl AsRef<Path>, offsets: &'static Offsets) -> Result<Self>
    where
        Self: Sized;

    fn begin_package<'pkg>(
        &mut self,
        name: &str,
        registry: &Rc<ClassRegistry>,
    ) -> Result<Box<dyn PackageGenerator + 'pkg>>;

    fn end(&mut self) -> Result<()> {
        Ok(())
    }
}
