//! Error types for string pool construction and internal validation.

/// Result type used by this crate.
pub type Result<T> = core::result::Result<T, Error>;

/// Errors produced by the public construction API.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Error {
    /// The number of strings exceeded what can be indexed by the configured
    /// string ID type.
    #[error(
        "cannot store {strings} strings: id type '{id_type}' is too small; use a larger StringId type"
    )]
    TooManyStrings {
        /// Attempted number of strings.
        strings: usize,
        /// ID type used by the pool/builder.
        id_type: &'static str,
    },
    /// The total byte length exceeded what can be represented by the chosen
    /// offset type.
    #[error(
        "cannot store {bytes} bytes of string data: offset type '{offset_type}' is too small; use a larger offset type"
    )]
    TooManyBytesForOffsetType {
        /// Attempted byte length.
        bytes: usize,
        /// Offset type used by the pool/builder.
        offset_type: &'static str,
    },
}

#[cfg(any(debug_assertions, test))]
pub(crate) type ValidationResult<T> = core::result::Result<T, ValidationError>;

#[cfg(any(debug_assertions, test))]
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub(crate) enum ValidationError {
    #[error("invalid string table: {strings} strings do not fit in id type '{id_type}'")]
    TooManyStrings {
        strings: usize,
        id_type: &'static str,
    },
    #[error("invalid string table: {bytes} bytes do not fit in offset type '{offset_type}'")]
    TooManyBytesForOffsetType {
        bytes: usize,
        offset_type: &'static str,
    },
    #[error("invalid string table: offsets must end with a sentinel equal to total byte length")]
    MissingSentinelOffset,
    #[error("invalid string table: final offset is {found}, but byte length is {expected}")]
    LastOffsetMismatch { found: usize, expected: usize },
    #[error("invalid string table: offset[{index}] = {offset} is out of bounds (byte length {bytes_len})")]
    OffsetOutOfBounds {
        index: usize,
        offset: usize,
        bytes_len: usize,
    },
    #[error(
        "invalid string table: offsets must be non-decreasing; offset[{index}] = {current}, previous = {previous}"
    )]
    OffsetsNotMonotonic {
        index: usize,
        previous: usize,
        current: usize,
    },
    #[error("invalid string table: bytes for string index {index} are not valid UTF-8")]
    InvalidUtf8 { index: usize },
    #[error("invalid string table: string index {index} in null-padded mode has no trailing byte")]
    NullPaddedStringMissingTerminatorByte { index: usize },
    #[error(
        "invalid string table: string index {index} in null-padded mode must end with a NUL byte"
    )]
    NullPaddedStringMissingTrailingNul { index: usize },
}
