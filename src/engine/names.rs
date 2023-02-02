use crate::{
    impl_offset_members,
    offsets::{OfFName, OfFNameEntry, Offsets},
    terminated,
};
use std::{borrow::Cow, marker::PhantomData};

const NAME_SIZE: usize = 1024;

union FNameEntryData {
    ansi: [u8; NAME_SIZE],
    wide: [u16; NAME_SIZE],
}

pub struct FNameEntry<O: Offsets>(PhantomData<O>);

impl_offset_members! { FNameEntry,
    pub HEADER => header as FNameEntryHeader<O>
}

impl<O: Offsets> FNameEntry<O> {
    pub fn to_str(&self) -> Option<Cow<str>> {
        let header = self.header();
        let data = unsafe {
            (self.header() as *const FNameEntryHeader<O>)
                .add(1)
                .cast::<FNameEntryData>()
                .as_ref()?
        };

        if header.is_empty() {
            Some("".into())
        } else if header.is_wide() {
            Some(String::from_utf16_lossy(terminated(unsafe { data.wide.as_ptr() }, &0)).into())
        } else {
            Some(String::from_utf8_lossy(terminated(
                unsafe { data.ansi.as_ptr() },
                &0,
            )))
        }
    }
}

#[repr(transparent)]
pub struct FNameEntryHeader<O: Offsets> {
    value: u16,
    _of: PhantomData<O>,
}

impl<O: Offsets> FNameEntryHeader<O> {
    #[inline]
    pub const fn is_wide(&self) -> bool {
        self.value & 1 << O::FNameEntry::WIDE_BIT != 0
    }

    #[inline]
    pub const fn is_ansi(&self) -> bool {
        !self.is_wide()
    }

    #[inline]
    pub const fn len(&self) -> usize {
        (self.value >> O::FNameEntry::LEN_BIT) as usize
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

// const FNAME_MAX_BLOCK_BITS: usize = 13;
const FNAME_BLOCK_OFFSET_BITS: usize = 16;
// const FNAME_MAX_BLOCKS: usize = 1 << FNAME_MAX_BLOCK_BITS;
const FNAME_BLOCK_OFFSETS: u32 = 1 << FNAME_BLOCK_OFFSET_BITS;
// const FNAME_ENTRY_ID_BITS: usize = FNAME_BLOCK_OFFSET_BITS + FNAME_MAX_BLOCK_BITS;
// const FNAME_ENTRY_ID_MASK: usize = (1 << FNAME_ENTRY_ID_BITS) - 1;

#[repr(C)]
pub struct FNameEntryHandle {
    pub block: u32,
    pub offset: u32,
}

impl From<FNameEntryId> for FNameEntryHandle {
    #[inline]
    fn from(value: FNameEntryId) -> Self {
        Self {
            block: value >> FNAME_BLOCK_OFFSET_BITS,
            offset: value & (FNAME_BLOCK_OFFSETS - 1),
        }
    }
}

impl From<FNameEntryHandle> for FNameEntryId {
    #[inline]
    fn from(value: FNameEntryHandle) -> Self {
        value.block << FNAME_BLOCK_OFFSET_BITS | value.offset
    }
}

pub type FNameEntryId = u32;

#[repr(transparent)]
pub struct FName<O: Offsets>([u8; O::FName::SIZE], PhantomData<O>)
// TODO: rustc bug
where
    [u8; O::FName::SIZE]: Sized;

#[test]
fn test_fname_entry_header() {
    use crate::offsets::presets::Default;

    let a: FNameEntryHeader<Default> = 0b0000000101_00000_1.into();
    assert_eq!(a.len(), 5);
    assert!(a.is_wide());
}
