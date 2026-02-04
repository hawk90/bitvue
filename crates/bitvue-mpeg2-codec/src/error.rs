//! Error types for MPEG-2 Video parsing.

use thiserror::Error;

/// MPEG-2 parsing error types.
#[derive(Debug, Error)]
pub enum Mpeg2Error {
    /// Not enough data to parse.
    #[error("not enough data: expected {expected} bytes, got {got}")]
    NotEnoughData { expected: usize, got: usize },

    /// Invalid start code.
    #[error("invalid start code: {0}")]
    InvalidStartCode(String),

    /// Invalid sequence header.
    #[error("invalid sequence header: {0}")]
    InvalidSequenceHeader(String),

    /// Invalid picture header.
    #[error("invalid picture header: {0}")]
    InvalidPictureHeader(String),

    /// Invalid GOP header.
    #[error("invalid GOP header: {0}")]
    InvalidGopHeader(String),

    /// Invalid slice header.
    #[error("invalid slice header: {0}")]
    InvalidSliceHeader(String),

    /// Bitstream error.
    #[error("bitstream error: {0}")]
    BitstreamError(String),

    /// Generic parse error.
    #[error("parse error: {0}")]
    ParseError(String),
}

/// Result type alias for MPEG-2 operations.
pub type Result<T> = std::result::Result<T, Mpeg2Error>;
