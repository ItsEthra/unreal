use std::{
    alloc::{alloc, Layout},
    char::decode_utf16,
    fmt,
    marker::PhantomData,
    mem::forget,
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

impl<T: fmt::Debug> fmt::Debug for TArray<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.as_slice())
    }
}

impl<T> From<Vec<T>> for TArray<T> {
    fn from(mut vec: Vec<T>) -> Self {
        let (ptr, len, capacity) = (vec.as_mut_ptr(), vec.len() as u32, vec.capacity() as u32);
        forget(vec);

        Self {
            ptr: NonNull::new(ptr),
            capacity,
            len,
        }
    }
}

impl<T: Clone> Clone for TArray<T> {
    fn clone(&self) -> Self {
        if let Some(ptr) = self.ptr {
            unsafe {
                let data = alloc(Layout::array::<T>(self.len as usize).unwrap()).cast::<T>();
                if !data.is_null() {
                    data.copy_from_nonoverlapping(ptr.as_ptr(), self.len as usize);
                }

                Self {
                    ptr: NonNull::new(data),
                    len: self.len,
                    capacity: self.capacity,
                }
            }
        } else {
            Self {
                ptr: None,
                len: 0,
                capacity: 0,
            }
        }
    }
}

impl<T> Drop for TArray<T> {
    fn drop(&mut self) {
        unsafe {
            if let Some(ptr) = self.ptr {
                drop(Vec::from_raw_parts(
                    ptr.as_ptr(),
                    self.len as usize,
                    self.capacity as usize,
                ));
            }
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

#[derive(Clone)]
#[repr(transparent)]
pub struct FString {
    data: TArray<u16>,
}

impl FString {
    #[inline]
    pub fn into_array(self) -> TArray<u16> {
        self.data
    }
}

impl<T: Into<String>> From<T> for FString {
    fn from(value: T) -> Self {
        Self {
            data: TArray::from(value.into().encode_utf16().collect::<Vec<_>>()),
        }
    }
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

impl fmt::Debug for FString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self}")
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

impl<T: AsRef<str>> PartialEq<T> for FString {
    fn eq(&self, other: &T) -> bool {
        self.data.len() == other.as_ref().len()
            && other
                .as_ref()
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

#[cfg(test)]
mod tests {
    use crate::FString;

    #[test]
    fn test_string() {
        let first: FString = "Foo".into();
        let second = first.clone();
        assert_ne!(first.as_ptr(), second.as_ptr());
        assert_eq!(second, "Foo");
        drop(second);
        assert_eq!(first, "Foo");
    }
}
