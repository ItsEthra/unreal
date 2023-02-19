use core::ptr::NonNull;
use std::ops::{Deref, DerefMut};

#[derive(Debug, Clone, Copy)]
#[repr(transparent)]
pub struct ClassPtr<T> {
    ptr: NonNull<T>,
}

impl<T> ClassPtr<T> {
    #[inline]
    pub fn as_mut<'a>(&mut self) -> &'a mut T {
        unsafe { self.ptr.as_mut() }
    }

    #[inline]
    pub fn as_ref<'a>(&self) -> &'a T {
        unsafe { self.ptr.as_ref() }
    }

    #[inline]
    pub fn cast<U>(self) -> ClassPtr<U> {
        ClassPtr::<U> {
            ptr: self.ptr.cast(),
        }
    }

    #[inline]
    pub fn as_ptr(self) -> *mut T {
        self.ptr.as_ptr()
    }
}

impl<T> Deref for ClassPtr<T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.as_ref()
    }
}

impl<T> DerefMut for ClassPtr<T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.as_mut()
    }
}
