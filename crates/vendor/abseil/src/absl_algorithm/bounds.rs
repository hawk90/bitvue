//! Binary search bounds - lower_bound, upper_bound, equal_range

/// Returns a range of indices equivalent to the given value in a sorted slice.
///
/// Returns (lower_bound, upper_bound) as a range.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_algorithm::equal_range;
///
/// let data = [1, 2, 2, 2, 3, 4];
/// let range = equal_range(&data, &2);
/// assert_eq!(range, 1..4);
/// ```
#[inline]
pub fn equal_range<T: Ord>(slice: &[T], value: &T) -> core::ops::Range<usize> {
    let lower = lower_bound(slice, value);
    let upper = upper_bound(slice, value);
    lower..upper
}

/// Finds the first position where a value could be inserted.
///
/// Returns the index of the first element >= value.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_algorithm::lower_bound;
///
/// let data = [1, 2, 2, 2, 3, 4];
/// assert_eq!(lower_bound(&data, &2), 1);
/// assert_eq!(lower_bound(&data, &3), 4);
/// assert_eq!(lower_bound(&data, &0), 0);
/// ```
#[inline]
pub fn lower_bound<T: Ord>(slice: &[T], value: &T) -> usize {
    let mut left = 0;
    let mut right = slice.len();

    while left < right {
        let mid = left + (right - left) / 2;
        if &slice[mid] < value {
            left = mid + 1;
        } else {
            right = mid;
        }
    }

    left
}

/// Finds the last position where a value could be inserted.
///
/// Returns the index of the first element > value.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_algorithm::upper_bound;
///
/// let data = [1, 2, 2, 2, 3, 4];
/// assert_eq!(upper_bound(&data, &2), 4);
/// assert_eq!(upper_bound(&data, &3), 5);
/// assert_eq!(upper_bound(&data, &5), 6);
/// ```
#[inline]
pub fn upper_bound<T: Ord>(slice: &[T], value: &T) -> usize {
    let mut left = 0;
    let mut right = slice.len();

    while left < right {
        let mid = left + (right - left) / 2;
        if value < &slice[mid] {
            right = mid;
        } else {
            left = mid + 1;
        }
    }

    left
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_equal_range() {
        let data = [1, 2, 2, 2, 3, 4];
        let range = equal_range(&data, &2);
        assert_eq!(range, 1..4);

        let range = equal_range(&data, &5);
        assert_eq!(range, 6..6);
    }

    #[test]
    fn test_lower_bound() {
        let data = [1, 2, 2, 2, 3, 4];
        assert_eq!(lower_bound(&data, &2), 1);
        assert_eq!(lower_bound(&data, &3), 4);
        assert_eq!(lower_bound(&data, &0), 0);
    }

    #[test]
    fn test_upper_bound() {
        let data = [1, 2, 2, 2, 3, 4];
        assert_eq!(upper_bound(&data, &2), 4);
        assert_eq!(upper_bound(&data, &3), 5);
        assert_eq!(upper_bound(&data, &5), 6);
    }
}
