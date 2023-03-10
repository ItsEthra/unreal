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
    Inline(IdName), // Enum field or Struct field can be a StructProperty or be inside other PropertyTypes
}

impl PropertyType {
    pub fn is_primitive(&self) -> bool {
        matches!(
            self,
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
                | Self::Float64
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BitMask {
    Full,
    Partial { len: u32, offset: u32 },
}

impl BitMask {
    pub fn determinate(mask: u8) -> Self {
        if mask == u8::MAX {
            Self::Full
        } else {
            let offset = mask.trailing_zeros();
            Self::Partial {
                len: (mask >> offset).trailing_ones(),
                offset,
            }
        }
    }
}

#[derive(Debug)]
pub struct BitField {
    pub name: String,
    pub mask: BitMask,
}

#[test]
fn test_bit_mask() {
    let mask = BitMask::determinate(0b11000);
    assert_eq!(mask, BitMask::Partial { len: 2, offset: 3 });

    let mask = BitMask::determinate(0b11111111);
    assert_eq!(mask, BitMask::Full);
}
