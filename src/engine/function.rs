use crate::offsets::Offsets;
use std::marker::PhantomData;

bitflags::bitflags! {
    struct EFunctionFlags : u32 {
        const NONE = 0x0;
        const FINAL = 0x1;
        const REQUIRED_API = 0x2;
        const BLUEPRINT_AUTHORITY_ONLY= 0x4;
        const BLUEPRINT_COSMETIC = 0x8;
        const NET = 0x40;
        const NET_RELIABLE = 0x80;
        const NET_REQUEST = 0x100;
        const EXEC = 0x200;
        const NATIVE = 0x400;
        const EVENT = 0x800;
        const NET_RESPONSE = 0x1000;
        const STATIC = 0x2000;
        const NET_MULTICAST = 0x4000;
        const UBERGRAPH_FUNCTION = 0x8000;
        const MULTICAST_DELEGATE = 0x10000;
        const PUBLIC = 0x20000;
        const PRIVATE = 0x40000;
        const PROTECTED = 0x80000;
        const DELEGATE = 0x100000;
        const NET_SERVER = 0x200000;
        const HAS_OUT_PARMS = 0x400000;
        const HAS_DEFAULTS = 0x800000;
        const NET_CLIENT = 0x1000000;
        const DLLIMPORT = 0x2000000;
        const BLUEPRINT_CALLABLE = 0x4000000;
        const BLUEPRINT_EVENT = 0x8000000;
        const BLUEPRINT_PURE = 0x10000000;
        const EDITOR_ONLY = 0x20000000;
        const CONST = 0x40000000;
        const NET_VALIDATE = 0x80000000;
        const ALL_FLAGS = 0xFFFFFFFF;
    }
}

pub struct UFunction<O: Offsets>(PhantomData<O>);
