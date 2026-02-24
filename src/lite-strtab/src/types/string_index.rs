//! Backing integer types for [`crate::StringId`].
//!
//! [`crate::StringId`] is the semantic handle used by the API, while
//! [`StringIndex`] defines which integer types can store its raw value.
//! Smaller types reduce memory per ID; larger types increase maximum string
//! count.
//!
//! Default [`u16`] supports up to 65_536 strings per table.

/// Contract for integer types used by [`crate::StringId`].
///
/// [`Self::try_from_usize`] is used at build and validation boundaries where
/// counts are computed as [`usize`]. [`Self::to_usize`] is used on lookup paths
/// for infallible indexing.
///
/// # Implementing this trait
///
/// This trait is already implemented for primitive unsigned integers (`u8`, `u16`, `u32`,
/// `u64`, `usize`). To implement it for custom wrapper types, use the
/// [`impl_string_index`](crate::impl_string_index) macro:
///
/// ```
/// use lite_strtab::impl_string_index;
///
/// #[derive(Clone, Copy)]
/// #[repr(transparent)]
/// struct ProviderIdx(u16);
///
/// impl_string_index!(ProviderIdx: u16);
/// ```
pub trait StringIndex: Copy + Send + Sync + 'static {
    /// Human-readable type name.
    const TYPE_NAME: &'static str;

    /// Converts a string index into this type.
    fn try_from_usize(value: usize) -> Option<Self>;

    /// Converts this index to [`usize`].
    fn to_usize(self) -> usize;
}

/// Implements [`StringIndex`] for one or more types.
///
/// # Examples
///
/// For wrapper types with an inner integer type:
///
/// ```
/// use lite_strtab::impl_string_index;
///
/// #[derive(Clone, Copy)]
/// #[repr(transparent)]
/// struct ProviderIdx(u16);
///
/// impl_string_index!(ProviderIdx: u16);
/// ```
#[macro_export]
macro_rules! impl_string_index {
    // Pattern for wrapper types: Type: InnerType
    ($wrapper:ty: $inner:ty) => {
        impl $crate::StringIndex for $wrapper {
            const TYPE_NAME: &'static str = stringify!($wrapper);

            #[inline]
            fn try_from_usize(value: usize) -> Option<Self> {
                <$inner as $crate::StringIndex>::try_from_usize(value).map(Self)
            }

            #[inline]
            fn to_usize(self) -> usize {
                <$inner as $crate::StringIndex>::to_usize(self.0)
            }
        }
    };

    // Pattern for multiple wrapper types
    ($wrapper:ty: $inner:ty, $($rest:tt)*) => {
        $crate::impl_string_index!($wrapper: $inner);
        $crate::impl_string_index!($($rest)*);
    };

    // Pattern for primitive types
    ($($ty:ty),+ $(,)?) => {
        $(
            impl $crate::StringIndex for $ty {
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
crate::impl_string_index!(u8, u16, u32, u64, usize);

#[cfg(target_pointer_width = "32")]
crate::impl_string_index!(u8, u16, u32, usize);

#[cfg(target_pointer_width = "16")]
crate::impl_string_index!(u8, u16, usize);

// Supported pointer widths for this crate.
#[cfg(not(any(
    target_pointer_width = "16",
    target_pointer_width = "32",
    target_pointer_width = "64"
)))]
compile_error!("lite-strtab requires a 16-bit, 32-bit, or 64-bit target");
