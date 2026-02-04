//! HEVC parser error types.

use thiserror::Error;

/// Result type for HEVC parsing operations.
pub type Result<T> = std::result::Result<T, HevcError>;

/// HEVC parsing errors.
///
/// This now uses the shared `CodecError` from bitvue-core for consistency
/// across all codec parsers, maintaining backward compatibility with existing code.
#[derive(Error, Debug)]
pub enum HevcError {
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

impl From<bitvue_core::CodecError> for HevcError {
    fn from(err: bitvue_core::CodecError) -> Self {
        match err {
            bitvue_core::CodecError::UnexpectedEof { codec: _, position } => {
                HevcError::UnexpectedEof(position)
            }
            bitvue_core::CodecError::InvalidData { codec: _, message } => {
                HevcError::InvalidData(message)
            }
            bitvue_core::CodecError::InsufficientData {
                codec: _,
                expected,
                actual,
            } => HevcError::InsufficientData { expected, actual },
            bitvue_core::CodecError::Io { codec: _, source } => HevcError::Io(source),
            bitvue_core::CodecError::Parse {
                codec: _,
                offset,
                message,
            } => HevcError::Parse { offset, message },
            bitvue_core::CodecError::Unsupported { codec: _, feature } => {
                HevcError::InvalidData(format!("Unsupported: {}", feature))
            }
            bitvue_core::CodecError::MissingParameter {
                codec: _,
                parameter,
            } => HevcError::InvalidData(format!("Missing parameter: {}", parameter)),
            bitvue_core::CodecError::CodecSpecific { codec: _, message } => {
                HevcError::InvalidData(message)
            }
        }
    }
}
