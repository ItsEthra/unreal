use crate::TSet;

#[repr(C)]
pub struct TMap<K, V> {
    data: TSet<(K, V)>,
}
