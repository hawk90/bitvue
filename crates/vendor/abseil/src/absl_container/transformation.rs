//! Container transformation utilities - transform, filter, filter_map, flatten

use alloc::vec::Vec;

/// Creates a new vector with elements transformed by a function.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_container::transform;
///
/// let input = vec![1, 2, 3, 4, 5];
/// let output: Vec<_> = transform(&input, |x| x * 2);
/// assert_eq!(output, vec![2, 4, 6, 8, 10]);
/// ```
#[inline]
pub fn transform<T, U, F>(source: &[T], mut f: F) -> Vec<U>
where
    F: FnMut(&T) -> U,
{
    source.iter().map(&mut f).collect()
}

/// Creates a new vector with elements filtered by a predicate.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_container::filter;
///
/// let input = vec![1, 2, 3, 4, 5, 6];
/// let output: Vec<_> = filter(&input, |&x| x % 2 == 0);
/// assert_eq!(output, vec![&2, &4, &6]);
/// ```
#[inline]
pub fn filter<T, F>(source: &[T], mut predicate: F) -> Vec<&T>
where
    F: FnMut(&T) -> bool,
{
    source.iter().filter(|&x| predicate(x)).collect()
}

/// Creates a new vector with elements filtered and transformed.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_container::filter_map;
///
/// let input = vec!["1", "2", "not_a_number", "3"];
/// let output: Vec<i32> = filter_map(&input, |&s| s.parse().ok());
/// assert_eq!(output, vec![1, 2, 3]);
/// ```
#[inline]
pub fn filter_map<T, U, F>(source: &[T], mut f: F) -> Vec<U>
where
    F: FnMut(&T) -> Option<U>,
{
    source.iter().filter_map(&mut f).collect()
}

/// Creates a new vector by flattening a nested structure.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_container::flatten;
///
/// let input = vec![vec![1, 2], vec![3], vec![4, 5]];
/// let output: Vec<i32> = flatten(&input);
/// assert_eq!(output, vec![1, 2, 3, 4, 5]);
/// ```
#[inline]
pub fn flatten<T, U>(source: &[U]) -> Vec<T>
where
    U: AsRef<[T]>,
{
    source.iter().flat_map(|v| v.as_ref().iter().copied()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transform() {
        let input = vec![1, 2, 3, 4, 5];
        let output: Vec<_> = transform(&input, |x| x * 2);
        assert_eq!(output, vec![2, 4, 6, 8, 10]);
    }

    #[test]
    fn test_filter() {
        let input = vec![1, 2, 3, 4, 5, 6];
        let output: Vec<_> = filter(&input, |&x| x % 2 == 0);
        assert_eq!(output, vec![&2, &4, &6]);
    }

    #[test]
    fn test_filter_map() {
        let input = vec!["1", "2", "not_a_number", "3"];
        let output: Vec<i32> = filter_map(&input, |&s| s.parse().ok());
        assert_eq!(output, vec![1, 2, 3]);
    }

    #[test]
    fn test_flatten() {
        let input = vec![vec![1, 2], vec![3], vec![4, 5]];
        let output: Vec<i32> = flatten(&input);
        assert_eq!(output, vec![1, 2, 3, 4, 5]);
    }
}
