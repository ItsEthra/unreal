mod default;

mod presets {
    pub use super::default::*;
}

pub trait OfFNameEntry {
    // Field: FNameEntryHeader
    const HEADER: usize;

    const WIDE_BIT: usize;
    const LEN_BIT: usize;
}

pub trait OfUObject {
    // Field: i32
    const INDEX: usize;
    // Field: UClass*
    const CLASS: usize;
    // Field: FName
    const NAME: usize;
    // Field: UObject*
    const OUTER: usize;
}

pub trait OfUStruct {
    // Field: UStruct*
    const SUPER: usize;
    // Field: UField*
    const CHILDREN: usize;
    // Field: FField*
    const CHILDREN_PROPS: usize;
    // Field: FField*
    const PROPS_SIZE: usize;
}

pub trait OfUFunction {
    // Field: EFunctionFlags
    const FLAGS: usize;
    const CODE: usize;
}

pub trait OfFField {
    // Field: FFieldClass*
    const CLASS: usize;
    // Field: FField*
    const NEXT: usize;
    // Field: FName
    const NAME: usize;
}

pub trait OfUField {
    // Field: UField*
    const NEXT: usize;
}

pub trait OfUEnum {
    const NAMES: usize;
}

pub trait OfFProperty {
    // Field: int32
    const ARRAY_DIM: usize;
    // Field: int32
    const ELEMENT_SIZE: usize;
    // Field: EPropertyFlags
    const FLAGS: usize;
    // Field: int32
    const OFFSET: usize;
    // ?
    const SIZE: usize;
}

pub trait Offsets {
    type NameEntry: OfFNameEntry;

    type UObject: OfUObject;
    type UStruct: OfUStruct;
    type UFunction: OfUFunction;

    type FField: OfFField;
    type UField: OfUField;
    type FProperty: OfFProperty;
}

#[macro_export]
macro_rules! offset_preset {
    (@proppass Header $value:expr) => { const HEADER: usize = $value; };
    (@proppass WideBit $value:expr) => { const WIDE_BIT: usize = $value; };
    (@proppass LenBit $value:expr) => { const LEN_BIT: usize = $value; };
    (@proppass ArrayDim $value:expr) => { const ARRAY_DIM: usize = $value; };
    (@proppass ElementSize $value:expr) => { const ELEMENT_SIZE: usize = $value; };
    (@proppass Flags $value:expr) => { const FLAGS: usize = $value; };
    (@proppass Offset $value:expr) => { const OFFSET: usize = $value; };
    (@proppass Size $value:expr) => { const SIZE: usize = $value; };
    (@proppass Class $value:expr) => { const CLASS: usize = $value; };
    (@proppass Next $value:expr) => { const NEXT: usize = $value; };
    (@proppass Name $value:expr) => { const NAME: usize = $value; };
    (@proppass Code $value:expr) => { const CODE: usize = $value; };
    (@proppass Names $value:expr) => { const NAMES: usize = $value; };
    (@proppass Super $value:expr) => { const SUPER: usize = $value; };
    (@proppass Children $value:expr) => { const CHILDREN: usize = $value; };
    (@proppass ChildrenProps $value:expr) => { const CHILDREN_PROPS: usize = $value; };
    (@proppass PropsSize $value:expr) => { const PROPS_SIZE: usize = $value; };
    (@proppass Outer $value:expr) => { const OUTER: usize = $value; };
    (@proppass Index $value:expr) => { const INDEX: usize = $value; };

    (@classpass $target:ident ; ) => { };
    (@classpass $target:ident ; $group:ident => { $($prop:ident = $value:expr);* $(;)? } $($t:tt)* ) => {
        $crate::__paste! {
            impl $crate::offsets:: [<Of $group>] for $target {
                $(
                    $crate::offset_preset!(@proppass $prop $value);
                )*
            }
        }

        $crate::offset_preset!(@classpass $target ; $($t)* );
    };

    ($vs:vis $target:ident => { $($t:tt)* }) => {
        $vs struct $target;
        $crate::offset_preset!(@classpass $target ; $($t)*);
    };
}
