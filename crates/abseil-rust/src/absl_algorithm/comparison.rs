//! Comparison utilities - clamp, min_element, max_element, minmax_element

/// Clamps a value between a minimum and maximum.
///
/// This is a convenience function for consistency with the numeric module.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_algorithm::clamp;
///
/// assert_eq!(clamp(5, 0, 10), 5);
/// assert_eq!(clamp(-5, 0, 10), 0);
/// assert_eq!(clamp(15, 0, 10), 10);
/// ```
#[inline]
pub fn clamp<T: Copy + PartialOrd + Ord>(value: T, min: T, max: T) -> T {
    if value < min {
        min
    } else if value > max {
        max
    } else {
        value
    }
}

/// Returns the minimum value in a slice.
///
/// Returns `None` if the slice is empty.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_algorithm::min_element;
///
/// assert_eq!(min_element(&[3, 1, 4, 1, 5, 9, 2, 6]), Some(&1));
/// assert_eq!(min_element(&[] as &[i32]), None);
/// ```
#[inline]
pub fn min_element<T: Copy + PartialOrd + Ord>(slice: &[T]) -> Option<&T> {
    slice.iter().min()
}

/// Returns the maximum value in a slice.
///
/// Returns `None` if the slice is empty.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_algorithm::max_element;
///
/// assert_eq!(max_element(&[3, 1, 4, 1, 5, 9, 2, 6]), Some(&9));
/// assert_eq!(max_element(&[] as &[i32]), None);
/// ```
#[inline]
pub fn max_element<T: Copy + PartialOrd + Ord>(slice: &[T]) -> Option<&T> {
    slice.iter().max()
}

/// Returns the minimum and maximum values in a slice.
///
/// Returns `None` if the slice is empty.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_algorithm::minmax_element;
///
/// let (min, max) = minmax_element(&[3, 1, 4, 1, 5, 9, 2, 6]).unwrap();
/// assert_eq!(*min, 1);
/// assert_eq!(*max, 9);
/// ```
#[inline]
pub fn minmax_element<'a, T: PartialOrd>(slice: &'a [T]) -> Option<(&'a T, &'a T)> {
    if slice.is_empty() {
        return None;
    }

    let mut min = &slice[0];
    let mut max = &slice[0];

    for item in &slice[1..] {
        if item < min {
            min = item;
        }
        if item > max {
            max = item;
        }
    }

    Some((min, max))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clamp() {
        assert_eq!(clamp(5, 0, 10), 5);
        assert_eq!(clamp(-5, 0, 10), 0);
        assert_eq!(clamp(15, 0, 10), 10);
    }

    #[test]
    fn test_min_max_element() {
        assert_eq!(min_element(&[3, 1, 4, 1, 5, 9, 2, 6]), Some(&1));
        assert_eq!(max_element(&[3, 1, 4, 1, 5, 9, 2, 6]), Some(&9));
        assert_eq!(min_element(&[] as &[i32]), None);

        let (min, max) = minmax_element(&[3, 1, 4, 1, 5, 9, 2, 6]).unwrap();
        assert_eq!(*min, 1);
        assert_eq!(*max, 9);
    }
}
