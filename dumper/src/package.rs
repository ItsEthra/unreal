#![allow(dead_code)]

use crate::{
    ptr::Ptr,
    utils::{get_uobject_name, get_uobject_outer, is_uobject_inherits},
    Info,
};
use eyre::Result;
use log::{info, trace};
use std::collections::HashMap;

pub struct Package {
    name: String,
    classes: Vec<Ptr>,
}

pub fn dump_packages(info: &Info) -> Result<Vec<Package>> {
    let mut map: HashMap<String, Vec<Ptr>> = HashMap::new();

    let struct_sc = info.objects.struct_static_class(info)?;
    let enum_sc = info.objects.enum_static_class(info)?;
    let function_sc = info.objects.function_static_class(info)?;

    for obj in info.objects.objs.iter().copied() {
        let Some(package) = get_uobject_outer(info, obj)? else { continue };

        let is_a = |sc: Ptr| is_uobject_inherits(info, obj, sc);

        if !is_a(struct_sc)? && !is_a(enum_sc)? && !is_a(function_sc)? {
            continue;
        }

        let package_name = get_uobject_name(info, package)?;
        if !map.contains_key(&package_name) {
            trace!("Found new package {package_name}");
        }

        let _classes = map.entry(package_name).or_insert(vec![]);
    }

    info!("Found {} packages", map.len());

    Ok(vec![])
}
