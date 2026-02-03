//! Heap utilities - is_heap, is_heap_by

/// Checks if a slice satisfies the heap property (max-heap).
///
/// # Examples
///
/// ```rust
/// use abseil::absl_algorithm::is_heap;
///
/// assert!(is_heap(&[9, 5, 8, 3, 2, 7, 1]));
/// assert!(!is_heap(&[1, 2, 3, 4, 5]));
/// ```
#[inline]
pub fn is_heap<T: PartialOrd>(slice: &[T]) -> bool {
    is_heap_by(slice, |a, b| a >= b)
}

/// Checks if a slice satisfies the heap property with a custom comparator.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_algorithm::is_heap_by;
///
/// // Max-heap
/// assert!(is_heap_by(&[9, 5, 8, 3, 2, 7, 1], |a, b| a >= b));
/// // Min-heap
/// assert!(is_heap_by(&[1, 2, 3, 5, 9, 8, 7], |a, b| a <= b));
/// ```
#[inline]
pub fn is_heap_by<T, F>(slice: &[T], mut compare: F) -> bool
where
    F: FnMut(&T, &T) -> bool,
{
    for i in 0..slice.len() {
        let left = 2 * i + 1;
        let right = 2 * i + 2;

        if left < slice.len() && !compare(&slice[i], &slice[left]) {
            return false;
        }
        if right < slice.len() && !compare(&slice[i], &slice[right]) {
            return false;
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_heap() {
        assert!(is_heap(&[9, 5, 8, 3, 2, 7, 1]));
        assert!(!is_heap(&[1, 2, 3, 4, 5]));
    }

    #[test]
    fn test_is_heap_by() {
        assert!(is_heap_by(&[9, 5, 8, 3, 2, 7, 1], |a, b| a >= b));
        assert!(is_heap_by(&[1, 2, 3, 5, 9, 8, 7], |a, b| a <= b));
    }
}
