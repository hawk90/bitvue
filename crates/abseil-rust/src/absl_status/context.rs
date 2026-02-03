//! Context propagation for error handling.

use alloc::format;
use alloc::string::{String, ToString};

use super::error_chain::ToStatus;
use super::status::Status;
use super::StatusCode;

/// Trait for propagating errors with additional context.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_status::{Status, WithContext};
///
/// fn inner_function() -> Result<(), Status> {
///     Err(Status::new(StatusCode::Internal, "Base error"))
/// }
///
/// fn outer_function() -> Result<(), Status> {
///     inner_function().with_context("In outer_function")
/// }
/// ```
pub trait WithContext<T, E>: Sized {
    /// Adds context to an error result.
    fn with_context(self, context: impl Into<String>) -> Result<T, E>;
}

impl<T, E: ToStatus> WithContext<T, Status> for Result<T, E> {
    fn with_context(self, context: impl Into<String>) -> Result<T, Status> {
        self.map_err(|e| {
            let base_status = e.to_status(StatusCode::Internal);
            let ctx: String = context.into();
            Status::new(
                base_status.code(),
                format!("{}: {}", ctx, base_status.message())
            )
        })
    }
}

/// Macro for creating a StatusBuilder with a code and message.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_status::{status_builder, StatusCode};
///
/// let status = status_builder!(StatusCode::NotFound, "Resource not found").build();
/// ```
#[macro_export]
macro_rules! status_builder {
    ($code:expr) => {
        $crate::absl_status::StatusBuilder::new($code)
    };
    ($code:expr, $msg:expr) => {
        $crate::absl_status::StatusBuilder::new($code).with_message($msg)
    };
    ($code:expr, $msg:expr, $($key:expr => $value:expr),+ $(,)?) => {{
        let mut builder = $crate::absl_status::StatusBuilder::new($code).with_message($msg);
        $(
            builder = builder.with_payload($key, $value);
        )+
        builder
    }};
}

/// Macro for creating an error result from a status code and message.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_status::{status_err, StatusCode};
///
/// fn get_user() -> Result<String, Status> {
///     Err(status_err!(StatusCode::NotFound, "User not found"))
/// }
/// ```
#[macro_export]
macro_rules! status_err {
    ($code:expr) => {
        Err($crate::absl_status::Status::from_code($code))
    };
    ($code:expr, $msg:expr) => {
        Err($crate::absl_status::Status::new($code, $msg))
    };
}

/// Macro for creating an error StatusOr.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_status::{statusor_err, StatusCode};
///
/// fn get_user() -> StatusOr<String> {
///     statusor_err!(StatusCode::NotFound, "User not found")
/// }
/// ```
#[macro_export]
macro_rules! statusor_err {
    ($code:expr) => {
        $crate::absl_status::StatusOr::new($crate::absl_status::Status::from_code($code))
    };
    ($code:expr, $msg:expr) => {
        $crate::absl_status::StatusOr::new($crate::absl_status::Status::new($code, $msg))
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_status_builder_macro() {
        let status = status_builder!(StatusCode::NotFound, "Not found").build();
        assert_eq!(status.code(), StatusCode::NotFound);
        assert_eq!(status.message(), "Not found");
    }

    #[test]
    fn test_status_builder_macro_with_payloads() {
        let status = status_builder!(
            StatusCode::Internal,
            "Error",
            "key1" => "value1",
            "key2" => "value2"
        ).build();

        assert_eq!(status.code(), StatusCode::Internal);
        assert_eq!(status.message(), "Error");
    }

    #[test]
    fn test_status_err_macro() {
        let result: Result<(), Status> = status_err!(StatusCode::NotFound, "Not found");
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().code(), StatusCode::NotFound);
    }

    #[test]
    fn test_statusor_err_macro() {
        let result: StatusOr<u32> = statusor_err!(StatusCode::NotFound, "Not found");
        assert!(result.is_err());
        assert_eq!(result.status().code(), StatusCode::NotFound);
    }
}
