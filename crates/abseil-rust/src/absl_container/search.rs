//! Container search utilities - find_if, find_last_if, find_all, count_if

/// Finds the first element matching a predicate.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_container::find_if;
///
/// let data = vec![1, 2, 3, 4, 5];
/// assert_eq!(find_if(&data, |&x| x > 3), Some(&4));
/// assert_eq!(find_if(&data, |&x| x > 10), None);
/// ```
#[inline]
pub fn find_if<T, F>(source: &[T], mut predicate: F) -> Option<&T>
where
    F: FnMut(&T) -> bool,
{
    source.iter().find(|&x| predicate(x))
}

/// Finds the last element matching a predicate.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_container::find_last_if;
///
/// let data = vec![1, 2, 3, 4, 3, 5];
/// assert_eq!(find_last_if(&data, |&x| x == 3), Some(&3));
/// ```
#[inline]
pub fn find_last_if<T, F>(source: &[T], mut predicate: F) -> Option<&T>
where
    F: FnMut(&T) -> bool,
{
    source.iter().rfind(|&x| predicate(x))
}

/// Finds all elements matching a predicate.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_container::find_all;
///
/// let data = vec![1, 2, 3, 4, 5, 6];
/// let evens: Vec<_> = find_all(&data, |&x| x % 2 == 0);
/// assert_eq!(evens, vec![&2, &4, &6]);
/// ```
#[inline]
pub fn find_all<'a, T, F>(source: &'a [T], mut predicate: F) -> Vec<&'a T>
where
    F: FnMut(&T) -> bool,
{
    source.iter().filter(|&x| predicate(x)).collect()
}

/// Counts elements matching a predicate.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_container::count_if;
///
/// let data = vec![1, 2, 3, 4, 5, 6];
/// assert_eq!(count_if(&data, |&x| x % 2 == 0), 3);
/// ```
#[inline]
pub fn count_if<T, F>(source: &[T], mut predicate: F) -> usize
where
    F: FnMut(&T) -> bool,
{
    source.iter().filter(|&x| predicate(x)).count()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_if() {
        let data = vec![1, 2, 3, 4, 5];
        assert_eq!(find_if(&data, |&x| x > 3), Some(&4));
        assert_eq!(find_if(&data, |&x| x > 10), None);
    }

    #[test]
    fn test_find_last_if() {
        let data = vec![1, 2, 3, 4, 3, 5];
        assert_eq!(find_last_if(&data, |&x| x == 3), Some(&3));
    }

    #[test]
    fn test_find_all() {
        let data = vec![1, 2, 3, 4, 5, 6];
        let evens: Vec<_> = find_all(&data, |&x| x % 2 == 0);
        assert_eq!(evens, vec![&2, &4, &6]);
    }

    #[test]
    fn test_count_if() {
        let data = vec![1, 2, 3, 4, 5, 6];
        assert_eq!(count_if(&data, |&x| x % 2 == 0), 3);
    }
}
