//! Search algorithms.
//!
//! This module provides search utilities similar to Abseil's `absl/algorithm/search.h`,
//! including binary search variants.
//!
//! # Example
//!
//! ```rust
//! use abseil::absl_algorithm::search::binary_search;
//!
//! let vec = vec![1, 3, 5, 7, 9];
//! assert_eq!(binary_search(&vec, &5), Some(2));
//! assert_eq!(binary_search(&vec, &4), None);
//! ```

/// Binary search on a sorted slice.
///
/// Returns `Some(index)` if found, `None` otherwise.
pub fn binary_search<T: Ord>(slice: &[T], value: &T) -> Option<usize> {
    slice.binary_search(value).ok()
}

/// Binary search with a custom key extraction function.
///
/// # Example
///
/// ```
/// use abseil::absl_algorithm::search::binary_search_by;
///
/// let vec = vec![(1, "a"), (2, "b"), (3, "c")];
/// assert_eq!(binary_search_by(&vec, |&(k, _)| k, &2), Some(1));
/// assert_eq!(binary_search_by(&vec, |&(k, _)| k, &4), None);
/// ```
pub fn binary_search_by<T, F, K>(slice: &[T], mut f: F, key: &K) -> Option<usize>
where
    F: FnMut(&T) -> K,
    K: Ord,
{
    slice.binary_search_by(|probe| f(probe).cmp(key)).ok()
}

/// Returns the index of the first element that is `>= value`.
///
/// If all elements are < value, returns `slice.len()`.
pub fn lower_bound<T: Ord>(slice: &[T], value: &T) -> usize {
    slice.partition_point(|x| x < value)
}

/// Returns the index of the first element that is `> value`.
///
/// If all elements are <= value, returns `slice.len()`.
pub fn upper_bound<T: Ord>(slice: &[T], value: &T) -> usize {
    slice.partition_point(|x| x <= value)
}

/// Returns `true` if the slice contains the value.
pub fn contains<T: PartialEq>(slice: &[T], value: &T) -> bool {
    slice.iter().any(|x| x == value)
}

/// Rotates the slice to the left by `mid` elements.
///
/// # Panics
///
/// Panics if `mid > slice.len()`.
///
/// # Example
///
/// ```rust
/// use abseil::absl_algorithm::search::rotate_left;
///
/// let mut vec = vec![1, 2, 3, 4, 5];
/// rotate_left(&mut vec, 2);
/// assert_eq!(vec, [3, 4, 5, 1, 2]);
/// ```
pub fn rotate_left<T>(slice: &mut [T], mid: usize) {
    slice.rotate_left(mid);
}

/// Rotates the slice to the right by `mid` elements.
///
/// # Panics
///
/// Panics if `mid > slice.len()`.
///
/// # Example
///
/// ```rust
/// use abseil::absl_algorithm::search::rotate_right;
///
/// let mut vec = vec![1, 2, 3, 4, 5];
/// rotate_right(&mut vec, 2);
/// assert_eq!(vec, [4, 5, 1, 2, 3]);
/// ```
pub fn rotate_right<T>(slice: &mut [T], mid: usize) {
    slice.rotate_right(mid);
}

/// Reverses the elements in place.
pub fn reverse<T>(slice: &mut [T]) {
    slice.reverse()
}

/// Checks if a slice is sorted in ascending order.
pub fn is_sorted<T: Ord>(slice: &[T]) -> bool {
    slice.windows(2).all(|w| w[0] <= w[1])
}

/// Finds the minimum element in a slice.
///
/// Returns `None` if the slice is empty.
pub fn min<T: Ord>(slice: &[T]) -> Option<&T> {
    slice.iter().min()
}

/// Finds the maximum element in a slice.
///
/// Returns `None` if the slice is empty.
pub fn max<T: Ord>(slice: &[T]) -> Option<&T> {
    slice.iter().max()
}

/// Finds the minimum and maximum elements in a slice.
///
/// Returns `None` if the slice is empty.
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

/// Returns the index of the first element matching the predicate.
pub fn find<T, F>(slice: &[T], predicate: F) -> Option<usize>
where
    F: FnMut(&T) -> bool,
{
    slice.iter().position(predicate)
}

/// Returns the index of the last element matching the predicate.
pub fn rfind<T, F>(slice: &[T], predicate: F) -> Option<usize>
where
    F: FnMut(&T) -> bool,
{
    slice.iter().rposition(predicate)
}

/// Returns the number of elements matching the predicate.
pub fn count_if<T, F>(slice: &[T], mut predicate: F) -> usize
where
    F: FnMut(&T) -> bool,
{
    slice.iter().filter(|x| predicate(x)).count()
}

/// Checks if all elements match the predicate.
pub fn all<T, F>(slice: &[T], predicate: F) -> bool
where
    F: FnMut(&T) -> bool,
{
    slice.iter().all(predicate)
}

/// Checks if any element matches the predicate.
pub fn any<T, F>(slice: &[T], predicate: F) -> bool
where
    F: FnMut(&T) -> bool,
{
    slice.iter().any(predicate)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_binary_search() {
        let vec = vec![1, 3, 5, 7, 9];
        assert_eq!(binary_search(&vec, &5), Some(2));
        assert_eq!(binary_search(&vec, &4), None);
        assert_eq!(binary_search(&vec, &1), Some(0));
        assert_eq!(binary_search(&vec, &9), Some(4));
    }

    #[test]
    fn test_binary_search_by() {
        let vec = vec![(1, "a"), (2, "b"), (3, "c")];
        assert_eq!(binary_search_by(&vec, |&(k, _)| k, &2), Some(1));
        assert_eq!(binary_search_by(&vec, |&(k, _)| k, &4), None);
    }

    #[test]
    fn test_lower_bound() {
        let vec = vec![1, 3, 3, 5, 7];
        assert_eq!(lower_bound(&vec, &3), 1);
        assert_eq!(lower_bound(&vec, &4), 3);
        assert_eq!(lower_bound(&vec, &0), 0);
        assert_eq!(lower_bound(&vec, &8), 5);
    }

    #[test]
    fn test_upper_bound() {
        let vec = vec![1, 3, 3, 5, 7];
        assert_eq!(upper_bound(&vec, &3), 3);
        assert_eq!(upper_bound(&vec, &2), 1);
        assert_eq!(upper_bound(&vec, &0), 0);
        assert_eq!(upper_bound(&vec, &8), 5);
    }

    #[test]
    fn test_contains() {
        let vec = vec![1, 2, 3, 4, 5];
        assert!(contains(&vec, &3));
        assert!(!contains(&vec, &6));
    }

    #[test]
    fn test_rotate_left() {
        let mut vec = vec![1, 2, 3, 4, 5];
        rotate_left(&mut vec, 2);
        assert_eq!(vec, [3, 4, 5, 1, 2]);
    }

    #[test]
    fn test_rotate_right() {
        let mut vec = vec![1, 2, 3, 4, 5];
        rotate_right(&mut vec, 2);
        assert_eq!(vec, [4, 5, 1, 2, 3]);
    }

    #[test]
    fn test_reverse() {
        let mut vec = vec![1, 2, 3, 4, 5];
        reverse(&mut vec);
        assert_eq!(vec, [5, 4, 3, 2, 1]);
    }

    #[test]
    fn test_is_sorted() {
        assert!(is_sorted(&[1, 2, 3, 4, 5]));
        assert!(!is_sorted(&[1, 3, 2, 4]));
        assert!(is_sorted::<i32>(&[]));
        assert!(is_sorted(&[42]));
    }

    #[test]
    fn test_min_max() {
        assert_eq!(min(&[3, 1, 4, 1, 5]), Some(&1));
        assert_eq!(max(&[3, 1, 4, 1, 5]), Some(&5));
        assert_eq!(min::<i32>(&[]), None);
        assert_eq!(max::<i32>(&[]), None);
    }

    #[test]
    fn test_min_max_pair() {
        assert_eq!(min_max(&[3, 1, 4, 1, 5]), Some((&1, &5)));
        assert_eq!(min_max(&[5]), Some((&5, &5)));
        assert_eq!(min_max::<i32>(&[]), None);
    }

    #[test]
    fn test_find() {
        let vec = vec![1, 2, 3, 4, 5];
        assert_eq!(find(&vec, |&x| x == 3), Some(2));
        assert_eq!(find(&vec, |&x| x == 6), None);
    }

    #[test]
    fn test_rfind() {
        let vec = vec![1, 2, 3, 2, 1];
        assert_eq!(rfind(&vec, |&x| x == 2), Some(3));
        assert_eq!(rfind(&vec, |&x| x == 5), None);
    }

    #[test]
    fn test_count_if() {
        let vec = vec![1, 2, 3, 2, 1];
        assert_eq!(count_if(&vec, |&x| x == 2), 2);
        assert_eq!(count_if(&vec, |&x| x == 5), 0);
    }

    #[test]
    fn test_all() {
        assert!(all(&[2, 4, 6, 8], |&x| x % 2 == 0));
        assert!(!all(&[2, 4, 5, 8], |&x| x % 2 == 0));
        assert!(all::<i32, _>(&[], |_| true));
    }

    #[test]
    fn test_any() {
        assert!(any(&[1, 2, 3, 4], |&x| x % 2 == 0));
        assert!(!any(&[1, 3, 5, 7], |&x| x % 2 == 0));
        assert!(!any::<i32, _>(&[], |_| false));
    }
}
