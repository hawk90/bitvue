//! Rotation and reversal utilities - rotate_left, rotate_right, reverse

/// Rotates a slice to the left by `mid` positions.
///
/// The element at index `mid` becomes the first element.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_algorithm::rotate_left;
///
/// let mut data = [1, 2, 3, 4, 5];
/// rotate_left(&mut data, 2);
/// assert_eq!(data, [3, 4, 5, 1, 2]);
/// ```
#[inline]
pub fn rotate_left<T>(slice: &mut [T], mid: usize) {
    let n = slice.len();
    if n == 0 || mid % n == 0 {
        return;
    }
    let k = mid % n;
    slice.reverse();
    slice[0..n - k].reverse();
    slice[n - k..].reverse();
}

/// Rotates a slice to the right by `mid` positions.
///
/// The element at index `slice.len() - mid` becomes the first element.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_algorithm::rotate_right;
///
/// let mut data = [1, 2, 3, 4, 5];
/// rotate_right(&mut data, 2);
/// assert_eq!(data, [4, 5, 1, 2, 3]);
/// ```
#[inline]
pub fn rotate_right<T>(slice: &mut [T], mid: usize) {
    let n = slice.len();
    if n == 0 || mid % n == 0 {
        return;
    }
    let k = mid % n;
    rotate_left(slice, n - k);
}

/// Reverses a slice in place.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_algorithm::reverse;
///
/// let mut data = [1, 2, 3, 4, 5];
/// reverse(&mut data);
/// assert_eq!(data, [5, 4, 3, 2, 1]);
/// ```
#[inline]
pub fn reverse<T>(slice: &mut [T]) {
    slice.reverse();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rotate_left() {
        let mut data = [1, 2, 3, 4, 5];
        rotate_left(&mut data, 2);
        assert_eq!(data, [3, 4, 5, 1, 2]);

        let mut data = [1, 2, 3, 4, 5];
        rotate_left(&mut data, 0);
        assert_eq!(data, [1, 2, 3, 4, 5]);

        let mut data = [1, 2, 3, 4, 5];
        rotate_left(&mut data, 5);
        assert_eq!(data, [1, 2, 3, 4, 5]);
    }

    #[test]
    fn test_rotate_right() {
        let mut data = [1, 2, 3, 4, 5];
        rotate_right(&mut data, 2);
        assert_eq!(data, [4, 5, 1, 2, 3]);
    }

    #[test]
    fn test_reverse() {
        let mut data = [1, 2, 3, 4, 5];
        reverse(&mut data);
        assert_eq!(data, [5, 4, 3, 2, 1]);
    }
}
