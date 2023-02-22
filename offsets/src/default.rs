use super::{
    OfFField, OfFName, OfFNameEntry, OfFProperty, OfFUObjectItem, OfUEnum, OfUField, OfUObject,
    OfUStruct, Offsets,
};

pub const DEFAULT: Offsets = Offsets {
    stride: 2,
    fuobjectitem: OfFUObjectItem { size: 0x18 },
    uobject: OfUObject {
        index: 0xC,
        class: 0x10,
        name: 0x18,
        outer: 0x20,
        size: 0x28,
    },
    fname: OfFName {
        size: 0x8,
        index: 0x0,
    },
    fnameentry: OfFNameEntry {
        header: 0x0,
        data: 0x2,
        wide_bit: 0x0,
        len_bit: 0x6,
    },
    ufield: OfUField { next: 0x28 },
    ustruct: OfUStruct {
        super_struct: 0x40,
        children: 0x48,
        children_props: 0x50,
        props_size: 0x58,
    },
    uenum: OfUEnum { names: 0x40 },
    ffield: OfFField {
        class: 0x8,
        next: 0x20,
        name: 0x28,
    },
    fproperty: OfFProperty {
        array_dim: 0x34,
        element_size: 0x38,
        prop_flags: 0x40,
        offset: 0x4C,
        size: 0x78,
    },
};
