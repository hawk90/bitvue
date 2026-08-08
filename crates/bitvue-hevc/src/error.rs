//! HEVC parser error types.

use thiserror::Error;

/// Result type for HEVC parsing operations.
pub type Result<T> = std::result::Result<T, HevcError>;

/// HEVC parsing errors.
///
/// This now uses the shared `CodecError` from bitvue-engine for consistency
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

impl From<bitvue_engine::CodecError> for HevcError {
    fn from(err: bitvue_engine::CodecError) -> Self {
        match err {
            bitvue_engine::CodecError::UnexpectedEof { codec: _, position } => {
                HevcError::UnexpectedEof(position)
            }
            bitvue_engine::CodecError::InvalidData { codec: _, message } => {
                HevcError::InvalidData(message)
            }
            bitvue_engine::CodecError::InsufficientData {
                codec: _,
                expected,
                actual,
            } => HevcError::InsufficientData { expected, actual },
            bitvue_engine::CodecError::Io { codec: _, source } => HevcError::Io(source),
            bitvue_engine::CodecError::Parse {
                codec: _,
                offset,
                message,
            } => HevcError::Parse { offset, message },
            bitvue_engine::CodecError::Unsupported { codec: _, feature } => {
                HevcError::InvalidData(format!("Unsupported: {}", feature))
            }
            bitvue_engine::CodecError::MissingParameter {
                codec: _,
                parameter,
            } => HevcError::InvalidData(format!("Missing parameter: {}", parameter)),
            bitvue_engine::CodecError::CodecSpecific { codec: _, message } => {
                HevcError::InvalidData(message)
            }
        }
    }
}
