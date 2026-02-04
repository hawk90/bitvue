//! Adjacent find utilities - adjacent_find, adjacent_find_by

/// Finds the first two adjacent elements that are equal.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_algorithm::adjacent_find;
///
/// let data = [1, 2, 2, 3, 4];
/// assert_eq!(adjacent_find(&data), Some(1));
/// ```
#[inline]
pub fn adjacent_find<T: PartialEq>(slice: &[T]) -> Option<usize> {
    for i in 0..slice.len().saturating_sub(1) {
        if slice[i] == slice[i + 1] {
            return Some(i);
        }
    }
    None
}

/// Finds the first two adjacent elements that satisfy a predicate.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_algorithm::adjacent_find_by;
///
/// let data = [1, 2, 3, 4, 5];
/// assert_eq!(adjacent_find_by(&data, |a, b| *a + 1 == *b), Some(0));
/// ```
#[inline]
pub fn adjacent_find_by<T, F>(slice: &[T], mut f: F) -> Option<usize>
where
    F: FnMut(&T, &T) -> bool,
{
    for i in 0..slice.len().saturating_sub(1) {
        if f(&slice[i], &slice[i + 1]) {
            return Some(i);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_adjacent_find() {
        let data = [1, 2, 2, 3, 4];
        assert_eq!(adjacent_find(&data), Some(1));
    }

    #[test]
    fn test_adjacent_find_by() {
        let data = [1, 2, 3, 4, 5];
        assert_eq!(adjacent_find_by(&data, |a, b| *a + 1 == *b), Some(0));
    }
}
