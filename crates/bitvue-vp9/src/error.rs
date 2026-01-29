//! VP9 parser error types.

use thiserror::Error;

/// Result type for VP9 parsing operations.
pub type Result<T> = std::result::Result<T, Vp9Error>;

/// VP9 parsing errors.
///
/// This now uses the shared `CodecError` from bitvue-core for consistency
/// across all codec parsers, maintaining backward compatibility with existing code.
#[derive(Error, Debug)]
pub enum Vp9Error {
    /// Unexpected end of data.
    #[error("Unexpected end of data at position {0}")]
    UnexpectedEof(u64),

    /// Invalid data encountered.
    #[error("Invalid data: {0}")]
    InvalidData(String),

    /// Insufficient data for operation.
    #[error("Insufficient data: expected {expected} bytes, got {actual}")]
    InsufficientData { expected: usize, actual: usize },

    /// IO error.
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Parse error at specific offset.
    #[error("Parse error at offset {offset}: {message}")]
    Parse { offset: u64, message: String },
}

impl From<bitvue_core::CodecError> for Vp9Error {
    fn from(err: bitvue_core::CodecError) -> Self {
        match err {
            bitvue_core::CodecError::UnexpectedEof { codec: _, position } => {
                Vp9Error::UnexpectedEof(position)
            }
            bitvue_core::CodecError::InvalidData { codec: _, message } => {
                Vp9Error::InvalidData(message)
            }
            bitvue_core::CodecError::InsufficientData { codec: _, expected, actual } => {
                Vp9Error::InsufficientData { expected, actual }
            }
            bitvue_core::CodecError::Io { codec: _, source } => {
                Vp9Error::Io(source)
            }
            bitvue_core::CodecError::Parse { codec: _, offset, message } => {
                Vp9Error::Parse { offset, message }
            }
            bitvue_core::CodecError::Unsupported { codec: _, feature } => {
                Vp9Error::InvalidData(format!("Unsupported: {}", feature))
            }
            bitvue_core::CodecError::MissingParameter { codec: _, parameter } => {
                Vp9Error::InvalidData(format!("Missing parameter: {}", parameter))
            }
            bitvue_core::CodecError::CodecSpecific { codec: _, message } => {
                Vp9Error::InvalidData(message)
            }
        }
    }
}
