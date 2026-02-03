//! Container comparison utilities - equal, equivalent, compare

/// Checks if two containers are equal (have the same elements in the same order).
///
/// # Examples
///
/// ```rust
/// use abseil::absl_container::equal;
///
/// assert!(equal(&[1, 2, 3], &[1, 2, 3]));
/// assert!(!equal(&[1, 2, 3], &[1, 2, 4]));
/// assert!(!equal(&[1, 2, 3], &[1, 2]));
/// ```
#[inline]
pub fn equal<T: PartialEq>(a: &[T], b: &[T]) -> bool {
    a == b
}

/// Checks if two containers have the same elements, regardless of order.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_container::equivalent;
///
/// assert!(equivalent(&[1, 2, 3], &[3, 2, 1]));
/// assert!(equivalent(&[1, 2, 2, 3], &[2, 3, 1, 2]));
/// assert!(!equivalent(&[1, 2, 3], &[1, 2, 4]));
/// ```
#[inline]
pub fn equivalent<T: PartialEq + Clone>(a: &[T], b: &[T]) -> bool {
    if a.len() != b.len() {
        return false;
    }

    let mut b_copy = b.to_vec();
    for item in a {
        if let Some(pos) = b_copy.iter().position(|x| x == item) {
            b_copy.remove(pos);
        } else {
            return false;
        }
    }

    b_copy.is_empty()
}

/// Compares two containers lexicographically.
///
/// Returns `Less` if `a < b`, `Equal` if `a == b`, or `Greater` if `a > b`.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_container::compare;
/// use core::cmp::Ordering;
///
/// assert_eq!(compare(&[1, 2, 3], &[1, 2, 3]), Ordering::Equal);
/// assert_eq!(compare(&[1, 2, 3], &[1, 2, 4]), Ordering::Less);
/// assert_eq!(compare(&[1, 2, 4], &[1, 2, 3]), Ordering::Greater);
/// ```
#[inline]
pub fn compare<T: PartialOrd>(a: &[T], b: &[T]) -> core::cmp::Ordering {
    a.partial_cmp(b).unwrap_or(core::cmp::Ordering::Equal)
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::cmp::Ordering;

    #[test]
    fn test_equal() {
        assert!(equal(&[1, 2, 3], &[1, 2, 3]));
        assert!(!equal(&[1, 2, 3], &[1, 2, 4]));
        assert!(!equal(&[1, 2, 3], &[1, 2]));
    }

    #[test]
    fn test_equivalent() {
        assert!(equivalent(&[1, 2, 3], &[3, 2, 1]));
        assert!(equivalent(&[1, 2, 2, 3], &[2, 3, 1, 2]));
        assert!(!equivalent(&[1, 2, 3], &[1, 2, 4]));
    }

    #[test]
    fn test_compare() {
        assert_eq!(compare(&[1, 2, 3], &[1, 2, 3]), Ordering::Equal);
        assert_eq!(compare(&[1, 2, 3], &[1, 2, 4]), Ordering::Less);
        assert_eq!(compare(&[1, 2, 4], &[1, 2, 3]), Ordering::Greater);
    }
}
