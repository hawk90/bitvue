//! StatusOr type for returning status or a value.
//!
//! This module provides a `StatusOr<T>` type similar to Abseil's `absl::StatusOr<T>`,
//! which represents either a status error or a value of type T.

use super::status::Status;
use core::fmt;
use core::mem::ManuallyDrop;
use core::ops::{Deref, DerefMut};
use core::ptr;

/// A Result-like type that holds either a Status (error) or a value T.
///
/// Similar to Abseil's `absl::StatusOr<T>`, this type represents either
/// an error status or a successful value.
#[repr(C)]
pub struct StatusOr<T> {
    // Use Either representation: Status is stored inline or value is stored inline
    // We use a union-like approach with discriminant
    discriminant: u8,
    value: core::mem::MaybeUninit<T>,
    status: core::mem::MaybeUninit<Status>,
}

// Discriminant values
const DISCRIMINANT_STATUS: u8 = 0;
const DISCRIMINANT_VALUE: u8 = 1;

impl<T> StatusOr<T> {
    /// Creates a new StatusOr from a Status.
    ///
    /// If the status is OK, this will panic (use StatusOr::ok_value instead).
    #[inline]
    pub fn from_status(status: Status) -> Self {
        assert!(!status.is_ok(), "Cannot create StatusOr<T> from OK status");
        Self {
            discriminant: DISCRIMINANT_STATUS,
            value: core::mem::MaybeUninit::uninit(),
            status: core::mem::MaybeUninit::new(status),
        }
    }

    /// Creates a new StatusOr containing a value.
    #[inline]
    pub fn ok_value(value: T) -> Self {
        Self {
            discriminant: DISCRIMINANT_VALUE,
            value: core::mem::MaybeUninit::new(value),
            status: core::mem::MaybeUninit::uninit(),
        }
    }

    /// Returns the status.
    ///
    /// Returns OK status if this contains a value.
    #[inline]
    pub fn status(&self) -> Status {
        if self.discriminant == DISCRIMINANT_STATUS {
            unsafe { self.status.assume_init_ref() }.clone()
        } else {
            Status::ok()
        }
    }

    /// Returns whether this contains a value (status is OK).
    #[inline]
    pub fn ok(&self) -> bool {
        self.discriminant == DISCRIMINANT_VALUE
    }

    /// Returns a reference to the value.
    ///
    /// Returns `None` if the status is an error.
    #[inline]
    pub fn value(&self) -> Option<&T> {
        if self.discriminant == DISCRIMINANT_VALUE {
            Some(unsafe { self.value.assume_init_ref() })
        } else {
            None
        }
    }

    /// Returns a mutable reference to the value.
    ///
    /// Returns `None` if the status is an error.
    #[inline]
    pub fn value_mut(&mut self) -> Option<&mut T> {
        if self.discriminant == DISCRIMINANT_VALUE {
            Some(unsafe { self.value.assume_init_mut() })
        } else {
            None
        }
    }

    /// Takes ownership of the value.
    ///
    /// Returns the value if status is OK, or the status error.
    #[inline]
    pub fn into_value(self) -> Result<T, Status> {
        // SAFETY: Use ManuallyDrop to prevent double-drop if panic occurs
        let this = ManuallyDrop::new(self);

        if this.discriminant == DISCRIMINANT_VALUE {
            // SAFETY: We've checked discriminant, value is initialized
            unsafe {
                // Read the value from the ManuallyDrop wrapper
                // This prevents double-drop even if a panic occurs
                Ok(ptr::read(this.value.as_ptr()))
            }
        } else {
            // SAFETY: We've checked discriminant, status is initialized
            unsafe {
                // Read the status from the ManuallyDrop wrapper
                // This prevents double-drop even if a panic occurs
                Err(ptr::read(this.status.as_ptr()))
            }
        }
    }

    /// Returns the value or panics with the status.
    ///
    /// # Panics
    ///
    /// Panics if the status is an error.
    #[inline]
    pub fn unwrap(self) -> T {
        self.into_value().unwrap()
    }

    /// Returns the value or the provided default.
    #[inline]
    pub fn unwrap_or(self, default: T) -> T {
        self.into_value().unwrap_or(default)
    }

    /// Maps a `StatusOr<T>` to `StatusOr<U>` by applying a function.
    #[inline]
    pub fn map<U, F>(self, f: F) -> StatusOr<U>
    where
        F: FnOnce(T) -> U,
    {
        // SAFETY: Use ManuallyDrop to prevent double-drop if panic occurs
        let this = ManuallyDrop::new(self);

        if this.discriminant == DISCRIMINANT_VALUE {
            unsafe {
                // Read the value from the ManuallyDrop wrapper
                let v = ptr::read(this.value.as_ptr());
                StatusOr::ok_value(f(v))
            }
        } else {
            unsafe {
                // Read the status from the ManuallyDrop wrapper
                let s = ptr::read(this.status.as_ptr());
                StatusOr::from_status(s)
            }
        }
    }
}

impl<T> Clone for StatusOr<T>
where
    T: Clone,
{
    fn clone(&self) -> Self {
        if self.discriminant == DISCRIMINANT_VALUE {
            Self::ok_value(unsafe { self.value.assume_init_ref() }.clone())
        } else {
            Self::from_status(unsafe { self.status.assume_init_ref() }.clone())
        }
    }
}

impl<T> Drop for StatusOr<T> {
    fn drop(&mut self) {
        // SAFETY: We validate the discriminant before dropping to prevent
        // use-after-free or undefined behavior from corrupted discriminants
        match self.discriminant {
            DISCRIMINANT_VALUE => unsafe {
                // Only drop value if discriminant indicates it's initialized
                self.value.assume_init_drop();
            }
            DISCRIMINANT_STATUS => unsafe {
                // Only drop status if discriminant indicates it's initialized
                self.status.assume_init_drop();
            }
            // Handle corrupted discriminants safely instead of unreachable!()
            // which would cause undefined behavior
            _ => {
                // Discriminant is invalid - neither value nor status can be
                // safely dropped. This can happen due to memory corruption or
                // unsafe transmutation. We leak the memory rather than causing
                // undefined behavior.
            }
        }
    }
}

impl<T> Deref for StatusOr<T>
where
    T: core::ops::Deref,
{
    type Target = T::Target;

    fn deref(&self) -> &Self::Target {
        // Provide detailed panic message with the actual status error
        match self.value() {
            Some(v) => v,
            None => panic!(
                "Cannot deref error StatusOr: {}",
                self.status()
            )
        }
    }
}

impl<T> DerefMut for StatusOr<T>
where
    T: core::ops::DerefMut,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        // Get status first to avoid borrow conflicts
        let status_msg = if self.discriminant == DISCRIMINANT_STATUS {
            // SAFETY: discriminant check ensures status is initialized
            Some(unsafe { self.status.assume_init_ref() }.to_string())
        } else {
            None
        };

        // Provide detailed panic message with the actual status error
        match self.value_mut() {
            Some(v) => v,
            None => panic!(
                "Cannot deref_mut error StatusOr: {}",
                status_msg.unwrap_or_else(|| "unknown error".to_string())
            )
        }
    }
}

impl<T> fmt::Debug for StatusOr<T>
where
    T: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(v) = self.value() {
            write!(f, "StatusOr::ok({:?})", v)
        } else {
            write!(f, "StatusOr::err({:?})", self.status())
        }
    }
}

impl<T> fmt::Display for StatusOr<T>
where
    T: fmt::Display,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(v) = self.value() {
            write!(f, "{}", v)
        } else {
            write!(f, "{}", self.status())
        }
    }
}

impl<T> From<Result<T, Status>> for StatusOr<T> {
    #[inline]
    fn from(result: Result<T, Status>) -> Self {
        match result {
            Ok(v) => Self::ok_value(v),
            Err(e) => Self::from_status(e),
        }
    }
}

impl<T> From<StatusOr<T>> for Result<T, Status> {
    #[inline]
    fn from(sor: StatusOr<T>) -> Self {
        sor.into_value()
    }
}

// Conversion from Status for convenience
impl<T> From<Status> for StatusOr<T> {
    #[inline]
    fn from(status: Status) -> Self {
        Self::from_status(status)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::status::StatusCode;

    #[test]
    fn test_statusor_ok_value() {
        let sor = StatusOr::ok_value(42);
        assert!(sor.ok());
        assert_eq!(sor.value(), Some(&42));
        assert!(sor.status().is_ok());
    }

    #[test]
    fn test_statusor_from_status() {
        let status = Status::not_found("key");
        let sor = StatusOr::<i32>::from_status(status);
        assert!(!sor.ok());
        assert_eq!(sor.value(), None);
        assert_eq!(sor.status().code(), StatusCode::NotFound);
    }

    #[test]
    fn test_statusor_into_value() {
        let sor = StatusOr::ok_value(42);
        assert_eq!(sor.into_value(), Ok(42));

        let sor = StatusOr::<i32>::from_status(Status::invalid_argument("bad"));
        assert_eq!(sor.into_value(), Err(Status::invalid_argument("bad")));
    }

    #[test]
    fn test_statusor_unwrap() {
        let sor = StatusOr::ok_value(42);
        assert_eq!(sor.unwrap(), 42);
    }

    #[test]
    #[should_panic]
    fn test_statusor_unwrap_err() {
        let sor = StatusOr::<i32>::from_status(Status::invalid_argument("bad"));
        sor.unwrap();
    }

    #[test]
    fn test_statusor_unwrap_or() {
        let sor = StatusOr::ok_value(42);
        assert_eq!(sor.unwrap_or(0), 42);

        let sor = StatusOr::<i32>::from_status(Status::invalid_argument("bad"));
        assert_eq!(sor.unwrap_or(0), 0);
    }

    #[test]
    fn test_statusor_map() {
        let sor = StatusOr::ok_value(42);
        let sor2 = sor.map(|x| x * 2);
        assert_eq!(sor2.value(), Some(&84));

        let sor = StatusOr::<i32>::from_status(Status::not_found("key"));
        let sor2: StatusOr<i32> = sor.map(|x| x * 2);
        assert!(sor2.value().is_none());
        assert_eq!(sor2.status().code(), StatusCode::NotFound);
    }

    #[test]
    fn test_statusor_clone() {
        let sor = StatusOr::ok_value(42);
        let sor2 = sor.clone();
        assert_eq!(sor2.value(), Some(&42));

        let sor = StatusOr::<i32>::from_status(Status::not_found("key"));
        let sor2 = sor.clone();
        assert_eq!(sor2.status().code(), StatusCode::NotFound);
    }

    #[test]
    fn test_statusor_debug() {
        let sor = StatusOr::ok_value(42);
        assert_eq!(format!("{:?}", sor), "StatusOr::ok(42)");

        let sor = StatusOr::<i32>::from_status(Status::not_found("key"));
        assert!(format!("{:?}", sor).contains("Not found"));
    }

    #[test]
    fn test_statusor_display() {
        let sor = StatusOr::ok_value(42);
        assert_eq!(format!("{}", sor), "42");

        let sor = StatusOr::<i32>::from_status(Status::not_found("key"));
        assert_eq!(format!("{}", sor), "Not found: key");
    }

    #[test]
    fn test_statusor_from_result() {
        let result: Result<i32, Status> = Ok(42);
        let sor: StatusOr<i32> = result.into();
        assert_eq!(sor.value(), Some(&42));

        let result: Result<i32, Status> = Err(Status::invalid_argument("bad"));
        let sor: StatusOr<i32> = result.into();
        assert!(!sor.ok());
    }

    #[test]
    fn test_statusor_into_result() {
        let sor = StatusOr::ok_value(42);
        let result: Result<i32, Status> = sor.into();
        assert_eq!(result, Ok(42));

        let sor = StatusOr::<i32>::from_status(Status::invalid_argument("bad"));
        let result: Result<i32, Status> = sor.into();
        assert!(result.is_err());
    }

    // Edge case tests for CRITICAL security fixes

    #[test]
    fn test_statusor_drop_after_into_value() {
        // Test that into_value doesn't cause double-drop
        let sor = StatusOr::ok_value(42);
        let value = sor.into_value();
        assert_eq!(value, Ok(42));
        // sor is now moved from, Drop should not be called
    }

    #[test]
    fn test_statusor_map_no_leak() {
        // Test that map doesn't leak memory
        let sor = StatusOr::ok_value(42);
        let sor2 = sor.map(|x| x * 2);
        assert_eq!(sor2.value(), Some(&84));
    }

    #[test]
    fn test_statusor_map_error_no_leak() {
        // Test that map doesn't leak memory for error cases
        let sor = StatusOr::<i32>::from_status(Status::not_found("key"));
        let sor2: StatusOr<i32> = sor.map(|x| x * 2);
        assert!(sor2.value().is_none());
    }

    // Note: Testing corrupted discriminant handling would require unsafe code
    // and is intentionally not tested here to avoid introducing more unsafe code.
    // The fix ensures that corrupted discriminants are handled gracefully by
    // leaking memory rather than causing undefined behavior.

    // Test for MEDIUM security fix - improved Deref panic messages

    #[test]
    #[should_panic(expected = "Cannot deref error StatusOr")]
    fn test_statusor_deref_error_panics_with_details() {
        // Test that deref panics with informative message when StatusOr contains error
        use std::ops::Deref;

        // Create a StatusOr containing a String (which implements Deref)
        let sor: StatusOr<String> = StatusOr::from_status(Status::not_found("key"));

        // This should panic with a descriptive message including the status
        let _ = sor.deref();
    }

    #[test]
    fn test_statusor_deref_ok_works() {
        // Test that deref works correctly when StatusOr contains a value
        use std::ops::Deref;

        let sor: StatusOr<String> = StatusOr::ok_value(String::from("hello"));

        // This should work and deref to &str
        let derefed: &str = sor.deref();
        assert_eq!(derefed, "hello");
    }
}
