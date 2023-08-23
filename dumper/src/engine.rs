use crate::{
    utils::{strip_package_name, Fqn},
    Config, State,
};
use anyhow::Result;
use memflex::sizeof;
use std::iter::successors;

macro_rules! mkfn {
    ($field:ident $kind:ident, = $offset:expr) => {
        #[allow(clippy::redundant_closure_call)]
        pub fn $field(&self) -> Result<$kind> {
            Ok(State::get()
                .proc
                .read(self.0 + ($offset)(&State::get().config))?)
        }
    };

    ($field:ident $kind:ident, @ $offset:expr) => {
        #[allow(clippy::redundant_closure_call)]
        pub fn $field(&self) -> $kind {
            $kind(self.0 + ($offset)(&State::get().config))
        }
    };
}

macro_rules! mkptr {
    {
        $(
            $ptr:ident {
                $(
                    $field:ident $kind:ident: $tt:tt $offset:expr
                ),* $(,)?
            }
        )*
    } => {
        $(
            #[derive(Clone, Copy, PartialEq, Eq, Hash)]
            #[repr(transparent)]
            pub struct $ptr(pub usize);

            #[allow(dead_code)]
            impl $ptr {
                #[inline]
                pub fn is_null(&self) -> bool {
                    self.0 == 0
                }

                #[inline]
                pub fn non_null(&self) -> Option<Self> {
                    if self.0 == 0 {
                        None
                    } else {
                        Some(*self)
                    }
                }

                $(
                    mkfn!($field $kind, $tt $offset);
                )*

                pub fn cast<T>(&self) -> T {
                    unsafe {
                        std::mem::transmute_copy::<_, T>(self)
                    }
                }
            }

            impl std::fmt::Debug for $ptr {
                fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                    write!(f, "{:#X}", self.0)
                }
            }
        )*

    };
}

type C = Config;

mkptr! {
    UObjectPtr {
        index u32: = |c: &C| c.uobject.index,
        class UClassPtr: = |c: &C| c.uobject.class,
        name FNamePtr: @ |c: &C| c.uobject.name,
        outer UObjectPtr: = |c: &C| c.uobject.outer
    }
    FPropertyPtr {
        array_dim u32: = |c: &C| c.fproperty.array_dim,
        element_size u32: = |c: &C| c.fproperty.element_size,
        flags PropertyFlags: = |c: &C| c.fproperty.flags,
        offset u32: = |c: &C| c.fproperty.offset,
        size u32: = |c: &C| c.fproperty.size,
    }
    FBoolProperty {
        vars BoolVars: = |c: &C| c.fproperty.size,
    }
    UFunctionPtr {
        flags FunctionFlags: = |c: &C| c.ufunction.flags,
        func usize: = |c: &C| c.ufunction.func,
    }
    FFieldPtr {
        class FFieldClassPtr: = |c: &C| c.ffield.class,
        name FNamePtr: @ |c: &C| c.ffield.name,
        next FFieldPtr: = |c: &C| c.ffield.next
    }
    FFieldClassPtr {
        name FNamePtr: @ |_| 0
    }
    UStructPtr {
        super_struct UStructPtr: = |c: &C| c.ustruct.super_struct,
        // children UFieldPtr: = |c: &C| c.ustruct.children,
        children_props FFieldPtr: = |c: &C| c.ustruct.children_props,
        props_size u32: = |c: &C| c.ustruct.props_size,
        min_align u32: = |c: &C| c.ustruct.props_size + sizeof!(u32),
    }
    UClassPtr {}
    UEnumPtr { }
    FNamePtr {}
}

impl UEnumPtr {
    pub fn names(&self) -> Result<TArray> {
        let State {
            proc,
            config: offsets,
            ..
        } = State::get();
        Ok(proc.read::<TArray>(self.0 + offsets.uenum.names)?)
    }
}

#[derive(Debug)]
#[repr(C)]
pub struct BoolVars {
    pub field_size: u8,
    pub byte_offset: u8,
    pub byte_mask: u8,
    pub field_mask: u8,
}

#[derive(Debug)]
#[repr(C)]
pub struct FunctionVars {
    pub flags: FunctionFlags,
}

memflex::bitflags! {
    #[derive(Debug)]
    pub struct FunctionFlags : u32 {
        const Final= 0x00000001;
        const RequiredAPI = 0x00000002;
        const BlueprintAuthorityOnly = 0x00000004;
        const BlueprintCosmetic	= 0x00000008;
        const Net = 0x00000040;
        const NetReliable = 0x00000080;
        const NetRequest = 0x00000100;
        const Exec = 0x00000200;
        const Native = 0x00000400;
        const Event = 0x00000800;
        const NetResponse = 0x00001000;
        const Static = 0x00002000;
        const NetMulticast = 0x00004000;
        const UbergraphFunction	= 0x00008000;
        const MulticastDelegate	= 0x00010000;
        const Public = 0x00020000;
        const Private = 0x00040000;
        const Protected = 0x00080000;
        const Delegate = 0x00100000;
        const NetServer = 0x00200000;
        const HasOutParms = 0x00400000;
        const HasDefaults = 0x00800000;
        const NetClient = 0x01000000;
        const DLLImport = 0x02000000;
        const BlueprintCallable	= 0x04000000;
        const BlueprintEvent = 0x08000000;
        const BlueprintPure = 0x10000000;
        const EditorOnly = 0x20000000;
        const Const = 0x40000000;
        const NetValidate = 0x80000000;
    }

    #[derive(Debug, Clone, Copy)]
    pub struct PropertyFlags : u64 {
        const Edit = 0x0000000000000001;
        const ConstParm = 0x0000000000000002;
        const BlueprintVisible = 0x0000000000000004;
        const ExportObject = 0x0000000000000008;
        const BlueprintReadOnly = 0x0000000000000010;
        const Net = 0x0000000000000020;
        const EditFixedSize = 0x0000000000000040;
        const Parm = 0x0000000000000080;
        const OutParm = 0x0000000000000100;
        const ZeroConstructor = 0x0000000000000200;
        const ReturnParm = 0x0000000000000400;
        const DisableEditOnTemplate = 0x0000000000000800;
        const NonNullable = 0x0000000000001000;
        const Transient = 0x0000000000002000;
        const Config = 0x0000000000004000;
        const RequiredParm = 0x0000000000008000;
        const DisableEditOnInstance = 0x0000000000010000;
        const EditConst = 0x0000000000020000;
        const GlobalConfig = 0x0000000000040000;
        const InstancedReference = 0x0000000000080000;
        const DuplicateTransient = 0x0000000000200000;
        const SaveGame = 0x0000000001000000;
        const NoClear = 0x0000000002000000;
        const ReferenceParm = 0x0000000008000000;
        const BlueprintAssignable = 0x0000000010000000;
        const Deprecated = 0x0000000020000000;
        const IsPlainOldData = 0x0000000040000000;
        const RepSkip = 0x0000000080000000;
        const RepNotify = 0x0000000100000000;
        const Interp = 0x0000000200000000;
        const NonTransactional = 0x0000000400000000;
        const EditorOnly = 0x0000000800000000;
        const NoDestructor = 0x0000001000000000;
        const AutoWeak = 0x0000004000000000;
        const ContainsInstancedReference = 0x0000008000000000;
        const AssetRegistrySearchable = 0x0000010000000000;
        const SimpleDisplay = 0x0000020000000000;
        const AdvancedDisplay = 0x0000040000000000;
        const Protected = 0x0000080000000000;
        const BlueprintCallable = 0x0000100000000000;
        const BlueprintAuthorityOnly = 0x0000200000000000;
        const TextExportTransient = 0x0000400000000000;
        const NonPIEDuplicateTransient = 0x0000800000000000;
        const ExposeOnSpawn = 0x0001000000000000;
        const PersistentInstance = 0x0002000000000000;
        const UObjectWrapper = 0x0004000000000000;
        const HasGetValueTypeHash = 0x0008000000000000;
        const NativeAccessSpecifierPublic = 0x0010000000000000;
        const NativeAccessSpecifierProtected = 0x0020000000000000;
        const NativeAccessSpecifierPrivate = 0x0040000000000000;
        const SkipSerialization = 0x0080000000000000;
    }
}

pub struct TArray {
    pub ptr: usize,
    pub len: u32,
}

impl TArray {
    pub fn iter<T>(&self) -> impl Iterator<Item = Result<T>> + '_ {
        (0..self.len as usize).map(|i| {
            State::get()
                .proc
                .read::<T>(self.ptr + i * sizeof!(T))
                .map_err(Into::into)
        })
    }
}

impl FNamePtr {
    pub fn read(&self) -> Result<u32> {
        Ok(State::get().proc.read(self.0)?)
    }

    pub(crate) fn get(&self) -> Result<&'static str> {
        State::get().get_name(self.read()?)
    }
}

impl UObjectPtr {
    pub(crate) fn fqn(&self) -> Result<Fqn> {
        let outer = self.outer()?;
        anyhow::ensure!(!outer.is_null(), "Can't get FQN of a package");

        let package = strip_package_name(outer.name().get()?);
        let name = self.name().get()?;
        Ok(Fqn::from_package_name(package, name))
    }

    pub(crate) fn is_a(&self, fqn: Fqn) -> Result<bool> {
        let descendant = self
            .inheritance()?
            .any(|v| v.cast::<UObjectPtr>().fqn().unwrap() == fqn);

        Ok(descendant)
    }

    pub(crate) fn inheritance(&self) -> Result<impl Iterator<Item = UStructPtr>> {
        Ok(successors(Some(self.class()?.cast::<UStructPtr>()), |s| {
            s.super_struct().unwrap().non_null()
        }))
    }
}
