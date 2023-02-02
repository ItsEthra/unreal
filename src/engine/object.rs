use crate::offsets::{OfFUObjectItem, Offsets};
use std::marker::PhantomData;

pub struct FUObjetcItem<O: Offsets>([u8; O::FUObjectItem::SIZE], PhantomData<O>)
// TODO: rustc bug
where
    [u8; O::FUObjectItem::SIZE]: Sized;
