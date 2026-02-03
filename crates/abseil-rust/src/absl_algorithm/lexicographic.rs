//! Lexicographical comparison - lexicographical_compare

/// Compares two slices lexicographically.
///
/// Returns `Ordering::Less` if the first slice is lexicographically less,
/// `Ordering::Equal` if they are equal, `Ordering::Greater` otherwise.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_algorithm::lexicographical_compare;
/// use core::cmp::Ordering;
///
/// assert_eq!(lexicographical_compare(&[1, 2, 3], &[1, 2, 4]), Ordering::Less);
/// assert_eq!(lexicographical_compare(&[1, 2, 3], &[1, 2, 3]), Ordering::Equal);
/// assert_eq!(lexicographical_compare(&[1, 2, 4], &[1, 2, 3]), Ordering::Greater);
/// ```
#[inline]
pub fn lexicographical_compare<T: PartialOrd>(a: &[T], b: &[T]) -> core::cmp::Ordering {
    let min_len = a.len().min(b.len());

    for i in 0..min_len {
        match a[i].partial_cmp(&b[i]) {
            Some(core::cmp::Ordering::Equal) => continue,
            Some(ord) => return ord,
            None => return core::cmp::Ordering::Equal,
        }
    }

    a.len().cmp(&b.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lexicographical_compare() {
        use core::cmp::Ordering;
        assert_eq!(lexicographical_compare(&[1, 2, 3], &[1, 2, 4]), Ordering::Less);
        assert_eq!(lexicographical_compare(&[1, 2, 3], &[1, 2, 3]), Ordering::Equal);
        assert_eq!(lexicographical_compare(&[1, 2, 4], &[1, 2, 3]), Ordering::Greater);
    }
}
