#![allow(dead_code)]

use crate::FNamePool;
use std::sync::OnceLock;

pub struct GlobalContext {
    names: *mut FNamePool,
    objects: *mut (),
}

unsafe impl Sync for GlobalContext {}
unsafe impl Send for GlobalContext {}

static CONTEXT: OnceLock<GlobalContext> = OnceLock::new();

impl GlobalContext {
    pub unsafe fn init(names: *mut FNamePool, objects: *mut ()) -> &'static Self {
        let this = Self { names, objects };
        assert!(
            CONTEXT.set(this).is_ok(),
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

    pub fn name_pool(&self) -> &'static FNamePool {
        unsafe { self.names.as_ref().unwrap() }
    }
}
