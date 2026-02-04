//! Swap utilities - swap_elements, reverse_range

/// Swaps two elements in a slice.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_algorithm::swap_elements;
///
/// let mut data = [1, 2, 3, 4, 5];
/// swap_elements(&mut data, 1, 3);
/// assert_eq!(data, [1, 4, 3, 2, 5]);
/// ```
#[inline]
pub fn swap_elements<T>(slice: &mut [T], a: usize, b: usize) {
    slice.swap(a, b);
}

/// Reverses a range of elements in a slice.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_algorithm::reverse_range;
///
/// let mut data = [1, 2, 3, 4, 5];
/// reverse_range(&mut data, 1..4);
/// assert_eq!(data, [1, 4, 3, 2, 5]);
/// ```
#[inline]
pub fn reverse_range<T>(slice: &mut [T], range: core::ops::Range<usize>) {
    if range.start >= range.end || range.end > slice.len() {
        return;
    }
    slice[range].reverse();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_swap_elements() {
        let mut data = [1, 2, 3, 4, 5];
        swap_elements(&mut data, 1, 3);
        assert_eq!(data, [1, 4, 3, 2, 5]);
    }

    #[test]
    fn test_reverse_range() {
        let mut data = [1, 2, 3, 4, 5];
        reverse_range(&mut data, 1..4);
        assert_eq!(data, [1, 4, 3, 2, 5]);
    }
}
