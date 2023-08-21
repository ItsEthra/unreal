use crate::{
    utils::{strip_package_name, Fqn},
    Offsets, State,
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
                .read(self.0 + ($offset)(&State::get().offsets))?)
        }
    };

    ($field:ident $kind:ident, @ $offset:expr) => {
        #[allow(clippy::redundant_closure_call)]
        pub fn $field(&self) -> $kind {
            $kind(self.0 + ($offset)(&State::get().offsets))
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

type O = Offsets;

mkptr! {
    UObjectPtr {
        index u32: = |o: &O| o.uobject.index,
        class UClassPtr: = |o: &O| o.uobject.class,
        name FNamePtr: @ |o: &O| o.uobject.name,
        outer UObjectPtr: = |o: &O| o.uobject.outer
    }
    FPropertyPtr {
        array_dim u32: = |o: &O| o.fproperty.array_dim,
        element_size u32: = |o: &O| o.fproperty.element_size,
        flags u32: = |o: &O| o.fproperty.flags,
        offset u32: = |o: &O| o.fproperty.offset,
        size u32: = |o: &O| o.fproperty.size,
    }
    FBoolProperty {
        vars BoolVars: = |o: &O| o.fproperty.size,
    }
    FFieldPtr {
        class FFieldClassPtr: = |o: &O| o.ffield.class,
        name FNamePtr: @ |o: &O| o.ffield.name,
        next FFieldPtr: = |o: &O| o.ffield.next
    }
    FFieldClassPtr {
        name FNamePtr: @ |_| 0
    }
    UStructPtr {
        super_struct UStructPtr: = |o: &O| o.ustruct.super_struct,
        // children UFieldPtr: = |o: &O| o.ustruct.children,
        children_props FFieldPtr: = |o: &O| o.ustruct.children_props,
        props_size u32: = |o: &O| o.ustruct.props_size,
        min_align u32: = |o: &O| o.ustruct.props_size + sizeof!(u32),
    }
    UClassPtr {}
    UEnumPtr { }
    FNamePtr {}
}

#[derive(Debug)]
#[repr(C)]
pub struct BoolVars {
    pub field_size: u8,
    pub byte_offset: u8,
    pub byte_mask: u8,
    pub field_mask: u8,
}

impl UEnumPtr {
    pub fn names(&self) -> Result<TArray> {
        let State { proc, offsets, .. } = State::get();
        Ok(proc.read::<TArray>(self.0 + offsets.uenum.names)?)
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
