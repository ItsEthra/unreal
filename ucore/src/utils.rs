use std::{
    fmt::{self, Debug},
    hash::Hasher,
    marker::PhantomData,
    ops::{Deref, DerefMut},
    ptr::NonNull,
};

use twox_hash::XxHash32;

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

#[repr(transparent)]
pub struct Ptr<T: ?Sized>(pub NonNull<T>);

impl<T: ?Sized> Copy for Ptr<T> {}
unsafe impl<T: ?Sized> Send for Ptr<T> {}
unsafe impl<T: ?Sized> Sync for Ptr<T> {}
impl<T: ?Sized> Eq for Ptr<T> {}

impl<T: ?Sized> PartialEq for Ptr<T> {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl<T: ?Sized> Ptr<T> {
    pub fn cast<U>(self) -> Ptr<U> {
        Ptr(self.0.cast::<U>())
    }

    pub fn as_ptr(&self) -> *mut T {
        self.0.as_ptr()
    }

    pub fn from_ref(r: &T) -> Self {
        unsafe { Self(NonNull::new_unchecked(r as *const T as _)) }
    }
}

impl<T: ?Sized> Clone for Ptr<T> {
    #[inline]
    fn clone(&self) -> Self {
        Self(self.0)
    }
}

impl<T: ?Sized> Debug for Ptr<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.0)
    }
}

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

const FQN_LEN: usize = 4;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct HashedFqn(pub(crate) [u32; FQN_LEN]);

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Fqn {
    parts: [&'static str; FQN_LEN],
    len: usize,
}

impl Fqn {
    pub fn from_human_readable(ident: &'static str) -> Self {
        let mut this = Self::from_iter(ident.split('.'));
        this.parts[..this.len].reverse();

        this
    }

    pub fn from_iter(iter: impl Iterator<Item = &'static str>) -> Self {
        let mut len = 0;
        let mut parts = [""; FQN_LEN];

        for (i, part) in iter.enumerate() {
            parts[i] = part;
            len = i + 1;
        }
        assert!(len != 0, "Empty Fqns are not allowed");

        Self { parts, len }
    }

    #[inline]
    pub fn parts(&self) -> &[&'static str] {
        &self.parts[..self.len]
    }

    #[inline]
    pub fn name(&self) -> &'static str {
        self.parts[0]
    }

    #[inline]
    pub fn hash(&self) -> HashedFqn {
        let mut out = [0; FQN_LEN];
        for (i, part) in self.parts().iter().enumerate() {
            let mut hasher = XxHash32::default();
            hasher.write(part.as_bytes());
            out[i] = hasher.finish() as u32;
        }

        HashedFqn(out)
    }
}

impl PartialEq<HashedFqn> for Fqn {
    fn eq(&self, other: &HashedFqn) -> bool {
        self.hash() == *other
    }
}

#[macro_export]
macro_rules! fqn {
    ( $($tt:tt)* ) => {
        $crate::Fqn::from_human_readable(stringify!($($tt)*))
    };
}

impl fmt::Display for Fqn {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (i, part) in self.parts[..self.len].iter().rev().enumerate() {
            match true {
                _ if i == self.len - 1 => write!(f, "{}", part)?,
                _ => write!(f, "{}.", part)?,
            }
        }

        Ok(())
    }
}

impl fmt::Debug for Fqn {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", &self.parts[..self.len])
    }
}
