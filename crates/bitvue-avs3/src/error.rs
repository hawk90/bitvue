//! AVS3 parser error types.

use thiserror::Error;

/// AVS3 parser result type.
pub type Result<T> = std::result::Result<T, Avs3Error>;

/// Errors produced by the AVS3 parser.
#[derive(Debug, Error)]
pub enum Avs3Error {
    #[error("Stream too short (need {need} bytes, have {have})")]
    TooShort { need: usize, have: usize },

    #[error("Invalid start code at offset {offset:#x}")]
    InvalidStartCode { offset: usize },

    #[error("Invalid sequence header: {0}")]
    InvalidSequenceHeader(String),

    #[error("Invalid picture header: {0}")]
    InvalidPictureHeader(String),

    #[error("Unsupported profile {0:#x}")]
    UnsupportedProfile(u8),

    #[error("Unsupported level {0}")]
    UnsupportedLevel(u8),

    #[error("Bitreader error: {0}")]
    BitreaderError(String),

    #[error("Unexpected end of NAL unit")]
    UnexpectedEof,
}
