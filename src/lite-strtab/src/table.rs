//! Immutable string storage backed by one byte buffer and one offset table.
//!
//! The layout is:
//!
//! - `bytes`: all UTF-8 string bytes concatenated
//! - `offsets`: start offset for each string, plus one final sentinel
//! - strings are not NUL-terminated by default; boundaries come from offsets
//!
//! For `n` strings, `offsets.len() == n + 1`.
//! String `i` is `bytes[offsets[i]..offsets[i + 1]]`.

use core::iter::FusedIterator;
use core::marker::PhantomData;
use core::ops::Range;
use core::str;

use crate::allocator::*;
#[cfg(any(debug_assertions, test))]
use crate::error::{ValidationError, ValidationResult};
use crate::{Offset, StringId, StringIndex};

/// Alias for [`StringTable`].
pub type StringPool<O = u32, I = u16, const NULL_PADDED: bool = false, A = Global> =
    StringTable<O, I, NULL_PADDED, A>;

/// Alias for [`StringTableIter`].
pub type StringPoolIter<'a, O = u32, const NULL_PADDED: bool = false> =
    StringTableIter<'a, O, NULL_PADDED>;

/// Immutable string storage.
///
/// All strings are stored in a single contiguous UTF-8 byte buffer.
/// An offset table maps each [`StringId`] to a byte range.
///
/// The table is immutable once built. This keeps references returned by [`Self::get`]
/// valid for the lifetime of `&self` and avoids mutation-related reallocation issues.
///
/// The offset table always contains one extra value at the end (a sentinel)
/// equal to `bytes.len()`. This allows `get` to resolve a range with two
/// offset reads.
///
/// Generic parameters control capacity and metadata size:
/// - `O` is the byte-offset type (see [`Offset`]). It bounds total UTF-8 bytes and costs
///   `size_of::<O>()` per string inside the [`crate::StringTable`].
/// - `I` is the string-ID type (see [`StringIndex`]) used by [`crate::StringId`].
///   It limits string count and costs `size_of::<I>()` per stored ID field
///   (table index) in your own structs.
///
/// The common choice is `O = u32, I = u16`: meaning `4 GiB` of UTF-8 data
/// and `64Ki` entries per table.
///
/// This is 4 bytes per string offset in the table and 2 bytes
/// per StringID (index into table) inside your structs.
/// For comparison: `Box<str>` == 16 bytes, `String` == 24 bytes.
///
/// By default, inserted strings are not NUL-terminated.
/// Set `NULL_PADDED = true` to store strings with a trailing NUL byte.
///
/// # Example
///
/// ```rust
/// use lite_strtab::StringTableBuilder;
///
/// let mut builder = StringTableBuilder::new();
/// let a = builder.try_push("cat").unwrap();
/// let b = builder.try_push("dog").unwrap();
///
/// let table = builder.build();
/// assert_eq!(table.get(a), Some("cat"));
/// assert_eq!(table.get(b), Some("dog"));
/// ```
pub struct StringTable<
    O = u32,
    I = u16,
    const NULL_PADDED: bool = false,
    A: Allocator + Clone = Global,
> where
    O: Offset,
    I: StringIndex,
{
    bytes: Box<[u8], A>,
    offsets: Box<[O], A>,
    _id: PhantomData<I>,
}

impl StringTable<u32, u16, false, Global> {
    /// Creates an empty table using the global allocator.
    #[inline]
    pub fn empty() -> Self {
        Self::empty_in(Global)
    }
}

impl<O: Offset, I: StringIndex, const NULL_PADDED: bool, A: Allocator + Clone>
    StringTable<O, I, NULL_PADDED, A>
{
    /// Creates an empty table with a custom allocator.
    pub fn empty_in(allocator: A) -> Self {
        let bytes = Vec::new_in(allocator.clone()).into_boxed_slice();
        let mut offsets = Vec::with_capacity_in(1, allocator);
        offsets.push(zero_offset::<O>());

        Self {
            bytes,
            offsets: offsets.into_boxed_slice(),
            _id: PhantomData,
        }
    }

    #[inline]
    pub(crate) fn from_parts_unchecked(bytes: Box<[u8], A>, offsets: Box<[O], A>) -> Self {
        Self {
            bytes,
            offsets,
            _id: PhantomData,
        }
    }

    /// Number of strings in the table.
    #[inline]
    pub fn len(&self) -> usize {
        self.offsets.len().saturating_sub(1)
    }

    /// Returns `true` when the table has no strings.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns the string for a given ID.
    #[inline]
    pub fn get(&self, id: StringId<I>) -> Option<&str> {
        let index = id.into_usize();
        // Failure (None) is unlikely; users typically provide valid indices.
        // Since likely/unlikely isn't stable, we structure this so the
        // success path falls through without jumps, improving pipelining.
        if index < self.len() {
            // SAFETY: Bounds check above guarantees both offset entries exist.
            let start = unsafe { self.offsets.get_unchecked(index) }.to_usize();
            let end = unsafe { self.offsets.get_unchecked(index + 1) }.to_usize();
            // Const generic: default (`false`) folds `saturating_sub(0)` to `end`.
            let logical_end = end.saturating_sub(usize::from(NULL_PADDED));
            debug_assert!(logical_end >= start);

            // SAFETY: Table invariants guarantee this range is in bounds and valid UTF-8.
            let bytes = unsafe { self.bytes.get_unchecked(start..logical_end) };
            // SAFETY: Invariants guarantee all ranges are valid UTF-8.
            Some(unsafe { str::from_utf8_unchecked(bytes) })
        } else {
            None
        }
    }

    /// Returns the string for a given ID without bounds checks.
    ///
    /// # Safety
    ///
    /// `id` must be in bounds (`id < self.len()`).
    #[inline]
    pub unsafe fn get_unchecked(&self, id: StringId<I>) -> &str {
        let index = id.into_usize();
        let start = unsafe { self.offsets.get_unchecked(index) }.to_usize();
        let end = unsafe { self.offsets.get_unchecked(index + 1) }.to_usize();
        // Const generic: default (`false`) folds `saturating_sub(0)` to `end`.
        let logical_end = end.saturating_sub(usize::from(NULL_PADDED));
        debug_assert!(logical_end >= start);
        let bytes = unsafe { self.bytes.get_unchecked(start..logical_end) };

        // SAFETY: Invariants guarantee all ranges are valid UTF-8.
        unsafe { str::from_utf8_unchecked(bytes) }
    }

    /// Returns an iterator over all strings.
    #[inline]
    pub fn iter(&self) -> StringTableIter<'_, O, NULL_PADDED> {
        let offsets = &self.offsets;
        let strings = offsets.len().saturating_sub(1);
        let cur_offset = offsets.as_ptr();

        StringTableIter {
            bytes: &self.bytes,
            cur_offset,
            // SAFETY: `strings` is at most `offsets.len() - 1`, so this stays
            // in-bounds and may equal `cur_offset` for an empty iterator.
            max_offset: unsafe { cur_offset.add(strings) },
            remaining: strings,
            _offsets: PhantomData,
        }
    }

    /// Returns the contiguous byte storage.
    #[inline]
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// Returns `true` if any stored string equals `value`.
    #[inline]
    pub fn contains(&self, value: &str) -> bool {
        self.iter().any(|item| item == value)
    }

    /// Returns the offset table, including the final sentinel.
    #[inline]
    pub fn offsets(&self) -> &[O] {
        &self.offsets
    }

    /// Returns the byte range for a given ID.
    #[inline]
    pub fn byte_range(&self, id: StringId<I>) -> Option<Range<usize>> {
        let index = id.into_usize();
        // Failure (None) is unlikely; users typically provide valid indices.
        // Since likely/unlikely isn't stable, we structure this so the
        // success path falls through without jumps, improving pipelining.
        if index < self.len() {
            // SAFETY: Bounds check above ensures `index` and `index + 1` are valid.
            let start = unsafe { self.offsets.get_unchecked(index) }.to_usize();
            let end = unsafe { self.offsets.get_unchecked(index + 1) }.to_usize();
            // Const generic: default (`false`) folds `saturating_sub(0)` to `end`.
            let logical_end = end.saturating_sub(usize::from(NULL_PADDED));
            debug_assert!(logical_end >= start);

            Some(start..logical_end)
        } else {
            None
        }
    }

    #[cfg(any(debug_assertions, test))]
    pub(crate) fn validate(&self) -> ValidationResult<()> {
        let bytes_len = self.bytes.len();
        if O::try_from_usize(bytes_len).is_none() {
            return Err(ValidationError::TooManyBytesForOffsetType {
                bytes: bytes_len,
                offset_type: O::TYPE_NAME,
            });
        }

        let strings = self.len();
        if strings > 0 && I::try_from_usize(strings - 1).is_none() {
            return Err(ValidationError::TooManyStrings {
                strings,
                id_type: I::TYPE_NAME,
            });
        }

        let offsets = &self.offsets;
        if offsets.is_empty() {
            return Err(ValidationError::MissingSentinelOffset);
        }

        let last_index = offsets.len() - 1;
        let found_last = offsets[last_index].to_usize();
        if found_last != bytes_len {
            return Err(ValidationError::LastOffsetMismatch {
                found: found_last,
                expected: bytes_len,
            });
        }

        let mut previous = 0usize;
        for (index, &offset) in offsets.iter().enumerate() {
            let current = offset.to_usize();

            if current > bytes_len {
                return Err(ValidationError::OffsetOutOfBounds {
                    index,
                    offset: current,
                    bytes_len,
                });
            }

            if index == 0 {
                previous = current;
                continue;
            }

            if current < previous {
                return Err(ValidationError::OffsetsNotMonotonic {
                    index,
                    previous,
                    current,
                });
            }

            if NULL_PADDED {
                if current == previous {
                    return Err(ValidationError::NullPaddedStringMissingTerminatorByte {
                        index: index - 1,
                    });
                }

                let terminator_index = current - 1;
                if self.bytes[terminator_index] != 0 {
                    return Err(ValidationError::NullPaddedStringMissingTrailingNul {
                        index: index - 1,
                    });
                }

                if str::from_utf8(&self.bytes[previous..terminator_index]).is_err() {
                    return Err(ValidationError::InvalidUtf8 { index: index - 1 });
                }
            } else if str::from_utf8(&self.bytes[previous..current]).is_err() {
                return Err(ValidationError::InvalidUtf8 { index: index - 1 });
            }

            previous = current;
        }

        Ok(())
    }
}

/// Iterator returned by [`StringTable::iter`].
pub struct StringTableIter<'a, O: Offset = u32, const NULL_PADDED: bool = false> {
    bytes: &'a [u8],
    cur_offset: *const O,
    max_offset: *const O,
    remaining: usize,
    _offsets: PhantomData<&'a [O]>,
}

impl<'a, O: Offset, const NULL_PADDED: bool> Iterator for StringTableIter<'a, O, NULL_PADDED> {
    type Item = &'a str;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        if self.cur_offset != self.max_offset {
            // SAFETY: `cur_offset != max_offset` guarantees at least one string
            // remains, so both `cur_offset` and `cur_offset + 1` are valid.
            let start = unsafe { (*self.cur_offset).to_usize() };
            self.cur_offset = unsafe { self.cur_offset.add(1) };
            let end = unsafe { (*self.cur_offset).to_usize() };
            self.remaining -= 1;

            // Const generic: default (`false`) folds `saturating_sub(0)` to `end`.
            let logical_end = end.saturating_sub(usize::from(NULL_PADDED));
            debug_assert!(logical_end >= start);

            // SAFETY: Pool invariants guarantee this slice is in bounds and valid UTF-8.
            let bytes = unsafe { self.bytes.get_unchecked(start..logical_end) };
            Some(unsafe { str::from_utf8_unchecked(bytes) })
        } else {
            None
        }
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.len();
        (remaining, Some(remaining))
    }
}

impl<O: Offset, const NULL_PADDED: bool> ExactSizeIterator for StringTableIter<'_, O, NULL_PADDED> {
    #[inline]
    fn len(&self) -> usize {
        self.remaining
    }
}

impl<O: Offset, const NULL_PADDED: bool> FusedIterator for StringTableIter<'_, O, NULL_PADDED> {}

#[inline]
fn zero_offset<O: Offset>() -> O {
    // SAFETY: All built-in integer implementations accept zero.
    unsafe { O::try_from_usize(0).unwrap_unchecked() }
}

#[cfg(test)]
mod tests {
    use crate::allocator::{Global, Vec};
    use crate::error::{ValidationError, ValidationResult};
    use crate::{Offset, StringId, StringIndex, StringTable};

    fn validate_parts<O: Offset, I: StringIndex, const NULL_PADDED: bool>(
        bytes: Vec<u8, Global>,
        offsets: Vec<O, Global>,
    ) -> ValidationResult<()> {
        let table = StringTable::<O, I, NULL_PADDED, Global>::from_parts_unchecked(
            bytes.into_boxed_slice(),
            offsets.into_boxed_slice(),
        );
        table.validate()
    }

    #[test]
    fn validate_rejects_missing_sentinel() {
        let mut bytes = Vec::new_in(Global);
        bytes.extend_from_slice(b"hello");

        let mut offsets = Vec::new_in(Global);
        offsets.push(0u32);

        let result = validate_parts::<u32, u32, false>(bytes, offsets);
        assert!(matches!(
            result,
            Err(ValidationError::LastOffsetMismatch { .. })
        ));
    }

    #[test]
    fn validate_rejects_non_monotonic_offsets() {
        let mut bytes = Vec::new_in(Global);
        bytes.extend_from_slice(b"abcd");

        let mut offsets = Vec::new_in(Global);
        offsets.push(0u32);
        offsets.push(3u32);
        offsets.push(2u32);
        offsets.push(4u32);

        let result = validate_parts::<u32, u32, false>(bytes, offsets);
        assert!(matches!(
            result,
            Err(ValidationError::OffsetsNotMonotonic { .. })
        ));
    }

    #[test]
    fn validate_rejects_invalid_utf8() {
        let mut bytes = Vec::new_in(Global);
        bytes.push(0xFF);

        let mut offsets = Vec::new_in(Global);
        offsets.push(0u32);
        offsets.push(1u32);

        let result = validate_parts::<u32, u32, false>(bytes, offsets);
        assert!(matches!(result, Err(ValidationError::InvalidUtf8 { .. })));
    }

    #[test]
    fn validate_null_padded_accepts_trailing_nul() {
        let mut bytes = Vec::new_in(Global);
        bytes.extend_from_slice(b"hello\0");

        let mut offsets = Vec::new_in(Global);
        offsets.push(0u32);
        offsets.push(6u32);

        let result = validate_parts::<u32, u32, true>(bytes, offsets);
        assert!(result.is_ok());
    }

    #[test]
    fn validate_null_padded_rejects_missing_trailing_nul() {
        let mut bytes = Vec::new_in(Global);
        bytes.extend_from_slice(b"hello");

        let mut offsets = Vec::new_in(Global);
        offsets.push(0u32);
        offsets.push(5u32);

        let result = validate_parts::<u32, u32, true>(bytes, offsets);
        assert!(matches!(
            result,
            Err(ValidationError::NullPaddedStringMissingTrailingNul { .. })
        ));
    }

    #[test]
    fn validate_rejects_offset_type_overflow() {
        let mut bytes = Vec::new_in(Global);
        bytes.extend_from_slice(b"abc");

        let mut offsets = Vec::new_in(Global);
        offsets.push(0u8);
        offsets.push(3u8);

        let result = validate_parts::<u8, u32, false>(bytes, offsets);
        assert!(result.is_ok());

        let mut too_big = Vec::new_in(Global);
        too_big.extend_from_slice(&[0u8; 300]);
        let mut offsets = Vec::new_in(Global);
        offsets.push(0u8);
        offsets.push(u8::MAX);

        let result = validate_parts::<u8, u32, false>(too_big, offsets);
        assert!(matches!(
            result,
            Err(ValidationError::TooManyBytesForOffsetType { .. })
        ));
    }

    #[test]
    fn validate_rejects_id_type_overflow() {
        let bytes = Vec::new_in(Global).into_boxed_slice();
        let mut offsets = Vec::new_in(Global);
        for _ in 0..258 {
            offsets.push(0u32);
        }

        let table = StringTable::<u32, u8>::from_parts_unchecked(bytes, offsets.into_boxed_slice());
        let result = table.validate();
        assert!(matches!(
            result,
            Err(ValidationError::TooManyStrings {
                strings: 257,
                id_type: "u8"
            })
        ));
    }

    #[test]
    fn get_returns_none_for_invalid_id() {
        let table = StringTable::empty();
        assert_eq!(table.get(StringId::new(0)), None);
    }
}
