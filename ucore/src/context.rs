#![allow(dead_code)]

use crate::{FChunkedFixedUObjectArray, FNamePool, Ptr};
use std::{
    ptr::{null_mut, NonNull},
    sync::OnceLock,
};

pub struct GlobalContext {
    names: *mut FNamePool,
    objects: *mut FChunkedFixedUObjectArray,
    engine: *mut (),
    world: *mut (),
}

unsafe impl Sync for GlobalContext {}
unsafe impl Send for GlobalContext {}

static CONTEXT: OnceLock<GlobalContext> = OnceLock::new();

impl GlobalContext {
    pub fn new(names: *mut FNamePool, objects: *mut FChunkedFixedUObjectArray) -> Self {
        Self {
            names,
            objects,
            engine: null_mut(),
            world: null_mut(),
        }
    }

    pub fn with_engine(mut self, engine: *mut ()) -> Self {
        self.engine = engine as _;
        self
    }

    pub fn with_world(mut self, world: *mut ()) -> Self {
        self.world = world as _;
        self
    }

    pub fn init(self) -> &'static Self {
        assert!(
            CONTEXT.set(self).is_ok(),
            "GlobalContext was already initialized"
        );
        CONTEXT.get().unwrap()
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

    pub fn chunked_fixed_uobject_array(&self) -> &'static FChunkedFixedUObjectArray {
        unsafe { self.objects.as_ref().unwrap() }
    }

    pub fn engine<Engine>(&self) -> Option<Ptr<Engine>> {
        NonNull::new(self.engine.cast::<Engine>()).map(Ptr)
    }

    pub fn world<World>(&self) -> Option<Ptr<World>> {
        NonNull::new(self.world.cast::<World>()).map(Ptr)
    }
}
