//! Swap and exchange utilities.

/// Swaps the values of two references.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_utility::swap_exchange::swap;
///
/// let mut a = 1;
/// let mut b = 2;
/// swap(&mut a, &mut b);
/// assert_eq!(a, 2);
/// assert_eq!(b, 1);
/// ```
#[inline]
pub fn swap<T>(a: &mut T, b: &mut T) {
    core::mem::swap(a, b);
}

/// Replaces a value with a new one, returning the old value.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_utility::swap_exchange::replace;
///
/// let mut value = 1;
/// let old = replace(&mut value, 2);
/// assert_eq!(old, 1);
/// assert_eq!(value, 2);
/// ```
#[inline]
pub fn replace<T>(dest: &mut T, src: T) -> T {
    core::mem::replace(dest, src)
}

/// Takes the value out of a reference, leaving a default value in its place.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_utility::swap_exchange::take;
///
/// let mut value = vec![1, 2, 3];
/// let vec = take(&mut value);
/// assert!(value.is_empty());
/// assert_eq!(vec, vec![1, 2, 3]);
/// ```
#[inline]
pub fn take<T: Default>(dest: &mut T) -> T {
    core::mem::take(dest)
}

/// Exchanges the values at two mutable references and returns both old values.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_utility::swap_exchange::exchange;
///
/// let mut a = 1;
/// let mut b = 2;
/// let (old_a, old_b) = exchange(&mut a, &mut b);
/// assert_eq!(old_a, 1);
/// assert_eq!(old_b, 2);
/// assert_eq!(a, 2);
/// assert_eq!(b, 1);
/// ```
#[inline]
pub fn exchange<T>(a: &mut T, b: &mut T) -> (T, T) {
    let old_a = core::mem::replace(a, core::mem::replace(b, core::mem::take(a)));
    (old_a, core::mem::take(b))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_swap() {
        let mut a = 1;
        let mut b = 2;
        swap(&mut a, &mut b);
        assert_eq!(a, 2);
        assert_eq!(b, 1);
    }

    #[test]
    fn test_replace() {
        let mut value = 1;
        let old = replace(&mut value, 2);
        assert_eq!(old, 1);
        assert_eq!(value, 2);
    }

    #[test]
    fn test_take() {
        let mut value = vec![1, 2, 3];
        let vec = take(&mut value);
        assert!(value.is_empty());
        assert_eq!(vec, vec![1, 2, 3]);
    }

    #[test]
    fn test_exchange() {
        let mut a = 1;
        let mut b = 2;
        let (old_a, old_b) = exchange(&mut a, &mut b);
        assert_eq!(old_a, 1);
        assert_eq!(old_b, 2);
        assert_eq!(a, 2);
        assert_eq!(b, 1);
    }
}
