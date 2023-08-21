use std::{
    char::decode_utf16,
    fmt,
    marker::PhantomData,
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

#[repr(transparent)]
pub struct FString {
    data: TArray<u16>,
}

impl fmt::Display for FString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for chr in decode_utf16(self.data.iter().copied()) {
            match chr {
                Ok(chr) => write!(f, "{chr}")?,
                Err(_) => write!(f, "{}", char::REPLACEMENT_CHARACTER)?,
            }
        }

        Ok(())
    }
}

impl Deref for FString {
    type Target = TArray<u16>;

    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl DerefMut for FString {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.data
    }
}

impl PartialEq<str> for FString {
    fn eq(&self, other: &str) -> bool {
        self.data.len() == other.len()
            && other
                .encode_utf16()
                .zip(self.data.as_slice())
                .all(|(a, b)| a == *b)
    }
}

#[repr(C)]
pub struct TSet<T>([u8; 0x50], PhantomData<T>);

#[repr(C)]
pub struct TMap<K, V> {
    data: TSet<(K, V)>,
}
