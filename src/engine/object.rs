use crate::offsets::Offsets;
use std::marker::PhantomData;

pub struct UObject<O: Offsets>(PhantomData<O>);
