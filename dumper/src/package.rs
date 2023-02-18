#![allow(dead_code)]

use crate::{
    ptr::Ptr,
    utils::{
        get_ffield_class, get_ffield_class_name, get_ffield_name, get_fproperty_array_dim,
        get_fproperty_element_size, get_fproperty_offset, get_uenum_names, get_uobject_code_name,
        get_uobject_full_name, get_uobject_name, get_uobject_package,
        get_uscript_struct_children_props, is_uobject_inherits, iter_ffield_linked_list,
        sanitize_ident,
    },
    Info,
};
use eyre::Result;
use log::{info, trace};
use sourcer::{
    DependencyTree, EnumGenerator, IdName, PackageGenerator, PropertyType, ScriptStructGenerator,
};
use std::{
    borrow::Cow,
    collections::{HashMap, HashSet},
};

pub struct Package {
    pub name: String,
    objects: Vec<Ptr>,
}

impl Package {
    pub fn process<'pkg>(
        &self,
        info: &Info,
        mut codegen: Box<dyn PackageGenerator + 'pkg>,
    ) -> Result<()> {
        let enum_sc = info.objects.enum_static_class(info)?;
        let script_struct_sc = info.objects.script_struct_static_class(info)?;

        for obj in self.objects.iter().copied() {
            let is_a = |sclass: Ptr| is_uobject_inherits(info, obj, sclass);

            if is_a(enum_sc)? {
                self.process_enum(info, obj, &mut *codegen.add_enum()?)?;
            } else if is_a(script_struct_sc)? {
                self.process_script_struct(info, obj, &mut *codegen.add_script_struct()?)?;
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
                name.into()
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

        enum_cg.begin(&code_name, IdName(full_name), min_max)?;
        for (name, value) in pairs {
            enum_cg.append_variant(&name, value)?;
        }
        enum_cg.end()?;

        Ok(())
    }

    fn process_script_struct<'cg>(
        &self,
        info: &Info,
        uscript_struct_ptr: Ptr,
        sstruct_cg: &mut (dyn ScriptStructGenerator + 'cg),
    ) -> Result<()> {
        let struct_name = get_uobject_code_name(info, uscript_struct_ptr)?;
        let full_name = get_uobject_full_name(info, uscript_struct_ptr)?;

        let callback = |ffield_ptr: Ptr| {
            let _field_name = get_ffield_name(info, ffield_ptr)?;
            let class = get_ffield_class(info, ffield_ptr)?;
            let classname = get_ffield_class_name(info, class)?;

            let _dim = get_fproperty_array_dim(info, ffield_ptr)?;
            let _elem_size = get_fproperty_element_size(info, ffield_ptr)?;
            let _offset = get_fproperty_offset(info, ffield_ptr)?;

            let _prop_ty = PropertyType::from_str(&classname);

            Ok(())
        };

        sstruct_cg.begin(&struct_name, IdName(full_name))?;

        if let Some(props) = get_uscript_struct_children_props(info, uscript_struct_ptr)? {
            iter_ffield_linked_list(info, props, callback)?;
        }

        sstruct_cg.end()?;

        Ok(())
    }
}

pub fn dump_packages(info: &Info, deps: &mut DependencyTree) -> Result<Vec<Package>> {
    let mut map: HashMap<String, Vec<Ptr>> = HashMap::new();

    let struct_sc = info.objects.struct_static_class(info)?;
    let enum_sc = info.objects.enum_static_class(info)?;
    let function_sc = info.objects.function_static_class(info)?;

    for obj in info.objects.objs.iter().copied() {
        let Some(package) = get_uobject_package(info, obj) else { continue };

        let is_a = |sclass: Ptr| is_uobject_inherits(info, obj, sclass);
        if !is_a(struct_sc)? && !is_a(enum_sc)? && !is_a(function_sc)? {
            continue;
        }

        let package_name = get_uobject_name(info, package)?;
        if !map.contains_key(&package_name) {
            trace!("Found new package {package_name}");
        }

        deps.set_owner(&package_name, IdName(get_uobject_full_name(info, obj)?));

        let classes = map.entry(package_name).or_insert(vec![]);
        classes.push(obj);
    }

    info!("Found {} packages", map.len());

    let packages = map
        .into_iter()
        .map(|(name, objects)| Package { name, objects })
        .collect();
    Ok(packages)
}
