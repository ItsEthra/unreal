use std::ptr::NonNull;

#[repr(C)]
pub struct TArray<T> {
    ptr: Option<NonNull<T>>,
    len: u32,
    capacity: u32,
}
