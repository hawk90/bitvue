//! Shared error types for Bitvue application
//!
//! This module defines consistent error types that can be used across
//! the application instead of using raw String error messages.
//!
//! # Example
//!
//! ```rust
//! use crate::error::{BitvueError, Result};
//!
//! pub fn validate_frame_index(idx: usize, total: usize) -> Result<()> {
//!     if idx >= total {
//!         return Err(BitvueError::InvalidParameter(format!(
//!             "Frame index {} out of range (total: {})",
//!             idx, total
//!         )));
//!     }
//!     Ok(())
//! }
//! ```

use std::path::PathBuf;

/// Application-wide Result type alias
pub type Result<T> = std::result::Result<T, BitvueError>;

/// Core error type for Bitvue application
///
/// Provides structured error information with context for better
/// error handling and user feedback.
#[derive(Debug, Clone, PartialEq)]
pub enum BitvueError {
    /// Path validation failed
    ///
    /// This error is returned when a file path fails validation checks,
    /// such as containing path traversal attempts or pointing to
    /// restricted system directories.
    PathValidation(String),

    /// File I/O error
    ///
    /// Wraps standard I/O errors with additional context about what
    /// operation was being performed.
    FileIo(String),

    /// Parse error
    ///
    /// Indicates failure to parse a file format or data structure.
    /// Includes details about what was being parsed and what went wrong.
    Parse(String),

    /// Decode error
    ///
    /// Returned when video frame decoding fails, typically due to
    /// corrupted data or unsupported codec features.
    Decode(String),

    /// Invalid parameter
    ///
    /// Indicates that a function received an invalid parameter value,
    /// such as a negative number where only positive is valid, or
    /// an index out of bounds.
    InvalidParameter(String),

    /// Resource exhausted
    ///
    /// Returned when a resource limit has been reached, such as
    /// memory allocation limits, file descriptor limits, or cache size limits.
    ResourceExhausted(String),

    /// Rate limited
    ///
    /// Indicates that an operation was rate-limited and should be
    /// retried after the specified duration.
    RateLimited { duration_secs: f64 },

    /// Unsupported operation
    ///
    /// Returned when attempting an operation that is not supported,
    /// such as decoding an unsupported codec format.
    Unsupported(String),
}

impl BitvueError {
    /// Create a path validation error
    pub fn path_validation(msg: impl Into<String>) -> Self {
        Self::PathValidation(msg.into())
    }

    /// Create a file I/O error
    pub fn file_io(msg: impl Into<String>) -> Self {
        Self::FileIo(msg.into())
    }

    /// Create a parse error
    pub fn parse(msg: impl Into<String>) -> Self {
        Self::Parse(msg.into())
    }

    /// Create a decode error
    pub fn decode(msg: impl Into<String>) -> Self {
        Self::Decode(msg.into())
    }

    /// Create an invalid parameter error
    pub fn invalid_parameter(msg: impl Into<String>) -> Self {
        Self::InvalidParameter(msg.into())
    }

    /// Create a resource exhausted error
    pub fn resource_exhausted(msg: impl Into<String>) -> Self {
        Self::ResourceExhausted(msg.into())
    }

    /// Create a rate limited error
    pub fn rate_locked(duration_secs: f64) -> Self {
        Self::RateLimited { duration_secs }
    }

    /// Create an unsupported operation error
    pub fn unsupported(msg: impl Into<String>) -> Self {
        Self::Unsupported(msg.into())
    }

    /// Check if this is a retryable error
    pub fn is_retryable(&self) -> bool {
        matches!(self, Self::RateLimited { .. })
    }

    /// Check if this is a user input error
    pub fn is_user_error(&self) -> bool {
        matches!(self, Self::InvalidParameter(_) | Self::PathValidation(_))
    }

    /// Get a user-friendly error message
    pub fn user_message(&self) -> String {
        match self {
            Self::PathValidation(msg) => format!("Invalid file path: {}", msg),
            Self::FileIo(msg) => format!("File operation failed: {}", msg),
            Self::Parse(msg) => format!("Failed to parse file: {}", msg),
            Self::Decode(msg) => format!("Failed to decode video: {}", msg),
            Self::InvalidParameter(msg) => format!("Invalid parameter: {}", msg),
            Self::ResourceExhausted(msg) => format!("Resource limit reached: {}", msg),
            Self::RateLimited { duration_secs } => {
                format!("Rate limited. Please wait {:.1}s before retrying.", duration_secs)
            }
            Self::Unsupported(msg) => format!("Unsupported operation: {}", msg),
        }
    }
}

impl std::fmt::Display for BitvueError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::PathValidation(msg) => write!(f, "Path validation failed: {}", msg),
            Self::FileIo(msg) => write!(f, "File I/O error: {}", msg),
            Self::Parse(msg) => write!(f, "Parse error: {}", msg),
            Self::Decode(msg) => write!(f, "Decode error: {}", msg),
            Self::InvalidParameter(msg) => write!(f, "Invalid parameter: {}", msg),
            Self::ResourceExhausted(msg) => write!(f, "Resource exhausted: {}", msg),
            Self::RateLimited { duration_secs } => {
                write!(f, "Rate limited (wait {:.1}s)", duration_secs)
            }
            Self::Unsupported(msg) => write!(f, "Unsupported: {}", msg),
        }
    }
}

impl std::error::Error for BitvueError {}

// Convenience conversion from std::io::Error
impl From<std::io::Error> for BitvueError {
    fn from(err: std::io::Error) -> Self {
        Self::FileIo(err.to_string())
    }
}

// Convenience conversion from PathBuf for path errors
impl From<PathBuf> for BitvueError {
    fn from(path: PathBuf) -> Self {
        Self::PathValidation(format!("Invalid path: {}", path.display()))
    }
}

// Convenience conversion from &str for general errors
impl From<&str> for BitvueError {
    fn from(msg: &str) -> Self {
        Self::Parse(msg.to_string())
    }
}

// Convenience conversion from String for general errors
impl From<String> for BitvueError {
    fn from(msg: String) -> Self {
        Self::Parse(msg)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = BitvueError::invalid_parameter("test error");
        assert_eq!(err.to_string(), "Invalid parameter: test error");
    }

    #[test]
    fn test_error_user_message() {
        let err = BitvueError::rate_locked(2.5);
        assert!(err.user_message().contains("2.5s"));
    }

    #[test]
    fn test_error_is_retryable() {
        let err = BitvueError::rate_locked(1.0);
        assert!(err.is_retryable());

        let err = BitvueError::invalid_parameter("test");
        assert!(!err.is_retryable());
    }

    #[test]
    fn test_error_is_user_error() {
        let err = BitvueError::invalid_parameter("test");
        assert!(err.is_user_error());

        let err = BitvueError::FileIo("test".to_string());
        assert!(!err.is_user_error());
    }

    #[test]
    fn test_from_string() {
        let err: BitvueError = "test error".into();
        assert!(matches!(err, BitvueError::Parse(_)));
    }
}
