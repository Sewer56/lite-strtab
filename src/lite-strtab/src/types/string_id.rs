//! Typed string handles used to query a [`crate::StringTable`].
//!
//! [`StringId`] is returned by [`crate::StringTableBuilder::try_push`] and passed to
//! lookup APIs such as [`crate::StringTable::get`]. It wraps a
//! [`crate::StringIndex`] integer so ID storage can be tuned without changing
//! call sites.

use super::StringIndex;
use core::fmt;

/// Identifier for one string in a [`crate::StringTable`].
///
/// `I` is the backing integer type (default [`u32`]) and must implement
/// [`crate::StringIndex`]. The value is table-local and indexes the table's
/// offset entries.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StringId<I = u32>(I);

impl<I> StringId<I> {
    /// Creates a new ID from a raw value.
    #[inline]
    pub const fn new(raw: I) -> Self {
        Self(raw)
    }

    /// Returns the raw index value.
    #[inline]
    pub fn into_raw(self) -> I {
        self.0
    }
}

impl<I: StringIndex> StringId<I> {
    /// Returns the value as [`usize`].
    #[inline]
    pub fn into_usize(self) -> usize {
        self.0.to_usize()
    }
}

impl StringId<u32> {
    /// Returns the raw [`u32`] value.
    #[inline]
    pub const fn into_u32(self) -> u32 {
        self.0
    }
}

impl<I> From<I> for StringId<I> {
    #[inline]
    fn from(value: I) -> Self {
        Self(value)
    }
}

/// Macro pattern:
/// - `$($ty:ty),+` - One or more comma-separated types
/// - `$(,)?` - Optional trailing comma
/// - `$( ... )+` wrapper repeats the inner block for each type
macro_rules! impl_raw_from_string_id {
    ($($ty:ty),+ $(,)?) => {
        $(
            impl From<StringId<$ty>> for $ty {
                #[inline]
                fn from(value: StringId<$ty>) -> Self {
                    value.0
                }
            }
        )+
    };
}

impl_raw_from_string_id!(u8, u16, u32, u64, usize);

impl<I: fmt::Display> fmt::Display for StringId<I> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}
