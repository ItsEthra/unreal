use crate::{EnumGenerator, PackageGenerator, SdkGenerator, StructGenerator};

pub struct DummySdkGenerator;

impl SdkGenerator for DummySdkGenerator {
    fn new(_: impl AsRef<std::path::Path>, _: &'static offsets::Offsets) -> eyre::Result<Self>
    where
        Self: Sized,
    {
        Ok(Self)
    }

    fn begin_package<'pkg>(
        &mut self,
        _: &str,
        _: &std::rc::Rc<crate::PackageRegistry>,
    ) -> eyre::Result<Box<dyn crate::PackageGenerator + 'pkg>> {
        Ok(Box::new(Self))
    }
}

impl PackageGenerator for DummySdkGenerator {
    fn add_enum<'new>(&'new mut self) -> eyre::Result<Box<dyn crate::EnumGenerator + 'new>> {
        Ok(Box::new(Self))
    }

    fn add_struct<'new>(&'new mut self) -> eyre::Result<Box<dyn crate::StructGenerator + 'new>> {
        Ok(Box::new(Self))
    }
}

impl EnumGenerator for DummySdkGenerator {
    fn begin(&mut self, _name: &str, _id_name: crate::IdName) -> eyre::Result<()> {
        Ok(())
    }

    fn append_variant(&mut self, _variant: &str, _value: i64) -> eyre::Result<()> {
        Ok(())
    }
}

impl StructGenerator for DummySdkGenerator {
    fn begin(
        &mut self,
        _: &str,
        _: crate::IdName,
        _: crate::Layout,
        _: Option<crate::IdName>,
    ) -> eyre::Result<()> {
        Ok(())
    }

    fn append_field(
        &mut self,
        _: &str,
        _: crate::PropertyType,
        _: usize,
        _: usize,
    ) -> eyre::Result<()> {
        Ok(())
    }
}
