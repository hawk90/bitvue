//! Status type for error handling.
//!
//! This module provides a `Status` type similar to Abseil's `absl::Status`,
//! representing success or error with an error code and message.

use core::fmt;

/// Error codes representing various status conditions.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum StatusCode {
    /// Ok - operation succeeded
    Ok = 0,
    /// Cancelled - operation was cancelled
    Cancelled = 1,
    /// Unknown error
    Unknown = 2,
    /// Invalid argument
    InvalidArgument = 3,
    /// Deadline exceeded
    DeadlineExceeded = 4,
    /// Not found
    NotFound = 5,
    /// Already exists
    AlreadyExists = 6,
    /// Permission denied
    PermissionDenied = 7,
    /// Resource exhausted
    ResourceExhausted = 8,
    /// Failed precondition
    FailedPrecondition = 9,
    /// Aborted
    Aborted = 10,
    /// Out of range
    OutOfRange = 11,
    /// Unimplemented
    Unimplemented = 12,
    /// Internal error
    Internal = 13,
    /// Unavailable
    Unavailable = 14,
    /// Data loss
    DataLoss = 15,
    /// Unauthenticated
    Unauthenticated = 16,
}

impl StatusCode {
    /// Returns the canonical code associated with this status.
    #[inline]
    pub fn code(&self) -> i32 {
        *self as i32
    }

    /// Returns the message associated with this status.
    #[inline]
    pub fn message(&self) -> &'static str {
        match self {
            StatusCode::Ok => "OK",
            StatusCode::Cancelled => "Cancelled",
            StatusCode::Unknown => "Unknown",
            StatusCode::InvalidArgument => "Invalid argument",
            StatusCode::DeadlineExceeded => "Deadline exceeded",
            StatusCode::NotFound => "Not found",
            StatusCode::AlreadyExists => "Already exists",
            StatusCode::PermissionDenied => "Permission denied",
            StatusCode::ResourceExhausted => "Resource exhausted",
            StatusCode::FailedPrecondition => "Failed precondition",
            StatusCode::Aborted => "Aborted",
            StatusCode::OutOfRange => "Out of range",
            StatusCode::Unimplemented => "Unimplemented",
            StatusCode::Internal => "Internal",
            StatusCode::Unavailable => "Unavailable",
            StatusCode::DataLoss => "Data loss",
            StatusCode::Unauthenticated => "Unauthenticated",
        }
    }

    /// Returns whether this status is OK.
    #[inline]
    pub fn is_ok(&self) -> bool {
        *self == StatusCode::Ok
    }
}

impl Default for StatusCode {
    #[inline]
    fn default() -> Self {
        StatusCode::Ok
    }
}

impl fmt::Display for StatusCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code(), self.message())
    }
}

/// Status type representing success or error.
///
/// Similar to Abseil's `absl::Status`, this type represents either success
/// or an error with a code and message.
#[derive(Clone, PartialEq, Eq)]
pub struct Status {
    code: StatusCode,
    message: String,
}

impl Status {
    /// Creates a new OK status.
    #[inline]
    pub fn ok() -> Self {
        Self {
            code: StatusCode::Ok,
            message: String::new(),
        }
    }

    /// Creates a new status with the given code and message.
    #[inline]
    pub fn new(code: StatusCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }

    /// Returns the status code.
    #[inline]
    pub fn code(&self) -> StatusCode {
        self.code
    }

    /// Returns the error message.
    #[inline]
    pub fn message(&self) -> &str {
        &self.message
    }

    /// Returns whether this status is OK.
    #[inline]
    pub fn is_ok(&self) -> bool {
        self.code.is_ok()
    }

    // Convenience constructors for common error types

    /// Cancelled operation.
    #[inline]
    pub fn cancelled(message: impl Into<String>) -> Self {
        Self::new(StatusCode::Cancelled, message)
    }

    /// Invalid argument.
    #[inline]
    pub fn invalid_argument(message: impl Into<String>) -> Self {
        Self::new(StatusCode::InvalidArgument, message)
    }

    /// Deadline exceeded.
    #[inline]
    pub fn deadline_exceeded(message: impl Into<String>) -> Self {
        Self::new(StatusCode::DeadlineExceeded, message)
    }

    /// Not found.
    #[inline]
    pub fn not_found(message: impl Into<String>) -> Self {
        Self::new(StatusCode::NotFound, message)
    }

    /// Already exists.
    #[inline]
    pub fn already_exists(message: impl Into<String>) -> Self {
        Self::new(StatusCode::AlreadyExists, message)
    }

    /// Permission denied.
    #[inline]
    pub fn permission_denied(message: impl Into<String>) -> Self {
        Self::new(StatusCode::PermissionDenied, message)
    }

    /// Resource exhausted.
    #[inline]
    pub fn resource_exhausted(message: impl Into<String>) -> Self {
        Self::new(StatusCode::ResourceExhausted, message)
    }

    /// Failed precondition.
    #[inline]
    pub fn failed_precondition(message: impl Into<String>) -> Self {
        Self::new(StatusCode::FailedPrecondition, message)
    }

    /// Aborted.
    #[inline]
    pub fn aborted(message: impl Into<String>) -> Self {
        Self::new(StatusCode::Aborted, message)
    }

    /// Out of range.
    #[inline]
    pub fn out_of_range(message: impl Into<String>) -> Self {
        Self::new(StatusCode::OutOfRange, message)
    }

    /// Unimplemented.
    #[inline]
    pub fn unimplemented(message: impl Into<String>) -> Self {
        Self::new(StatusCode::Unimplemented, message)
    }

    /// Internal error.
    #[inline]
    pub fn internal(message: impl Into<String>) -> Self {
        Self::new(StatusCode::Internal, message)
    }

    /// Unavailable.
    #[inline]
    pub fn unavailable(message: impl Into<String>) -> Self {
        Self::new(StatusCode::Unavailable, message)
    }

    /// Data loss.
    #[inline]
    pub fn data_loss(message: impl Into<String>) -> Self {
        Self::new(StatusCode::DataLoss, message)
    }

    /// Unauthenticated.
    #[inline]
    pub fn unauthenticated(message: impl Into<String>) -> Self {
        Self::new(StatusCode::Unauthenticated, message)
    }
}

impl Default for Status {
    #[inline]
    fn default() -> Self {
        Self::ok()
    }
}

impl fmt::Debug for Status {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_ok() {
            write!(f, "Status::OK")
        } else {
            write!(f, "Status({}, {:?})", self.code.message(), self.message)
        }
    }
}

impl fmt::Display for Status {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_ok() {
            write!(f, "OK")
        } else if self.message.is_empty() {
            write!(f, "{}", self.code.message())
        } else {
            write!(f, "{}: {}", self.code.message(), self.message)
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for Status {}

/// From<std::convert::Infallible> implementation for Status.
impl From<core::convert::Infallible> for Status {
    #[inline]
    fn from(_: core::convert::Infallible) -> Self {
        Self::ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_status_ok() {
        let s = Status::ok();
        assert!(s.is_ok());
        assert_eq!(s.code(), StatusCode::Ok);
        assert!(s.message().is_empty());
    }

    #[test]
    fn test_status_new() {
        let s = Status::new(StatusCode::NotFound, "Resource not found".to_string());
        assert!(!s.is_ok());
        assert_eq!(s.code(), StatusCode::NotFound);
        assert_eq!(s.message(), "Resource not found");
    }

    #[test]
    fn test_status_convenience_constructors() {
        let s = Status::invalid_argument("Bad argument");
        assert_eq!(s.code(), StatusCode::InvalidArgument);
        assert_eq!(s.message(), "Bad argument");

        let s = Status::not_found("Missing key");
        assert_eq!(s.code(), StatusCode::NotFound);
        assert_eq!(s.message(), "Missing key");

        let s = Status::permission_denied("Access denied");
        assert_eq!(s.code(), StatusCode::PermissionDenied);
        assert_eq!(s.message(), "Access denied");
    }

    #[test]
    fn test_status_display() {
        let s = Status::ok();
        assert_eq!(format!("{}", s), "OK");

        let s = Status::not_found("key".to_string());
        assert_eq!(format!("{}", s), "Not found: key");

        let s = Status::new(StatusCode::NotFound, "");
        assert_eq!(format!("{}", s), "Not found");
    }

    #[test]
    fn test_status_debug() {
        let s = Status::ok();
        assert_eq!(format!("{:?}", s), "Status::OK");

        let s = Status::not_found("key".to_string());
        assert_eq!(format!("{:?}", s), "Status(Not found, \"key\")");
    }

    #[test]
    fn test_status_default() {
        let s: Status = Default::default();
        assert!(s.is_ok());
    }

    #[test]
    fn test_status_code() {
        assert_eq!(StatusCode::Ok.code(), 0);
        assert_eq!(StatusCode::Cancelled.code(), 1);
        assert_eq!(StatusCode::Unknown.code(), 2);
        assert_eq!(StatusCode::InvalidArgument.code(), 3);
        assert_eq!(StatusCode::NotFound.code(), 5);
        assert_eq!(StatusCode::AlreadyExists.code(), 6);
    }

    #[test]
    fn test_status_code_message() {
        assert_eq!(StatusCode::Ok.message(), "OK");
        assert_eq!(StatusCode::NotFound.message(), "Not found");
        assert_eq!(StatusCode::PermissionDenied.message(), "Permission denied");
    }

    #[test]
    fn test_status_code_display() {
        assert_eq!(format!("{}", StatusCode::Ok), "0: OK");
        assert_eq!(format!("{}", StatusCode::NotFound), "5: Not found");
    }

    #[test]
    fn test_status_eq() {
        let s1 = Status::not_found("key");
        let s2 = Status::not_found("key");
        assert_eq!(s1, s2);

        let s3 = Status::not_found("other");
        assert_ne!(s1, s3);
    }
}
