#![allow(dead_code)]

use crate::FNamePool;
use std::{mem::transmute, ptr::NonNull};

static mut CONTEXT: Option<GlobalContext> = None;

pub struct GlobalContext {
    objects: NonNull<()>,
    names: NonNull<()>,
}

impl GlobalContext {
    pub fn new_init_global(objects: NonNull<()>, names: NonNull<()>) -> &'static GlobalContext {
        unsafe {
            if CONTEXT.is_none() {
                CONTEXT = Some(transmute(Self { objects, names }));
                Self::get()
            } else {
                panic!("GlobalContext is already initialized")
            }
        }
    }

    #[inline]
    pub(crate) fn get() -> &'static GlobalContext {
        unsafe { CONTEXT.as_ref().expect("GlobalContext was not initialized") }
    }

    #[inline]
    pub(crate) fn name_pool<const STRIDE: usize>(&self) -> &FNamePool<STRIDE> {
        unsafe { self.names.cast::<FNamePool<STRIDE>>().as_ref() }
    }
}
