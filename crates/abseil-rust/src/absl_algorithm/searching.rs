//! Searching utilities - search_subsequence, find_last, find_all, count

/// Searches for a subsequence within a slice.
///
/// Returns the starting index of the first occurrence, or None if not found.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_algorithm::search_subsequence;
///
/// assert_eq!(search_subsequence(&[1, 2, 3, 4, 5], &[3, 4]), Some(2));
/// assert_eq!(search_subsequence(&[1, 2, 3, 4, 5], &[6, 7]), None);
/// ```
#[inline]
pub fn search_subsequence<T: PartialEq>(haystack: &[T], needle: &[T]) -> Option<usize> {
    if needle.is_empty() {
        return Some(0);
    }
    if needle.len() > haystack.len() {
        return None;
    }

    for i in 0..=(haystack.len() - needle.len()) {
        if &haystack[i..i + needle.len()] == needle {
            return Some(i);
        }
    }

    None
}

/// Finds the last occurrence of a value in a slice.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_algorithm::find_last;
///
/// let data = [1, 2, 3, 2, 1];
/// assert_eq!(find_last(&data, &2), Some(3));
/// assert_eq!(find_last(&data, &5), None);
/// ```
#[inline]
pub fn find_last<T: PartialEq>(slice: &[T], value: &T) -> Option<usize> {
    slice.iter()
        .rposition(|x| x == value)
}

/// Finds all occurrences of a value in a slice.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_algorithm::find_all;
///
/// let data = [1, 2, 3, 2, 1, 2];
/// assert_eq!(find_all(&data, &2), vec![1, 3, 5]);
/// ```
#[inline]
pub fn find_all<T: PartialEq>(slice: &[T], value: &T) -> Vec<usize> {
    slice.iter()
        .enumerate()
        .filter(|(_, x)| x == value)
        .map(|(i, _)| i)
        .collect()
}

/// Counts occurrences of a value in a slice.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_algorithm::count;
///
/// let data = [1, 2, 3, 2, 1, 2];
/// assert_eq!(count(&data, &2), 3);
/// assert_eq!(count(&data, &5), 0);
/// ```
#[inline]
pub fn count<T: PartialEq>(slice: &[T], value: &T) -> usize {
    slice.iter()
        .filter(|x| x == value)
        .count()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_search_subsequence() {
        assert_eq!(search_subsequence(&[1, 2, 3, 4, 5], &[3, 4]), Some(2));
        assert_eq!(search_subsequence(&[1, 2, 3, 4, 5], &[6, 7]), None);
        assert_eq!(search_subsequence(&[1, 2, 3], &[]), Some(0));
    }

    #[test]
    fn test_find_last() {
        let data = [1, 2, 3, 2, 1];
        assert_eq!(find_last(&data, &2), Some(3));
        assert_eq!(find_last(&data, &5), None);
    }

    #[test]
    fn test_find_all() {
        let data = [1, 2, 3, 2, 1, 2];
        assert_eq!(find_all(&data, &2), vec![1, 3, 5]);
    }

    #[test]
    fn test_count() {
        let data = [1, 2, 3, 2, 1, 2];
        assert_eq!(count(&data, &2), 3);
        assert_eq!(count(&data, &5), 0);
    }
}
