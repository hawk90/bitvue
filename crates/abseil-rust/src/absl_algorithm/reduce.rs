//! Accumulate and reduce utilities - accumulate, reduce

/// Accumulates values starting from the first element.
///
/// Equivalent to `fold` but starts with the first element as the initial value.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_algorithm::accumulate;
///
/// assert_eq!(accumulate(&[1, 2, 3, 4], |a, b| a + b), Some(10));
/// assert_eq!(accumulate(&[1, 2, 3, 4], |a, b| a * b), Some(24));
/// ```
#[inline]
pub fn accumulate<T, F>(slice: &[T], mut op: F) -> Option<T>
where
    T: Clone,
    F: FnMut(T, T) -> T,
{
    if slice.is_empty() {
        return None;
    }

    let mut result = slice[0].clone();
    for item in &slice[1..] {
        result = op(result, item.clone());
    }
    Some(result)
}

/// Reduces a slice using a binary operation.
///
/// Returns None if the slice is empty.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_algorithm::reduce;
///
/// assert_eq!(reduce(&[1, 2, 3, 4], |a, b| a + b), Some(10));
/// assert_eq!(reduce::<i32, _>(&[], |a, b| a + b), None);
/// ```
#[inline]
pub fn reduce<T, F>(slice: &[T], op: F) -> Option<T>
where
    T: Clone,
    F: FnMut(T, T) -> T,
{
    accumulate(slice, op)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_accumulate() {
        assert_eq!(accumulate(&[1, 2, 3, 4], |a, b| a + b), Some(10));
        assert_eq!(accumulate(&[1, 2, 3, 4], |a, b| a * b), Some(24));
        assert_eq!(accumulate::<i32, _>(&[], |a, b| a + b), None);
    }

    #[test]
    fn test_reduce() {
        assert_eq!(reduce(&[1, 2, 3, 4], |a, b| a + b), Some(10));
        assert_eq!(reduce::<i32, _>(&[], |a, b| a + b), None);
    }
}
