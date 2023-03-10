use crate::GlobalContext;
use std::{
    borrow::Cow,
    fmt::{self, Display},
    marker::PhantomData,
    mem::size_of,
    str::from_utf8_unchecked,
};

const NAME_SIZE: usize = 1024;

// const FNAME_MAX_BLOCK_BITS: u32 = 13;
const FNAME_BLOCK_OFFSET_BITS: u32 = 16;
// const FNAME_MAX_BLOCKS: u32 = 1 << FNAME_MAX_BLOCK_BITS;
const FNAME_BLOCK_OFFSETS: u32 = 1 << FNAME_BLOCK_OFFSET_BITS;
// const FNAME_ENTRY_ID_BITS: u32 = FNAME_BLOCK_OFFSET_BITS + FNAME_MAX_BLOCK_BITS;
// const FNAME_ENTRY_ID_MASK: u32 = (1 << FNAME_ENTRY_ID_BITS) - 1;

#[repr(transparent)]
struct FNameEntryHeader<const WIDE_BIT: usize, const LEN_BIT: usize>(u16);

impl<const WIDE_BIT: usize, const LEN_BIT: usize> FNameEntryHeader<WIDE_BIT, LEN_BIT> {
    #[inline]
    fn is_wide(&self) -> bool {
        (self.0 >> WIDE_BIT) & 1 != 0
    }

    fn len(&self) -> usize {
        (self.0 >> LEN_BIT) as usize
    }
}

#[repr(C)]
union FNameEntryData {
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

struct FNameEntry<
    const HEADER: usize,
    const DATA: usize,
    const WIDE_BIT: usize,
    const LEN_BIT: usize,
>(PhantomData<()>);

impl<const HEADER: usize, const DATA: usize, const WIDE_BIT: usize, const LEN_BIT: usize>
    FNameEntry<HEADER, DATA, WIDE_BIT, LEN_BIT>
{
    unsafe fn header(&self) -> &FNameEntryHeader<WIDE_BIT, LEN_BIT> {
        (self as *const Self)
            .cast::<u8>()
            .add(HEADER)
            .cast::<FNameEntryHeader<WIDE_BIT, LEN_BIT>>()
            .as_ref()
            .unwrap()
    }

    unsafe fn data(&self) -> &FNameEntryData {
        (self as *const Self)
            .cast::<u8>()
            .add(DATA)
            .cast::<FNameEntryData>()
            .as_ref()
            .unwrap()
    }

    fn to_str(&self) -> Cow<str> {
        let (header, data) = unsafe { (self.header(), self.data()) };
        let len = header.len();

        unsafe {
            if header.is_wide() {
                from_utf8_unchecked(data.as_ansi(len)).into()
            } else {
                String::from_utf16_lossy(data.as_wide(len)).into()
            }
        }
    }
}

impl<const HEADER: usize, const DATA: usize, const WIDE_BIT: usize, const LEN_BIT: usize>
    PartialEq<str> for FNameEntry<HEADER, DATA, WIDE_BIT, LEN_BIT>
{
    fn eq(&self, other: &str) -> bool {
        let (header, data) = unsafe { (self.header(), self.data()) };
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
struct FNameEntryHandle {
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

pub(crate) struct FNamePool<const STRIDE: usize>(PhantomData<()>);
impl<const STRIDE: usize> FNamePool<STRIDE> {
    fn resolve<
        const HEADER: usize,
        const DATA: usize,
        const WIDE_BIT: usize,
        const LEN_BIT: usize,
    >(
        &self,
        handle: impl Into<FNameEntryHandle>,
    ) -> &FNameEntry<HEADER, DATA, WIDE_BIT, LEN_BIT> {
        let FNameEntryHandle { block, offset } = handle.into();
        unsafe {
            (self as *const Self)
                .cast::<u8>()
                .add(size_of::<usize>() * (2 + block as usize))
                .cast::<*const u8>()
                .read_volatile()
                .add(offset as usize)
                .cast::<FNameEntry<HEADER, DATA, WIDE_BIT, LEN_BIT>>()
                .as_ref()
                .unwrap()
        }
    }
}

#[repr(transparent)]
pub struct FName<
    const STRIDE: usize,
    const SIZE: usize,
    const INDEX: usize,
    const HEADER: usize,
    const DATA: usize,
    const WIDE_BIT: usize,
    const LEN_BIT: usize,
>([u8; SIZE]);

impl<
        const STRIDE: usize,
        const SIZE: usize,
        const INDEX: usize,
        const HEADER: usize,
        const DATA: usize,
        const WIDE_BIT: usize,
        const LEN_BIT: usize,
    > FName<STRIDE, SIZE, INDEX, HEADER, DATA, WIDE_BIT, LEN_BIT>
{
    fn index(&self) -> FNameEntryId {
        unsafe {
            (self as *const Self)
                .cast::<u8>()
                .add(INDEX)
                .cast::<FNameEntryId>()
                .read_volatile()
        }
    }
}

impl<
        const STRIDE: usize,
        const SIZE: usize,
        const INDEX: usize,
        const HEADER: usize,
        const DATA: usize,
        const WIDE_BIT: usize,
        const LEN_BIT: usize,
    > Display for FName<STRIDE, SIZE, INDEX, DATA, HEADER, WIDE_BIT, LEN_BIT>
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let idx = self.index();
        let string = GlobalContext::get()
            .name_pool::<STRIDE>()
            .resolve::<HEADER, DATA, WIDE_BIT, LEN_BIT>(idx)
            .to_str();
        write!(f, "{string}")
    }
}
