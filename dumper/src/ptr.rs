use std::{
    fmt,
    ops::{Add, AddAssign, Sub, SubAssign},
};

// Pointer inside target process
#[derive(Clone, Copy)]
pub struct Ptr(pub usize);

impl Add<usize> for Ptr {
    type Output = Self;

    #[inline]
    fn add(self, rhs: usize) -> Self::Output {
        Self(self.0 + rhs)
    }
}

impl AddAssign<usize> for Ptr {
    #[inline]
    fn add_assign(&mut self, rhs: usize) {
        self.0 += rhs
    }
}

impl Sub<usize> for Ptr {
    type Output = Self;

    #[inline]
    fn sub(self, rhs: usize) -> Self::Output {
        Self(self.0 - rhs)
    }
}

impl SubAssign<usize> for Ptr {
    fn sub_assign(&mut self, rhs: usize) {
        self.0 -= rhs;
    }
}

impl fmt::Debug for Ptr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "0x{:X}", self.0)
    }
}

impl From<usize> for Ptr {
    #[inline]
    fn from(value: usize) -> Self {
        Self(value)
    }
}
