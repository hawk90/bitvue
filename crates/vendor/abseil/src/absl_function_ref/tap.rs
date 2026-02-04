//! Tap/side effect utilities - tap, tap_mut

/// Calls a function with a value, then returns the value.
///
/// Useful for side effects like logging.
#[inline]
pub fn tap<T, F>(mut value: T, mut f: F) -> T
where
    F: FnMut(&T),
{
    f(&value);
    value
}

/// Calls a function with a mutable reference, then returns the value.
#[inline]
pub fn tap_mut<T, F>(mut value: T, mut f: F) -> T
where
    F: FnMut(&mut T),
{
    f(&mut value);
    value
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tap() {
        let mut log = Vec::new();
        let result = tap(5, |&x| log.push(x));

        assert_eq!(result, 5);
        assert_eq!(log, vec![5]);
    }

    #[test]
    fn test_tap_mut() {
        let mut value = 5;
        let result = tap_mut(value, |v| *v *= 2);

        assert_eq!(result, 5); // Original value unchanged
        assert_eq!(value, 5);
    }
}
