use std::{io::Result, path::Path};

pub mod lang;

#[rustfmt::skip]
pub enum PropertyType {
    Int8, Int16, Int32, Int64,
    UInt8, UInt16, UInt32, UInt64,
    Float32, Float64,
    Bool,    
}

impl PropertyType {
    fn from_str(s: &str) -> Option<Self> {
        match s {
            "BoolProperty" => Some(Self::Bool),
            "FloatProperty" => Some(Self::Float32),
            "DoubleProperty" => Some(Self::Float64),
            "Int8Property" => Some(Self::Int8),
            "Int16Property" => Some(Self::Int16),
            "IntProperty" => Some(Self::Int32),
            "Int64Property" => Some(Self::Int64),
            "ByteProperty" => Some(Self::UInt8),
            "UInt16Property" => Some(Self::UInt16),
            "UInt32Property" => Some(Self::UInt32),
            "UInt64Property" => Some(Self::UInt64),
            "FieldPathProperty" => None,
            "ArrayProperty" => None,
            "ClassProperty" => None,
            "ClassPtrProperty" => None,
            "DelegateProperty" => None,
            "EnumProperty" => None,
            "InterfaceProperty" => None,
            "LazyObjectProperty" => None,
            "MapProperty" => None,
            "NameProperty" => None,
            "ObjectProperty" => None,
            "SetProperty" => None,
            "SoftClassProperty" => None,
            "SoftObjectProperty" => None,
            "StrProperty" => None,
            "StructProperty" => None,
            "TextProperty" => None,
            "WeakObjectProperty" => None,
            _ => None,
        }
    }
}

pub trait EnumGenerator {
    fn begin(&mut self, _name: &str, _full_name: &str, _min_max: Option<(i64, i64)>) -> Result<()> {
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
