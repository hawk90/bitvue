//! Internal utilities shared across algorithm modules.
//!
//! This module contains shared implementations to avoid code duplication
//! between different algorithm modules.

/// Finds the minimum and maximum elements in a slice.
///
/// Returns `None` if the slice is empty.
///
/// This is a shared implementation used by both search and sorting modules.
#[inline]
pub fn min_max<T: Ord>(slice: &[T]) -> Option<(&T, &T)> {
    if slice.is_empty() {
        return None;
    }
    let mut min = &slice[0];
    let mut max = &slice[0];
    for item in &slice[1..] {
        if item < min {
            min = item;
        }
        if item > max {
            max = item;
        }
    }
    Some((min, max))
}

/// Checks if a slice is sorted in ascending order.
///
/// This is a shared implementation used by both search and sorting modules.
#[inline]
pub fn is_sorted<T: Ord>(slice: &[T]) -> bool {
    slice.windows(2).all(|w| w[0] <= w[1])
}

/// Checks if a slice is sorted with a custom comparison function.
///
/// This is a shared implementation used by both search and sorting modules.
#[inline]
pub fn is_sorted_by<T, F>(slice: &[T], mut compare: F) -> bool
where
    F: FnMut(&T, &T) -> core::cmp::Ordering,
{
    slice
        .windows(2)
        .all(|w| compare(&w[0], &w[1]) != core::cmp::Ordering::Greater)
}

/// Finds the minimum element in a slice.
///
/// Returns `None` if the slice is empty.
///
/// This is a shared implementation used by both search and sorting modules.
#[inline]
pub fn min<T: Ord>(slice: &[T]) -> Option<&T> {
    slice.iter().min()
}

/// Finds the maximum element in a slice.
///
/// Returns `None` if the slice is empty.
///
/// This is a shared implementation used by both search and sorting modules.
#[inline]
pub fn max<T: Ord>(slice: &[T]) -> Option<&T> {
    slice.iter().max()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_min_max() {
        assert_eq!(min_max(&[3, 1, 4, 1, 5]), Some((&1, &5)));
        assert_eq!(min_max(&[5]), Some((&5, &5)));
        assert_eq!(min_max::<i32>(&[]), None);
    }

    #[test]
    fn test_is_sorted() {
        assert!(is_sorted(&[1, 2, 3, 4, 5]));
        assert!(!is_sorted(&[1, 3, 2, 4]));
        assert!(is_sorted::<i32>(&[]));
        assert!(is_sorted(&[42]));
    }

    #[test]
    fn test_min_max_individual() {
        assert_eq!(min(&[3, 1, 4, 1, 5]), Some(&1));
        assert_eq!(max(&[3, 1, 4, 1, 5]), Some(&5));
        assert_eq!(min::<i32>(&[]), None);
        assert_eq!(max::<i32>(&[]), None);
    }
}
