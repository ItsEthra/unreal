#[cfg(all(feature = "parking_lot", feature = "spin"))]
compile_error!("Only one of `parking_lot` and `spin` features must be enabled");
#[cfg(all(not(feature = "parking_lot"), not(feature = "spin")))]
compile_error!("Either `parking_lot` or `spin` feature must be enabled");

mod macros;

mod cache;
pub use cache::*;
mod utils;
pub use utils::*;
mod api;
pub use api::*;
mod name;
pub use name::*;
mod context;
pub use context::*;
mod object;
pub use object::*;

pub use once_cell::{sync::Lazy as SyncLazy, unsync::Lazy as UnsyncLazy};
