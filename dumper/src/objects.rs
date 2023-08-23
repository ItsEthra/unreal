use crate::{engine::UObjectPtr, State};
use anyhow::Result;
use log::{debug, info, trace};
use memflex::sizeof;

const NUM_ELEMENTS_PER_CHUNK: usize = 64 * 1024;

pub(crate) fn dump_objects() -> Result<Vec<UObjectPtr>> {
    let State {
        options,
        proc,
        base,
        ..
    } = State::get();

    let [max_elements, num_elements, max_chunks, num_chunks] =
        proc.read::<[u32; 4]>(*base + options.objects + sizeof!(usize) * 2)?;
    info!("FChunkedFixedUObjectArray:\nMaxElements = {max_elements}\nNumElements = {num_elements}\nMaxChunks = {max_chunks}\nNumChunks = {num_chunks}");

    let array = proc.read::<usize>(*base + options.objects)?;
    let mut objects = vec![];
    for idx in 0..num_chunks as usize {
        dump_chunk(array, idx, &mut objects)?;
    }

    info!("Found {} UObjects", objects.len());

    Ok(objects)
}

fn dump_chunk(array: usize, idx: usize, objects: &mut Vec<UObjectPtr>) -> Result<usize> {
    let State { proc, offsets, .. } = State::get();

    let chunk = proc.read::<usize>(array + idx * sizeof!(usize))?;
    debug!("Dumping chunk {idx} at address {chunk:#X}");

    let mut num_objects = 0;
    for i in 0..NUM_ELEMENTS_PER_CHUNK {
        let ptr = proc.read::<usize>(chunk + i * offsets.fuobject_item.size)?;
        trace!("Found UObject {ptr:#X}");

        if ptr == 0 {
            num_objects = i;
            break;
        }

        objects.push(UObjectPtr(ptr));
    }

    Ok(num_objects)
}
