//! Linear search utilities - linear_search, linear_search_by

/// Performs a linear search for a value in a slice.
///
/// Returns the index of the first occurrence, or `None` if not found.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_algorithm::linear_search;
///
/// assert_eq!(linear_search(&[1, 2, 3, 4, 5], &3), Some(2));
/// assert_eq!(linear_search(&[1, 2, 3, 4, 5], &6), None);
/// ```
#[inline]
pub const fn linear_search<T: Copy + PartialEq>(slice: &[T], value: &T) -> Option<usize> {
    let mut i = 0;
    while i < slice.len() {
        if slice[i] == *value {
            return Some(i);
        }
        i += 1;
    }
    None
}

/// Performs a linear search with a custom predicate.
///
/// Returns the index of the first element where the predicate returns true,
/// or `None` if not found.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_algorithm::linear_search_by;
///
/// let data = [(1, "a"), (2, "b"), (3, "c")];
/// assert_eq!(linear_search_by(&data, |x| x.0 == 2), Some(1));
/// assert_eq!(linear_search_by(&data, |x| x.0 == 5), None);
/// ```
#[inline]
pub fn linear_search_by<T, F>(slice: &[T], mut predicate: F) -> Option<usize>
where
    F: FnMut(&T) -> bool,
{
    for (i, item) in slice.iter().enumerate() {
        if predicate(item) {
            return Some(i);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_linear_search() {
        assert_eq!(linear_search(&[1, 2, 3, 4, 5], &3), Some(2));
        assert_eq!(linear_search(&[1, 2, 3, 4, 5], &6), None);
        assert_eq!(linear_search(&[1, 2, 2, 3], &2), Some(1));
    }

    #[test]
    fn test_linear_search_by() {
        let data = [(1, "a"), (2, "b"), (3, "c")];
        assert_eq!(linear_search_by(&data, |x| x.0 == 2), Some(1));
        assert_eq!(linear_search_by(&data, |x| x.0 == 5), None);
    }
}
