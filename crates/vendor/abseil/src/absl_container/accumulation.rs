//! Container accumulation utilities - sum, product, min_element, max_element, minmax_element

/// Sums all elements in a container.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_container::sum;
///
/// assert_eq!(sum(&[1, 2, 3, 4, 5]), 15);
/// assert_eq!(sum(&[0i32; 0]), 0);
/// ```
#[inline]
pub fn sum<T: Copy + core::ops::Add<Output = T>>(source: &[T]) -> T {
    let mut result = unsafe { core::mem::zeroed() };
    for &item in source {
        result = result + item;
    }
    result
}

/// Computes the product of all elements in a container.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_container::product;
///
/// assert_eq!(product(&[1, 2, 3, 4, 5]), 120);
/// assert_eq!(product(&[2, 3, 4]), 24);
/// ```
#[inline]
pub fn product<T: Copy + core::ops::Mul<Output = T>>(source: &[T]) -> T {
    let mut result = unsafe { core::mem::zeroed() };
    // Initialize to 1 for the first element
    let mut first = true;
    for &item in source {
        if first {
            result = item;
            first = false;
        } else {
            result = result * item;
        }
    }
    result
}

/// Finds the minimum element in a container.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_container::min_element;
///
/// assert_eq!(min_element(&[3, 1, 4, 1, 5, 9, 2, 6]), Some(&1));
/// assert_eq!(min_element(&[] as &[i32]), None);
/// ```
#[inline]
pub fn min_element<T: PartialOrd>(source: &[T]) -> Option<&T> {
    source.iter().min()
}

/// Finds the maximum element in a container.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_container::max_element;
///
/// assert_eq!(max_element(&[3, 1, 4, 1, 5, 9, 2, 6]), Some(&9));
/// assert_eq!(max_element(&[] as &[i32]), None);
/// ```
#[inline]
pub fn max_element<T: PartialOrd>(source: &[T]) -> Option<&T> {
    source.iter().max()
}

/// Finds both the minimum and maximum elements in a container.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_container::minmax_element;
///
/// let (min, max) = minmax_element(&[3, 1, 4, 1, 5, 9, 2, 6]).unwrap();
/// assert_eq!(*min, 1);
/// assert_eq!(*max, 9);
/// ```
#[inline]
pub fn minmax_element<'a, T: PartialOrd>(source: &'a [T]) -> Option<(&'a T, &'a T)> {
    if source.is_empty() {
        return None;
    }

    let mut min = &source[0];
    let mut max = &source[0];

    for item in &source[1..] {
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
    fn test_sum() {
        assert_eq!(sum(&[1, 2, 3, 4, 5]), 15);
        assert_eq!(sum(&[0i32; 0]), 0);
    }

    #[test]
    fn test_product() {
        assert_eq!(product(&[1, 2, 3, 4, 5]), 120);
        assert_eq!(product(&[2, 3, 4]), 24);
    }

    #[test]
    fn test_min_max_element() {
        assert_eq!(min_element(&[3, 1, 4, 1, 5, 9, 2, 6]), Some(&1));
        assert_eq!(max_element(&[3, 1, 4, 1, 5, 9, 2, 6]), Some(&9));

        let (min, max) = minmax_element(&[3, 1, 4, 1, 5, 9, 2, 6]).unwrap();
        assert_eq!(*min, 1);
        assert_eq!(*max, 9);
    }
}
