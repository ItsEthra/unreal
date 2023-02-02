use crate::offsets::Offsets;
use std::marker::PhantomData;

pub struct UStruct<O: Offsets>(PhantomData<O>);
