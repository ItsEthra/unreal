use crate::offsets::Offsets;
use std::marker::PhantomData;

pub struct UField<O: Offsets>(PhantomData<O>);
pub struct FField<O: Offsets>(PhantomData<O>);
