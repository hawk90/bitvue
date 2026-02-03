//! Transform utilities - transform, transform_copy, for_each_pair, equal_by

/// Transforms a slice in-place using a function.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_algorithm::transform;
///
/// let mut data = [1, 2, 3, 4, 5];
/// transform(&mut data, |x| x * 2);
/// assert_eq!(data, [2, 4, 6, 8, 10]);
/// ```
#[inline]
pub fn transform<T, F>(slice: &mut [T], mut f: F)
where
    F: FnMut(T) -> T,
{
    for item in slice.iter_mut() {
        *item = f(core::mem::replace(item, unsafe { core::mem::zeroed() }));
    }
}

/// Transforms one slice into another.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_algorithm::transform_copy;
///
/// let src = [1, 2, 3, 4, 5];
/// let mut dest = [0; 5];
/// transform_copy(&src, &mut dest, |x| x * 2);
/// assert_eq!(dest, [2, 4, 6, 8, 10]);
/// ```
#[inline]
pub fn transform_copy<S, T, F>(src: &[S], dest: &mut [T], mut f: F)
where
    F: FnMut(&S) -> T,
{
    let len = src.len().min(dest.len());
    for i in 0..len {
        dest[i] = f(&src[i]);
    }
}

/// Applies a function to all elements of two slices.
///
/// Stops at the shorter of the two slices.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_algorithm::for_each_pair;
///
/// let a = [1, 2, 3, 4];
/// let b = [10, 20, 30, 40];
/// let mut sum = 0;
/// for_each_pair(&a, &b, |x, y| sum += x + y);
/// assert_eq!(sum, 100);
/// ```
#[inline]
pub fn for_each_pair<A, B, F>(a: &[A], b: &[B], mut f: F)
where
    F: FnMut(&A, &B),
{
    let len = a.len().min(b.len());
    for i in 0..len {
        f(&a[i], &b[i]);
    }
}

/// Checks if two slices are equal according to a predicate.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_algorithm::equal_by;
///
/// assert!(equal_by(&[1.0, 2.0, 3.0], &[1.01, 2.01, 3.01], |a, b| (a - b).abs() < 0.1));
/// ```
#[inline]
pub fn equal_by<T, U, F>(a: &[T], b: &[U], mut f: F) -> bool
where
    F: FnMut(&T, &U) -> bool,
{
    if a.len() != b.len() {
        return false;
    }

    for i in 0..a.len() {
        if !f(&a[i], &b[i]) {
            return false;
        }
    }

    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transform() {
        let mut data = [1, 2, 3, 4, 5];
        transform(&mut data, |x| x * 2);
        assert_eq!(data, [2, 4, 6, 8, 10]);
    }

    #[test]
    fn test_transform_copy() {
        let src = [1, 2, 3, 4, 5];
        let mut dest = [0; 5];
        transform_copy(&src, &mut dest, |x| x * 2);
        assert_eq!(dest, [2, 4, 6, 8, 10]);
    }

    #[test]
    fn test_equal_by() {
        assert!(equal_by(&[1.0, 2.0, 3.0], &[1.01, 2.01, 3.01], |a, b| (a - b).abs() < 0.1));
        assert!(!equal_by(&[1, 2, 3], &[1, 2, 4], |a, b| a == b));
    }
}
