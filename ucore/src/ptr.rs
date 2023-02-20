use core::ptr::NonNull;
use std::ops::{Deref, DerefMut};

#[derive(Debug, Clone, Copy)]
#[repr(transparent)]
pub struct Ptr<T> {
    ptr: NonNull<T>,
}

#[allow(clippy::should_implement_trait)]
impl<T> Ptr<T> {
    /// # Safety
    /// See [`NonNull::as_ref`]
    #[inline]
    pub unsafe fn as_ref<'a>(&self) -> &'a T {
        self.ptr.as_ref()
    }

    /// # Safety
    /// See [`NonNull::as_mut`]
    #[inline]
    pub unsafe fn as_mut<'a>(&mut self) -> &'a mut T {
        self.ptr.as_mut()
    }

    #[inline]
    pub fn cast<U>(self) -> Ptr<U> {
        Ptr::<U> {
            ptr: self.ptr.cast(),
        }
    }

    #[inline]
    pub fn as_ptr(self) -> *mut T {
        self.ptr.as_ptr()
    }
}

impl<T> Deref for Ptr<T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        unsafe { self.as_ref() }
    }
}

impl<T> DerefMut for Ptr<T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { self.as_mut() }
    }
}
