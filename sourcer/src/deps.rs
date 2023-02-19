use crate::{IdName, Layout};
use std::collections::HashMap;

#[derive(Debug)]
#[allow(dead_code)]
pub struct ClassData {
    pub package: String,
    pub code_name: String,
    // Not set for enums
    pub layout: Option<Layout>,
}

#[derive(Debug, Default)]
pub struct ClassRegistry {
    packages: HashMap<IdName, ClassData>,
}

impl ClassRegistry {
    pub fn set_owner(
        &mut self,
        identifier: impl Into<IdName>,
        package: impl Into<String>,
        code_name: impl Into<String>,
        layout: Option<Layout>,
    ) {
        self.packages.entry(identifier.into()).or_insert(ClassData {
            package: package.into(),
            code_name: code_name.into(),
            layout,
        });
    }

    pub fn lookup(&self, identifier: &IdName) -> Option<&ClassData> {
        self.packages.get(identifier)
    }

    pub fn lookup_mut(&mut self, identifier: &IdName) -> Option<&mut ClassData> {
        self.packages.get_mut(identifier)
    }

    pub fn len(&self) -> usize {
        self.packages.len()
    }
}
