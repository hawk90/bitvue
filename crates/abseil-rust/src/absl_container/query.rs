//! Container query utilities - contains, size, is_empty, front, back

/// Checks if a container contains a specific value.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_container::contains;
///
/// assert!(contains(&[1, 2, 3, 4, 5], &3));
/// assert!(!contains(&[1, 2, 3, 4, 5], &6));
/// assert!(contains(&vec![1, 2, 3], &2));
/// ```
#[inline]
pub fn contains<T: PartialEq>(container: impl IntoIterator<Item = T>, value: &T) -> bool {
    container.into_iter().any(|item| &item == value)
}

/// Returns the number of elements in a container.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_container::size;
///
/// assert_eq!(size(&[1, 2, 3, 4, 5]), 5);
/// assert_eq!(size(&[] as &[i32]), 0);
/// ```
#[inline]
pub fn size<T>(container: &[T]) -> usize {
    container.len()
}

/// Checks if a container is empty.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_container::is_empty;
///
/// assert!(!is_empty(&[1, 2, 3]));
/// assert!(is_empty(&[] as &[i32]));
/// ```
#[inline]
pub fn is_empty<T>(container: &[T]) -> bool {
    container.is_empty()
}

/// Returns the first element of a container, or `None` if empty.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_container::front;
///
/// assert_eq!(front(&[1, 2, 3, 4, 5]), Some(&1));
/// assert_eq!(front(&[] as &[i32]), None);
/// ```
#[inline]
pub fn front<T>(container: &[T]) -> Option<&T> {
    container.first()
}

/// Returns the last element of a container, or `None` if empty.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_container::back;
///
/// assert_eq!(back(&[1, 2, 3, 4, 5]), Some(&5));
/// assert_eq!(back(&[] as &[i32]), None);
/// ```
#[inline]
pub fn back<T>(container: &[T]) -> Option<&T> {
    container.last()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_contains() {
        assert!(contains(&[1, 2, 3, 4, 5], &3));
        assert!(!contains(&[1, 2, 3, 4, 5], &6));
        assert!(contains(&vec![1, 2, 3], &2));
    }

    #[test]
    fn test_size() {
        assert_eq!(size(&[1, 2, 3, 4, 5]), 5);
        assert_eq!(size(&[] as &[i32]), 0);
    }

    #[test]
    fn test_is_empty() {
        assert!(!is_empty(&[1, 2, 3]));
        assert!(is_empty(&[] as &[i32]));
    }

    #[test]
    fn test_front_back() {
        assert_eq!(front(&[1, 2, 3, 4, 5]), Some(&1));
        assert_eq!(back(&[1, 2, 3, 4, 5]), Some(&5));
        assert_eq!(front(&[] as &[i32]), None);
        assert_eq!(back(&[] as &[i32]), None);
    }
}
