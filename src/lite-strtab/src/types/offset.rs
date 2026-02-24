//! Backing integer types for byte offsets in [`crate::StringTable`].
//!
//! Offsets index into the contiguous UTF-8 byte buffer and include a final
//! sentinel equal to the total byte length. The chosen [`Offset`] type
//! controls offset-table memory use and maximum total byte size.
//!
//! Default [`u32`] supports up to 4 GiB of string bytes.

/// Contract for integer types used as byte offsets.
///
/// Unlike [`crate::StringIndex`] (which bounds string count), [`Offset`]
/// bounds total UTF-8 byte size. Builders and validators use
/// [`Self::try_from_usize`] for checked growth, while lookup code uses
/// [`Self::to_usize`] for infallible slicing.
///
/// # Implementing this trait
///
/// This trait is already implemented for primitive unsigned integers (`u8`, `u16`, `u32`,
/// `u64`, `usize`). To implement it for custom wrapper types, use the
/// [`impl_offset`](crate::impl_offset) macro:
///
/// ```
/// use lite_strtab::impl_offset;
///
/// #[derive(Clone, Copy)]
/// #[repr(transparent)]
/// struct ByteOffset(u32);
///
/// impl_offset!(ByteOffset: u32);
/// ```
pub trait Offset: Copy + Send + Sync + 'static {
    /// Human-readable type name.
    const TYPE_NAME: &'static str;

    /// Converts a byte length/offset into this type.
    fn try_from_usize(value: usize) -> Option<Self>;

    /// Converts this offset to [`usize`].
    fn to_usize(self) -> usize;
}

/// Implements [`Offset`] for one or more types.
///
/// # Examples
///
/// For wrapper types with an inner integer type:
///
/// ```
/// use lite_strtab::impl_offset;
///
/// #[derive(Clone, Copy)]
/// #[repr(transparent)]
/// struct ByteOffset(u32);
///
/// impl_offset!(ByteOffset: u32);
/// ```
#[macro_export]
macro_rules! impl_offset {
    // Pattern for wrapper types: Type: InnerType
    ($wrapper:ty: $inner:ty) => {
        impl $crate::Offset for $wrapper {
            const TYPE_NAME: &'static str = stringify!($wrapper);

            #[inline]
            fn try_from_usize(value: usize) -> Option<Self> {
                <$inner as $crate::Offset>::try_from_usize(value).map(Self)
            }

            #[inline]
            fn to_usize(self) -> usize {
                <$inner as $crate::Offset>::to_usize(self.0)
            }
        }
    };

    // Pattern for multiple wrapper types
    ($wrapper:ty: $inner:ty, $($rest:tt)*) => {
        $crate::impl_offset!($wrapper: $inner);
        $crate::impl_offset!($($rest)*);
    };

    // Pattern for primitive types
    ($($ty:ty),+ $(,)?) => {
        $(
            impl $crate::Offset for $ty {
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
crate::impl_offset!(u8, u16, u32, u64, usize);

#[cfg(target_pointer_width = "32")]
crate::impl_offset!(u8, u16, u32, usize);

#[cfg(target_pointer_width = "16")]
crate::impl_offset!(u8, u16, usize);

// Supported pointer widths for this crate.
#[cfg(not(any(
    target_pointer_width = "16",
    target_pointer_width = "32",
    target_pointer_width = "64"
)))]
compile_error!("lite-strtab requires a 16-bit, 32-bit, or 64-bit target");
