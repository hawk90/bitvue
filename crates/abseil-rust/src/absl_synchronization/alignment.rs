//! Alignment utilities for memory alignment operations.

/// Checks if a value would align with the given alignment.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_synchronization::is_aligned;
///
/// assert!(is_aligned(0x1000usize, 0x1000));
/// assert!(is_aligned(0x1001usize, 0x1000));
/// assert!(!is_aligned(0x1001usize, 0x100));
/// ```
#[inline]
pub const fn is_aligned(value: usize, alignment: usize) -> bool {
    value & (alignment - 1) == 0
}

/// Rounds a value up to the nearest multiple of alignment.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_synchronization::align_up;
///
/// assert_eq!(align_up(0, 4), 0);
/// assert_eq!(align_up(1, 4), 4);
/// assert_eq!(align_up(4, 4), 4);
/// assert_eq!(align_up(5, 4), 8);
/// ```
#[inline]
pub const fn align_up(value: usize, alignment: usize) -> usize {
    (value + alignment - 1) & !(alignment - 1)
}

/// Rounds a value down to the nearest multiple of alignment.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_synchronization::align_down;
///
/// assert_eq!(align_down(0, 4), 0);
/// assert_eq!(align_down(1, 4), 0);
/// assert_eq!(align_down(4, 4), 4);
/// assert_eq!(align_down(5, 4), 4);
/// assert_eq!(align_down(8, 4), 8);
/// ```
#[inline]
pub const fn align_down(value: usize, alignment: usize) -> usize {
    value & !(alignment - 1)
}

/// Computes the difference to align a value up to the given alignment.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_synchronization::align_up_diff;
///
/// assert_eq!(align_up_diff(0, 4), 0);
/// assert_eq!(align_up_diff(1, 4), 3);
/// assert_eq!(align_up_diff(4, 4), 0);
/// assert_eq!(align_up_diff(8, 4), 0);
/// ```
#[inline]
pub const fn align_up_diff(value: usize, alignment: usize) -> usize {
    align_up(value, alignment) - value
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_aligned() {
        assert!(is_aligned(0x1000usize, 0x1000));
        assert!(is_aligned(0x1001usize, 0x1000));
        assert!(!is_aligned(0x1001usize, 0x100));
    }

    #[test]
    fn test_align_up() {
        assert_eq!(align_up(0, 4), 0);
        assert_eq!(align_up(1, 4), 4);
        assert_eq!(align_up(4, 4), 4);
        assert_eq!(align_up(5, 4), 8);
        assert_eq!(align_up(8, 4), 8);
        assert_eq!(align_up(15, 8), 16);
    }

    #[test]
    fn test_align_down() {
        assert_eq!(align_down(0, 4), 0);
        assert_eq!(align_down(1, 4), 0);
        assert_eq!(align_down(4, 4), 4);
        assert_eq!(align_down(5, 4), 4);
        assert_eq!(align_down(8, 4), 8);
        assert_eq!(align_down(15, 8), 8);
    }

    #[test]
    fn test_align_up_diff() {
        assert_eq!(align_up_diff(0, 4), 0);
        assert_eq!(align_up_diff(1, 4), 3);
        assert_eq!(align_up_diff(4, 4), 0);
        assert_eq!(align_up_diff(8, 4), 0);
    }
}
