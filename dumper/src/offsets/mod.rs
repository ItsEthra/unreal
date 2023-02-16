mod default;
pub use default::*;

pub struct OfFUObjectItem {
    pub size: usize,
}

pub struct OfFName {
    pub size: usize,
    pub number: usize,
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

pub struct Offsets {
    pub stride: usize,

    pub fuobject_item: OfFUObjectItem,
    pub uobject: OfUObject,
    pub fname: OfFName,
    pub fnameentry: OfFNameEntry,
}
