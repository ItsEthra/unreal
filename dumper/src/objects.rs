use crate::{
    names::{FName, FNameEntryId, GNames},
    ptr::Ptr,
    Info,
};
use eyre::Result;
use log::trace;
use std::mem::size_of;

pub fn dump_objects(info: &Info, gnames: &GNames, objects: Ptr) -> Result<()> {
    let mut bytes = [0u32; 4];
    info.process.read_buf(
        objects + size_of::<usize>() * 2,
        bytemuck::cast_slice_mut(&mut bytes),
    )?;

    let [max_elements, num_elements, max_chunks, num_chunks] = bytes;
    trace!("Max elements: {max_elements} Num elements: {num_elements} Max chunks: {max_chunks} Num chunks: {num_chunks}");

    let mut chunk_array_ptr: Ptr = Ptr(0);
    unsafe {
        info.process.read_val(
            objects,
            &mut chunk_array_ptr as *mut _ as _,
            size_of::<usize>(),
        )?;
    }
    trace!("Chunk array ptr: {chunk_array_ptr:?}");

    for _ in 0..num_chunks as usize {
        let mut chunk_ptr = Ptr(0);
        unsafe {
            info.process.read_val(
                chunk_array_ptr,
                &mut chunk_ptr as *mut _ as _,
                size_of::<usize>(),
            )?;
        }

        trace!("Chunk ptr: {chunk_ptr:?}");
        dump_chunk(info, gnames, chunk_ptr)?;

        chunk_array_ptr += size_of::<usize>();
    }

    Ok(())
}

const NUM_ELEMENTS_PER_CHUNK: usize = 64 * 1024;

fn dump_chunk(info: &Info, gnames: &GNames, chunk_ptr: Ptr) -> Result<()> {
    let mut item_ptr = chunk_ptr;
    for _ in 0..NUM_ELEMENTS_PER_CHUNK {
        let mut uobject_ptr = Ptr(0);
        unsafe {
            info.process.read_val(
                item_ptr,
                &mut uobject_ptr as *mut _ as _,
                size_of::<usize>(),
            )?;
        }
        if uobject_ptr.0 == 0 {
            break;
        }

        // trace!("UObject: {uobject_ptr:?}");
        dump_object(info, gnames, uobject_ptr)?;

        item_ptr += info.offsets.fuobjectitem.size;
    }

    Ok(())
}

fn dump_object_name<'n>(info: &Info, gnames: &'n GNames, uobject_ptr: Ptr) -> Result<FName<'n>> {
    let mut index = FNameEntryId::default();
    info.process.read_buf(
        uobject_ptr + info.offsets.uobject.name + info.offsets.fname.index,
        bytemuck::bytes_of_mut(&mut index),
    )?;

    Ok(gnames.get(index, info.offsets))
}

fn dump_object(info: &Info, gnames: &GNames, uobject_ptr: Ptr) -> Result<()> {
    let name = dump_object_name(info, gnames, uobject_ptr)?;
    trace!("{}", name.text);

    Ok(())
}
