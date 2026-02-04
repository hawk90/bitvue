//! VVC parser error types.

use thiserror::Error;

/// Result type for VVC parsing operations.
pub type Result<T> = std::result::Result<T, VvcError>;

/// VVC parsing errors.
///
/// This now uses the shared `CodecError` from bitvue-core for consistency
/// across all codec parsers, maintaining backward compatibility with existing code.
#[derive(Error, Debug)]
pub enum VvcError {
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

impl From<bitvue_core::CodecError> for VvcError {
    fn from(err: bitvue_core::CodecError) -> Self {
        match err {
            bitvue_core::CodecError::UnexpectedEof { codec: _, position } => {
                VvcError::UnexpectedEof(position)
            }
            bitvue_core::CodecError::InvalidData { codec: _, message } => {
                VvcError::InvalidData(message)
            }
            bitvue_core::CodecError::InsufficientData {
                codec: _,
                expected,
                actual,
            } => VvcError::InsufficientData { expected, actual },
            bitvue_core::CodecError::Io { codec: _, source } => VvcError::Io(source),
            bitvue_core::CodecError::Parse {
                codec: _,
                offset,
                message,
            } => VvcError::Parse { offset, message },
            bitvue_core::CodecError::Unsupported { codec: _, feature } => {
                VvcError::InvalidData(format!("Unsupported: {}", feature))
            }
            bitvue_core::CodecError::MissingParameter {
                codec: _,
                parameter,
            } => VvcError::InvalidData(format!("Missing parameter: {}", parameter)),
            bitvue_core::CodecError::CodecSpecific { codec: _, message } => {
                VvcError::InvalidData(message)
            }
        }
    }
}
