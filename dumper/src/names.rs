use crate::{ptr::Ptr, Info};
use bytemuck::{bytes_of_mut, Pod, Zeroable};
use eyre::Result;
use log::{info, trace};
use offsets::Offsets;
use std::{
    array::TryFromSliceError, borrow::Cow, fmt, mem::size_of, rc::Rc, slice::from_raw_parts_mut,
};

// const FNAME_MAX_BLOCK_BITS: u32 = 13;
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

// Refcounted vector of all name blocks.
#[derive(Clone)]
pub struct GNames {
    offsets: &'static Offsets,
    blocks: Rc<Vec<NameBlock>>,
}

impl GNames {
    pub fn get(&self, id: FNameEntryId) -> FName {
        let block = id.value >> FNAME_BLOCK_OFFSET_BITS;
        let offset = id.value & (FNAME_BLOCK_OFFSETS - 1);

        let block = &self.blocks[block as usize];
        block.at(self.offsets.stride * offset as usize)
    }
}

pub fn dump_names(info: &Info, gnames: Ptr) -> Result<GNames> {
    let current_block_ptr = gnames + size_of::<usize>() * if cfg!(windows) { 1 } else { 0 };
    let mut current_block = 0u32;
    info.process
        .read_buf(current_block_ptr, bytes_of_mut(&mut current_block))?;

    let current_byte_cursor_ptr =
        gnames + size_of::<usize>() * if cfg!(windows) { 1 } else { 0 } + size_of::<u32>();
    let mut current_byte_cursor = 0u32;
    info.process.read_buf(
        current_byte_cursor_ptr,
        bytes_of_mut(&mut current_byte_cursor),
    )?;
    trace!("Current block: {current_block}. Current byte cursor: 0x{current_byte_cursor:X}");

    let first_block_slot_ptr = current_byte_cursor_ptr + size_of::<u32>();
    let mut block_ptr = Ptr(0);
    let mut blocks = vec![];

    let mut name_count = 0;
    for i in 0..current_block as usize {
        info.process.read_buf(
            first_block_slot_ptr + size_of::<usize>() * i,
            bytes_of_mut(&mut block_ptr),
        )?;
        let block_size = info.offsets.stride * FNAME_BLOCK_OFFSETS as usize;

        trace!("Dumping name block: {block_ptr:?}");
        let block = dump_name_block(info, block_ptr, block_size, &mut name_count)?;
        blocks.push(block);
    }

    // Dump last block
    {
        info.process.read_buf(
            first_block_slot_ptr + size_of::<usize>() * current_block as usize,
            bytes_of_mut(&mut block_ptr),
        )?;

        trace!("Dumping name block: {block_ptr:?}");
        let last_block = dump_name_block(
            info,
            block_ptr,
            current_byte_cursor as usize,
            &mut name_count,
        )?;
        blocks.push(last_block);
    }

    info!("Dumped {name_count} names");

    Ok(GNames {
        blocks: blocks.into(),
        offsets: info.offsets,
    })
}

pub struct FName<'a> {
    pub header: FNameEntryHeader,
    pub text: Cow<'a, str>,
}

impl<'a> fmt::Debug for FName<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.text)
    }
}

pub struct NameBlock {
    offsets: &'static Offsets,
    data: Box<[u8]>,
}

impl NameBlock {
    fn at(&self, pos: usize) -> FName {
        let header: FNameEntryHeader = self.data[pos..pos + 2].try_into().unwrap();
        let len = header.len(self.offsets);

        let data = &self.data[pos + 2..pos + 2 + len];
        let text = if header.is_wide(self.offsets) {
            Cow::Owned(String::from_utf16_lossy(bytemuck::cast_slice(data)))
        } else {
            String::from_utf8_lossy(data)
        };

        FName { header, text }
    }

    fn for_each_name(&self, mut callback: impl FnMut(Cow<str>)) {
        let mut pos = 0;

        while pos < self.data.len() {
            let name = self.at(pos);

            if name.text.is_empty() {
                break;
            }

            callback(name.text);

            pos += 2 + name.header.size(self.offsets);
        }
    }
}

fn dump_name_block(
    info: &Info,
    name_block_ptr: Ptr,
    block_size: usize,
    name_count: &mut usize,
) -> Result<NameBlock> {
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

    let block = NameBlock {
        data: data.into_boxed_slice(),
        offsets: info.offsets,
    };

    let mut f = info.names_dump.borrow_mut();
    block.for_each_name(|n| {
        writeln!(f, "{n}").unwrap();
        *name_count += 1;
    });

    Ok(block)
}

#[derive(Zeroable, Pod, Clone, Copy)]
#[repr(transparent)]
pub struct FNameEntryHeader(u16);

impl<'a> TryFrom<&'a [u8]> for FNameEntryHeader {
    type Error = TryFromSliceError;

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
