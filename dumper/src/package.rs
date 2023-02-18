#![allow(dead_code)]

use crate::{
    ptr::Ptr,
    utils::{get_uenum_names, get_uobject_name, get_uobject_package, is_uobject_inherits},
    Info,
};
use eyre::Result;
use log::{info, trace};
use std::{borrow::Cow, collections::HashMap};

pub struct Package {
    name: String,
    objects: Vec<Ptr>,
}

impl Package {
    pub fn process(&self, info: &Info) -> Result<()> {
        let enum_sc = info.objects.enum_static_class(info)?;

        for obj in self.objects.iter().copied() {
            let is_a = |sclass: Ptr| is_uobject_inherits(info, obj, sclass);

            if is_a(enum_sc)? {
                self.process_enum(info, obj)?;
            }
        }

        Ok(())
    }

    fn process_enum(&self, info: &Info, uenum_ptr: Ptr) -> Result<()> {
        let callback = |name: Cow<str>, _value: i64| {
            if name.ends_with("_MAX") {
                return;
            }

            let _name = name.split_once("::").map(|(_, b)| b).unwrap_or(&name);
        };

        get_uenum_names(info, uenum_ptr, callback)?;

        Ok(())
    }
}

pub fn dump_packages(info: &Info) -> Result<Vec<Package>> {
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
