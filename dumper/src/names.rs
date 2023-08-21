use crate::State;
use anyhow::Result;
use log::{debug, info};
use memflex::sizeof;
use std::collections::HashMap;
use ucore::{FNameEntry, FNameEntryHeader};

const FNAME_BLOCK_OFFSET_BITS: u32 = 16;
const FNAME_BLOCK_OFFSETS: u32 = 1 << FNAME_BLOCK_OFFSET_BITS;

#[derive(Default, Debug)]
pub(crate) struct NamePool(pub HashMap<u32, String>);

impl NamePool {
    pub fn insert(&mut self, block: u32, offset: u32, name: String) {
        self.0
            .insert((block << FNAME_BLOCK_OFFSET_BITS) | offset, name);
    }

    pub fn get(&self, id: u32) -> Option<&str> {
        self.0.get(&id).map(|s| s.as_str())
    }
}

pub(crate) fn dump_names() -> Result<NamePool> {
    let State {
        options,
        proc,
        base,
        offsets,
        ..
    } = State::get();

    let pool_ptr = *base + options.names;
    let current_block = proc.read::<u32>(pool_ptr + sizeof!(usize))?;
    let current_block_byte_cursor = proc.read::<u32>(pool_ptr + sizeof!(usize) + sizeof!(u32))?;

    info!("FNamePool: CurrentBlock = {current_block} CurrentBlockByteCursor = {current_block_byte_cursor}");

    let mut pool = NamePool::default();
    let mut total_names = 0;
    for idx in 0..current_block + 1 {
        let size = if idx == current_block {
            current_block_byte_cursor
        } else {
            offsets.stride * FNAME_BLOCK_OFFSETS
        };
        let block = dump_block(pool_ptr, idx as usize, size as usize)?;

        unsafe {
            let mut entry = block.as_ptr().cast::<FNameEntry>();
            while entry.cast::<u8>().offset_from(block.as_ptr()) < size as isize {
                let name = (*entry).to_str();

                pool.insert(
                    idx,
                    entry.cast::<u8>().offset_from(block.as_ptr()) as u32 / offsets.stride,
                    name.into_owned(),
                );

                entry = entry
                    .cast::<u8>()
                    .add(sizeof!(FNameEntryHeader) + (*entry).size_in_bytes())
                    .cast();
                total_names += 1;
            }
        };
    }

    info!("Found {total_names} names");

    Ok(pool)
}

fn dump_block(pool: usize, idx: usize, size: usize) -> Result<Box<[u8]>> {
    let State { proc, .. } = State::get();

    let address =
        proc.read::<usize>(pool + sizeof!(usize) + sizeof!(u32) * 2 + idx * sizeof!(usize))?;
    debug!("FNamePool: Dumping block {idx} at address {address:#X}");

    let mut data = Vec::with_capacity(size);
    #[allow(clippy::uninit_vec)]
    unsafe {
        data.set_len(size)
    };

    let mut offset = 0;
    for chunk in data.chunks_mut(1024) {
        proc.read_buf(address + offset, &mut chunk[..])?;
        offset += chunk.len();
    }

    Ok(data.into_boxed_slice())
}
