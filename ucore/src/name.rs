#![allow(dead_code)]

use std::{borrow::Cow, marker::PhantomData, str::from_utf8_unchecked};

const NAME_SIZE: usize = 1024;

struct FNameEntryHeader<const WIDE_BIT: usize, const LEN_BIT: usize>(u16);

impl<const WIDE_BIT: usize, const LEN_BIT: usize> FNameEntryHeader<WIDE_BIT, LEN_BIT> {
    #[inline]
    fn is_wide(&self) -> bool {
        (self.0 >> WIDE_BIT) & 1 != 0
    }

    #[inline]
    fn is_ansii(&self) -> bool {
        (self.0 >> WIDE_BIT) & 1 == 0
    }

    fn len(&self) -> usize {
        (self.0 >> LEN_BIT) as usize
    }
}

#[repr(C)]
union FNameEntryData {
    wide: [u16; NAME_SIZE],
    ansi: [u8; NAME_SIZE],
}

impl FNameEntryData {
    #[inline]
    fn as_wide(&self, len: usize) -> &[u16] {
        unsafe { &self.wide[..len] }
    }

    #[inline]
    fn as_ansi(&self, len: usize) -> &[u8] {
        unsafe { &self.ansi[..len] }
    }
}

struct FNameEntry<
    const HEADER: usize,
    const DATA: usize,
    const WIDE_BIT: usize,
    const LEN_BIT: usize,
>(PhantomData<()>);

impl<const HEADER: usize, const DATA: usize, const WIDE_BIT: usize, const LEN_BIT: usize>
    FNameEntry<HEADER, DATA, WIDE_BIT, LEN_BIT>
{
    unsafe fn header(&self) -> &FNameEntryHeader<WIDE_BIT, LEN_BIT> {
        (self as *const Self)
            .cast::<u8>()
            .add(HEADER)
            .cast::<FNameEntryHeader<WIDE_BIT, LEN_BIT>>()
            .as_ref()
            .unwrap()
    }

    unsafe fn data(&self) -> &FNameEntryData {
        (self as *const Self)
            .cast::<u8>()
            .add(DATA)
            .cast::<FNameEntryData>()
            .as_ref()
            .unwrap()
    }

    fn to_str(&self) -> Cow<str> {
        let (header, data) = unsafe { (self.header(), self.data()) };
        let len = header.len();

        unsafe {
            if header.is_wide() {
                from_utf8_unchecked(data.as_ansi(len)).into()
            } else {
                String::from_utf16_lossy(data.as_wide(len)).into()
            }
        }
    }
}

impl<const HEADER: usize, const DATA: usize, const WIDE_BIT: usize, const LEN_BIT: usize>
    PartialEq<str> for FNameEntry<HEADER, DATA, WIDE_BIT, LEN_BIT>
{
    fn eq(&self, other: &str) -> bool {
        let (header, data) = unsafe { (self.header(), self.data()) };
        let len = header.len();

        if header.is_wide() {
            other.len() == header.len()
                && other
                    .encode_utf16()
                    .zip(data.as_wide(len))
                    .all(|(l, r)| l == *r)
        } else {
            other.len() == header.len() && other.as_bytes() == data.as_ansi(len)
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
struct FNameEntryId {
    id: u32,
}

#[repr(C)]
pub struct FName<const SIZE: usize, const INDEX: usize>([u8; SIZE]);

impl<const SIZE: usize, const INDEX: usize> FName<SIZE, INDEX> {}
