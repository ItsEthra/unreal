use std::ptr::NonNull;

#[repr(C)]
pub struct TSharedRef<T> {
    ptr: Option<NonNull<T>>,
}
