use crate::TArray;

#[repr(C)]
pub struct FString {
    data: TArray<u16>,
}
