//! FlagValue - A type-safe flag wrapper for any flag type

/// A type-safe flag wrapper for any flag type.
///
/// This provides a generic wrapper around flag values with conversion support.
#[derive(Clone, Debug)]
pub struct FlagValue<T> {
    inner: T,
    specified: bool,
}

impl<T: Clone> FlagValue<T> {
    /// Creates a new flag value.
    #[inline]
    pub const fn new(value: T) -> Self {
        Self {
            inner: value,
            specified: false,
        }
    }

    /// Gets the flag value.
    #[inline]
    pub const fn value(&self) -> &T {
        &self.inner
    }

    /// Checks if the flag was explicitly specified.
    #[inline]
    pub const fn is_specified(&self) -> bool {
        self.specified
    }

    /// Sets the flag value and marks it as specified.
    #[inline]
    pub fn set(&mut self, value: T) {
        self.inner = value;
        self.specified = true;
    }

    /// Converts the flag value.
    #[inline]
    pub fn map<U, F>(self, f: F) -> FlagValue<U>
    where
        F: FnOnce(T) -> U,
    {
        FlagValue {
            inner: f(self.inner),
            specified: self.specified,
        }
    }
}

impl<T: Default> Default for FlagValue<T> {
    #[inline]
    fn default() -> Self {
        Self {
            inner: T::default(),
            specified: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_flag_value() {
        let flag = FlagValue::new(42);
        assert_eq!(*flag.value(), 42);
        assert!(!flag.is_specified());

        let mut flag = FlagValue::new(10);
        flag.set(20);
        assert_eq!(*flag.value(), 20);
        assert!(flag.is_specified());
    }

    #[test]
    fn test_flag_value_map() {
        let flag = FlagValue::new(42);
        let mapped = flag.map(|x| x * 2);
        assert_eq!(*mapped.value(), 84);
        assert!(!mapped.is_specified());
    }
}
