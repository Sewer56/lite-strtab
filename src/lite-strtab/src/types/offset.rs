//! Backing integer types for byte offsets in [`crate::StringTable`].
//!
//! Offsets index into the contiguous UTF-8 byte buffer and include a final
//! sentinel equal to the total byte length. The chosen [`Offset`] type
//! controls offset-table memory use and maximum total byte size.
//!
//! Default [`u32`] supports up to 4 GiB of string bytes.

use core::fmt::Debug;

/// Contract for integer types used as byte offsets.
///
/// Unlike [`crate::StringIndex`] (which bounds string count), [`Offset`]
/// bounds total UTF-8 byte size. Builders and validators use
/// [`Self::try_from_usize`] for checked growth, while lookup code uses
/// [`Self::to_usize`] for infallible slicing.
///
/// This trait is sealed and only implemented for unsigned integers that fit in
/// [`usize`] on the current target.
pub trait Offset: private::Sealed + Copy + Eq + Ord + Debug + Send + Sync + 'static {
    /// Human-readable type name.
    const TYPE_NAME: &'static str;

    /// Converts a byte length/offset into this type.
    fn try_from_usize(value: usize) -> Option<Self>;

    /// Converts this offset to [`usize`].
    fn to_usize(self) -> usize;
}

macro_rules! impl_offset {
    ($($ty:ty),+ $(,)?) => {
        $(
            impl Offset for $ty {
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
impl_offset!(u8, u16, u32, u64, usize);

#[cfg(target_pointer_width = "32")]
impl_offset!(u8, u16, u32, usize);

#[cfg(target_pointer_width = "16")]
impl_offset!(u8, u16, usize);

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
