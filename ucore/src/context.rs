#![allow(dead_code)]

use crate::FNamePool;
use std::sync::OnceLock;

pub struct GlobalContext {
    objects: *mut (),
    names: *mut (),
}

unsafe impl Sync for GlobalContext {}
unsafe impl Send for GlobalContext {}

static CONTEXT: OnceLock<GlobalContext> = OnceLock::new();

impl GlobalContext {
    pub fn init(self) -> &'static Self {
        assert!(
            CONTEXT.set(self).is_ok(),
            "GlobalContext is already initialized"
        );

        Self::get()
    }

    #[inline]
    pub fn get() -> &'static Self {
        CONTEXT
            .get()
            .expect("GlobalContext has not yet been initialized")
    }

    pub(crate) fn name_pool(&self) -> &FNamePool {
        todo!()
    }
}
