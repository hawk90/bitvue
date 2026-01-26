//! Error types for AV3 parsing.

use thiserror::Error;

/// AV3 parsing error type.
#[derive(Debug, Error)]
pub enum Av3Error {
    /// I/O error.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Invalid data encountered.
    #[error("Invalid data: {0}")]
    InvalidData(String),

    /// Unsupported feature.
    #[error("Unsupported feature: {0}")]
    Unsupported(String),

    /// Insufficient data.
    #[error("Insufficient data: expected {expected} bytes, got {actual} bytes")]
    InsufficientData { expected: usize, actual: usize },

    /// Parse error.
    #[error("Parse error: {0}")]
    ParseError(String),
}

/// Result type for AV3 parsing operations.
pub type Result<T> = std::result::Result<T, Av3Error>;
