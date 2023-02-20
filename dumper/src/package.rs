#![allow(dead_code)]

use crate::{
    ptr::Ptr,
    utils::{
        get_ffield_name, get_fproperty_element_size, get_fproperty_offset, get_fproperty_type,
        get_uenum_names, get_uobject_code_name, get_uobject_full_name, get_uobject_name,
        get_uobject_package, get_ustruct_alignment, get_ustruct_children_props, get_ustruct_layout,
        get_ustruct_parent, get_ustruct_size, is_uobject_inherits, iter_ffield_linked_list,
        sanitize_ident,
    },
    Info,
};
use eyre::{eyre, Result};
use log::{info, trace};
use sourcer::{ClassRegistry, EnumGenerator, IdName, Layout, PackageGenerator, StructGenerator};
use std::{
    borrow::Cow,
    collections::{HashMap, HashSet},
};

struct Object {
    ptr: Ptr,
    id: IdName,
}

pub struct Package {
    pub name: String,
    objects: Vec<Object>,
}

impl Package {
    pub fn process<'pkg>(
        &self,
        info: &Info,
        codegen: &mut (dyn PackageGenerator + 'pkg),
    ) -> Result<()> {
        let enum_sc = info.objects.enum_static_class(info)?;
        let script_struct_sc = info.objects.script_struct_static_class(info)?;
        let class_sc = info.objects.class_static_class(info)?;

        for Object { ptr, .. } in self.objects.iter() {
            let is_a = |sclass: Ptr| is_uobject_inherits(info, *ptr, sclass);

            if is_a(enum_sc)? {
                self.process_enum(info, *ptr, &mut *codegen.add_enum()?)?;
            } else if is_a(script_struct_sc)? || is_a(class_sc)? {
                self.process_ustruct(info, *ptr, &mut *codegen.add_struct()?)?;
            }
        }

        Ok(())
    }

    fn process_enum<'cg>(
        &self,
        info: &Info,
        uenum_ptr: Ptr,
        enum_cg: &mut (dyn EnumGenerator + 'cg),
    ) -> Result<()> {
        let mut variants = HashSet::new();
        let mut pairs = vec![];

        let callback = |name: Cow<str>, value: i64| {
            if name.ends_with("_MAX") {
                return Ok(());
            }

            let name = sanitize_ident(name.split_once("::").map(|(_, b)| b).unwrap_or(&name));
            let name: Cow<str> = if variants.contains(&*name) {
                format!("{name}_{value}").into()
            } else {
                name
            };

            variants.insert(name.to_string());
            pairs.push((name.into_owned(), value));

            Ok(())
        };

        get_uenum_names(info, uenum_ptr, callback)?;

        let code_name = get_uobject_code_name(info, uenum_ptr)?;
        let full_name = get_uobject_full_name(info, uenum_ptr)?;

        let min_max = pairs
            .iter()
            .map(|(_, k)| k)
            .min()
            .copied()
            .zip(pairs.iter().map(|(_, k)| k).max().copied());

        enum_cg.begin(&code_name, full_name.into(), min_max)?;
        for (name, value) in pairs {
            enum_cg.append_variant(&name, value)?;
        }
        enum_cg.end()?;

        Ok(())
    }

    fn process_ustruct<'cg>(
        &self,
        info: &Info,
        ustruct_ptr: Ptr,
        ustruct_cg: &mut (dyn StructGenerator + 'cg),
    ) -> Result<()> {
        let struct_name = get_uobject_code_name(info, ustruct_ptr)?;
        let full_name = get_uobject_full_name(info, ustruct_ptr)?;

        let size = get_ustruct_size(info, ustruct_ptr)?;
        let alignment = get_ustruct_alignment(info, ustruct_ptr)?;

        let parent: Option<IdName> = get_ustruct_parent(info, ustruct_ptr)?
            .map(|p| get_uobject_full_name(info, p))
            .transpose()?
            .map(Into::into);

        trace!(
            "Processing {struct_name}({full_name})[Size: 0x{size:X}. Alignment: 0x{alignment:X}]"
        );

        ustruct_cg.begin(
            &struct_name,
            full_name.into(),
            Layout { size, alignment },
            parent,
        )?;

        let mut field_names = HashMap::new();
        let callback = |ffield_ptr: Ptr| {
            let mut field_name = get_ffield_name(info, ffield_ptr)?;

            let elem_size = get_fproperty_element_size(info, ffield_ptr)?;
            let offset = get_fproperty_offset(info, ffield_ptr)?;

            if let Some(&count) = field_names.get(&field_name) {
                field_name = format!("{field_name}_{count}").into();
            }
            *field_names.entry(field_name.clone()).or_insert(0) += 1;

            if let Some(prop_ty) = get_fproperty_type(info, ffield_ptr)? {
                trace!(
                    "\t{field_name}: {prop_ty:?}. Elem_size: 0x{elem_size:X}. Offset: 0x{offset:X}"
                );
                ustruct_cg.append_field(&field_name, prop_ty, elem_size, offset)?;
            } else {
                trace!(
                    "\t{field_name}: _UNKNOWN_. Elem_size: 0x{elem_size:X}. Offset: 0x{offset:X}"
                );
            }

            Ok(())
        };

        if let Some(props) = get_ustruct_children_props(info, ustruct_ptr)? {
            iter_ffield_linked_list(info, props, callback)?;
        }

        ustruct_cg.end()?;

        Ok(())
    }
}

pub fn dump_packages(info: &Info, registry: &mut ClassRegistry) -> Result<Vec<Package>> {
    let mut map: HashMap<String, Vec<Object>> = HashMap::new();

    let struct_sc = info.objects.struct_static_class(info)?;
    let enum_sc = info.objects.enum_static_class(info)?;
    let function_sc = info.objects.function_static_class(info)?;

    for ptr in info.objects.objs.iter().copied() {
        let Some(package) = get_uobject_package(info, ptr) else { continue };

        let is_a = |sclass: Ptr| is_uobject_inherits(info, ptr, sclass);
        if !is_a(struct_sc)? && !is_a(enum_sc)? && !is_a(function_sc)? {
            continue;
        }

        let package_name = get_uobject_name(info, package)?;
        if !map.contains_key(&package_name) {
            trace!("Found new package {package_name}");
        }

        let layout = if is_a(struct_sc)? {
            Some(get_ustruct_layout(info, ptr)?)
        } else {
            None
        };

        let obj_full_name = get_uobject_full_name(info, ptr)?;
        let obj_code_name = get_uobject_code_name(info, ptr)?;
        registry.set_owner(obj_full_name, &package_name, obj_code_name, layout);

        let classes = map.entry(package_name).or_insert(vec![]);
        classes.push(Object {
            ptr,
            id: get_uobject_full_name(info, ptr)?.into(),
        });
    }

    info!("Found {} packages", map.len());

    let packages = map
        .into_iter()
        .map(|(name, objects)| Package { name, objects })
        .collect();
    Ok(packages)
}

// Merges `target` into `merger`
pub fn merge(
    merger: &str,
    target: &str,
    registry: &mut ClassRegistry,
    packages: &mut Vec<Package>,
) -> Result<()> {
    let target = packages.remove(
        packages
            .iter()
            .position(|p| p.name == target)
            .ok_or(eyre!("Failed to find package {target}"))?,
    );

    for Object { id, .. } in target.objects.iter() {
        registry.lookup_mut(id).unwrap().package = merger.to_owned();
    }

    let merger = packages
        .iter_mut()
        .find(|p| p.name == merger)
        .ok_or(eyre!("Failed to find package {merger}"))?;
    merger.objects.extend(target.objects);

    Ok(())
}
