use std::{
    ops::{Deref, DerefMut},
    ptr::NonNull,
    slice::{from_raw_parts, from_raw_parts_mut},
};

#[repr(C)]
pub struct TArray<T> {
    ptr: Option<NonNull<T>>,
    len: u32,
    capacity: u32,
}

impl<T> TArray<T> {
    #[inline]
    pub fn as_slice(&self) -> &[T] {
        match self.ptr {
            Some(ptr) => unsafe { from_raw_parts(ptr.as_ptr(), self.len as usize) },
            None => &[],
        }
    }

    #[inline]
    pub fn as_mut_slice(&mut self) -> &mut [T] {
        match self.ptr {
            Some(ptr) => unsafe { from_raw_parts_mut(ptr.as_ptr(), self.len as usize) },
            None => &mut [],
        }
    }
}

impl<T> Deref for TArray<T> {
    type Target = [T];

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.as_slice()
    }
}

impl<T> DerefMut for TArray<T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.as_mut_slice()
    }
}
