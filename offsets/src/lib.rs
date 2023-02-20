mod default;
pub use default::*;

pub struct OfFUObjectItem {
    pub size: usize,
}

pub struct OfFName {
    pub size: usize,
    pub index: usize,
}

pub struct OfUObject {
    pub size: usize,
    pub index: usize,
    pub class: usize,
    pub name: usize,
    pub outer: usize,
}

pub struct OfFNameEntry {
    pub header: usize,
    pub name: usize,

    pub wide_bit: usize,
    pub len_bit: usize,
}

pub struct OfUField {
    pub next: usize,
}

pub struct OfUStruct {
    pub super_struct: usize,
    pub children: usize,
    pub children_props: usize,
    pub props_size: usize,
}

pub struct OfUEnum {
    pub names: usize,
}

pub struct OfFField {
    pub class: usize,
    pub next: usize,
    pub name: usize,
}

pub struct OfFProperty {
    pub array_dim: usize,
    pub element_size: usize,
    pub prop_flags: usize,
    pub offset: usize,
    pub size: usize,
}

pub struct Offsets {
    pub stride: usize,

    pub fuobjectitem: OfFUObjectItem,
    pub uobject: OfUObject,
    pub fname: OfFName,
    pub fnameentry: OfFNameEntry,
    pub ufield: OfUField,
    pub ustruct: OfUStruct,
    pub uenum: OfUEnum,
    pub ffield: OfFField,
    pub fproperty: OfFProperty,
}
