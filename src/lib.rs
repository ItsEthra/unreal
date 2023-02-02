#![feature(generic_const_exprs)]
#![feature(associated_type_defaults)]
#![allow(incomplete_features)]

mod engine;
pub use engine::*;

mod utils;
pub(crate) use utils::*;

mod macros;

pub mod offsets;

#[doc(hidden)]
pub use paste::paste as __paste;
