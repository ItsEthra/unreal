use crate::TArray;

#[repr(C)]
pub struct TMap<K, V> {
    data: TArray<(K, V)>,
}
