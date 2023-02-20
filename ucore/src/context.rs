#![allow(dead_code)]

use std::ptr::NonNull;

static mut CONTEXT: Option<GlobalContext> = None;

pub struct GlobalContext {
    objects: NonNull<()>,
    names: NonNull<()>,
}

impl GlobalContext {
    pub fn new_init_global(objects: NonNull<()>, names: NonNull<()>) -> &'static GlobalContext {
        unsafe {
            if CONTEXT.is_none() {
                CONTEXT = Some(Self { objects, names });
                CONTEXT.as_ref().unwrap()
            } else {
                panic!("GlobalContext is already initialized")
            }
        }
    }
}
