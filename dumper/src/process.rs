use eyre::Error;
use std::{fmt, slice::from_raw_parts_mut};

// Pointer inside target process
#[derive(Clone, Copy)]
pub struct Ptr(pub usize);

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

pub trait Process {
    fn new(id: u32) -> Result<Self, Error>
    where
        Self: Sized;

    unsafe fn read_val(&self, address: Ptr, value: *mut (), size: usize) -> Result<(), Error>;
    fn read_buf(&self, address: Ptr, buf: &mut [u8]) -> Result<(), Error>;
}

pub struct ExternalProcess(memflex::external::OwnedProcess);

impl Process for ExternalProcess {
    fn new(id: u32) -> Result<Self, Error> {
        memflex::external::find_process_by_id(id)
            .map(Self)
            .map_err(Into::into)
    }

    unsafe fn read_val(&self, Ptr(address): Ptr, value: *mut (), size: usize) -> Result<(), Error> {
        self.0
            .read_buf(address, from_raw_parts_mut(value as _, size))
            .map(|_| ())
            .map_err(Into::into)
    }

    fn read_buf(&self, Ptr(address): Ptr, buf: &mut [u8]) -> Result<(), Error> {
        self.0
            .read_buf(address, buf)
            .map(|_| ())
            .map_err(Into::into)
    }
}
