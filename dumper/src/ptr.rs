use bytemuck::{Pod, Zeroable};
use std::{
    fmt,
    ops::{Add, AddAssign, Sub, SubAssign},
};

// Pointer inside target process
#[derive(Zeroable, Pod, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct Ptr(pub usize);

impl Ptr {
    #[inline]
    pub fn is_zero(&self) -> bool {
        self.0 == 0
    }

    #[inline]
    pub fn to_option(self) -> Option<Ptr> {
        if self.is_zero() {
            None
        } else {
            Some(self)
        }
    }
}

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
