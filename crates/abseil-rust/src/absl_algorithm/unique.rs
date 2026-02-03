//! Unique utilities - unique, is_unique

/// Removes consecutive duplicate elements from a slice.
///
/// Returns the new length of the slice (with unique elements at the front).
///
/// # Examples
///
/// ```rust
/// use abseil::absl_algorithm::unique;
///
/// let mut data = [1, 1, 2, 2, 2, 3, 4, 4, 5];
/// let len = unique(&mut data);
/// assert_eq!(len, 5);
/// assert_eq!(&data[..len], [1, 2, 3, 4, 5]);
/// ```
#[inline]
pub fn unique<T: PartialEq>(slice: &mut [T]) -> usize {
    if slice.is_empty() {
        return 0;
    }

    let mut write = 1;
    for read in 1..slice.len() {
        if slice[read] != slice[write - 1] {
            if read != write {
                slice[write] = slice[read];
            }
            write += 1;
        }
    }

    write
}

/// Checks if a slice contains only unique elements (no duplicates).
///
/// # Examples
///
/// ```rust
/// use abseil::absl_algorithm::is_unique;
///
/// assert!(is_unique(&[1, 2, 3, 4, 5]));
/// assert!(!is_unique(&[1, 2, 3, 2, 5]));
/// ```
#[inline]
pub fn is_unique<T: PartialEq + Copy>(slice: &[T]) -> bool {
    for i in 0..slice.len() {
        for j in (i + 1)..slice.len() {
            if slice[i] == slice[j] {
                return false;
            }
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unique() {
        let mut data = [1, 1, 2, 2, 2, 3, 4, 4, 5];
        let len = unique(&mut data);
        assert_eq!(len, 5);
        assert_eq!(&data[..len], [1, 2, 3, 4, 5]);
    }

    #[test]
    fn test_is_unique() {
        assert!(is_unique(&[1, 2, 3, 4, 5]));
        assert!(!is_unique(&[1, 2, 3, 2, 5]));
    }
}
