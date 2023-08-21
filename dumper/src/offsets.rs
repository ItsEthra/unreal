pub struct Offsets {
    pub stride: u32,
    pub fuobject_item_size: usize,

    pub uobject: OfUObject,
    pub ufield: OfUField,
    pub ustruct: OfUStruct,
    pub uenum: OfUEnum,
    pub ffield: OfFField,
    pub fproperty: OfFProperty,
}

pub struct OfUObject {
    pub index: usize,
    pub class: usize,
    pub name: usize,
    pub outer: usize,
}

pub struct OfUField {
    pub next: usize,
}

pub struct OfUStruct {
    pub super_struct: usize,
    // pub children: usize,
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
    pub flags: usize,
    pub offset: usize,
    pub size: usize,
}

impl Offsets {
    pub const DEFAULT: Self = DEFAULT;
}

const DEFAULT: Offsets = Offsets {
    stride: 2,
    fuobject_item_size: 24,
    uobject: OfUObject {
        index: 0xC,
        class: 0x10,
        name: 0x18,
        outer: 0x20,
    },
    ufield: OfUField { next: 0x28 },
    ustruct: OfUStruct {
        super_struct: 0x40,
        // children: 0x48,
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
        array_dim: 0x38,
        element_size: 0x3C,
        flags: 0x40,
        offset: 0x4C,
        size: 0x78,
    },
};
