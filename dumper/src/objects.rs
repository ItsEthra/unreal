use crate::{
    names::{FName, FNameEntryId},
    ptr::Ptr,
    Info, OFFSETS,
};
use bytemuck::bytes_of_mut;
use eyre::Result;
use log::{info, trace};
use std::mem::size_of;

#[allow(dead_code)]
pub struct GObjects {
    // Pointers to UObjectBase
    objs: Vec<Ptr>,
}

pub fn dump_objects(info: &Info, gobjects: Ptr) -> Result<GObjects> {
    let mut bytes = [0u32; 4];
    info.process.read_buf(
        gobjects + size_of::<usize>() * 2,
        bytemuck::cast_slice_mut(&mut bytes),
    )?;

    let [max_elements, num_elements, max_chunks, num_chunks] = bytes;
    trace!("Max elements: {max_elements} Num elements: {num_elements} Max chunks: {max_chunks} Num chunks: {num_chunks}");

    let mut chunk_array_ptr: Ptr = Ptr(0);
    info.process
        .read_buf(gobjects, bytes_of_mut(&mut chunk_array_ptr))?;
    trace!("Chunk array ptr: {chunk_array_ptr:?}");

    let mut objs = vec![];
    for _ in 0..num_chunks as usize {
        let mut chunk_ptr = Ptr(0);
        info.process
            .read_buf(chunk_array_ptr, bytes_of_mut(&mut chunk_ptr))?;
        trace!("Chunk ptr: {chunk_ptr:?}");

        dump_chunk(info, chunk_ptr, &mut objs)?;
        chunk_array_ptr += size_of::<usize>();
    }

    info!("Dumped {} objects", objs.len());

    Ok(GObjects { objs })
}

const NUM_ELEMENTS_PER_CHUNK: usize = 64 * 1024;

fn dump_chunk(info: &Info, chunk_ptr: Ptr, objs: &mut Vec<Ptr>) -> Result<()> {
    let mut item_ptr = chunk_ptr;
    for _ in 0..NUM_ELEMENTS_PER_CHUNK {
        let mut uobject_ptr = Ptr(0);
        info.process
            .read_buf(item_ptr, bytes_of_mut(&mut uobject_ptr))?;

        if uobject_ptr.0 == 0 {
            break;
        }

        // trace!("UObject: {uobject_ptr:?}");
        objs.push(uobject_ptr);
        dump_object(info, uobject_ptr)?;

        item_ptr += OFFSETS.fuobjectitem.size;
    }

    Ok(())
}

fn dump_object_name(info: &Info, uobject_ptr: Ptr) -> Result<FName> {
    let mut index = FNameEntryId::default();
    info.process.read_buf(
        uobject_ptr + OFFSETS.uobject.name + OFFSETS.fname.index,
        bytes_of_mut(&mut index),
    )?;

    Ok(info.names.get(index))
}

fn dump_object_index(info: &Info, uobject_ptr: Ptr) -> Result<u32> {
    let mut index = 0u32;
    info.process.read_buf(
        uobject_ptr + OFFSETS.uobject.index,
        bytes_of_mut(&mut index),
    )?;

    Ok(index)
}

fn dump_object(info: &Info, uobject_ptr: Ptr) -> Result<()> {
    let name = dump_object_name(info, uobject_ptr)?;
    let index = dump_object_index(info, uobject_ptr)?;

    let mut f = info.objects_dump.borrow_mut();
    writeln!(f, "UObject[{index}] - {}", name.text)?;

    Ok(())
}
