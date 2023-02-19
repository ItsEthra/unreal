use derive_more::Display;
use std::rc::Rc;

#[derive(Debug, Default, Clone, Copy)]
pub struct Layout {
    pub size: usize,
    pub alignment: usize,
}

impl Layout {
    pub fn align(&self) -> usize {
        if self.alignment == 0 {
            return 0;
        }

        if self.size % self.alignment != 0 {
            self.size + (self.alignment - self.size % self.alignment)
        } else {
            self.size
        }
    }
}

#[test]
fn test_align() {
    let layout = Layout {
        size: 14,
        alignment: 8,
    };
    assert_eq!(layout.align(), 16);
}

/// Fully qualified name
#[derive(Debug, Display, PartialEq, Eq, Hash, Clone)]
pub struct IdName(pub Rc<String>);

impl From<String> for IdName {
    #[inline]
    fn from(value: String) -> Self {
        Self(value.into())
    }
}

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
#[rustfmt::skip]
pub enum PropertyType {
    Int8, Int16, Int32, Int64,
    UInt8, UInt16, UInt32, UInt64,
    Float32, Float64,
    Bool,    
    Array {
        ty: Box<PropertyType>,
        size: u32,
    }, // Static array
    Vector(Box<PropertyType>), // TArray
    Map {
        key: Box<PropertyType>,
        value: Box<PropertyType>,
    }, // TMap
    Set(Box<PropertyType>), // TSet
    ClassPtr(Box<PropertyType>), // Pointer to object instance can be a ObjectProperty or ClassProperty
    Name, // FName
    String, // FString
    Text, // FText
    InlineClass(IdName), // Struct field can be a StructProperty or be inside other PropertyTypes
    InlineEnum(IdName), // Enum field
}

impl PropertyType {
    pub fn is_primitive(&self) -> bool {
        match self {
            Self::Bool
            | Self::Int8
            | Self::Int16
            | Self::Int32
            | Self::Int64
            | Self::UInt8
            | Self::UInt16
            | Self::UInt32
            | Self::UInt64
            | Self::Float32
            | Self::Float64 => true,
            _ => false,
        }
    }
}
