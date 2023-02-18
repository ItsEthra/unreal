use crate::IdName;
use std::collections::HashMap;

#[derive(Debug)]
#[allow(dead_code)]
pub struct OwnerData {
    package: String,
    code_name: String,
}

#[derive(Debug, Default)]
pub struct DependencyTree {
    packages: HashMap<IdName, OwnerData>,
}

impl DependencyTree {
    pub fn set_owner(
        &mut self,
        identifier: impl Into<IdName>,
        package: impl Into<String>,
        code_name: impl Into<String>,
    ) {
        self.packages.entry(identifier.into()).or_insert(OwnerData {
            package: package.into(),
            code_name: code_name.into(),
        });
    }

    pub fn lookup(&self, identifier: &IdName) -> Option<&OwnerData> {
        self.packages.get(identifier)
    }
}
