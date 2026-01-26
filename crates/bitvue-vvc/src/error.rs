//! VVC parser error types.

use thiserror::Error;

/// Result type for VVC parsing operations.
pub type Result<T> = std::result::Result<T, VvcError>;

/// VVC parsing errors.
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
