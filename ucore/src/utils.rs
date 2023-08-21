use std::{
    marker::PhantomData,
    ops::{Deref, DerefMut},
    ptr::NonNull,
};

#[macro_export]
macro_rules! assert_size {
    ($target:path, $size:tt) => {
        const _: () = if core::mem::size_of::<$target>() != $size {
            panic!(concat!(
                "Size assertion failed! sizeof(",
                stringify!($target),
                ") != ",
                stringify!($size)
            ))
        } else {
            ()
        };
    };
}

pub struct Shrink<const SIZE: usize, T> {
    buf: [u8; SIZE],
    pd: PhantomData<T>,
}

impl<const SIZE: usize, T> Deref for Shrink<SIZE, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { self.buf.as_ptr().cast::<T>().as_ref().unwrap() }
    }
}

impl<const SIZE: usize, T> DerefMut for Shrink<SIZE, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { self.buf.as_mut_ptr().cast::<T>().as_mut().unwrap() }
    }
}

#[derive(PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct Ptr<T: ?Sized>(pub NonNull<T>);

impl<T: ?Sized> Clone for Ptr<T> {
    #[inline]
    fn clone(&self) -> Self {
        Self(self.0)
    }
}
impl<T: ?Sized> Copy for Ptr<T> {}

unsafe impl<T: ?Sized> Send for Ptr<T> {}
unsafe impl<T: ?Sized> Sync for Ptr<T> {}

impl<T: ?Sized> From<NonNull<T>> for Ptr<T> {
    #[inline]
    fn from(ptr: NonNull<T>) -> Self {
        Self(ptr)
    }
}

impl<T: ?Sized> Deref for Ptr<T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        unsafe { self.0.as_ref() }
    }
}

impl<T: ?Sized> DerefMut for Ptr<T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { self.0.as_mut() }
    }
}
