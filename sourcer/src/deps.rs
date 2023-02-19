use crate::IdName;
use std::collections::HashMap;

#[derive(Debug)]
#[allow(dead_code)]
pub struct ClassData {
    pub package: String,
    pub code_name: String,
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
    ) {
        self.packages.entry(identifier.into()).or_insert(ClassData {
            package: package.into(),
            code_name: code_name.into(),
        });
    }

    pub fn lookup(&self, identifier: &IdName) -> Option<&ClassData> {
        self.packages.get(identifier)
    }

    pub fn len(&self) -> usize {
        self.packages.len()
    }
}
