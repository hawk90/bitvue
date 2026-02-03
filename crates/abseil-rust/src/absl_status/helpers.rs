//! Helper functions for working with Status types.

use alloc::vec::Vec;

use super::status::Status;
use super::statusor::StatusOr;
use super::StatusCode;

/// Helper functions for working with Status types.
pub struct StatusHelpers;

impl StatusHelpers {
    /// Returns the first error in a list, or Ok if all are Ok.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use abseil::absl_status::{Status, StatusHelpers};
    ///
    /// let results = vec![
    ///     Ok(()),
    ///     Ok(()),
    /// ];
    /// assert!(StatusHelpers::first_error(results).is_ok());
    ///
    /// let results = vec![
    ///     Ok(()),
    ///     Err(Status::new(StatusCode::Internal, "Error")),
    /// ];
    /// assert!(StatusHelpers::first_error(results).is_err());
    /// ```
    pub fn first_error(results: Vec<Result<(), Status>>) -> Status {
        for result in results {
            if let Err(status) = result {
                return status;
            }
        }
        Status::ok()
    }

    /// Collects all errors from a list of results.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use abseil::absl_status::{Status, StatusHelpers};
    ///
    /// let results = vec![
    ///     Err(Status::new(StatusCode::Internal, "Error 1")),
    ///     Ok(()),
    ///     Err(Status::new(StatusCode::Internal, "Error 2")),
    /// ];
    /// let errors = StatusHelpers::collect_errors(results);
    /// assert_eq!(errors.len(), 2);
    /// ```
    pub fn collect_errors(results: Vec<Result<(), Status>>) -> Vec<Status> {
        results.into_iter()
            .filter_map(|r| r.err())
            .collect()
    }

    /// Combines multiple StatusOr values into one.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use abseil::absl_status::{StatusOr, StatusHelpers};
    ///
    /// fn combine() -> StatusOr<Vec<u32>> {
    ///     let a: StatusOr<u32> = Ok(1);
    ///     let b: StatusOr<u32> = Ok(2);
    ///     let c: StatusOr<u32> = Ok(3);
    ///
    ///     StatusHelpers::combine_all(vec![a, b, c])
    /// }
    /// ```
    pub fn combine_all(results: Vec<StatusOr<u32>>) -> StatusOr<Vec<u32>> {
        let mut values = Vec::new();
        for result in results {
            match result {
                Ok(value) => values.push(value),
                Err(status) => return Err(status),
            }
        }
        Ok(values)
    }

    /// Checks if a status is a specific error code.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use abseil::absl_status::{Status, StatusCode, StatusHelpers};
    ///
    /// let status = Status::new(StatusCode::NotFound, "Not found");
    /// assert!(StatusHelpers::is_code(&status, StatusCode::NotFound));
    /// ```
    pub fn is_code(status: &Status, code: StatusCode) -> bool {
        status.code() == code
    }

    /// Checks if a status is any of the given error codes.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use abseil::absl_status::{Status, StatusCode, StatusHelpers};
    ///
    /// let status = Status::new(StatusCode::NotFound, "Not found");
    /// assert!(StatusHelpers::is_any_code(&status, &[StatusCode::NotFound, StatusCode::PermissionDenied]));
    /// ```
    pub fn is_any_code(status: &Status, codes: &[StatusCode]) -> bool {
        codes.contains(&status.code())
    }

    /// Returns true if the status is OK (success).
    ///
    /// # Examples
    ///
    /// ```rust
    /// use abseil::absl_status::{Status, StatusHelpers};
    ///
    /// let status = Status::ok();
    /// assert!(StatusHelpers::is_ok(&status));
    /// ```
    pub fn is_ok(status: &Status) -> bool {
        status.is_ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_helpers_first_error() {
        let results = vec![
            Ok(()),
            Err(Status::new(StatusCode::Internal, "Error 1")),
            Err(Status::new(StatusCode::Internal, "Error 2")),
        ];

        let status = StatusHelpers::first_error(results);
        assert_eq!(status.message(), "Error 1");
    }

    #[test]
    fn test_helpers_first_error_all_ok() {
        let results = vec![Ok(()), Ok(())];
        let status = StatusHelpers::first_error(results);
        assert!(status.is_ok());
    }

    #[test]
    fn test_helpers_collect_errors() {
        let results = vec![
            Err(Status::new(StatusCode::Internal, "Error 1")),
            Ok(()),
            Err(Status::new(StatusCode::Internal, "Error 2")),
        ];

        let errors = StatusHelpers::collect_errors(results);
        assert_eq!(errors.len(), 2);
    }

    #[test]
    fn test_helpers_combine_all() {
        let results: Vec<StatusOr<u32>> = vec![Ok(1), Ok(2), Ok(3)];
        let combined = StatusHelpers::combine_all(results);
        assert_eq!(combined.unwrap(), vec![1, 2, 3]);
    }

    #[test]
    fn test_helpers_combine_all_with_error() {
        let results: Vec<StatusOr<u32>> = vec![
            Ok(1),
            Err(Status::new(StatusCode::Internal, "Error")),
            Ok(3),
        ];

        let combined = StatusHelpers::combine_all(results);
        assert!(combined.is_err());
    }

    #[test]
    fn test_helpers_is_code() {
        let status = Status::new(StatusCode::NotFound, "Not found");
        assert!(StatusHelpers::is_code(&status, StatusCode::NotFound));
        assert!(!StatusHelpers::is_code(&status, StatusCode::Internal));
    }

    #[test]
    fn test_helpers_is_any_code() {
        let status = Status::new(StatusCode::NotFound, "Not found");
        assert!(StatusHelpers::is_any_code(
            &status,
            &[StatusCode::NotFound, StatusCode::PermissionDenied]
        ));
        assert!(!StatusHelpers::is_any_code(
            &status,
            &[StatusCode::Internal, StatusCode::PermissionDenied]
        ));
    }

    #[test]
    fn test_helpers_is_ok() {
        let ok_status = Status::ok();
        let err_status = Status::new(StatusCode::Internal, "Error");

        assert!(StatusHelpers::is_ok(&ok_status));
        assert!(!StatusHelpers::is_ok(&err_status));
    }
}
