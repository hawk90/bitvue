//! Error chain for tracking error context through multiple layers.

use alloc::string::String;
use alloc::vec::Vec;

use super::status::Status;
use super::statusor::StatusOr;
use super::StatusCode;

/// A chain of errors with context.
///
/// This allows building a chain of errors where each level adds more context.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_status::{ErrorChain, StatusCode};
///
/// let chain = ErrorChain::new(StatusCode::Internal, "Operation failed")
///     .push_context("In function: process_data")
///     .push_context("While processing user_id: 12345");
/// ```
pub struct ErrorChain {
    errors: Vec<(StatusCode, String)>,
}

impl ErrorChain {
    /// Creates a new error chain.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use abseil::absl_status::{ErrorChain, StatusCode};
    ///
    /// let chain = ErrorChain::new(StatusCode::Internal, "Database error");
    /// ```
    pub fn new(code: StatusCode, message: impl Into<String>) -> Self {
        Self {
            errors: vec![(code, message.into())],
        }
    }

    /// Pushes additional context onto the error chain.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use abseil::absl_status::{ErrorChain, StatusCode};
    ///
    /// let chain = ErrorChain::new(StatusCode::Internal, "Base error")
    ///     .push_context("In function: foo");
    /// ```
    pub fn push_context(mut self, context: impl Into<String>) -> Self {
        self.errors.push((StatusCode::Internal, context.into()));
        self
    }

    /// Returns the number of errors in the chain.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use abseil::absl_status::{ErrorChain, StatusCode};
    ///
    /// let chain = ErrorChain::new(StatusCode::Internal, "Error")
    ///     .push_context("Context 1")
    ///     .push_context("Context 2");
    /// assert_eq!(chain.len(), 3);
    /// ```
    pub fn len(&self) -> usize {
        self.errors.len()
    }

    /// Returns true if the chain is empty.
    pub fn is_empty(&self) -> bool {
        self.errors.is_empty()
    }

    /// Returns the root (first) error code.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use abseil::absl_status::{ErrorChain, StatusCode};
    ///
    /// let chain = ErrorChain::new(StatusCode::NotFound, "Not found")
    ///     .push_context("Context");
    /// assert_eq!(chain.root_code(), StatusCode::NotFound);
    /// ```
    pub fn root_code(&self) -> StatusCode {
        self.errors.first().map(|(code, _)| *code).unwrap_or(StatusCode::Ok)
    }

    /// Returns the root (first) error message.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use abseil::absl_status::{ErrorChain, StatusCode};
    ///
    /// let chain = ErrorChain::new(StatusCode::Internal, "Root error")
    ///     .push_context("Context");
    /// assert_eq!(chain.root_message(), "Root error");
    /// ```
    pub fn root_message(&self) -> &str {
        self.errors.first().map(|(_, msg)| msg.as_str()).unwrap_or("")
    }

    /// Converts the chain to a Status with all messages joined.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use abseil::absl_status::{ErrorChain, StatusCode};
    ///
    /// let chain = ErrorChain::new(StatusCode::Internal, "Root error")
    ///     .push_context("Context 1")
    ///     .push_context("Context 2");
    /// let status = chain.to_status();
    /// ```
    pub fn to_status(&self) -> Status {
        if self.errors.is_empty() {
            return Status::ok();
        }

        let (code, _) = self.errors[0];
        let message = self.errors.iter()
            .map(|(_, msg)| msg.as_str())
            .collect::<Vec<_>>()
            .join(": ");

        Status::new(code, message)
    }

    /// Iterates over the error chain from root to most recent.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use abseil::absl_status::{ErrorChain, StatusCode};
    ///
    /// let chain = ErrorChain::new(StatusCode::Internal, "Error 1")
    ///     .push_context("Error 2");
    ///
    /// for (code, message) in chain.iter() {
    ///     println!("{}: {}", code, message);
    /// }
    /// ```
    pub fn iter(&self) -> ErrorChainIter<'_> {
        ErrorChainIter {
            chain: self,
            index: 0,
        }
    }
}

/// Iterator over an error chain.
pub struct ErrorChainIter<'a> {
    chain: &'a ErrorChain,
    index: usize,
}

impl<'a> Iterator for ErrorChainIter<'a> {
    type Item = (StatusCode, &'a str);

    fn next(&mut self) -> Option<Self::Item> {
        if self.index < self.chain.errors.len() {
            let (code, msg) = &self.chain.errors[self.index];
            self.index += 1;
            Some((*code, msg.as_str()))
        } else {
            None
        }
    }
}

/// Trait for converting types to Status.
///
/// This is similar to the `std::error::Error` trait but specifically for Status.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_status::{ToStatus, StatusCode};
///
/// fn handle_error<E: ToStatus>(error: E) -> Status {
///     error.to_status(StatusCode::Internal)
/// }
/// ```
pub trait ToStatus {
    /// Converts this type to a Status with the given code.
    fn to_status(&self, code: StatusCode) -> Status;
}

impl ToStatus for Status {
    fn to_status(&self, _code: StatusCode) -> Status {
        self.clone()
    }
}

impl ToStatus for alloc::string::String {
    fn to_status(&self, code: StatusCode) -> Status {
        Status::new(code, self.as_str())
    }
}

impl<'a> ToStatus for &'a str {
    fn to_status(&self, code: StatusCode) -> Status {
        Status::new(code, *self)
    }
}

/// Trait for types that can be checked if they represent an error.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_status::IsError;
///
/// fn check_status<T: IsError>(value: &T) -> bool {
///     value.is_error()
/// }
/// ```
pub trait IsError {
    /// Returns true if this represents an error condition.
    fn is_error(&self) -> bool;

    /// Returns true if this represents success.
    fn is_ok(&self) -> bool {
        !self.is_error()
    }
}

impl IsError for Status {
    fn is_error(&self) -> bool {
        !self.is_ok()
    }
}

impl<T, E> IsError for Result<T, E>
where
    E: IsError,
{
    fn is_error(&self) -> bool {
        match self {
            Err(e) => e.is_error(),
            Ok(_) => false,
        }
    }
}

impl<T> IsError for StatusOr<T> {
    fn is_error(&self) -> bool {
        !self.is_ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_chain() {
        let chain = ErrorChain::new(StatusCode::Internal, "Root error")
            .push_context("Context 1")
            .push_context("Context 2");

        assert_eq!(chain.len(), 3);
        assert_eq!(chain.root_code(), StatusCode::Internal);
        assert_eq!(chain.root_message(), "Root error");
    }

    #[test]
    fn test_error_chain_iter() {
        let chain = ErrorChain::new(StatusCode::Internal, "Error 1")
            .push_context("Error 2")
            .push_context("Error 3");

        let codes: Vec<_> = chain.iter().map(|(code, _)| code).collect();
        assert_eq!(codes.len(), 3);
    }

    #[test]
    fn test_error_chain_to_status() {
        let chain = ErrorChain::new(StatusCode::Internal, "Root")
            .push_context("Ctx1");

        let status = chain.to_status();
        assert_eq!(status.code(), StatusCode::Internal);
    }

    #[test]
    fn test_to_status_for_string() {
        let msg = "Error message";
        let status = msg.to_status(StatusCode::Internal);
        assert_eq!(status.code(), StatusCode::Internal);
        assert_eq!(status.message(), "Error message");
    }

    #[test]
    fn test_to_status_for_str() {
        let msg = "Error message";
        let status = msg.to_status(StatusCode::NotFound);
        assert_eq!(status.code(), StatusCode::NotFound);
        assert_eq!(status.message(), "Error message");
    }

    #[test]
    fn test_is_error_for_status() {
        let ok_status = Status::ok();
        let err_status = Status::new(StatusCode::Internal, "Error");

        assert!(!ok_status.is_error());
        assert!(err_status.is_error());
    }

    #[test]
    fn test_is_error_for_result() {
        let ok_result: Result<(), Status> = Ok(());
        let err_result: Result<(), Status> = Err(Status::new(StatusCode::Internal, "Error"));

        assert!(!ok_result.is_error());
        assert!(err_result.is_error());
    }

    #[test]
    fn test_is_error_for_statusor() {
        let ok_result: StatusOr<u32> = Ok(42);
        let err_result: StatusOr<u32> = StatusOr::new(Status::new(StatusCode::Internal, "Error"));

        assert!(!ok_result.is_error());
        assert!(err_result.is_error());
    }
}
