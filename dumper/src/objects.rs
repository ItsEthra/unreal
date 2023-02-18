use crate::{
    generate_gobjects_static_classes,
    ptr::Ptr,
    utils::{get_uobject_class, get_uobject_index, get_uobject_name, get_uobject_outer},
    Info, OFFSETS,
};
use bytemuck::bytes_of_mut;
use eyre::{eyre, Result};
use log::{info, trace};
use std::{iter::successors, mem::size_of, rc::Rc};

// Refcounted vector of all UObject pointers.
#[derive(Clone)]
pub struct GObjects {
    // Pointers to UObjectBase
    pub objs: Rc<Vec<Ptr>>,
}

generate_gobjects_static_classes! {
    actor_static_class, "Class Engine.Actor",
    uobject_static_class, "Class CoreUObject.Object",
    field_static_class, "Class CoreUObject.Field",
    property_static_class, "Class CoreUObject.Property",
    struct_static_class, "Class CoreUObject.Struct",
    enum_static_class, "Class CoreUObject.Enum",
    class_static_class, "Class CoreUObject.Class",
    function_static_class, "Class CoreUObject.Function",
    script_struct_static_class, "Class CoreUObject.ScriptStruct",
    f64_prop_static_class, "Class CoreUObject.DoubleProperty",
    struct_prop_static_class, "Class CoreUObject.StructProperty",
    name_prop_static_class, "Class CoreUObject.NameProperty",
    object_prop_base_static_class, "Class CoreUObject.ObjectPropertyBase",
    array_prop_static_class, "Class CoreUObject.ArrayProperty",
    u8_prop_static_class, "Class CoreUObject.ByteProperty",
    bool_prop_static_class, "Class CoreUObject.BoolProperty",
    f32_prop_static_class, "Class CoreUObject.FloatProperty",
    i32_prop_static_class, "Class CoreUObject.IntProperty",
    i16_prop_static_class, "Class CoreUObject.Int16Property",
    i64_prop_static_class, "Class CoreUObject.Int64Property",
    i8_prop_static_class, "Class CoreUObject.Int8Property",
    u16_prop_static_class, "Class CoreUObject.UInt16Property",
    u32_prop_static_class, "Class CoreUObject.UInt32Property",
    u64_prop_static_class, "Class CoreUObject.UInt64Property",
    text_prop_static_class, "Class CoreUObject.TextProperty",
    str_prop_static_class, "Class CoreUObject.StrProperty",
    enum_prop_static_class, "Class CoreUObject.EnumProperty",
    class_prop_static_class, "Class CoreUObject.ClassProperty",
    set_prop_static_class, "Class CoreUObject.SetProperty",
    map_prop_static_class, "Class CoreUObject.MapProperty",
    interface_prop_static_class, "Class CoreUObject.InterfaceProperty",
    multicast_delegate_prop_static_class, "Class CoreUObject.MulticastDelegateProperty",
    weak_object_prop_static_class, "Class CoreUObject.WeakObjectProperty",
}

#[allow(dead_code)]
impl GObjects {
    pub fn find_by_full_name(&self, info: &Info, full_name: &str) -> Result<Option<Ptr>> {
        let (expected_class_name, expected_prefixed_name) = full_name
            .split_once(' ')
            .map(|(head, tail)| (head, tail.rsplit('.')))
            .ok_or(eyre!(
                "Full name must be in the format of `Class Engine.Pawn`"
            ))?;

        // Implementing it manually instead of using `get_uobject_full_name` should be faster
        // because this require less allocations.
        for obj in self.objs.iter().copied() {
            let class = get_uobject_class(info, obj)?;
            let class_name = get_uobject_name(info, class)?;

            if class_name != expected_class_name {
                continue;
            }

            if successors(Some(obj), |obj| {
                get_uobject_outer(info, *obj).ok().flatten()
            })
            .filter_map(|obj| get_uobject_name(info, obj).ok())
            .zip(expected_prefixed_name.clone())
            .all(|(a, b)| a == b)
            {
                return Ok(Some(obj));
            }
        }

        Ok(None)
    }
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

    Ok(GObjects { objs: objs.into() })
}

const NUM_ELEMENTS_PER_CHUNK: usize = 64 * 1024;

fn dump_chunk(info: &Info, chunk_ptr: Ptr, objs: &mut Vec<Ptr>) -> Result<()> {
    let mut item_ptr = chunk_ptr;
    for _ in 0..NUM_ELEMENTS_PER_CHUNK {
        let mut uobject_ptr = Ptr(0);
        info.process
            .read_buf(item_ptr, bytes_of_mut(&mut uobject_ptr))?;

        if uobject_ptr.is_zero() {
            break;
        }

        // trace!("UObject: {uobject_ptr:?}");
        objs.push(uobject_ptr);
        dump_object(info, uobject_ptr)?;

        item_ptr += OFFSETS.fuobjectitem.size;
    }

    Ok(())
}

fn dump_object(info: &Info, uobject_ptr: Ptr) -> Result<()> {
    let name = get_uobject_name(info, uobject_ptr)?;
    let index = get_uobject_index(info, uobject_ptr)?;

    let mut f = info.objects_dump.borrow_mut();
    writeln!(f, "UObject[{index}] - {name}")?;

    Ok(())
}
