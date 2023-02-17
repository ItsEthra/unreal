use crate::{offsets::Offsets, ptr::Ptr, Info};
use bytemuck::{bytes_of_mut, Pod, Zeroable};
use eyre::Result;
use log::trace;
use std::{borrow::Cow, fmt, mem::size_of, slice::from_raw_parts_mut};

// const FNAME_MAX_BLOCK_BITS: u32 = 13; // Limit block array a bit, still allowing 8k * block size = 1GB - 2G of FName entry data
const FNAME_BLOCK_OFFSET_BITS: u32 = 16;
// const FNAME_MAX_BLOCKS: u32 = 1 << FNAME_MAX_BLOCK_BITS;
const FNAME_BLOCK_OFFSETS: u32 = 1 << FNAME_BLOCK_OFFSET_BITS;
// const FNAME_ENTRY_ID_BITS: u32 = FNAME_BLOCK_OFFSET_BITS + FNAME_MAX_BLOCK_BITS;
// const FNAME_ENTRY_ID_MASK: u32 = (1 << FNAME_ENTRY_ID_BITS) - 1;

#[derive(Default, Zeroable, Pod, Clone, Copy)]
#[repr(C)]
pub struct FNameEntryId {
    value: u32,
    _pad: u32,
}

impl fmt::Debug for FNameEntryId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "0x{:X}", self.value)
    }
}

pub struct GNames {
    blocks: Vec<NameBlock>,
}

impl GNames {
    pub fn get(&self, id: FNameEntryId, offsets: &Offsets) -> FName {
        let block = id.value >> FNAME_BLOCK_OFFSET_BITS;
        let offset = id.value & (FNAME_BLOCK_OFFSETS - 1);

        let block = &self.blocks[block as usize];
        block.at(offsets.stride * offset as usize, offsets)
    }
}

pub fn dump_names(info: &Info, names: Ptr) -> Result<GNames> {
    let mut block_slot_ptr = names + size_of::<usize>();
    let mut block_ptr = Ptr(0);
    let mut blocks = vec![];

    loop {
        info.process
            .read_buf(block_slot_ptr, bytes_of_mut(&mut block_ptr))?;

        if block_ptr.0 == 0 {
            break;
        }

        trace!("Current name block: {block_ptr:?}");
        let block = dump_name_block(info, block_ptr)?;
        blocks.push(block);

        block_slot_ptr += size_of::<usize>();
    }

    Ok(GNames { blocks })
}
pub struct NameBlock(Box<[u8]>);

pub struct FName<'a> {
    pub header: FNameEntryHeader,
    pub text: Cow<'a, str>,
}

impl NameBlock {
    fn at(&self, pos: usize, offsets: &Offsets) -> FName {
        let header: FNameEntryHeader = self.0[pos..pos + 2].try_into().unwrap();
        let len = header.len(offsets);

        let data = &self.0[pos + 2..pos + 2 + len];
        let text = if header.is_wide(offsets) {
            Cow::Owned(String::from_utf16_lossy(bytemuck::cast_slice(data)))
        } else {
            String::from_utf8_lossy(data)
        };

        FName { header, text }
    }

    fn for_each_name(&self, offsets: &Offsets, mut cb: impl FnMut(Cow<str>)) {
        let mut pos = 0;

        while pos < self.0.len() {
            let name = self.at(pos, offsets);

            if name.text.is_empty() {
                break;
            }

            cb(name.text);

            pos += 2 + name.header.size(offsets);
        }
    }
}

fn dump_name_block(info: &Info, name_block_ptr: Ptr) -> Result<NameBlock> {
    let block_size = info.offsets.stride * FNAME_BLOCK_OFFSETS as usize;

    // I am not using `MaybeUninit<u8>` here because its making everything else
    // look ugly because rustc can't stabilize very useful feature smh.
    let mut data: Vec<u8> = Vec::with_capacity(block_size);

    #[allow(clippy::uninit_vec)]
    unsafe {
        data.set_len(block_size);

        info.process.read_buf(
            name_block_ptr,
            from_raw_parts_mut(data.as_mut_ptr(), block_size),
        )?;
    }

    let block = NameBlock(data.into_boxed_slice());

    let mut f = info.names_dump.borrow_mut();
    block.for_each_name(info.offsets, |n| _ = writeln!(f, "{n}"));

    Ok(block)
}

#[derive(Zeroable, Pod, Clone, Copy)]
#[repr(transparent)]
pub struct FNameEntryHeader(u16);

impl<'a> TryFrom<&'a [u8]> for FNameEntryHeader {
    type Error = <[u8; 2] as TryFrom<&'a [u8]>>::Error;

    fn try_from(value: &[u8]) -> std::result::Result<Self, Self::Error> {
        let buf: [u8; 2] = value.try_into()?;
        Ok(Self(u16::from_le_bytes(buf)))
    }
}

impl FNameEntryHeader {
    #[inline]
    pub fn is_wide(&self, offsets: &Offsets) -> bool {
        (self.0 >> offsets.fnameentry.wide_bit) & 1 != 0
    }

    // Size of data in bytes.
    #[inline]
    pub fn len(&self, offsets: &Offsets) -> usize {
        (self.0 >> offsets.fnameentry.len_bit) as usize * if self.is_wide(offsets) { 2 } else { 1 }
    }

    // Size of data in bytes aligned to 2.
    #[inline]
    pub fn size(&self, offsets: &Offsets) -> usize {
        let mut size = self.len(offsets);

        if size % 2 != 0 {
            size += 1;
        }

        size
    }
}
