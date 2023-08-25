use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Config {
    pub stride: u32,
    pub process_event: u32,
    pub level_actors: Option<u32>,

    pub offsets: Option<Offsets>,

    #[serde(rename = "FUObjectItem")]
    pub fuobject_item: OfFUObjectItem,
    #[serde(rename = "UObject")]
    pub uobject: OfUObject,
    #[serde(rename = "UField")]
    pub ufield: OfUField,
    #[serde(rename = "UStruct")]
    pub ustruct: OfUStruct,
    #[serde(rename = "UEnum")]
    pub uenum: OfUEnum,
    #[serde(rename = "FField")]
    pub ffield: OfFField,
    #[serde(rename = "FProperty")]
    pub fproperty: OfFProperty,
    #[serde(rename = "UFunction")]
    pub ufunction: OfUFunction,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Offsets {
    pub names: Option<usize>,
    pub objects: Option<usize>,
    pub world: Option<usize>,
    pub engine: Option<usize>,
}

impl Default for Config {
    fn default() -> Self {
        DEFAULT
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct OfFUObjectItem {
    pub size: usize,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct OfUFunction {
    pub flags: usize,
    pub func: usize,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct OfUObject {
    pub index: usize,
    pub class: usize,
    pub name: usize,
    pub outer: usize,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct OfUField {
    pub next: usize,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct OfUStruct {
    pub super_struct: usize,
    // pub children: usize,
    pub children_props: usize,
    pub props_size: usize,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct OfUEnum {
    pub names: usize,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct OfFField {
    pub class: usize,
    pub next: usize,
    pub name: usize,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct OfFProperty {
    pub array_dim: usize,
    pub element_size: usize,
    pub flags: usize,
    pub offset: usize,
    pub size: usize,
}

const DEFAULT: Config = Config {
    stride: 2,
    process_event: 0x4D,
    level_actors: None,
    offsets: None,

    fuobject_item: OfFUObjectItem { size: 0x18 },
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
    ufunction: OfUFunction {
        flags: 0xB0,
        func: 0xB0 + 0x28,
    },
};
