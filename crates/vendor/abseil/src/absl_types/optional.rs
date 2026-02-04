//! Optional wrapper type.
//!
//! This module provides `Optional`, a wrapper similar to Rust's `Option` but
//! with additional tracking of whether the value was explicitly set.
//!
//! # Example
//!
//! ```rust
//! use abseil::absl_types::optional::Optional;
//!
//! let opt: Optional<i32> = Optional::none();
//! assert!(opt.is_none());
//! assert!(!opt.was_specified()); // Not explicitly set
//!
//! let opt2: Optional<i32> = Optional::some(42);
//! assert!(opt2.is_some());
//! assert!(opt2.was_specified());
//! ```

use core::fmt;
use core::ops::{Deref, DerefMut};

/// A wrapper type that represents an optional value.
///
/// Unlike Rust's `Option`, this type tracks whether a value was explicitly
/// specified or if it's just the default "none" state.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Optional<T> {
    value: Option<T>,
    specified: bool,
}

impl<T> Optional<T> {
    /// Creates an `Optional` with no value (unspecified).
    #[inline]
    pub const fn none() -> Self {
        Self {
            value: None,
            specified: false,
        }
    }

    /// Creates an `Optional` with no value (explicitly specified).
    #[inline]
    pub const fn unspecified() -> Self {
        Self {
            value: None,
            specified: true,
        }
    }

    /// Creates an `Optional` with a value.
    #[inline]
    pub const fn some(value: T) -> Self {
        Self {
            value: Some(value),
            specified: true,
        }
    }

    /// Creates an `Optional` from a Rust `Option`.
    #[inline]
    pub fn from_option(opt: Option<T>) -> Self {
        let specified = opt.is_some();
        Self {
            value: opt,
            specified,
        }
    }

    /// Returns `true` if there's a value present.
    #[inline]
    pub fn is_some(&self) -> bool {
        self.value.is_some()
    }

    /// Returns `true` if there's no value.
    #[inline]
    pub fn is_none(&self) -> bool {
        self.value.is_none()
    }

    /// Returns `true` if the value was explicitly specified (even if specified as none).
    #[inline]
    pub const fn was_specified(&self) -> bool {
        self.specified
    }

    /// Returns the value if present.
    #[inline]
    pub fn as_ref(&self) -> Option<&T> {
        self.value.as_ref()
    }

    /// Returns the value if present, converting to `Option`.
    #[inline]
    pub fn as_deref(&self) -> Option<&T> {
        self.as_ref()
    }

    /// Converts to `Option`.
    #[inline]
    pub fn to_option(&self) -> Option<&T> {
        self.value.as_ref()
    }

    /// Converts to `Option`, taking ownership.
    #[inline]
    pub fn into_option(self) -> Option<T> {
        self.value
    }

    /// Takes the value, leaving `None` in its place.
    #[inline]
    pub fn take(&mut self) -> Option<T> {
        self.value.take()
    }

    /// Unwraps the value.
    ///
    /// # Panics
    ///
    /// Panics if the value is `None`.
    #[inline]
    pub fn unwrap(&self) -> &T {
        self.value
            .as_ref()
            .expect("Optional::unwrap() called on None")
    }

    /// Unwraps the value mutably.
    ///
    /// # Panics
    ///
    /// Panics if the value is `None`.
    #[inline]
    pub fn unwrap_mut(&mut self) -> &mut T {
        self.value
            .as_mut()
            .expect("Optional::unwrap_mut() called on None")
    }

    /// Returns the value or a default.
    #[inline]
    pub fn unwrap_or<'a>(&'a self, default: &'a T) -> &'a T {
        self.value.as_ref().unwrap_or(default)
    }

    /// Returns the value or a default (cloned version).
    #[inline]
    pub fn unwrap_or_clone(&self) -> T
    where
        T: Clone + Default,
    {
        self.value.as_ref().cloned().unwrap_or_default()
    }

    /// Maps an `Optional<T>` to `Optional<U>` by applying a function.
    pub fn map<U, F>(self, f: F) -> Optional<U>
    where
        F: FnOnce(T) -> U,
    {
        Optional {
            value: self.value.map(f),
            specified: self.specified,
        }
    }

    /// Returns the value if present, otherwise returns `default`.
    #[inline]
    pub fn unwrap_or_else<F>(self, default: F) -> T
    where
        F: FnOnce() -> T,
    {
        match self.value {
            Some(v) => v,
            None => default(),
        }
    }

    /// Sets the value.
    pub fn set(&mut self, value: T) {
        self.value = Some(value);
        self.specified = true;
    }

    /// Clears the value.
    pub fn clear(&mut self) {
        self.value = None;
        // Keep specified flag to indicate it was once set
    }

    /// Resets to unspecified state.
    pub fn reset(&mut self) {
        self.value = None;
        self.specified = false;
    }
}

impl<T> Default for Optional<T> {
    fn default() -> Self {
        Self::none()
    }
}

impl<T> Deref for Optional<T> {
    type Target = Option<T>;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

impl<T> DerefMut for Optional<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.value
    }
}

impl<T> fmt::Debug for Optional<T>
where
    T: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Optional")
            .field(&self.value)
            .field(&self.specified)
            .finish()
    }
}

impl<T> fmt::Display for Optional<T>
where
    T: fmt::Display,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.value {
            Some(v) => write!(f, "Some({})", v),
            None => write!(f, "None"),
        }
    }
}

impl<T> From<Option<T>> for Optional<T> {
    fn from(opt: Option<T>) -> Self {
        Self::from_option(opt)
    }
}

impl<T> From<T> for Optional<T> {
    fn from(value: T) -> Self {
        Self::some(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_none() {
        let opt = Optional::<i32>::none();
        assert!(opt.is_none());
        assert!(!opt.was_specified());
    }

    #[test]
    fn test_unspecified() {
        let opt = Optional::<i32>::unspecified();
        assert!(opt.is_none());
        assert!(opt.was_specified());
    }

    #[test]
    fn test_some() {
        let opt = Optional::some(42);
        assert!(opt.is_some());
        assert!(opt.was_specified());
        assert_eq!(*opt.unwrap(), 42);
    }

    #[test]
    fn test_from_option() {
        let opt = Optional::from_option(Some(42));
        assert!(opt.is_some());
        assert!(opt.was_specified());
        assert_eq!(*opt.unwrap(), 42);

        let none: Optional<i32> = Optional::from_option(None);
        assert!(none.is_none());
        assert!(!none.was_specified());
    }

    #[test]
    fn test_take() {
        let mut opt = Optional::some(42);
        assert_eq!(opt.take(), Some(42));
        assert!(opt.is_none());
        assert!(opt.was_specified()); // Still marked as specified
    }

    #[test]
    fn test_clear() {
        let mut opt = Optional::some(42);
        opt.clear();
        assert!(opt.is_none());
        assert!(opt.was_specified()); // Still marked as specified
    }

    #[test]
    fn test_reset() {
        let mut opt = Optional::some(42);
        opt.reset();
        assert!(opt.is_none());
        assert!(!opt.was_specified());
    }

    #[test]
    fn test_map() {
        let opt = Optional::some(42);
        let mapped = opt.map(|x| x * 2);
        assert_eq!(*mapped.unwrap(), 84);
        assert!(mapped.was_specified());
    }

    #[test]
    fn test_from() {
        let opt: Optional<i32> = Optional::from(42);
        assert!(opt.is_some());

        let from_opt: Optional<i32> = Option::<i32>::Some(42).into();
        assert!(from_opt.is_some());
    }

    #[test]
    fn test_display() {
        let some = Optional::some(42);
        assert_eq!(format!("{}", some), "Some(42)");

        let none: Optional<i32> = Optional::none();
        assert_eq!(format!("{}", none), "None");
    }
}
