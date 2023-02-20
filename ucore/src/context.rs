#![allow(dead_code)]

use crate::FNamePool;
use std::{mem::transmute, ptr::NonNull};

static mut CONTEXT: Option<GlobalContext<0>> = None;

pub struct GlobalContext<const STRIDE: usize> {
    objects: NonNull<()>,
    names: NonNull<()>,
}

impl<const STRIDE: usize> GlobalContext<STRIDE> {
    pub fn new_init_global(
        objects: NonNull<()>,
        names: NonNull<()>,
    ) -> &'static GlobalContext<STRIDE> {
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
    pub(crate) fn get() -> &'static GlobalContext<STRIDE> {
        unsafe {
            let s0: &'static GlobalContext<0> =
                CONTEXT.as_ref().expect("GlobalContext is not initialized");
            transmute(s0)
        }
    }

    #[inline]
    pub(crate) fn name_pool(&self) -> &FNamePool<STRIDE> {
        unsafe { self.names.cast::<FNamePool<STRIDE>>().as_ref() }
    }
}
