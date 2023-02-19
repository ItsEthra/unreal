use std::marker::PhantomData;

#[repr(C)]
pub struct TSet<T>([u8; 0x50], PhantomData<T>);
