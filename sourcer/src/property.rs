use derive_more::Display;

/// Fully qualified name
#[derive(Debug, Display, PartialEq, Eq, Hash, Clone)]
pub struct IdName(pub String);

impl From<String> for IdName {
    #[inline]
    fn from(value: String) -> Self {
        Self(value)
    }
}

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
#[rustfmt::skip]
pub enum PropertyType {
    Int8, Int16, Int32, Int64,
    UInt8, UInt16, UInt32, UInt64,
    Float32, Float64,
    Bool,    
    Vector(Box<PropertyType>), // TArray
    Map {
        key: Box<PropertyType>,
        value: Box<PropertyType>,
    }, // TMap
    Set(Box<PropertyType>), // TSet
    Object(Box<PropertyType>), Class(Box<PropertyType>), // Pointer to object instance
    Enum(Box<PropertyType>), // Enum field
    Name, // FName
    String, // FString
    Text, // FText
    Struct(IdName), // Struct field
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
