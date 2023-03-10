use crate::{IdName, Layout};
use std::collections::HashMap;

#[derive(Debug)]
pub enum RegistrationExtra {
    ClassLayout(Layout),
    EnumMinMax(Option<(i64, i64)>),
}

impl RegistrationExtra {
    pub fn unwrap_enum(&self) -> Option<(i64, i64)> {
        match self {
            RegistrationExtra::ClassLayout(_) => unreachable!(),
            RegistrationExtra::EnumMinMax(e) => e.clone(),
        }
    }
}

#[derive(Debug)]
pub struct RegistrationData {
    pub package: String,
    pub code_name: String,
    pub extra: RegistrationExtra,
}

#[derive(Debug, Default)]
pub struct PackageRegistry {
    packages: HashMap<IdName, RegistrationData>,
}

impl PackageRegistry {
    pub fn set_class_owner(
        &mut self,
        identifier: impl Into<IdName>,
        package: impl Into<String>,
        class_code_name: impl Into<String>,
        layout: Layout,
    ) {
        self.packages
            .entry(identifier.into())
            .or_insert(RegistrationData {
                package: package.into(),
                code_name: class_code_name.into(),
                extra: RegistrationExtra::ClassLayout(layout),
            });
    }

    pub fn set_enum_owner(
        &mut self,
        identifier: impl Into<IdName>,
        package: impl Into<String>,
        enum_code_name: impl Into<String>,
        min_max: Option<(i64, i64)>,
    ) {
        self.packages
            .entry(identifier.into())
            .or_insert(RegistrationData {
                package: package.into(),
                code_name: enum_code_name.into(),
                extra: RegistrationExtra::EnumMinMax(min_max),
            });
    }

    pub fn lookup(&self, identifier: &IdName) -> Option<&RegistrationData> {
        self.packages.get(identifier)
    }

    pub fn lookup_mut(&mut self, identifier: &IdName) -> Option<&mut RegistrationData> {
        self.packages.get_mut(identifier)
    }

    #[allow(clippy::len_without_is_empty)]
    pub fn len(&self) -> usize {
        self.packages.len()
    }
}
