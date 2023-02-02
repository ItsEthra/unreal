mod default;

mod presets {
    pub use super::default::*;
}

pub trait OfFNameEntryHeader {
    const WIDE_BIT: usize;
    const LEN_BIT: usize;
}

pub trait OfFNameEntry {
    // Field: FNameEntryHeader
    const HEADER: usize;
    // Size of the header
    const HEADER_SIZE: usize;
}

pub trait OfUObject {
    // Field: i32
    const INDEX: usize;
    // Field: UClass*
    const CLASS: usize;
    // Field: FName
    const NAME: usize;
    // Field: UObject*
    const OUTTER: usize;
}

pub trait OfUStruct {
    // Field: UStruct*
    const SUPER: usize;
    // Field: UField*
    const CHILDREN: usize;
    // Field: FField*
    const CHILDREN_PROPS: usize;
    // Field: FField*
    const PROPS_SIZE: usize;
}

pub trait OfUFunction {
    // Field: EFunctionFlags
    const FLAGS: usize;
    const NATIVE: usize;
}

pub trait OfFField {
    // Field: FFieldClass*
    const CLASS: usize;
    // Field: FField*
    const NEXT: usize;
    // Field: FName
    const NAME: usize;
}

pub trait OfUField {
    // Field: UField*
    const NEXT: usize;
}

pub trait OfFProperty {
    // Field: int32
    const ARRAY_DIM: usize;
    // Field: int32
    const ELEMENT_SIZE: usize;
    // Field: EPropertyFlags
    const FLAGS: usize;
    // Field: int32
    const OFFSET: usize;
    // ?
    const SIZE: usize;
}

pub trait OfUProperty {
    // Field: int32
    const ARRAY_DIM: usize;
    // Field: int32
    const ELEMENT_SIZE: usize;
    // Field: EPropertyFlags
    const FLAGS: usize;
    // Field: int32
    const OFFSET: usize;
    const SIZE: usize;
}

pub trait Offsets {
    type NameEntry: OfFNameEntry;
    type NameEntryHeader: OfFNameEntryHeader;
    type UObject: OfUObject;
    type UField: OfUField;
    type UStruct: OfUStruct;
}
