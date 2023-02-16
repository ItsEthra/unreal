use crate::{offsets::Offsets, process::Ptr, Info};
use bytemuck::{Pod, Zeroable};
use eyre::Result;
use log::trace;
use std::{borrow::Cow, mem::size_of, slice::from_raw_parts_mut};

// const FNAME_MAX_BLOCK_BITS: u32 = 13; // Limit block array a bit, still allowing 8k * block size = 1GB - 2G of FName entry data
const FNAME_BLOCK_OFFSET_BITS: u32 = 16;
// const FNAME_MAX_BLOCKS: u32 = 1 << FNAME_MAX_BLOCK_BITS;
const FNAME_BLOCK_OFFSETS: u32 = 1 << FNAME_BLOCK_OFFSET_BITS;
// const FNAME_ENTRY_ID_BITS: u32 = FNAME_BLOCK_OFFSET_BITS + FNAME_MAX_BLOCK_BITS;
// const FNAME_ENTRY_ID_MASK: u32 = (1 << FNAME_ENTRY_ID_BITS) - 1;

#[allow(dead_code)]
pub struct GNames {
    blocks: Vec<NameBlock>,
}

pub fn dump_names(info: &Info, names: Ptr) -> Result<GNames> {
    let mut block_count = 0u32;
    unsafe {
        info.process
            .read_val(names, &mut block_count as *mut u32 as _, size_of::<u32>())?;
    }

    let mut current_block_ptr = Ptr(0);
    let mut blocks = vec![];

    for i in 0..block_count as usize {
        unsafe {
            info.process.read_val(
                Ptr(names.0 + (i + 1) * size_of::<usize>()),
                &mut current_block_ptr as *mut Ptr as _,
                size_of::<usize>(),
            )?;
        }

        if current_block_ptr.0 == 0 {
            break;
        }

        trace!("Current name block: {current_block_ptr:?}");
        blocks.push(dump_name_block(info, current_block_ptr)?);
    }

    Ok(GNames { blocks })
}

pub struct NameBlock(Box<[u8]>);

impl NameBlock {
    fn for_each_name(&self, offsets: &Offsets, mut cb: impl FnMut(Cow<str>)) {
        let mut pos = 0;

        while pos < self.0.len() {
            let header: FNameEntryHeader = self.0[pos..pos + 2].try_into().unwrap();
            let (len, size) = header.len_size(offsets);

            if pos + 2 + len > self.0.len() {
                break;
            }

            let data = &self.0[pos + 2..pos + 2 + len];
            let name = if header.is_wide(offsets) {
                Cow::Owned(String::from_utf16_lossy(bytemuck::cast_slice(data)))
            } else {
                String::from_utf8_lossy(data)
            };

            if name.is_empty() {
                break;
            }

            cb(name);

            pos += 2 + size;
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
    block.for_each_name(info.offsets, |_| ());

    Ok(block)
}

#[derive(Zeroable, Pod, Clone, Copy)]
#[repr(transparent)]
struct FNameEntryHeader(u16);

impl<'a> TryFrom<&'a [u8]> for FNameEntryHeader {
    type Error = <[u8; 2] as TryFrom<&'a [u8]>>::Error;

    fn try_from(value: &[u8]) -> std::result::Result<Self, Self::Error> {
        let buf: [u8; 2] = value.try_into()?;
        Ok(Self(u16::from_le_bytes(buf)))
    }
}

impl FNameEntryHeader {
    #[inline]
    fn is_wide(&self, offsets: &Offsets) -> bool {
        (self.0 >> offsets.fnameentry.wide_bit) & 1 != 0
    }

    // Len is size of data in bytes.
    // Size is size of data in bytes but aligned.
    #[inline]
    fn len_size(&self, offsets: &Offsets) -> (usize, usize) {
        let len = (self.0 >> offsets.fnameentry.len_bit) as usize
            * if self.is_wide(offsets) { 2 } else { 1 };

        let mut size = len;

        if size % 2 != 0 {
            size += 1;
        }

        (len, size)
    }
}
