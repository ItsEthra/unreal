#[repr(transparent)]
pub struct UObject<const SIZE: usize, const PROCESS_EVENT: usize>([u8; SIZE]);
