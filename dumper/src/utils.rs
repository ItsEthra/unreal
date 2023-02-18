#![allow(dead_code)]

use crate::{names::FNameEntryId, ptr::Ptr, Info, OFFSETS};
use bytemuck::bytes_of_mut;
use eyre::Result;
use std::iter::successors;

pub fn get_uobject_name(info: &Info, uobject_ptr: Ptr) -> Result<String> {
    let mut index = FNameEntryId::default();
    info.process.read_buf(
        uobject_ptr + OFFSETS.uobject.name + OFFSETS.fname.index,
        bytes_of_mut(&mut index),
    )?;

    let name = info.names.get(index);
    let text = name
        .text
        .rsplit_once('/')
        .map(|(_, tail)| tail)
        .unwrap_or(&name.text);

    Ok(text.to_owned())
}

pub fn is_uclass_inherits(info: &Info, uclass_ptr: Ptr, static_class: Ptr) -> bool {
    successors(Some(uclass_ptr), |class| {
        get_uclass_super(info, *class).ok().flatten()
    })
    .any(|class| class == static_class)
}

pub fn is_uobject_inherits(info: &Info, uobject_ptr: Ptr, static_class: Ptr) -> Result<bool> {
    let class = get_uobject_class(info, uobject_ptr)?;
    Ok(is_uclass_inherits(info, class, static_class))
}

pub fn get_uclass_super(info: &Info, uclass_ptr: Ptr) -> Result<Option<Ptr>> {
    let mut parent = Ptr(0);
    info.process.read_buf(
        uclass_ptr + OFFSETS.ustruct.super_struct,
        bytes_of_mut(&mut parent),
    )?;

    if parent.0 == 0 {
        Ok(None)
    } else {
        Ok(Some(parent))
    }
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

pub fn get_uobject_full_name(info: &Info, uobject_ptr: Ptr) -> Result<String> {
    let mut nodes = successors(Some(uobject_ptr), |obj| {
        get_uobject_outer(info, *obj).ok().flatten()
    })
    .filter_map(|obj| get_uobject_name(info, obj).ok())
    .collect::<Vec<_>>();
    nodes.reverse();

    let class = get_uobject_class(info, uobject_ptr)?;
    let classname = get_uobject_name(info, class)?;

    Ok(format!("{classname} {}", nodes.join(".")))
}
