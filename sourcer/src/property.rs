use derive_more::Display;

/// Fully qualified name
#[derive(Debug, Display, PartialEq, Eq, Hash, Clone)]
pub struct IdName(pub String);

/// Extra data for some property types.
#[derive(Debug, PartialEq, Eq, Hash)]
pub enum PropertyData {
    Array { ty: IdName },
    Map { key: IdName, value: IdName },
    Set { ty: IdName },
    Object { ty: IdName },
    Class { ty: IdName },
    Enum { ty: IdName },
    Struct { ty: IdName },
}

#[rustfmt::skip]
pub enum PropertyType {
    Int8, Int16, Int32, Int64,
    UInt8, UInt16, UInt32, UInt64,
    Float32, Float64,
    Bool,    
    Array, // TArray
    Map, // TMap
    Set, // TSet
    Object, Class, // Pointer to object instance
    Enum, // Enum field
    Name, // FName
    String, // FString
    Text, // FText
    Struct, // Struct field
}

impl PropertyType {
    pub fn from_str(s: &str) -> Option<Self> {
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
            "ObjectProperty" => Some(Self::Object),
            "ArrayProperty" => Some(Self::Array), // TArray
            // "FieldPathProperty" => None,
            "ClassProperty" => Some(Self::Class),
            // "ClassPtrProperty" => Some(Self::ClassPtr),
            // "DelegateProperty" => None,
            "EnumProperty" => Some(Self::Enum),
            // "InterfaceProperty" => None,
            // "LazyObjectProperty" => None,
            "MapProperty" => Some(Self::Map),
            "NameProperty" => Some(Self::Name),
            "SetProperty" => Some(Self::Set),
            // "SoftClassProperty" => None,
            // "SoftObjectProperty" => None,
            "StrProperty" => Some(Self::String),
            "StructProperty" => Some(Self::Struct),
            "TextProperty" => Some(Self::Text),
            // "WeakObjectProperty" => None,
            _ => None,
        }
    }
}
