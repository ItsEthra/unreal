#![allow(dead_code)]

use crate::{names::FNameEntryId, ptr::Ptr, Info, OFFSETS};
use bytemuck::bytes_of_mut;
use eyre::Result;
use std::{
    borrow::Cow,
    iter::successors,
    mem::{size_of, MaybeUninit},
};

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

    Ok(parent.to_option())
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

pub fn get_uobject_package(info: &Info, uobject_ptr: Ptr) -> Option<Ptr> {
    successors(Some(uobject_ptr), |obj| {
        get_uobject_outer(info, *obj).ok().flatten()
    })
    .last()
}

pub fn get_uobject_outer(info: &Info, uobject_ptr: Ptr) -> Result<Option<Ptr>> {
    let mut outer = Ptr(0);
    info.process.read_buf(
        uobject_ptr + OFFSETS.uobject.outer,
        bytes_of_mut(&mut outer),
    )?;

    Ok(outer.to_option())
}

pub fn get_uenum_names<'n>(
    info: &'n Info,
    uenum_ptr: Ptr,
    mut callback: impl FnMut(Cow<'n, str>, i64),
) -> Result<()> {
    unsafe {
        iter_tarray::<(FNameEntryId, i64)>(
            info,
            uenum_ptr + OFFSETS.uenum.names,
            |&(name, value)| {
                let name = info.names.get(name);
                callback(name.text, value)
            },
        )?;
    }

    Ok(())
}

pub unsafe fn iter_tarray<T>(
    info: &Info,
    tarray_ptr: Ptr,
    mut callback: impl FnMut(&T),
) -> Result<()> {
    let mut len = 0u32;
    info.process
        .read_buf(tarray_ptr + size_of::<usize>(), bytes_of_mut(&mut len))?;

    let mut data_ptr = Ptr(0);
    info.process
        .read_buf(tarray_ptr, bytes_of_mut(&mut data_ptr))?;

    if data_ptr.0 == 0 {
        return Ok(());
    }

    for i in 0..len as usize {
        let slot_ptr = data_ptr + i * size_of::<T>();
        let mut val: MaybeUninit<T> = MaybeUninit::uninit();
        info.process
            .read_val(slot_ptr, val.as_mut_ptr() as _, size_of::<T>())?;

        callback(val.assume_init_ref());
    }

    Ok(())
}

pub fn iter_ffield_linked_list(
    info: &Info,
    ffield_ptr: Ptr,
    mut callback: impl FnMut(Ptr) -> Result<()>,
) -> Result<()> {
    for ffield in successors(Some(ffield_ptr), |ffield| {
        let mut next = Ptr(0);
        info.process
            .read_buf(*ffield + OFFSETS.ffield.next, bytes_of_mut(&mut next))
            .ok()?;

        next.to_option()
    }) {
        callback(ffield)?;
    }

    Ok(())
}

pub fn get_uscript_struct_children_props(info: &Info, uscript_struct: Ptr) -> Result<Option<Ptr>> {
    let mut ffield_ptr = Ptr(0);
    info.process.read_buf(
        uscript_struct + OFFSETS.ustruct.children_props,
        bytes_of_mut(&mut ffield_ptr),
    )?;

    Ok(ffield_ptr.to_option())
}

pub fn get_ffield_name(info: &Info, ffield_ptr: Ptr) -> Result<Cow<str>> {
    let mut name = FNameEntryId::default();

    info.process
        .read_buf(ffield_ptr + OFFSETS.ffield.name, bytes_of_mut(&mut name))?;

    Ok(info.names.get(name).text)
}

pub fn get_ffield_class(info: &Info, ffield_ptr: Ptr) -> Result<Ptr> {
    let mut class = Ptr(0);

    info.process
        .read_buf(ffield_ptr + OFFSETS.ffield.class, bytes_of_mut(&mut class))?;

    Ok(class)
}

pub fn get_ffield_class_name(info: &Info, ffield_class_ptr: Ptr) -> Result<Cow<str>> {
    let mut name = FNameEntryId::default();

    info.process.read_buf(
        ffield_class_ptr + /* TODO: Maybe add to offsets? */ 0x0,
        bytes_of_mut(&mut name),
    )?;

    Ok(info.names.get(name).text)
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

// TODO: Replace characters that are not allowed in identifiers.
pub fn get_uobject_code_name(info: &Info, uobject_ptr: Ptr) -> Result<String> {
    let prefix = if is_uobject_inherits(info, uobject_ptr, info.objects.class_static_class(info)?)?
    {
        if is_uclass_inherits(info, uobject_ptr, info.objects.actor_static_class(info)?) {
            "A"
        } else {
            "U"
        }
    } else if is_uobject_inherits(info, uobject_ptr, info.objects.enum_static_class(info)?)? {
        ""
    } else {
        "F"
    };
    let name = get_uobject_name(info, uobject_ptr)?;

    Ok(format!("{prefix}{name}"))
}
