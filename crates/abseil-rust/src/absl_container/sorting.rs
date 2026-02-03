//! Container sorting utilities - is_sorted, sort, sort_by, reverse

/// Checks if a container is sorted.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_container::is_sorted;
///
/// assert!(is_sorted(&[1, 2, 3, 4, 5]));
/// assert!(!is_sorted(&[1, 3, 2, 4]));
/// ```
#[inline]
pub fn is_sorted<T: PartialOrd>(source: &[T]) -> bool {
    source.windows(2).all(|w| w[0] <= w[1])
}

/// Sorts a container in ascending order.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_container::sort;
///
/// let mut data = vec![3, 1, 4, 1, 5, 9, 2, 6];
/// sort(&mut data);
/// assert_eq!(data, vec![1, 1, 2, 3, 4, 5, 6, 9]);
/// ```
#[inline]
pub fn sort<T: PartialOrd>(container: &mut [T]) {
    container.sort();
}

/// Sorts a container using a custom comparison function.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_container::sort_by;
///
/// let mut data = vec![3, 1, 4, 1, 5, 9, 2, 6];
/// sort_by(&mut data, |a, b| b.cmp(a)); // Descending order
/// assert_eq!(data, vec![9, 6, 5, 4, 3, 1, 1, 2]);
/// ```
#[inline]
pub fn sort_by<T, F: FnMut(&T, &T) -> core::cmp::Ordering>(container: &mut [T], mut compare: F) {
    container.sort_by(compare);
}

/// Reverses a container in place.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_container::reverse;
///
/// let mut data = vec![1, 2, 3, 4, 5];
/// reverse(&mut data);
/// assert_eq!(data, vec![5, 4, 3, 2, 1]);
/// ```
#[inline]
pub fn reverse<T>(container: &mut [T]) {
    container.reverse();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_sorted() {
        assert!(is_sorted(&[1, 2, 3, 4, 5]));
        assert!(!is_sorted(&[1, 3, 2, 4]));
    }

    #[test]
    fn test_sort() {
        let mut data = vec![3, 1, 4, 1, 5, 9, 2, 6];
        sort(&mut data);
        assert_eq!(data, vec![1, 1, 2, 3, 4, 5, 6, 9]);
    }

    #[test]
    fn test_sort_by() {
        let mut data = vec![3, 1, 4, 1, 5, 9, 2, 6];
        sort_by(&mut data, |a, b| b.cmp(a));
        assert_eq!(data, vec![9, 6, 5, 4, 3, 1, 1, 2]);
    }

    #[test]
    fn test_reverse_container() {
        let mut data = vec![1, 2, 3, 4, 5];
        reverse(&mut data);
        assert_eq!(data, vec![5, 4, 3, 2, 1]);
    }
}
