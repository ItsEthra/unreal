#[repr(C)]
pub struct FText {
    vmt: *mut (),
    shared_ref: u32,
    flags: u32,
    _pad: usize,
}
