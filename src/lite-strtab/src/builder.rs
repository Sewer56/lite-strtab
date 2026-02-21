//! Builder for creating an immutable [`crate::StringTable`].
//!
//! The builder stores data in growable vectors while constructing.
//! [`StringTableBuilder::build`] converts those vectors to boxed slices,
//! making the final table immutable and compact.

use core::marker::PhantomData;

use crate::allocator::*;
use crate::{Error, Offset, Result, StringId, StringIndex, StringTable};

/// Alias for [`StringTableBuilder`].
pub type StringPoolBuilder<O = u32, I = u32, A = Global, const NULL_PADDED: bool = false> =
    StringTableBuilder<O, I, A, NULL_PADDED>;

/// Incremental builder for [`crate::StringTable`].
///
/// Each call to [`Self::try_push`] appends string bytes to a single byte buffer and
/// appends one offset.
///
/// By default offsets and IDs use [`u32`], and inserted strings are not
/// NUL-terminated. Set `NULL_PADDED = true` to store strings with a trailing
/// NUL byte.
pub struct StringTableBuilder<
    O = u32,
    I = u32,
    A: Allocator + Clone = Global,
    const NULL_PADDED: bool = false,
> where
    O: Offset,
    I: StringIndex,
{
    bytes: Vec<u8, A>,
    offsets: Vec<O, A>,
    _id: PhantomData<I>,
}

impl StringTableBuilder<u32, u32, Global, false> {
    /// Creates an empty builder using the global allocator.
    #[inline]
    pub fn new() -> Self {
        Self::new_in(Global)
    }

    /// Creates a builder with reserved capacities using the global allocator.
    ///
    /// `strings` is the expected number of strings, `bytes` is the expected
    /// total number of UTF-8 bytes.
    #[inline]
    pub fn with_capacity(strings: usize, bytes: usize) -> Self {
        Self::with_capacity_in(strings, bytes, Global)
    }
}

impl Default for StringTableBuilder<u32, u32, Global, false> {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl<O: Offset, I: StringIndex, A: Allocator + Clone, const NULL_PADDED: bool>
    StringTableBuilder<O, I, A, NULL_PADDED>
{
    /// Creates an empty builder with a custom allocator.
    pub fn new_in(allocator: A) -> Self {
        let mut offsets = Vec::with_capacity_in(1, allocator.clone());
        offsets.push(zero_offset::<O>());

        Self {
            bytes: Vec::new_in(allocator),
            offsets,
            _id: PhantomData,
        }
    }

    /// Creates a builder with reserved capacities and a custom allocator.
    ///
    /// `strings` is the expected number of strings, `bytes` is the expected
    /// total number of UTF-8 bytes.
    pub fn with_capacity_in(strings: usize, bytes: usize, allocator: A) -> Self {
        let mut offsets = Vec::with_capacity_in(strings.saturating_add(1), allocator.clone());
        offsets.push(zero_offset::<O>());

        Self {
            bytes: Vec::with_capacity_in(bytes, allocator),
            offsets,
            _id: PhantomData,
        }
    }

    /// Number of strings currently pushed.
    #[inline]
    pub fn len(&self) -> usize {
        self.offsets.len().saturating_sub(1)
    }

    /// Returns `true` when the builder has no strings.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Current total byte length of pushed string data.
    #[inline]
    pub fn bytes_len(&self) -> usize {
        self.bytes.len()
    }

    /// Appends a string and returns its [`StringId`].
    ///
    /// Returns an error when total string count exceeds the configured ID
    /// type, or when the byte length cannot be represented by the configured
    /// offset type.
    pub fn try_push(&mut self, value: &str) -> Result<StringId<I>> {
        let id = self.len();
        let id_value = I::try_from_usize(id).ok_or(Error::TooManyStrings {
            strings: id.saturating_add(1),
            id_type: I::TYPE_NAME,
        })?;

        let start = self.bytes.len();
        let end = start
            .checked_add(value.len())
            .ok_or(Error::TooManyBytesForOffsetType {
                bytes: start,
                offset_type: O::TYPE_NAME,
            })?;
        // Branch resolved at compile time; no runtime cost.
        let end = if NULL_PADDED {
            end.checked_add(1).ok_or(Error::TooManyBytesForOffsetType {
                bytes: start,
                offset_type: O::TYPE_NAME,
            })?
        } else {
            end
        };

        let end_offset = O::try_from_usize(end).ok_or(Error::TooManyBytesForOffsetType {
            bytes: end,
            offset_type: O::TYPE_NAME,
        })?;

        self.bytes.extend_from_slice(value.as_bytes());
        if NULL_PADDED {
            self.bytes.push(0);
        }
        self.offsets.push(end_offset);
        Ok(StringId::new(id_value))
    }

    /// Finalizes into an immutable [`crate::StringTable`].
    ///
    /// This does not copy string bytes. Internal vectors are converted into
    /// boxed slices so the resulting table is immutable and compact.
    #[inline]
    pub fn build(self) -> StringTable<O, I, A, NULL_PADDED> {
        let table = StringTable::from_parts_unchecked(
            self.bytes.into_boxed_slice(),
            self.offsets.into_boxed_slice(),
        );
        #[cfg(any(debug_assertions, test))]
        debug_assert!(table.validate().is_ok());
        table
    }
}

#[inline]
fn zero_offset<O: Offset>() -> O {
    // SAFETY: All built-in integer implementations accept zero.
    unsafe { O::try_from_usize(0).unwrap_unchecked() }
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use crate::allocator::Global;
    use crate::{Error, StringId, StringTableBuilder};

    #[test]
    fn empty_table() {
        let table = StringTableBuilder::new().build();
        assert_eq!(table.len(), 0);
        assert!(table.is_empty());
        assert_eq!(table.as_bytes(), b"");
        assert_eq!(table.offsets(), &[0u32]);
    }

    #[test]
    fn single_string() {
        let mut builder = StringTableBuilder::new();
        let id = builder.try_push("hello").unwrap();
        let table = builder.build();

        assert_eq!(id, StringId::new(0));
        assert_eq!(table.len(), 1);
        assert_eq!(table.get(id), Some("hello"));
        assert_eq!(table.offsets(), &[0u32, 5u32]);
    }

    #[test]
    fn null_padded_single_string() {
        let mut builder = StringTableBuilder::<u32, u32, Global, true>::new_in(Global);
        let id = builder.try_push("hello").unwrap();
        let table = builder.build();

        assert_eq!(table.get(id), Some("hello"));
        assert_eq!(table.as_bytes(), b"hello\0");
        assert_eq!(table.offsets(), &[0u32, 6u32]);
        assert_eq!(table.byte_range(id), Some(0..5));
    }

    #[test]
    fn null_padded_empty_string() {
        let mut builder = StringTableBuilder::<u32, u32, Global, true>::new_in(Global);
        let id = builder.try_push("").unwrap();
        let table = builder.build();

        assert_eq!(table.get(id), Some(""));
        assert_eq!(table.as_bytes(), b"\0");
        assert_eq!(table.offsets(), &[0u32, 1u32]);
        assert_eq!(table.byte_range(id), Some(0..0));
    }

    #[test]
    fn multiple_with_empty_string() {
        let mut builder = StringTableBuilder::new();
        let a = builder.try_push("a").unwrap();
        let b = builder.try_push("").unwrap();
        let c = builder.try_push("ccc").unwrap();
        let table = builder.build();

        assert_eq!(table.get(a), Some("a"));
        assert_eq!(table.get(b), Some(""));
        assert_eq!(table.get(c), Some("ccc"));
        assert_eq!(table.offsets(), &[0u32, 1u32, 1u32, 4u32]);
    }

    #[test]
    fn unicode_strings() {
        let mut builder = StringTableBuilder::new();
        let a = builder.try_push("猫").unwrap();
        let b = builder.try_push("дом").unwrap();
        let c = builder.try_push("music/曲").unwrap();
        let table = builder.build();

        assert_eq!(table.get(a), Some("猫"));
        assert_eq!(table.get(b), Some("дом"));
        assert_eq!(table.get(c), Some("music/曲"));
    }

    #[test]
    fn iter_matches_insert_order() {
        let mut builder = StringTableBuilder::new();
        builder.try_push("z").unwrap();
        builder.try_push("a").unwrap();
        builder.try_push("m").unwrap();

        let table = builder.build();
        let got: alloc::vec::Vec<&str> = table.iter().collect();
        assert_eq!(got, alloc::vec!["z", "a", "m"]);
    }

    #[test]
    fn supports_custom_allocator() {
        let mut builder = StringTableBuilder::<u32>::new_in(Global);
        let id = builder.try_push("hello").unwrap();
        let table = builder.build();
        assert_eq!(table.get(id), Some("hello"));
    }

    #[test]
    fn supports_small_offset_type() {
        let mut builder = StringTableBuilder::<u8>::new_in(Global);
        let id = builder.try_push("abc").unwrap();
        let table = builder.build();
        assert_eq!(table.get(id), Some("abc"));
        assert_eq!(table.offsets(), &[0u8, 3u8]);
    }

    #[test]
    fn supports_small_id_type() {
        let mut builder = StringTableBuilder::<u32, u8>::new_in(Global);
        let id = builder.try_push("abc").unwrap();
        let table = builder.build();

        assert_eq!(id, StringId::<u8>::new(0));
        assert_eq!(table.get(id), Some("abc"));
    }

    #[test]
    fn small_offset_type_reports_overflow() {
        let mut builder = StringTableBuilder::<u8>::new_in(Global);
        let long = "a".repeat(300);
        let result = builder.try_push(&long);

        assert!(matches!(
            result,
            Err(Error::TooManyBytesForOffsetType {
                offset_type: "u8",
                ..
            })
        ));
    }

    #[test]
    fn small_id_type_reports_overflow() {
        let mut builder = StringTableBuilder::<u32, u8>::new_in(Global);
        for _ in 0..=u8::MAX {
            builder.try_push("a").unwrap();
        }

        let result = builder.try_push("overflow");
        assert!(matches!(
            result,
            Err(Error::TooManyStrings { id_type: "u8", .. })
        ));
    }

    proptest! {
        #[test]
        fn roundtrip_vec_of_strings(values in proptest::collection::vec(".*", 0..256)) {
            let mut builder = StringTableBuilder::new();
            let mut ids = alloc::vec::Vec::with_capacity(values.len());

            for value in &values {
                ids.push(builder.try_push(value).unwrap());
            }

            let table = builder.build();

            prop_assert_eq!(table.len(), values.len());
            prop_assert_eq!(table.offsets().len(), values.len() + 1);

            for (index, value) in values.iter().enumerate() {
                let id = ids[index];
                prop_assert_eq!(table.get(id), Some(value.as_str()));
                prop_assert_eq!(
                    table.byte_range(id).map(|r| &table.as_bytes()[r]),
                    Some(value.as_bytes())
                );
            }
        }
    }
}
