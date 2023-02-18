use crate::IdName;
use std::collections::HashMap;

#[derive(Debug, Default)]
pub struct DependencyTree {
    packages: HashMap<IdName, String>,
}

impl DependencyTree {
    pub fn set_owner(&mut self, package_name: &str, identifier: IdName) {
        self.packages
            .entry(identifier)
            .or_insert(package_name.to_owned());
    }

    pub fn lookup(&self, identifier: &IdName) -> Option<&String> {
        self.packages.get(identifier)
    }
}
