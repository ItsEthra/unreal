#![allow(dead_code)]

use crate::{
    names::{FName, FNameEntryId},
    ptr::Ptr,
    Info, OFFSETS,
};
use bytemuck::bytes_of_mut;
use eyre::Result;

pub fn get_uobject_name(info: &Info, uobject_ptr: Ptr) -> Result<FName> {
    let mut index = FNameEntryId::default();
    info.process.read_buf(
        uobject_ptr + OFFSETS.uobject.name + OFFSETS.fname.index,
        bytes_of_mut(&mut index),
    )?;

    Ok(info.names.get(index))
}

pub fn get_uobject_index(info: &Info, uobject_ptr: Ptr) -> Result<u32> {
    let mut index = 0u32;
    info.process.read_buf(
        uobject_ptr + OFFSETS.uobject.index,
        bytes_of_mut(&mut index),
    )?;

    Ok(index)
}

pub fn get_uobject_class(info: &Info, uobject_ptr: Ptr) -> Result<Ptr> {
    let mut class = Ptr(0);
    info.process.read_buf(
        uobject_ptr + OFFSETS.uobject.class,
        bytes_of_mut(&mut class),
    )?;

    Ok(class)
}

pub fn get_uobject_outer(info: &Info, uobject_ptr: Ptr) -> Result<Option<Ptr>> {
    let mut outer = Ptr(0);
    info.process.read_buf(
        uobject_ptr + OFFSETS.uobject.outer,
        bytes_of_mut(&mut outer),
    )?;

    if outer.0 == 0 {
        Ok(None)
    } else {
        Ok(Some(outer))
    }
}
