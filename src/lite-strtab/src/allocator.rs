//! Allocator API compatibility imports.
//!
//! This crate uses these re-exports everywhere instead of importing from
//! `allocator-api2` directly. On stable channels, they come from
//! `allocator-api2`. With the `nightly` feature enabled, they come from the
//! unstable allocator API in [`alloc`].

#[cfg(not(feature = "nightly"))]
pub use allocator_api2::alloc::{Allocator, Global};
#[cfg(not(feature = "nightly"))]
pub use allocator_api2::boxed::Box;
#[cfg(not(feature = "nightly"))]
pub use allocator_api2::vec::Vec;

#[cfg(feature = "nightly")]
pub use crate::alloc::alloc::{Allocator, Global};
#[cfg(feature = "nightly")]
pub use crate::alloc::boxed::Box;
#[cfg(feature = "nightly")]
pub use crate::alloc::vec::Vec;
