use crate::offsets::Offsets;
use std::marker::PhantomData;

pub struct FUObjetcItem<O: Offsets>(PhantomData<O>);
