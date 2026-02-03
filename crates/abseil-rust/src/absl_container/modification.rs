//! Container modification utilities - append, clear, reserve, shrink_to_fit

use alloc::vec::Vec;

/// Appends all elements from one container to another.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_container::append;
///
/// let mut a = vec![1, 2, 3];
/// let b = vec![4, 5, 6];
/// append(&mut a, &b);
/// assert_eq!(a, vec![1, 2, 3, 4, 5, 6]);
/// ```
#[inline]
pub fn append<T: Clone>(dest: &mut Vec<T>, source: &[T]) {
    dest.extend_from_slice(source);
}

/// Removes all elements from a container.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_container::clear;
///
/// let mut data = vec![1, 2, 3, 4, 5];
/// clear(&mut data);
/// assert!(data.is_empty());
/// ```
#[inline]
pub fn clear<T>(container: &mut Vec<T>) {
    container.clear();
}

/// Reserves capacity for at least `additional` more elements.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_container::reserve;
///
/// let mut data = vec![1, 2, 3];
/// reserve(&mut data, 10);
/// assert!(data.capacity() >= 13);
/// ```
#[inline]
pub fn reserve<T>(container: &mut Vec<T>, additional: usize) {
    container.reserve(additional);
}

/// Shrinks the capacity of a container to fit its size.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_container::shrink_to_fit;
///
/// let mut data = Vec::with_capacity(100);
/// data.push(1);
/// data.push(2);
/// shrink_to_fit(&mut data);
/// assert_eq!(data.capacity(), 2);
/// ```
#[inline]
pub fn shrink_to_fit<T>(container: &mut Vec<T>) {
    container.shrink_to_fit();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_append() {
        let mut a = vec![1, 2, 3];
        let b = vec![4, 5, 6];
        append(&mut a, &b);
        assert_eq!(a, vec![1, 2, 3, 4, 5, 6]);
    }

    #[test]
    fn test_clear_container() {
        let mut data = vec![1, 2, 3, 4, 5];
        clear(&mut data);
        assert!(data.is_empty());
    }

    #[test]
    fn test_reserve() {
        let mut data = vec![1, 2, 3];
        reserve(&mut data, 10);
        assert!(data.capacity() >= 13);
    }

    #[test]
    fn test_shrink_to_fit() {
        let mut data = Vec::with_capacity(100);
        data.push(1);
        data.push(2);
        shrink_to_fit(&mut data);
        assert_eq!(data.capacity(), 2);
    }
}
