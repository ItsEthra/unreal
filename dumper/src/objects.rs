use crate::{engine::UObjectPtr, State};
use anyhow::Result;
use log::{info, trace};
use memflex::sizeof;

const NUM_ELEMENTS_PER_CHUNK: usize = 64 * 1024;

pub(crate) fn dump_objects() -> Result<Vec<UObjectPtr>> {
    let State {
        options,
        external: proc,
        base,
        ..
    } = State::get();

    let [max_elements, num_elements, max_chunks, num_chunks] =
        proc.read::<[u32; 4]>(*base + options.objects + sizeof!(usize) * 2)?;
    info!("FChunkedFixedUObjectArray:\nMaxElements = {max_elements}\nNumElements = {num_elements}\nMaxChunks = {max_chunks}\nNumChunks = {num_chunks}");

    let array = proc.read::<usize>(*base + options.objects)?;
    let mut objects = vec![];
    for idx in 0..num_elements as usize {
        if let Some(object) = get_nth_object(array, idx)? {
            objects.push(object);
        }
    }

    info!("Found {} UObjects", objects.len());

    Ok(objects)
}

fn get_nth_object(array: usize, idx: usize) -> Result<Option<UObjectPtr>> {
    let State {
        external: proc,
        config: offsets,
        ..
    } = State::get();

    let chunk_id = idx / NUM_ELEMENTS_PER_CHUNK;
    let chunk = proc.read::<usize>(array + chunk_id * sizeof!(usize))?;

    let ptr =
        proc.read::<usize>(chunk + (idx % NUM_ELEMENTS_PER_CHUNK) * offsets.fuobject_item.size)?;
    trace!("Found UObject {ptr:#X} in chunk {chunk_id}");

    Ok(UObjectPtr(ptr).non_null())
}
