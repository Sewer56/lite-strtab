//! Backing integer types for [`crate::StringId`].
//!
//! [`crate::StringId`] is the semantic handle used by the API, while
//! [`StringIndex`] defines which integer types can store its raw value.
//! Smaller types reduce memory per ID; larger types increase maximum string
//! count.
//!
//! Default [`u16`] supports up to 65_536 strings per table.

use core::fmt::{Debug, Display};

/// Contract for integer types used by [`crate::StringId`].
///
/// [`Self::try_from_usize`] is used at build and validation boundaries where
/// counts are computed as [`usize`]. [`Self::to_usize`] is used on lookup paths
/// for infallible indexing.
///
/// This trait is sealed and only implemented for unsigned integers that fit in
/// [`usize`] on the current target.
pub trait StringIndex:
    private::Sealed + Copy + Eq + Ord + Debug + Display + Send + Sync + 'static
{
    /// Human-readable type name.
    const TYPE_NAME: &'static str;

    /// Converts a string index into this type.
    fn try_from_usize(value: usize) -> Option<Self>;

    /// Converts this index to [`usize`].
    fn to_usize(self) -> usize;
}

/// Macro pattern:
/// - `$($ty:ty),+` - One or more comma-separated types
/// - `$(,)?` - Optional trailing comma
/// - `$( ... )+` wrapper repeats the inner block for each type
macro_rules! impl_string_index {
    ($($ty:ty),+ $(,)?) => {
        $(
            impl StringIndex for $ty {
                const TYPE_NAME: &'static str = stringify!($ty);

                #[inline]
                fn try_from_usize(value: usize) -> Option<Self> {
                    <Self as core::convert::TryFrom<usize>>::try_from(value).ok()
                }

                #[inline]
                fn to_usize(self) -> usize {
                    self as usize
                }
            }
        )+
    };
}

#[cfg(target_pointer_width = "64")]
impl_string_index!(u8, u16, u32, u64, usize);

#[cfg(target_pointer_width = "32")]
impl_string_index!(u8, u16, u32, usize);

#[cfg(target_pointer_width = "16")]
impl_string_index!(u8, u16, usize);

// Supported pointer widths for this crate.
#[cfg(not(any(
    target_pointer_width = "16",
    target_pointer_width = "32",
    target_pointer_width = "64"
)))]
compile_error!("lite-strtab requires a 16-bit, 32-bit, or 64-bit target");

// Seal the trait so only this crate can define valid implementations.
mod private {
    pub trait Sealed {}

    macro_rules! impl_sealed {
        ($($ty:ty),+ $(,)?) => {
            $(
                impl Sealed for $ty {}
            )+
        };
    }

    #[cfg(target_pointer_width = "64")]
    impl_sealed!(u8, u16, u32, u64, usize);

    #[cfg(target_pointer_width = "32")]
    impl_sealed!(u8, u16, u32, usize);

    #[cfg(target_pointer_width = "16")]
    impl_sealed!(u8, u16, usize);
}
