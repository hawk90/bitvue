//! Permutation utilities - is_permutation, next_permutation, prev_permutation

/// Checks if one slice is a permutation of another.
///
/// Both slices must contain the same elements (including duplicates),
/// but potentially in a different order.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_algorithm::is_permutation;
///
/// assert!(is_permutation(&[1, 2, 3], &[3, 2, 1]));
/// assert!(is_permutation(&[1, 2, 2, 3], &[2, 3, 1, 2]));
/// assert!(!is_permutation(&[1, 2, 3], &[1, 2, 4]));
/// ```
#[inline]
pub fn is_permutation<T: PartialEq>(a: &[T], b: &[T]) -> bool {
    if a.len() != b.len() {
        return false;
    }

    let mut b_copy = b.to_vec();
    for item in a {
        if let Some(pos) = b_copy.iter().position(|x| x == item) {
            b_copy.remove(pos);
        } else {
            return false;
        }
    }

    b_copy.is_empty()
}

/// Generates the next lexicographic permutation.
///
/// Transforms the slice into the next permutation in lexicographic order.
/// Returns `true` if a next permutation exists, `false` if this was the last one.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_algorithm::next_permutation;
///
/// let mut data = [1, 2, 3];
/// assert!(next_permutation(&mut data));
/// assert_eq!(data, [1, 3, 2]);
/// assert!(next_permutation(&mut data));
/// assert_eq!(data, [2, 1, 3]);
/// ```
#[inline]
pub fn next_permutation<T: PartialOrd>(slice: &mut [T]) -> bool {
    if slice.len() <= 1 {
        return false;
    }

    // Find the largest index i such that slice[i] < slice[i + 1]
    let mut i = slice.len() - 2;
    while i > 0 && slice[i] >= slice[i + 1] {
        i -= 1;
    }

    if slice[i] >= slice[i + 1] {
        return false;
    }

    // Find the largest index j > i such that slice[i] < slice[j]
    let mut j = slice.len() - 1;
    while slice[j] <= slice[i] {
        j -= 1;
    }

    // Swap slice[i] and slice[j]
    slice.swap(i, j);

    // Reverse the suffix starting at i + 1
    slice[i + 1..].reverse();

    true
}

/// Generates the previous lexicographic permutation.
///
/// Transforms the slice into the previous permutation in lexicographic order.
/// Returns `true` if a previous permutation exists, `false` if this was the first one.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_algorithm::prev_permutation;
///
/// let mut data = [3, 2, 1];
/// assert!(prev_permutation(&mut data));
/// assert_eq!(data, [3, 1, 2]);
/// ```
#[inline]
pub fn prev_permutation<T: PartialOrd>(slice: &mut [T]) -> bool {
    if slice.len() <= 1 {
        return false;
    }

    // Find the largest index i such that slice[i] > slice[i + 1]
    let mut i = slice.len() - 2;
    while i > 0 && slice[i] <= slice[i + 1] {
        i -= 1;
    }

    if slice[i] <= slice[i + 1] {
        return false;
    }

    // Find the largest index j > i such that slice[i] > slice[j]
    let mut j = slice.len() - 1;
    while slice[j] >= slice[i] {
        j -= 1;
    }

    // Swap slice[i] and slice[j]
    slice.swap(i, j);

    // Reverse the suffix starting at i + 1
    slice[i + 1..].reverse();

    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_permutation() {
        assert!(is_permutation(&[1, 2, 3], &[3, 2, 1]));
        assert!(is_permutation(&[1, 2, 2, 3], &[2, 3, 1, 2]));
        assert!(!is_permutation(&[1, 2, 3], &[1, 2, 4]));
    }

    #[test]
    fn test_next_permutation() {
        let mut data = [1, 2, 3];
        assert!(next_permutation(&mut data));
        assert_eq!(data, [1, 3, 2]);
        assert!(next_permutation(&mut data));
        assert_eq!(data, [2, 1, 3]);
    }

    #[test]
    fn test_prev_permutation() {
        let mut data = [3, 2, 1];
        assert!(prev_permutation(&mut data));
        assert_eq!(data, [3, 1, 2]);
    }
}
