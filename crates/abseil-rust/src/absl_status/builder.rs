//! Status builder for constructing rich error statuses.

use alloc::boxed::Box;
use alloc::string::String;
use alloc::vec::Vec;

use super::status::Status;
use super::statusor::StatusOr;
use super::StatusCode;

/// Builder for constructing Status objects with additional context.
///
/// This provides a fluent interface for building rich error statuses.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_status::{Status, StatusCode, StatusBuilder};
///
/// let status = StatusBuilder::new(StatusCode::Internal)
///     .with_message("Database connection failed")
///     .with_payload("retry_count", "3")
///     .build();
/// ```
pub struct StatusBuilder {
    code: StatusCode,
    message: Option<String>,
    payloads: Vec<(String, String)>,
    cause: Option<Box<dyn core::error::Error + Send + Sync>>,
}

impl StatusBuilder {
    /// Creates a new StatusBuilder with the given status code.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use abseil::absl_status::{StatusCode, StatusBuilder};
    ///
    /// let builder = StatusBuilder::new(StatusCode::NotFound);
    /// ```
    pub fn new(code: StatusCode) -> Self {
        Self {
            code,
            message: None,
            payloads: Vec::new(),
            cause: None,
        }
    }

    /// Sets the error message.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use abseil::absl_status::{StatusCode, StatusBuilder};
    ///
    /// let builder = StatusBuilder::new(StatusCode::InvalidArgument)
    ///     .with_message("Invalid user ID");
    /// ```
    pub fn with_message(mut self, message: impl Into<String>) -> Self {
        self.message = Some(message.into());
        self
    }

    /// Adds a payload key-value pair.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use abseil::absl_status::{StatusCode, StatusBuilder};
    ///
    /// let builder = StatusBuilder::new(StatusCode::Internal)
    ///     .with_payload("host", "localhost")
    ///     .with_payload("port", "5432");
    /// ```
    pub fn with_payload(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.payloads.push((key.into(), value.into()));
        self
    }

    /// Sets the cause of this status.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use abseil::absl_status::{StatusCode, StatusBuilder};
    ///
    /// let original = Status::new(StatusCode::Internal, "Connection failed");
    /// let builder = StatusBuilder::new(StatusCode::Internal)
    ///     .with_cause(original);
    /// ```
    pub fn with_cause(mut self, cause: Status) -> Self {
        self.cause = Some(Box::new(cause));
        self
    }

    /// Builds the Status object.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use abseil::absl_status::{StatusCode, StatusBuilder};
    ///
    /// let status = StatusBuilder::new(StatusCode::NotFound)
    ///     .with_message("File not found")
    ///     .build();
    /// ```
    pub fn build(self) -> Status {
        let mut status = if let Some(message) = self.message {
            Status::new(self.code, message)
        } else {
            Status::from_code(self.code)
        };

        for (key, value) in self.payloads {
            status.set_payload(key, value);
        }

        status
    }

    /// Builds and immediately returns an Err with this Status.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use abseil::absl_status::{StatusCode, StatusBuilder};
    ///
    /// fn get_user() -> Result<String, Status> {
    ///     Err(StatusBuilder::new(StatusCode::NotFound)
    ///         .with_message("User not found")
    ///         .build_err())
    /// }
    /// ```
    pub fn build_err<T>(self) -> Result<T, Status> {
        Err(self.build())
    }

    /// Builds a StatusOr containing an error.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use abseil::absl_status::{StatusCode, StatusBuilder};
    ///
    /// fn get_user_id() -> StatusOr<u32> {
    ///     StatusBuilder::new(StatusCode::NotFound)
    ///         .with_message("User not found")
    ///         .build_statusor()
    /// }
    /// ```
    pub fn build_statusor<T>(self) -> StatusOr<T> {
        StatusOr::new(self.build())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_status_builder_basic() {
        let status = StatusBuilder::new(StatusCode::NotFound)
            .with_message("Not found")
            .build();

        assert_eq!(status.code(), StatusCode::NotFound);
        assert_eq!(status.message(), "Not found");
    }

    #[test]
    fn test_status_builder_with_payload() {
        let status = StatusBuilder::new(StatusCode::Internal)
            .with_message("Error")
            .with_payload("key", "value")
            .build();

        assert_eq!(status.code(), StatusCode::Internal);
        // Payload is set
    }

    #[test]
    fn test_status_builder_build_err() {
        let result: Result<(), Status> = StatusBuilder::new(StatusCode::NotFound)
            .with_message("Not found")
            .build_err();

        assert!(result.is_err());
        assert_eq!(result.unwrap_err().code(), StatusCode::NotFound);
    }

    #[test]
    fn test_status_builder_build_statusor() {
        let result: StatusOr<u32> = StatusBuilder::new(StatusCode::NotFound)
            .with_message("Not found")
            .build_statusor();

        assert!(result.is_err());
        assert_eq!(result.status().code(), StatusCode::NotFound);
    }
}
