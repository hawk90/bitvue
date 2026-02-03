//! Sorting utilities - is_sorted, is_sorted_by, is_sorted_descending

/// Checks if a slice is sorted in ascending order.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_algorithm::is_sorted;
///
/// assert!(is_sorted(&[1, 2, 3, 4, 5]));
/// assert!(!is_sorted(&[1, 3, 2, 4]));
/// assert!(is_sorted(&[] as &[i32]));
/// assert!(is_sorted(&[42]));
/// ```
#[inline]
pub const fn is_sorted<T: Copy + PartialOrd>(slice: &[T]) -> bool {
    if slice.len() <= 1 {
        return true;
    }

    let mut i = 0;
    while i < slice.len() - 1 {
        if slice[i] > slice[i + 1] {
            return false;
        }
        i += 1;
    }
    true
}

/// Checks if a slice is sorted according to the given comparator.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_algorithm::is_sorted_by;
///
/// assert!(is_sorted_by(&[1, 2, 3, 4, 5], |a, b| a < b));
/// assert!(!is_sorted_by(&[5, 4, 3, 2, 1], |a, b| a < b));
/// assert!(is_sorted_by(&[5, 4, 3, 2, 1], |a, b| a > b));
/// ```
#[inline]
pub fn is_sorted_by<T, F>(slice: &[T], mut compare: F) -> bool
where
    F: FnMut(&T, &T) -> bool,
{
    if slice.len() <= 1 {
        return true;
    }

    for i in 0..slice.len() - 1 {
        if !compare(&slice[i], &slice[i + 1]) {
            return false;
        }
    }
    true
}

/// Checks if a slice is sorted in descending order.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_algorithm::is_sorted_descending;
///
/// assert!(is_sorted_descending(&[5, 4, 3, 2, 1]));
/// assert!(!is_sorted_descending(&[1, 2, 3, 4, 5]));
/// ```
#[inline]
pub const fn is_sorted_descending<T: Copy + PartialOrd>(slice: &[T]) -> bool {
    if slice.len() <= 1 {
        return true;
    }

    let mut i = 0;
    while i < slice.len() - 1 {
        if slice[i] < slice[i + 1] {
            return false;
        }
        i += 1;
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_sorted() {
        assert!(is_sorted(&[1, 2, 3, 4, 5]));
        assert!(!is_sorted(&[1, 3, 2, 4]));
        assert!(is_sorted(&[] as &[i32]));
        assert!(is_sorted(&[42]));
    }

    #[test]
    fn test_is_sorted_descending() {
        assert!(is_sorted_descending(&[5, 4, 3, 2, 1]));
        assert!(!is_sorted_descending(&[1, 2, 3, 4, 5]));
    }

    #[test]
    fn test_is_sorted_by() {
        assert!(is_sorted_by(&[1, 2, 3, 4, 5], |a, b| a < b));
        assert!(is_sorted_by(&[5, 4, 3, 2, 1], |a, b| a > b));
    }
}
