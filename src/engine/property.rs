use crate::offsets::Offsets;
use std::marker::PhantomData;

bitflags::bitflags! {
    struct EPropertyFlags : u64 {
        const NONE = 0;
        const EDIT = 0x1;
        const CONST_PARM = 0x2;
        const BLUEPRINT_VISIBLE = 0x4;
        const EXPORT_OBJECT = 0x8;
        const BLUEPRINT_READ_ONLY = 0x10;
        const NET = 0x20;
        const EDIT_FIXED_SIZE = 0x40;
        const PARM = 0x80;
        const OUT_PARM = 0x100;
        const ZERO_CONSTRUCTOR = 0x200;
        const RETURN_PARM = 0x400;
        const DISABLE_EDIT_ON_TEMPLATE = 0x800;
        const NON_NULLABLE = 0x1000;
        const TRANSIENT = 0x2000;
        const CONFIG = 0x4000;
        const DISABLE_EDIT_ON_INSTANCE = 0x10000;
        const EDIT_CONST = 0x20000;
        const GLOBAL_CONFIG = 0x40000;
        const INSTANCED_REFERENCE = 0x80000;
        const DUPLICATE_TRANSIENT = 0x200000;
        const SAVE_GAME = 0x1000000;
        const NO_CLEAR = 0x2000000;
        const REFERENCE_PARM = 0x8000000;
        const BLUEPRINT_ASSIGNABLE = 0x10000000;
        const DEPRECATED = 0x20000000;
        const IS_PLAIN_OLD_DATA = 0x40000000;
        const REP_SKIP = 0x80000000;
        const REP_NOTIFY = 0x100000000;
        const INTERP = 0x200000000;
        const NON_TRANSACTIONAL = 0x400000000;
        const EDITOR_ONLY = 0x800000000;
        const NO_DESTRUCTOR = 0x1000000000;
        const AUTO_WEAK = 0x4000000000;
        const CONTAINS_INSTANCED_REFERENCE = 0x8000000000;
        const ASSET_REGISTRY_SEARCHABLE = 0x10000000000;
        const SIMPLE_DISPLAY = 0x20000000000;
        const ADVANCED_DISPLAY = 0x40000000000;
        const PROTECTED = 0x80000000000;
        const BLUEPRINT_CALLABLE = 0x100000000000;
        const BLUEPRINT_AUTHORITY_ONLY = 0x200000000000;
        const TEXT_EXPORT_TRANSIENT = 0x400000000000;
        const NON_PIEDUPLICATE_TRANSIENT = 0x800000000000;
        const EXPOSE_ON_SPAWN = 0x1000000000000;
        const PERSISTENT_INSTANCE = 0x2000000000000;
        const UOBJECT_WRAPPER = 0x4000000000000;
        const HAS_GET_VALUE_TYPE_HASH = 0x8000000000000;
        const NATIVE_ACCESS_SPECIFIER_PUBLIC = 0x10000000000000;
        const NATIVE_ACCESS_SPECIFIER_PROTECTED = 0x20000000000000;
        const NATIVE_ACCESS_SPECIFIER_PRIVATE = 0x40000000000000;
        const SKIP_SERIALIZATION = 0x80000000000000;
    }
}

pub struct FProperty<O: Offsets>(PhantomData<O>);

pub struct UProperty<O: Offsets>(PhantomData<O>);
