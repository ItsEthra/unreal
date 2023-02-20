#![allow(dead_code)]

use crate::{names::FNameEntryId, ptr::Ptr, Info};
use bytemuck::bytes_of_mut;
use eyre::Result;
use sourcer::{Layout, PropertyType};
use std::{
    borrow::Cow,
    iter::successors,
    mem::{size_of, MaybeUninit},
};

pub fn get_uobject_name(info: &Info, uobject_ptr: Ptr) -> Result<String> {
    let mut index = FNameEntryId::default();
    info.process.read_buf(
        uobject_ptr + info.offsets.uobject.name + info.offsets.fname.index,
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
        uclass_ptr + info.offsets.ustruct.super_struct,
        bytes_of_mut(&mut parent),
    )?;

    Ok(parent.to_option())
}

pub fn get_uobject_index(info: &Info, uobject_ptr: Ptr) -> Result<u32> {
    let mut index = 0u32;
    info.process.read_buf(
        uobject_ptr + info.offsets.uobject.index,
        bytes_of_mut(&mut index),
    )?;

    Ok(index)
}

pub fn get_uobject_class(info: &Info, uobject_ptr: Ptr) -> Result<Ptr> {
    let mut class = Ptr(0);
    info.process.read_buf(
        uobject_ptr + info.offsets.uobject.class,
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
        uobject_ptr + info.offsets.uobject.outer,
        bytes_of_mut(&mut outer),
    )?;

    Ok(outer.to_option())
}

pub fn get_uenum_names<'n>(
    info: &'n Info,
    uenum_ptr: Ptr,
    mut callback: impl FnMut(Cow<'n, str>, i64) -> Result<()>,
) -> Result<()> {
    unsafe {
        iter_tarray::<(FNameEntryId, i64)>(
            info,
            uenum_ptr + info.offsets.uenum.names,
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
    mut callback: impl FnMut(&T) -> Result<()>,
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

        callback(val.assume_init_ref())?;
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
            .read_buf(*ffield + info.offsets.ffield.next, bytes_of_mut(&mut next))
            .ok()?;

        next.to_option()
    }) {
        callback(ffield)?;
    }

    Ok(())
}

pub fn iter_ufield_linked_list(
    info: &Info,
    ffield_ptr: Ptr,
    mut callback: impl FnMut(Ptr) -> Result<()>,
) -> Result<()> {
    for ffield in successors(Some(ffield_ptr), |ffield| {
        let mut next = Ptr(0);
        info.process
            .read_buf(*ffield + info.offsets.ufield.next, bytes_of_mut(&mut next))
            .ok()?;

        next.to_option()
    }) {
        callback(ffield)?;
    }

    Ok(())
}

pub fn get_ustruct_children_props(info: &Info, ustruct: Ptr) -> Result<Option<Ptr>> {
    let mut ffield_ptr = Ptr(0);
    info.process.read_buf(
        ustruct + info.offsets.ustruct.children_props,
        bytes_of_mut(&mut ffield_ptr),
    )?;

    Ok(ffield_ptr.to_option())
}

pub fn get_ustruct_children(info: &Info, ustruct: Ptr) -> Result<Option<Ptr>> {
    let mut ffield_ptr = Ptr(0);
    info.process.read_buf(
        ustruct + info.offsets.ustruct.children,
        bytes_of_mut(&mut ffield_ptr),
    )?;

    Ok(ffield_ptr.to_option())
}

pub fn get_ffield_name(info: &Info, ffield_ptr: Ptr) -> Result<Cow<str>> {
    let mut name = FNameEntryId::default();

    info.process.read_buf(
        ffield_ptr + info.offsets.ffield.name,
        bytes_of_mut(&mut name),
    )?;

    Ok(info.names.get(name).text)
}

pub fn get_ffield_class(info: &Info, ffield_ptr: Ptr) -> Result<Ptr> {
    let mut class = Ptr(0);

    info.process.read_buf(
        ffield_ptr + info.offsets.ffield.class,
        bytes_of_mut(&mut class),
    )?;

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

pub fn get_fproperty_array_dim(info: &Info, fproperty_ptr: Ptr) -> Result<u32> {
    let mut dim = 0u32;
    info.process.read_buf(
        fproperty_ptr + info.offsets.fproperty.array_dim,
        bytes_of_mut(&mut dim),
    )?;

    Ok(dim)
}

pub fn get_fproperty_offset(info: &Info, fproperty_ptr: Ptr) -> Result<usize> {
    let mut offset = 0u32;
    info.process.read_buf(
        fproperty_ptr + info.offsets.fproperty.offset,
        bytes_of_mut(&mut offset),
    )?;

    Ok(offset as usize)
}

pub fn get_fproperty_element_size(info: &Info, fproperty_ptr: Ptr) -> Result<usize> {
    let mut elem_size = 0u32;
    info.process.read_buf(
        fproperty_ptr + info.offsets.fproperty.element_size,
        bytes_of_mut(&mut elem_size),
    )?;

    Ok(elem_size as usize)
}

pub fn get_ustruct_parent(info: &Info, ustruct_ptr: Ptr) -> Result<Option<Ptr>> {
    let mut parent = Ptr(0);
    info.process.read_buf(
        ustruct_ptr + info.offsets.ustruct.super_struct,
        bytes_of_mut(&mut parent),
    )?;

    Ok(parent.to_option())
}

pub fn get_ustruct_layout(info: &Info, ustruct_ptr: Ptr) -> Result<Layout> {
    Ok(Layout {
        size: get_ustruct_size(info, ustruct_ptr)?,
        alignment: get_ustruct_alignment(info, ustruct_ptr)?,
    })
}

pub fn get_ustruct_size(info: &Info, ustruct_ptr: Ptr) -> Result<usize> {
    let mut size = 0u32;
    info.process.read_buf(
        ustruct_ptr + info.offsets.ustruct.props_size,
        bytes_of_mut(&mut size),
    )?;

    Ok(size as usize)
}

pub fn get_ustruct_alignment(info: &Info, ustruct_ptr: Ptr) -> Result<usize> {
    let mut alignment = 0u32;
    info.process.read_buf(
        // TODO: Maybe add to offsets?
        ustruct_ptr + info.offsets.ustruct.props_size + 4,
        bytes_of_mut(&mut alignment),
    )?;

    Ok(alignment as usize)
}

pub fn get_tarray_prop_inner_prop(info: &Info, fproperty_ptr: Ptr) -> Result<Ptr> {
    let mut inner_prop = Ptr(0);
    info.process.read_buf(
        fproperty_ptr + info.offsets.fproperty.size,
        bytes_of_mut(&mut inner_prop),
    )?;

    Ok(inner_prop)
}

pub fn get_fobject_prop_pointee_class(info: &Info, fproperty_ptr: Ptr) -> Result<Ptr> {
    let mut class = Ptr(0);
    info.process.read_buf(
        fproperty_ptr + info.offsets.fproperty.size,
        bytes_of_mut(&mut class),
    )?;

    Ok(class)
}

pub fn get_fclass_prop_pointee_prop(info: &Info, fproperty_ptr: Ptr) -> Result<Ptr> {
    let mut class = Ptr(0);
    info.process.read_buf(
        // TODO: Maybe add to offsets?
        fproperty_ptr + info.offsets.fproperty.size + size_of::<usize>(),
        bytes_of_mut(&mut class),
    )?;

    Ok(class)
}

pub fn get_fenum_prop_inner_enum(info: &Info, fproperty_ptr: Ptr) -> Result<Ptr> {
    let mut uenum = Ptr(0);
    info.process.read_buf(
        fproperty_ptr + info.offsets.fproperty.size + size_of::<usize>(),
        bytes_of_mut(&mut uenum),
    )?;

    Ok(uenum)
}

pub fn get_tset_prop_inner_prop(info: &Info, fproperty_ptr: Ptr) -> Result<Ptr> {
    let mut prop = Ptr(0);
    info.process.read_buf(
        fproperty_ptr + info.offsets.fproperty.size,
        bytes_of_mut(&mut prop),
    )?;

    Ok(prop)
}

pub fn get_tmap_prop_key_value_props(info: &Info, fproperty_ptr: Ptr) -> Result<(Ptr, Ptr)> {
    let (mut key, mut value) = (Ptr(0), Ptr(0));
    info.process.read_buf(
        fproperty_ptr + info.offsets.fproperty.size,
        bytes_of_mut(&mut key),
    )?;
    info.process.read_buf(
        fproperty_ptr + info.offsets.fproperty.size + size_of::<usize>(),
        bytes_of_mut(&mut value),
    )?;

    Ok((key, value))
}

pub fn get_fproperty_type(info: &Info, fproperty_ptr: Ptr) -> Result<Option<PropertyType>> {
    let class = get_ffield_class(info, fproperty_ptr)?;
    let classname = get_ffield_class_name(info, class)?;
    let array_dim = get_fproperty_array_dim(info, fproperty_ptr)?;

    let ty = match &*classname {
        "BoolProperty" => PropertyType::Bool,
        "FloatProperty" => PropertyType::Float32,
        "DoubleProperty" => PropertyType::Float64,
        "Int8Property" => PropertyType::Int8,
        "Int16Property" => PropertyType::Int16,
        "IntProperty" => PropertyType::Int32,
        "Int64Property" => PropertyType::Int64,
        "ByteProperty" => PropertyType::UInt8,
        "UInt16Property" => PropertyType::UInt16,
        "UInt32Property" => PropertyType::UInt32,
        "UInt64Property" => PropertyType::UInt64,
        "NameProperty" => PropertyType::Name,
        "StrProperty" => PropertyType::String,
        "TextProperty" => PropertyType::Text,
        "ObjectProperty" => PropertyType::ClassPtr({
            let inner = get_fobject_prop_pointee_class(info, fproperty_ptr)?;
            PropertyType::InlineClass(get_uobject_full_name(info, inner)?.into()).into()
        }),
        "ArrayProperty" => PropertyType::Vector({
            let inner = get_tarray_prop_inner_prop(info, fproperty_ptr)?;
            if let Some(prop) = get_fproperty_type(info, inner)? {
                prop.into()
            } else {
                return Ok(None);
            }
        }),
        "ClassProperty" => PropertyType::ClassPtr({
            let inner = get_fclass_prop_pointee_prop(info, fproperty_ptr)?;
            PropertyType::InlineClass(get_uobject_full_name(info, inner)?.into()).into()
        }),
        "StructProperty" => PropertyType::InlineClass({
            let inner = get_fobject_prop_pointee_class(info, fproperty_ptr)?;
            get_uobject_full_name(info, inner)?.into()
        }),
        "EnumProperty" => PropertyType::InlineEnum({
            let inner = get_fenum_prop_inner_enum(info, fproperty_ptr)?;
            get_uobject_full_name(info, inner)?.into()
        }),
        "SetProperty" => PropertyType::Set({
            let inner = get_tset_prop_inner_prop(info, fproperty_ptr)?;
            if let Some(prop) = get_fproperty_type(info, inner)? {
                prop.into()
            } else {
                return Ok(None);
            }
        }),
        "MapProperty" => {
            let (key, value) = get_tmap_prop_key_value_props(info, fproperty_ptr)?;
            if let Some((key, value)) =
                get_fproperty_type(info, key)?.zip(get_fproperty_type(info, value)?)
            {
                PropertyType::Map {
                    key: key.into(),
                    value: value.into(),
                }
            } else {
                return Ok(None);
            }
        }
        // "ClassPtrProperty" => todo!(),
        // "DelegateProperty" => todo!(),
        // "FieldPathProperty" => todo!(),
        // "InterfaceProperty" => todo!(),
        // "LazyObjectProperty" => todo!(),
        // "SoftClassProperty" => todo!(),
        // "SoftObjectProperty" => todo!(),
        // "WeakObjectProperty" => todo!(),
        _ => return Ok(None),
    };

    match array_dim {
        2.. => Ok(Some(PropertyType::Array {
            ty: ty.into(),
            size: array_dim,
        })),
        1 => Ok(Some(ty)),
        _ => unreachable!(),
    }
}

pub fn sanitize_ident<'s>(ident: impl Into<Cow<'s, str>>) -> Cow<'s, str> {
    let mut ident = ident.into();
    if ident == "Self" {
        ident = "This".into();
    }

    ident
}
