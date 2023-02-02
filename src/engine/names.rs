use crate::offsets::{OfFNameEntryHeader, Offsets};
use std::marker::PhantomData;

#[repr(C)]
pub struct FNameEntryId {
    pub value: u32,
}

#[repr(transparent)]
pub struct FNameEntryHeader<O: Offsets> {
    value: u16,
    _of: PhantomData<O>,
}

impl<O: Offsets> FNameEntryHeader<O> {
    #[inline]
    pub const fn is_wide(&self) -> bool {
        self.value & 1 << O::NameEntryHeader::WIDE_BIT != 0
    }

    #[inline]
    pub const fn is_ansii(&self) -> bool {
        !self.is_wide()
    }

    #[inline]
    pub const fn len(&self) -> usize {
        (self.value >> O::NameEntryHeader::LEN_BIT) as usize
    }

    #[inline]
    pub const fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl<O: Offsets> From<u16> for FNameEntryHeader<O> {
    #[inline]
    fn from(value: u16) -> Self {
        Self {
            value,
            _of: PhantomData,
        }
    }
}

impl<O: Offsets> From<FNameEntryHeader<O>> for u16 {
    #[inline]
    fn from(value: FNameEntryHeader<O>) -> Self {
        value.value
    }
}

#[test]
fn test_fname_entry_header() {
    // let a: FNameEntryHeader = 0b0000000101_00000_1.into();
    // assert_eq!(a.len(), 5);
    // assert!(a.is_wide());
}
