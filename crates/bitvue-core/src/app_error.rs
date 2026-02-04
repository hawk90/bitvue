//! Unified Error Handling - Consistent error types across the application
//!
//! This module provides a unified error handling system with:
//! - Standardized error types
//! - Error conversion utilities
//! - User-friendly error messages
//! - Proper error propagation

use std::fmt;
use std::io;

// =============================================================================
// Error Categories
// =============================================================================

/// Error category for display and logging
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCategory {
    /// Input validation failed
    Validation,
    /// File or resource not found
    NotFound,
    /// Permission denied
    Permission,
    /// Parsing/decoding error
    Parse,
    /// I/O operation failed
    Io,
    /// Network operation failed
    Network,
    /// Codec-specific error
    Codec,
    /// Internal logic error
    Internal,
    /// Feature not implemented
    NotImplemented,
}

impl ErrorCategory {
    /// Get display name for the category
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Validation => "Validation Error",
            Self::NotFound => "Not Found",
            Self::Permission => "Permission Denied",
            Self::Parse => "Parse Error",
            Self::Io => "I/O Error",
            Self::Network => "Network Error",
            Self::Codec => "Codec Error",
            Self::Internal => "Internal Error",
            Self::NotImplemented => "Not Implemented",
        }
    }

    /// Get icon for the category (for UI)
    pub fn icon(&self) -> &'static str {
        match self {
            Self::Validation => "⚠️",
            Self::NotFound => "🔍",
            Self::Permission => "🔒",
            Self::Parse => "📄",
            Self::Io => "💾",
            Self::Network => "🌐",
            Self::Codec => "🎬",
            Self::Internal => "⚙️",
            Self::NotImplemented => "🚧",
        }
    }
}

// =============================================================================
// Application Error
// =============================================================================

/// Unified application error type
#[derive(Debug, Clone)]
pub struct AppError {
    /// Error category
    pub category: ErrorCategory,
    /// Error code for programmatic handling
    pub code: &'static str,
    /// User-friendly error message
    pub message: String,
    /// Detailed error for debugging (optional)
    pub details: Option<String>,
    /// Source location (file:line) (optional)
    pub source: Option<String>,
}

impl AppError {
    /// Create a new error
    pub fn new(category: ErrorCategory, code: &'static str, message: impl Into<String>) -> Self {
        Self {
            category,
            code,
            message: message.into(),
            details: None,
            source: None,
        }
    }

    /// Add details to the error
    pub fn with_details(mut self, details: impl Into<String>) -> Self {
        self.details = Some(details.into());
        self
    }

    /// Add source location to the error
    pub fn with_source(mut self, source: impl Into<String>) -> Self {
        self.source = Some(source.into());
        self
    }

    /// Create a validation error
    pub fn validation(code: &'static str, message: impl Into<String>) -> Self {
        Self::new(ErrorCategory::Validation, code, message)
    }

    /// Create a not found error
    pub fn not_found(code: &'static str, message: impl Into<String>) -> Self {
        Self::new(ErrorCategory::NotFound, code, message)
    }

    /// Create a permission error
    pub fn permission(code: &'static str, message: impl Into<String>) -> Self {
        Self::new(ErrorCategory::Permission, code, message)
    }

    /// Create a parse error
    pub fn parse(code: &'static str, message: impl Into<String>) -> Self {
        Self::new(ErrorCategory::Parse, code, message)
    }

    /// Create an I/O error
    pub fn io(code: &'static str, message: impl Into<String>) -> Self {
        Self::new(ErrorCategory::Io, code, message)
    }

    /// Create a codec error
    pub fn codec(code: &'static str, message: impl Into<String>) -> Self {
        Self::new(ErrorCategory::Codec, code, message)
    }

    /// Create an internal error
    pub fn internal(code: &'static str, message: impl Into<String>) -> Self {
        Self::new(ErrorCategory::Internal, code, message)
    }

    /// Get a user-friendly display string
    pub fn display(&self) -> String {
        let mut result = format!("{} {}", self.category.icon(), self.message);

        if let Some(details) = &self.details {
            result.push_str(&format!("\n\nDetails: {}", details));
        }

        result
    }

    /// Get a short error message (for tooltips, etc.)
    pub fn short_message(&self) -> String {
        self.message.clone()
    }

    /// Convert to JSON for serialization
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "category": format!("{:?}", self.category),
            "code": self.code,
            "message": self.message,
            "details": self.details,
            "source": self.source,
        })
    }
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "[{}:{}] {}",
            self.category.display_name(),
            self.code,
            self.message
        )
    }
}

impl std::error::Error for AppError {}

// =============================================================================
// Error Codes
// =============================================================================

pub mod codes {
    /// Validation error codes
    pub mod validation {
        pub const INVALID_PATH: &str = "VAL_001";
        pub const INVALID_RANGE: &str = "VAL_002";
        pub const MISSING_FIELD: &str = "VAL_003";
        pub const INVALID_TYPE: &str = "VAL_004";
    }

    /// File not found error codes
    pub mod not_found {
        pub const FILE_NOT_FOUND: &str = "NF_001";
        pub const STREAM_NOT_FOUND: &str = "NF_002";
        pub const FRAME_NOT_FOUND: &str = "NF_003";
        pub const CODEC_NOT_FOUND: &str = "NF_004";
    }

    /// Parse error codes
    pub mod parse {
        pub const INVALID_HEADER: &str = "PARSE_001";
        pub const INVALID_SYNTAX: &str = "PARSE_002";
        pub const TRUNCATED_DATA: &str = "PARSE_003";
        pub const UNKNOWN_CODEC: &str = "PARSE_004";
    }

    /// I/O error codes
    pub mod io {
        pub const READ_FAILED: &str = "IO_001";
        pub const WRITE_FAILED: &str = "IO_002";
        pub const OPEN_FAILED: &str = "IO_003";
        pub const CREATE_FAILED: &str = "IO_004";
    }

    /// Codec error codes
    pub mod codec {
        pub const UNSUPPORTED_CODEC: &str = "CODEC_001";
        pub const INVALID_BITSTREAM: &str = "CODEC_002";
        pub const DECODE_FAILED: &str = "CODEC_003";
        pub const MISSING_REFERENCE: &str = "CODEC_004";
    }
}

// =============================================================================
// Error Conversion Traits
// =============================================================================

/// Convert std::io::Error to AppError
pub trait IntoAppError {
    fn into_app_error(self) -> AppError;
}

impl IntoAppError for io::Error {
    fn into_app_error(self) -> AppError {
        let (code, message) = match self.kind() {
            io::ErrorKind::NotFound => (
                codes::not_found::FILE_NOT_FOUND,
                format!("File not found: {}", self),
            ),
            io::ErrorKind::PermissionDenied => (
                codes::validation::INVALID_PATH,
                format!("Permission denied: {}", self),
            ),
            io::ErrorKind::InvalidInput => (
                codes::validation::INVALID_TYPE,
                format!("Invalid input: {}", self),
            ),
            _ => (codes::io::READ_FAILED, format!("I/O error: {}", self)),
        };

        AppError::new(ErrorCategory::Io, code, message).with_details(self.to_string())
    }
}

impl IntoAppError for serde_json::Error {
    fn into_app_error(self) -> AppError {
        AppError::parse(codes::parse::INVALID_SYNTAX, "Failed to parse JSON data")
            .with_details(self.to_string())
    }
}

// =============================================================================
// Result Type Alias
// =============================================================================

/// Result type alias with AppError
pub type AppResult<T> = std::result::Result<T, AppError>;

/// Convert Result<T, E> to AppResult<T> using IntoAppError
pub trait ToAppResult<T> {
    fn to_app_result(self) -> AppResult<T>;
}

impl<T, E: IntoAppError> ToAppResult<T> for std::result::Result<T, E> {
    fn to_app_result(self) -> AppResult<T> {
        self.map_err(|e| e.into_app_error())
    }
}

// =============================================================================
// Convenience Builders
// =============================================================================

/// Builder for creating common errors
pub struct ErrorBuilder {
    category: ErrorCategory,
    code: &'static str,
}

impl ErrorBuilder {
    /// Create a validation error builder
    pub fn validation(code: &'static str) -> Self {
        Self {
            category: ErrorCategory::Validation,
            code,
        }
    }

    /// Create a not found error builder
    pub fn not_found(code: &'static str) -> Self {
        Self {
            category: ErrorCategory::NotFound,
            code,
        }
    }

    /// Create a parse error builder
    pub fn parse(code: &'static str) -> Self {
        Self {
            category: ErrorCategory::Parse,
            code,
        }
    }

    /// Build the error with a message
    pub fn build(self, message: impl Into<String>) -> AppError {
        AppError::new(self.category, self.code, message)
    }

    /// Build the error with a message and details
    pub fn build_with_details(
        self,
        message: impl Into<String>,
        details: impl Into<String>,
    ) -> AppError {
        AppError::new(self.category, self.code, message).with_details(details)
    }
}

// =============================================================================
// User-Friendly Error Messages
// =============================================================================

impl AppError {
    /// Create an error for invalid file path
    pub fn invalid_path(path: impl Into<String>, reason: impl Into<String>) -> Self {
        AppError::validation(
            codes::validation::INVALID_PATH,
            format!("Invalid file path: {}", reason.into()),
        )
        .with_details(format!("Path: {}", path.into()))
    }

    /// Create an error for file not found
    pub fn file_not_found(path: impl Into<String>) -> Self {
        AppError::not_found(
            codes::not_found::FILE_NOT_FOUND,
            format!("File not found: {}", path.into()),
        )
    }

    /// Create an error for invalid frame index
    pub fn invalid_frame_index(frame: u64, max_frame: u64) -> Self {
        AppError::validation(
            codes::validation::INVALID_RANGE,
            format!("Frame index {} is out of range (max: {})", frame, max_frame),
        )
    }

    /// Create an error for invalid byte range
    pub fn invalid_byte_range(start: u64, end: u64, file_size: u64) -> Self {
        AppError::validation(
            codes::validation::INVALID_RANGE,
            format!(
                "Invalid byte range: {}-{} (file size: {})",
                start, end, file_size
            ),
        )
    }

    /// Create an error for parse failure
    pub fn parse_failure(codec: impl Into<String>, context: impl Into<String>) -> Self {
        AppError::parse(
            codes::parse::INVALID_SYNTAX,
            format!("Failed to parse {} bitstream", codec.into()),
        )
        .with_details(context.into())
    }

    /// Create an error for unsupported codec
    pub fn unsupported_codec(codec: impl Into<String>) -> Self {
        AppError::codec(
            codes::codec::UNSUPPORTED_CODEC,
            format!("Unsupported codec: {}", codec.into()),
        )
    }

    /// Create an error for decode failure
    pub fn decode_failure(frame: u64, reason: impl Into<String>) -> Self {
        AppError::codec(
            codes::codec::DECODE_FAILED,
            format!("Failed to decode frame {}", frame),
        )
        .with_details(reason.into())
    }
}

// =============================================================================
// Error Display for UI
// =============================================================================

/// Format error for UI display
pub fn format_error_for_ui(error: &AppError) -> String {
    match error.category {
        ErrorCategory::Validation => {
            format!(
                "⚠️ Validation Failed\n\n{}\n\n💡 Please check your input and try again.",
                error.message
            )
        }
        ErrorCategory::NotFound => {
            format!(
                "🔍 Not Found\n\n{}\n\n💡 The requested resource could not be found.",
                error.message
            )
        }
        ErrorCategory::Permission => {
            format!(
                "🔒 Permission Denied\n\n{}\n\n💡 You don't have permission to access this resource.",
                error.message
            )
        }
        ErrorCategory::Parse => {
            format!(
                "📄 Parse Error\n\n{}\n\n💡 The file could not be parsed. It may be corrupted or in an unsupported format.",
                error.message
            )
        }
        ErrorCategory::Io => {
            format!(
                "💾 File Error\n\n{}\n\n💡 Please check if the file exists and you have permission to access it.",
                error.message
            )
        }
        ErrorCategory::Codec => {
            format!(
                "🎬 Codec Error\n\n{}\n\n💡 This appears to be a codec-related issue. Try opening a different file.",
                error.message
            )
        }
        ErrorCategory::Internal => {
            format!(
                "⚙️ Internal Error\n\n{}\n\n💡 An unexpected error occurred. Please try again or report this issue.",
                error.message
            )
        }
        ErrorCategory::NotImplemented => {
            format!(
                "🚧 Not Implemented\n\n{}\n\n💡 This feature is not yet implemented.",
                error.message
            )
        }
        ErrorCategory::Network => {
            format!(
                "🌐 Network Error\n\n{}\n\n💡 Please check your internet connection.",
                error.message
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_category_display() {
        assert_eq!(ErrorCategory::Validation.display_name(), "Validation Error");
        assert_eq!(ErrorCategory::Validation.icon(), "⚠️");
        assert_eq!(ErrorCategory::NotFound.icon(), "🔍");
        assert_eq!(ErrorCategory::Permission.icon(), "🔒");
    }

    #[test]
    fn test_app_error_creation() {
        let error = AppError::validation(codes::validation::INVALID_PATH, "Test validation error");

        assert_eq!(error.category, ErrorCategory::Validation);
        assert_eq!(error.code, "VAL_001");
        assert_eq!(error.message, "Test validation error");
        assert!(error.details.is_none());
    }

    #[test]
    fn test_convenience_errors() {
        let err1 = AppError::invalid_path("/path/to/file", "Path traversal detected");
        assert_eq!(err1.category, ErrorCategory::Validation);

        let err2 = AppError::file_not_found("/path/to/file");
        assert_eq!(err2.category, ErrorCategory::NotFound);

        let err3 = AppError::invalid_frame_index(100, 50);
        assert!(err3.message.contains("out of range"));
    }

    #[test]
    fn test_format_error_for_ui() {
        let error = AppError::file_not_found("/path/to/file");
        let ui_message = format_error_for_ui(&error);

        assert!(ui_message.contains("🔍"));
        assert!(ui_message.contains("Not Found"));
        assert!(ui_message.contains("File not found"));
    }
}
