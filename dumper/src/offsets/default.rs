use super::{OfFName, OfFNameEntry, OfFUObjectItem, OfUObject, Offsets};

pub const DEFAULT: Offsets = Offsets {
    stride: 4,
    fuobject_item: OfFUObjectItem { size: 0x18 },
    uobject: OfUObject {
        index: 0x8,
        class: 0x10,
        name: 0x18,
        outer: 0x20,
        size: 0x28,
    },
    fname: OfFName {
        size: 0x8,
        number: 0x4,
    },
    fnameentry: OfFNameEntry {
        header: 0x0,
        name: 0x2,
        wide_bit: 0x0,
        len_bit: 0x6,
    },
};