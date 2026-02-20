#![doc = include_str!(concat!("../", env!("CARGO_PKG_README")))]
#![no_std]
#![cfg_attr(feature = "nightly", feature(allocator_api))]
#![forbid(unsafe_op_in_unsafe_fn)]
#![warn(missing_docs)]

extern crate alloc;
#[cfg(feature = "std")]
extern crate std;

mod allocator;

/// Builder for creating immutable pools.
pub mod builder;
/// Error types.
pub mod error;
/// Immutable pool storage and iteration.
pub mod table;
/// Data and type definitions.
pub mod types;

pub use allocator::{Allocator, Global};
pub use builder::{StringPoolBuilder, StringTableBuilder};
pub use error::{Error, Result};
pub use table::{StringPool, StringPoolIter, StringTable, StringTableIter};
pub use types::{Offset, StringId, StringIndex};
