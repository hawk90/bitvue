//! Alignment and size utilities.

use core::ops::RangeInclusive;

/// Checks if a type is aligned to a given boundary.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_utility::alignment::is_aligned;
///
/// let ptr = 0x1000 as *const i32;
/// assert!(is_aligned(ptr as usize, 4));
/// ```
#[inline]
pub const fn is_aligned(addr: usize, alignment: usize) -> bool {
    addr % alignment == 0
}

/// Aligns an address up to the given alignment boundary.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_utility::alignment::align_up;
///
/// assert_eq!(align_up(0x1003, 4), 0x1004);
/// assert_eq!(align_up(0x1004, 4), 0x1004);
/// ```
///
/// # Panics
///
/// Panics if alignment is not a power of two.
#[inline]
pub const fn align_up(addr: usize, alignment: usize) -> usize {
    // SAFETY: Validate alignment is power of two to prevent underflow in (alignment - 1)
    if !alignment.is_power_of_two() {
        panic!("align_up: alignment must be a power of two, got {}", alignment);
    }
    // SAFETY: Use checked arithmetic to prevent overflow when addr is near usize::MAX
    // If adding (alignment - 1) would overflow, we're already at or past the alignment boundary
    match addr.checked_add(alignment - 1) {
        Some(adjusted) => adjusted & !(alignment - 1),
        None => {
            // Overflow occurred - addr must be near usize::MAX
            // Since alignment is a power of two, the aligned value would be 0 or alignment
            addr & !(alignment - 1)
        }
    }
}

/// Aligns an address down to the given alignment boundary.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_utility::alignment::align_down;
///
/// assert_eq!(align_down(0x1003, 4), 0x1000);
/// assert_eq!(align_down(0x1004, 4), 0x1004);
/// ```
#[inline]
pub const fn align_down(addr: usize, alignment: usize) -> usize {
    addr & !(alignment - 1)
}

/// Checks if a pointer is aligned to the specified alignment.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_utility::alignment::is_ptr_aligned;
///
/// let value = 42i32;
/// assert!(is_ptr_aligned(&value, 4));
/// ```
#[inline]
pub fn is_ptr_aligned<T>(ptr: &T, alignment: usize) -> bool {
    (ptr as *const T as usize) % alignment == 0
}

/// Checks if the alignment is a power of two.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_utility::alignment::is_valid_alignment;
///
/// assert!(is_valid_alignment(1));
/// assert!(is_valid_alignment(2));
/// assert!(is_valid_alignment(4));
/// assert!(!is_valid_alignment(3));
/// ```
#[inline]
pub const fn is_valid_alignment(alignment: usize) -> bool {
    alignment.is_power_of_two()
}

/// Returns the next power of two greater than or equal to the value.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_utility::alignment::next_power_of_two;
///
/// assert_eq!(next_power_of_two(0), 1);
/// assert_eq!(next_power_of_two(1), 1);
/// assert_eq!(next_power_of_two(5), 8);
/// assert_eq!(next_power_of_two(16), 16);
/// ```
#[inline]
pub const fn next_power_of_two(n: usize) -> usize {
    if n == 0 {
        return 1;
    }
    let n = n - 1;
    let n = n | (n >> 1);
    let n = n | (n >> 2);
    let n = n | (n >> 4);
    let n = n | (n >> 8);
    let n = n | (n >> 16);
    #[cfg(target_pointer_width = "64")]
    let n = n | (n >> 32);
    n + 1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_align() {
        assert_eq!(align_up(0x1003, 4), 0x1004);
        assert_eq!(align_up(0x1004, 4), 0x1004);
        assert_eq!(align_down(0x1003, 4), 0x1000);
        assert_eq!(align_down(0x1004, 4), 0x1004);
    }

    #[test]
    fn test_is_aligned() {
        assert!(is_aligned(0x1000, 4));
        assert!(!is_aligned(0x1001, 4));
    }

    #[test]
    fn test_is_valid_alignment() {
        assert!(is_valid_alignment(1));
        assert!(is_valid_alignment(2));
        assert!(is_valid_alignment(4));
        assert!(!is_valid_alignment(3));
        assert!(!is_valid_alignment(0));
    }

    #[test]
    fn test_next_power_of_two() {
        assert_eq!(next_power_of_two(0), 1);
        assert_eq!(next_power_of_two(1), 1);
        assert_eq!(next_power_of_two(5), 8);
        assert_eq!(next_power_of_two(16), 16);
        assert_eq!(next_power_of_two(17), 32);
    }
}
