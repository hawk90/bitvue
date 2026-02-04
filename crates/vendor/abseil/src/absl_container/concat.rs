//! Concatenation utilities - concat, concat_vecs

use alloc::vec::Vec;

/// Concatenates multiple slices into a new vector.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_container::concat;
///
/// let result: Vec<i32> = concat(&[&[1, 2], &[3, 4], &[5]]);
/// assert_eq!(result, vec![1, 2, 3, 4, 5]);
/// ```
#[inline]
pub fn concat<T, S>(slices: &[S]) -> Vec<T>
where
    T: Clone,
    S: AsRef<[T]>,
{
    let mut result = Vec::new();
    for slice in slices {
        result.extend_from_slice(slice.as_ref());
    }
    result
}

/// Concatenates multiple vectors.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_container::concat_vecs;
///
/// let result = concat_vecs(&[vec![1, 2], vec![3, 4]]);
/// assert_eq!(result, vec![1, 2, 3, 4]);
/// ```
#[inline]
pub fn concat_vecs<T: Clone>(vecs: &[Vec<T>]) -> Vec<T> {
    let total_len: usize = vecs.iter().map(|v| v.len()).sum();
    let mut result = Vec::with_capacity(total_len);
    for vec in vecs {
        result.extend(vec.iter().cloned());
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_concat() {
        let result: Vec<i32> = concat(&[&[1, 2], &[3, 4], &[5]]);
        assert_eq!(result, vec![1, 2, 3, 4, 5]);
    }

    #[test]
    fn test_concat_vecs() {
        let result = concat_vecs(&[vec![1, 2], vec![3, 4]]);
        assert_eq!(result, vec![1, 2, 3, 4]);
    }
}
