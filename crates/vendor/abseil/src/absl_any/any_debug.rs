//! AnyDebug and AnyDisplay - Debug and display for type-erased values.

use alloc::format;
use crate::absl_any::any_box::AnyBox;

/// Trait for getting debug representation of type-erased values.
pub trait AnyDebug {
    /// Returns the debug representation of the value.
    fn any_debug(&self) -> String;
}

impl<T: core::fmt::Debug + 'static> AnyDebug for T {
    fn any_debug(&self) -> String {
        format!("{:?}", self)
    }
}

/// Returns the debug representation of a type-erased value.
///
/// # Examples
///
/// ```
/// use abseil::absl_any::{any_debug, AnyBox};
///
/// let boxed = AnyBox::new(42i32);
/// let debug_str = any_debug(&boxed);
/// assert!(debug_str.contains("42"));
/// ```
pub fn any_debug(value: &AnyBox) -> String {
    format!("{:?}", value.inner.as_ref())
}

/// Trait for displaying type-erased values.
pub trait AnyDisplay {
    /// Returns the display representation of the value.
    fn any_display(&self) -> String;
}

impl<T: core::fmt::Display + 'static> AnyDisplay for T {
    fn any_display(&self) -> String {
        format!("{}", self)
    }
}

/// Returns the display representation of a type-erased value.
///
/// # Examples
///
/// ```
/// use abseil::absl_any::{any_display, AnyBox};
///
/// let boxed = AnyBox::new("hello");
/// let display_str = any_display(&boxed);
/// assert_eq!(display_str, "hello");
/// ```
pub fn any_display(value: &AnyBox) -> String {
    if let Some(&v) = value.downcast_ref::<i32>() {
        format!("{}", v)
    } else if let Some(&v) = value.downcast_ref::<i64>() {
        format!("{}", v)
    } else if let Some(v) = value.downcast_ref::<alloc::string::String>() {
        v.clone()
    } else if let Some(&v) = value.downcast_ref::<&str>() {
        v.to_string()
    } else if let Some(&v) = value.downcast_ref::<bool>() {
        format!("{}", v)
    } else {
        format!("{:?}", value.inner.as_ref())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_any_debug() {
        let boxed = AnyBox::new(42i32);
        let debug_str = any_debug(&boxed);
        assert!(debug_str.contains("42"));
    }

    #[test]
    fn test_any_display_i32() {
        let boxed = AnyBox::new(42i32);
        assert_eq!(any_display(&boxed), "42");
    }

    #[test]
    fn test_any_display_string() {
        let boxed = AnyBox::new("hello");
        assert_eq!(any_display(&boxed), "hello");
    }

    #[test]
    fn test_any_display_bool() {
        let boxed = AnyBox::new(true);
        assert_eq!(any_display(&boxed), "true");
    }
}
