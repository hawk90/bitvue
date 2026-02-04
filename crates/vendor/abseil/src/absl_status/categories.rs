//! Error categories and classification for status codes.

use super::status::Status;
use super::StatusCode;

/// Categories of errors for grouping and handling.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ErrorCategory {
    /// Transient errors that may succeed on retry (e.g., network timeout)
    Transient,
    /// Permanent errors that won't change on retry (e.g., not found)
    Permanent,
    /// Errors related to authentication/authorization
    Authentication,
    /// Errors related to resource quotas/limits
    ResourceExhausted,
    /// Errors from invalid arguments
    InvalidArgument,
    /// Internal system errors
    Internal,
}

impl StatusCode {
    /// Returns the category of this status code.
    pub const fn category(&self) -> ErrorCategory {
        match self {
            StatusCode::Ok => ErrorCategory::Internal,
            StatusCode::Cancelled => ErrorCategory::Transient,
            StatusCode::Unknown => ErrorCategory::Internal,
            StatusCode::InvalidArgument => ErrorCategory::InvalidArgument,
            StatusCode::DeadlineExceeded => ErrorCategory::Transient,
            StatusCode::NotFound => ErrorCategory::Permanent,
            StatusCode::AlreadyExists => ErrorCategory::Permanent,
            StatusCode::PermissionDenied => ErrorCategory::Authentication,
            StatusCode::ResourceExhausted => ErrorCategory::ResourceExhausted,
            StatusCode::FailedPrecondition => ErrorCategory::Permanent,
            StatusCode::Aborted => ErrorCategory::Transient,
            StatusCode::OutOfRange => ErrorCategory::InvalidArgument,
            StatusCode::Unimplemented => ErrorCategory::Permanent,
            StatusCode::Internal => ErrorCategory::Internal,
            StatusCode::Unavailable => ErrorCategory::Transient,
            StatusCode::DataLoss => ErrorCategory::Internal,
            StatusCode::Unauthenticated => ErrorCategory::Authentication,
        }
    }

    /// Returns true if this error is typically transient (may succeed on retry).
    pub const fn is_transient(&self) -> bool {
        matches!(
            self,
            StatusCode::DeadlineExceeded
                | StatusCode::Unavailable
                | StatusCode::Aborted
                | StatusCode::Cancelled
        )
    }

    /// Returns true if this error is permanent (won't change on retry).
    pub const fn is_permanent(&self) -> bool {
        matches!(
            self,
            StatusCode::NotFound
                | StatusCode::AlreadyExists
                | StatusCode::FailedPrecondition
                | StatusCode::Unimplemented
                | StatusCode::InvalidArgument
                | StatusCode::OutOfRange
        )
    }

    /// Returns true if this is an authentication/authorization error.
    pub const fn is_auth_error(&self) -> bool {
        matches!(self, StatusCode::PermissionDenied | StatusCode::Unauthenticated)
    }
}

impl Status {
    /// Returns the category of this status.
    pub fn category(&self) -> ErrorCategory {
        self.code().category()
    }

    /// Returns true if this error is transient.
    pub fn is_transient(&self) -> bool {
        self.code().is_transient()
    }

    /// Returns true if this error is permanent.
    pub fn is_permanent(&self) -> bool {
        self.code().is_permanent()
    }

    /// Returns true if this is an auth error.
    pub fn is_auth_error(&self) -> bool {
        self.code().is_auth_error()
    }

    /// Checks if this status matches any of the given codes.
    pub fn is_any_code(&self, codes: &[StatusCode]) -> bool {
        codes.contains(&self.code())
    }

    /// Returns a detailed description of the status.
    pub fn description(&self) -> String {
        format!("{}: {}", self.code().name(), self.message())
    }

    /// Returns true if the status is OK (success).
    pub fn is_ok(&self) -> bool {
        self.code() == StatusCode::Ok
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ErrorCategory tests
    #[test]
    fn test_status_code_category() {
        assert_eq!(StatusCode::NotFound.category(), ErrorCategory::Permanent);
        assert_eq!(StatusCode::Unavailable.category(), ErrorCategory::Transient);
        assert_eq!(StatusCode::PermissionDenied.category(), ErrorCategory::Authentication);
        assert_eq!(StatusCode::InvalidArgument.category(), ErrorCategory::InvalidArgument);
        assert_eq!(StatusCode::ResourceExhausted.category(), ErrorCategory::ResourceExhausted);
        assert_eq!(StatusCode::Internal.category(), ErrorCategory::Internal);
    }

    #[test]
    fn test_status_code_is_transient() {
        assert!(StatusCode::DeadlineExceeded.is_transient());
        assert!(StatusCode::Unavailable.is_transient());
        assert!(!StatusCode::NotFound.is_transient());
        assert!(!StatusCode::InvalidArgument.is_transient());
    }

    #[test]
    fn test_status_code_is_permanent() {
        assert!(StatusCode::NotFound.is_permanent());
        assert!(StatusCode::InvalidArgument.is_permanent());
        assert!(!StatusCode::Unavailable.is_permanent());
    }

    #[test]
    fn test_status_code_is_auth_error() {
        assert!(StatusCode::PermissionDenied.is_auth_error());
        assert!(StatusCode::Unauthenticated.is_auth_error());
        assert!(!StatusCode::NotFound.is_auth_error());
    }

    #[test]
    fn test_status_category() {
        let status = Status::new(StatusCode::NotFound, "Not found");
        assert_eq!(status.category(), ErrorCategory::Permanent);
    }

    #[test]
    fn test_status_is_transient() {
        let status = Status::new(StatusCode::Unavailable, "Service down");
        assert!(status.is_transient());

        let status = Status::new(StatusCode::NotFound, "Not found");
        assert!(!status.is_transient());
    }

    #[test]
    fn test_status_is_permanent() {
        let status = Status::new(StatusCode::NotFound, "Not found");
        assert!(status.is_permanent());

        let status = Status::new(StatusCode::Unavailable, "Service down");
        assert!(!status.is_permanent());
    }

    #[test]
    fn test_status_is_auth_error() {
        let status = Status::new(StatusCode::PermissionDenied, "Access denied");
        assert!(status.is_auth_error());
    }

    #[test]
    fn test_status_is_any_code() {
        let status = Status::new(StatusCode::NotFound, "Not found");
        assert!(status.is_any_code(&[StatusCode::NotFound, StatusCode::Internal]));
        assert!(!status.is_any_code(&[StatusCode::Internal, StatusCode::Unavailable]));
    }

    #[test]
    fn test_status_description() {
        let status = Status::new(StatusCode::NotFound, "Not found");
        let desc = status.description();
        assert!(desc.contains("NotFound"));
        assert!(desc.contains("Not found"));
    }
}
