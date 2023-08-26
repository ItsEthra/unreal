use crate::GlobalContext;
use std::{
    borrow::Cow,
    fmt::{self, Debug, Display},
    hash::Hasher,
    marker::PhantomData,
    mem::size_of,
    str::from_utf8_unchecked,
};
use twox_hash::XxHash32;

// const FNAME_MAX_BLOCK_BITS: u32 = 13;
const FNAME_BLOCK_OFFSET_BITS: u32 = 16;
// const FNAME_MAX_BLOCKS: u32 = 1 << FNAME_MAX_BLOCK_BITS;
const FNAME_BLOCK_OFFSETS: u32 = 1 << FNAME_BLOCK_OFFSET_BITS;
// const FNAME_ENTRY_ID_BITS: u32 = FNAME_BLOCK_OFFSET_BITS + FNAME_MAX_BLOCK_BITS;
// const FNAME_ENTRY_ID_MASK: u32 = (1 << FNAME_ENTRY_ID_BITS) - 1;

const WIDE_BIT: usize = 0;
const LEN_BIT: usize = 6;

#[repr(transparent)]
pub struct FNameEntryHeader(u16);

impl FNameEntryHeader {
    #[inline]
    pub fn is_wide(&self) -> bool {
        (self.0 >> WIDE_BIT) & 1 != 0
    }

    #[inline]
    pub fn len(&self) -> usize {
        (self.0 >> LEN_BIT) as usize
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

const NAME_SIZE: usize = 1024;

#[repr(C)]
pub union FNameEntryData {
    wide: [u16; NAME_SIZE],
    ansi: [u8; NAME_SIZE],
}

impl FNameEntryData {
    #[inline]
    fn as_wide(&self, len: usize) -> &[u16] {
        unsafe { &self.wide[..len] }
    }

    #[inline]
    fn as_ansi(&self, len: usize) -> &[u8] {
        unsafe { &self.ansi[..len] }
    }
}

#[repr(C)]
pub struct FNameEntry(PhantomData<()>);

impl FNameEntry {
    pub fn header(&self) -> &FNameEntryHeader {
        unsafe {
            (self as *const Self)
                .cast::<FNameEntryHeader>()
                .as_ref()
                .unwrap()
        }
    }

    pub fn data(&self) -> &FNameEntryData {
        unsafe {
            (self as *const Self)
                .cast::<u8>()
                .add(size_of::<FNameEntryHeader>())
                .cast::<FNameEntryData>()
                .as_ref()
                .unwrap()
        }
    }

    pub fn size_in_bytes(&self) -> usize {
        let header = self.header();
        let unaligned = if header.is_wide() { 2 } else { 1 } * header.len();
        unaligned + (unaligned & 1)
    }

    pub fn to_str(&self) -> Cow<str> {
        let (header, data) = (self.header(), self.data());
        let len = header.len();

        unsafe {
            if header.is_wide() {
                String::from_utf16_lossy(&data.wide[..len]).into()
            } else {
                from_utf8_unchecked(&data.ansi[..len]).into()
            }
        }
    }

    pub fn hash(&self) -> u32 {
        let header = self.header();
        if header.is_wide() {
            let data = self.data().as_wide(header.len());
            let i = data
                .iter()
                .rposition(|b| char::from_u32(*b as _) == Some('/'))
                .unwrap_or(usize::MAX)
                .wrapping_add(1);
            let mut hash = XxHash32::default();
            for char in char::decode_utf16(data[i..].iter().copied())
                .map(|c| c.unwrap_or(char::REPLACEMENT_CHARACTER))
            {
                hash.write_u32(char as u32);
            }
            hash.finish() as u32
        } else {
            let data = self.data().as_ansi(header.len());
            let i = data
                .iter()
                .rposition(|b| *b == b'/')
                .unwrap_or(usize::MAX)
                .wrapping_add(1);
            let mut hash = XxHash32::default();
            for chunk in data[i..].chunks(4) {
                hash.write(chunk);
            }
            hash.finish() as u32
        }
    }
}

impl PartialEq<str> for FNameEntry {
    fn eq(&self, other: &str) -> bool {
        let (header, data) = (self.header(), self.data());
        let len = header.len();

        if header.is_wide() {
            other.len() == header.len()
                && other
                    .encode_utf16()
                    .zip(data.as_wide(len))
                    .all(|(l, r)| l == *r)
        } else {
            other.len() == header.len() && other.as_bytes() == data.as_ansi(len)
        }
    }
}

type FNameEntryId = u32;

#[repr(C)]
pub struct FNameEntryHandle {
    block: u32,
    offset: u32,
}

impl From<FNameEntryId> for FNameEntryHandle {
    #[inline]
    fn from(id: FNameEntryId) -> Self {
        Self {
            block: id >> FNAME_BLOCK_OFFSET_BITS,
            offset: id & (FNAME_BLOCK_OFFSETS - 1),
        }
    }
}

#[repr(C)]
pub struct FNamePool(PhantomData<()>);

impl FNamePool {
    pub fn resolve(&self, handle: impl Into<FNameEntryHandle>) -> &FNameEntry {
        let FNameEntryHandle { block, offset } = handle.into();
        unsafe {
            (self as *const Self)
                .cast::<u8>()
                .add(0x10 + size_of::<usize>() * block as usize)
                .cast::<*const u8>()
                .read()
                .add(offset as usize * 2)
                .cast::<FNameEntry>()
                .as_ref()
                .unwrap()
        }
    }
}

#[derive(Clone, Copy)]
#[repr(C)]
pub struct FName([u8; 8]);

impl FName {
    pub fn index(&self) -> FNameEntryId {
        unsafe { (self as *const Self).cast::<FNameEntryId>().read() }
    }
}

impl Display for FName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let text = GlobalContext::get()
            .name_pool()
            .resolve(self.index())
            .to_str();

        write!(f, "{text}")
    }
}

impl Debug for FName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self}")
    }
}
