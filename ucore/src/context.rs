#![allow(dead_code)]

use crate::{FChunkedFixedUObjectArray, FNamePool, Ptr};
use std::ptr::{null_mut, NonNull};

pub struct GlobalContext {
    names: *mut FNamePool,
    objects: *mut FChunkedFixedUObjectArray,
    engine: *mut *mut (),
    world: *mut *mut (),
}

unsafe impl Sync for GlobalContext {}
unsafe impl Send for GlobalContext {}

static mut CONTEXT: Option<Box<GlobalContext>> = None;

impl GlobalContext {
    pub fn new(names: *mut FNamePool, objects: *mut FChunkedFixedUObjectArray) -> Self {
        Self {
            names,
            objects,
            engine: null_mut(),
            world: null_mut(),
        }
    }

    pub fn with_engine(mut self, engine: *mut *mut ()) -> Self {
        self.engine = engine;
        self
    }

    pub fn with_world(mut self, world: *mut *mut ()) -> Self {
        self.world = world;
        self
    }

    pub fn init(self) -> &'static Self {
        unsafe {
            if CONTEXT.is_none() {
                CONTEXT = Some(self.into());
                CONTEXT.as_ref().unwrap()
            } else {
                panic!("GlobalContext was already initialized")
            }
        }
    }

    #[inline]
    pub fn get() -> &'static Self {
        unsafe {
            CONTEXT
                .as_ref()
                .expect("GlobalContext has not yet been initialized")
        }
    }

    pub fn name_pool(&self) -> &'static FNamePool {
        unsafe { self.names.as_ref().unwrap() }
    }

    pub fn chunked_fixed_uobject_array(&self) -> &'static FChunkedFixedUObjectArray {
        unsafe { self.objects.as_ref().unwrap() }
    }

    /// # Safety
    /// * Pointer that was used passed to `with_engine` must be available for reads.
    pub unsafe fn engine<Engine>(&self) -> Option<Ptr<Engine>> {
        NonNull::new(self.engine.read().cast::<Engine>()).map(Ptr)
    }

    /// # Safety
    /// * Pointer that was used passed to `with_world` must be available for reads.
    pub unsafe fn world<World>(&self) -> Option<Ptr<World>> {
        NonNull::new(self.world.read().cast::<World>()).map(Ptr)
    }
}
