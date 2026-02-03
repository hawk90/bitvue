//! AnyComparison - Equality and ordering for type-erased values.

use alloc::format;
use core::cmp::Ordering;
use crate::absl_any::any_box::AnyBox;

/// Trait for comparing type-erased values.
///
/// # Examples
///
/// ```
/// use abseil::absl_any::{AnyBox, AnyEq};
///
/// let a = AnyBox::new(42i32);
/// let b = AnyBox::new(42i32);
/// assert!(AnyEq::any_eq(&a, &b));
/// ```
pub trait AnyEq {
    /// Returns true if two type-erased values are equal.
    ///
    /// Returns false if the types don't match.
    fn any_eq(a: &AnyBox, b: &AnyBox) -> bool;
}

/// Implementation of AnyEq for types implementing PartialEq.
impl<T: PartialEq + 'static> AnyEq for T {
    fn any_eq(a: &AnyBox, b: &AnyBox) -> bool {
        match (a.downcast_ref::<T>(), b.downcast_ref::<T>()) {
            (Some(av), Some(bv)) => av == bv,
            _ => false,
        }
    }
}

/// Compares two AnyBox values for equality.
///
/// Returns false if they contain different types.
///
/// # Examples
///
/// ```
/// use abseil::absl_any::{any_eq, AnyBox};
///
/// let a = AnyBox::new(42i32);
/// let b = AnyBox::new(42i32);
/// assert!(any_eq(&a, &b));
///
/// let c = AnyBox::new(100i32);
/// assert!(!any_eq(&a, &c));
/// ```
pub fn any_eq(a: &AnyBox, b: &AnyBox) -> bool {
    if a.type_id() != b.type_id() {
        return false;
    }
    // For primitive types, we can do direct comparison
    if a.is::<i32>() {
        match (a.downcast_ref::<i32>(), b.downcast_ref::<i32>()) {
            (Some(av), Some(bv)) => return av == bv,
            _ => return false,
        }
    }
    if a.is::<i64>() {
        match (a.downcast_ref::<i64>(), b.downcast_ref::<i64>()) {
            (Some(av), Some(bv)) => return av == bv,
            _ => return false,
        }
    }
    if a.is::<alloc::string::String>() {
        match (a.downcast_ref::<alloc::string::String>(), b.downcast_ref::<alloc::string::String>()) {
            (Some(av), Some(bv)) => return av == bv,
            _ => return false,
        }
    }
    // Fall back to debug string comparison
    format!("{:?}", a) == format!("{:?}", b)
}

/// Compares two AnyBox values for ordering.
///
/// Returns None if they contain different types or aren't comparable.
///
/// # Examples
///
/// ```
/// use abseil::absl_any::{any_cmp, AnyBox};
/// use core::cmp::Ordering;
///
/// let a = AnyBox::new(42i32);
/// let b = AnyBox::new(50i32);
/// assert_eq!(any_cmp(&a, &b), Some(Ordering::Less));
/// ```
pub fn any_cmp(a: &AnyBox, b: &AnyBox) -> Option<Ordering> {
    if a.type_id() != b.type_id() {
        return None;
    }
    if let (Some(av), Some(bv)) = (a.downcast_ref::<i32>(), b.downcast_ref::<i32>()) {
        return Some(av.cmp(bv));
    }
    if let (Some(av), Some(bv)) = (a.downcast_ref::<i64>(), b.downcast_ref::<i64>()) {
        return Some(av.cmp(bv));
    }
    if let (Some(av), Some(bv)) = (a.downcast_ref::<alloc::string::String>(), b.downcast_ref::<alloc::string::String>()) {
        return Some(av.cmp(bv));
    }
    if let (Some(av), Some(bv)) = (a.downcast_ref::<&str>(), b.downcast_ref::<&str>()) {
        return Some(av.cmp(bv));
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_any_eq_same_values() {
        let a = AnyBox::new(42i32);
        let b = AnyBox::new(42i32);
        assert!(any_eq(&a, &b));
    }

    #[test]
    fn test_any_eq_different_values() {
        let a = AnyBox::new(42i32);
        let b = AnyBox::new(100i32);
        assert!(!any_eq(&a, &b));
    }

    #[test]
    fn test_any_eq_different_types() {
        let a = AnyBox::new(42i32);
        let b = AnyBox::new("hello");
        assert!(!any_eq(&a, &b));
    }

    #[test]
    fn test_any_cmp_less() {
        let a = AnyBox::new(42i32);
        let b = AnyBox::new(50i32);
        assert_eq!(any_cmp(&a, &b), Some(Ordering::Less));
    }

    #[test]
    fn test_any_cmp_equal() {
        let a = AnyBox::new(42i32);
        let b = AnyBox::new(42i32);
        assert_eq!(any_cmp(&a, &b), Some(Ordering::Equal));
    }

    #[test]
    fn test_any_cmp_greater() {
        let a = AnyBox::new(50i32);
        let b = AnyBox::new(42i32);
        assert_eq!(any_cmp(&a, &b), Some(Ordering::Greater));
    }

    #[test]
    fn test_any_cmp_different_types() {
        let a = AnyBox::new(42i32);
        let b = AnyBox::new("hello");
        assert!(any_cmp(&a, &b).is_none());
    }
}
