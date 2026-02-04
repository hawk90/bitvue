//! Comparison utilities - compare_by, compare_by_desc

/// Creates a comparator function from a key extraction function.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_function_ref::compare_by;
///
/// let data = [(1, "a"), (3, "c"), (2, "b")];
/// let mut sorted = data;
/// sorted.sort_by(compare_by(|&(x, _)| x));
/// assert_eq!(sorted, [(1, "a"), (2, "b"), (3, "c")]);
/// ```
#[inline]
pub fn compare_by<A, B, F>(mut f: F) -> impl FnMut(&A, &A) -> core::cmp::Ordering
where
    B: PartialOrd,
    F: FnMut(&A) -> B,
{
    move |a, b| f(a).partial_cmp(f(b)).unwrap_or(core::cmp::Ordering::Equal)
}

/// Creates a comparator function from a key extraction function (reverse).
///
/// # Examples
///
/// ```rust
/// use abseil::absl_function_ref::compare_by_desc;
///
/// let data = [(1, "a"), (3, "c"), (2, "b")];
/// let mut sorted = data;
/// sorted.sort_by(compare_by_desc(|&(x, _)| x));
/// assert_eq!(sorted, [(3, "c"), (2, "b"), (1, "a")]);
/// ```
#[inline]
pub fn compare_by_desc<A, B, F>(mut f: F) -> impl FnMut(&A, &A) -> core::cmp::Ordering
where
    B: PartialOrd,
    F: FnMut(&A) -> B,
{
    move |a, b| f(b).partial_cmp(f(a)).unwrap_or(core::cmp::Ordering::Equal)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compare_by() {
        let data = [(1, "a"), (3, "c"), (2, "b")];
        let mut sorted = data;
        sorted.sort_by(compare_by(|&(x, _)| x));
        assert_eq!(sorted, [(1, "a"), (2, "b"), (3, "c")]);
    }

    #[test]
    fn test_compare_by_desc() {
        let data = [(1, "a"), (3, "c"), (2, "b")];
        let mut sorted = data;
        sorted.sort_by(compare_by_desc(|&(x, _)| x));
        assert_eq!(sorted, [(3, "c"), (2, "b"), (1, "a")]);
    }
}
