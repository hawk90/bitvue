//! Status transformations for modifying error codes and messages.

use super::status::Status;
use super::categories::{ErrorCategory, StatusCode};

/// Applies transformations to status codes.
pub struct StatusTransformer;

impl StatusTransformer {
    /// Maps a status code to another.
    pub fn map_code(status: Status, code: StatusCode) -> Status {
        Status::new(code, status.message())
    }

    /// Maps the status message.
    pub fn map_message<F>(status: Status, f: F) -> Status
    where
        F: FnOnce(&str) -> String,
    {
        Status::new(status.code(), f(status.message()))
    }

    /// Adds a prefix to the message.
    pub fn prefix_message(status: Status, prefix: &str) -> Status {
        Status::new(status.code(), format!("{}: {}", prefix, status.message()))
    }

    /// Adds a suffix to the message.
    pub fn suffix_message(status: Status, suffix: &str) -> Status {
        Status::new(status.code(), format!("{}: {}", status.message(), suffix))
    }

    /// Wraps a status with additional context.
    pub fn wrap(status: Status, context: &str) -> Status {
        Status::new(status.code(), format!("{}: {}", context, status.message()))
    }

    /// Converts transient errors to a different code.
    pub fn on_transient_map_to(status: Status, target_code: StatusCode) -> Status {
        if status.is_transient() {
            Status::new(target_code, status.message())
        } else {
            status
        }
    }

    /// Combines two statuses, keeping the more severe error.
    pub fn combine(a: Status, b: Status) -> Status {
        if a.is_ok() {
            return b;
        }
        if b.is_ok() {
            return a;
        }
        // Both are errors - prefer the one with "more severe" code
        if a.code().category() == ErrorCategory::Internal {
            a
        } else {
            b
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_status_transformer_map_code() {
        let status = Status::new(StatusCode::Internal, "Error");
        let transformed = StatusTransformer::map_code(status, StatusCode::Unknown);
        assert_eq!(transformed.code(), StatusCode::Unknown);
    }

    #[test]
    fn test_status_transformer_map_message() {
        let status = Status::new(StatusCode::Internal, "Error");
        let transformed = StatusTransformer::map_message(status, |msg| format!("PREFIX: {}", msg));
        assert!(transformed.message().starts_with("PREFIX:"));
    }

    #[test]
    fn test_status_transformer_prefix_message() {
        let status = Status::new(StatusCode::Internal, "Error");
        let transformed = StatusTransformer::prefix_message(status, "In function:");
        assert!(transformed.message().starts_with("In function:"));
    }

    #[test]
    fn test_status_transformer_suffix_message() {
        let status = Status::new(StatusCode::Internal, "Error");
        let transformed = StatusTransformer::suffix_message(status, "suffix");
        assert!(transformed.message().ends_with("suffix"));
    }

    #[test]
    fn test_status_transformer_wrap() {
        let status = Status::new(StatusCode::Internal, "Error");
        let wrapped = StatusTransformer::wrap(status, "Context");
        assert!(wrapped.message().starts_with("Context:"));
    }

    #[test]
    fn test_status_transformer_on_transient_map_to() {
        let status = Status::new(StatusCode::Unavailable, "Service down");
        let transformed = StatusTransformer::on_transient_map_to(status, StatusCode::Internal);
        assert_eq!(transformed.code(), StatusCode::Internal);

        let status = Status::new(StatusCode::NotFound, "Not found");
        let transformed = StatusTransformer::on_transient_map_to(status, StatusCode::Internal);
        assert_eq!(transformed.code(), StatusCode::NotFound);
    }

    #[test]
    fn test_status_transformer_combine() {
        let ok_status = Status::ok();
        let err_status = Status::new(StatusCode::Internal, "Error");
        let combined = StatusTransformer::combine(ok_status, err_status);
        assert_eq!(combined.code(), StatusCode::Internal);

        let combined = StatusTransformer::combine(err_status, ok_status);
        assert_eq!(combined.code(), StatusCode::Internal);
    }
}
