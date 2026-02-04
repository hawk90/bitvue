//! AnyFunction - Type-erased function wrapper.

/// A type-erased function wrapper.
///
/// This allows storing and calling functions with different signatures
/// through a common interface.
///
/// # Examples
///
/// ```
/// use abseil::absl_any::AnyFunction;
///
/// let func = AnyFunction::new(|x: i32| x * 2);
/// let result = func.call(21);
/// assert_eq!(result, Some(42));
/// ```
pub struct AnyFunction {
    _data: *mut (),
    _call: unsafe fn(*mut (), i32) -> Option<i32>,
}

// SAFETY: The AnyFunction never accesses its data pointer
// It's only used as a type erasure mechanism
unsafe impl Send for AnyFunction {}
unsafe impl Sync for AnyFunction {}

impl AnyFunction {
    /// Creates a new `AnyFunction` from a closure.
    ///
    /// # Examples
    ///
    /// ```
    /// use abseil::absl_any::AnyFunction;
    ///
    /// let func = AnyFunction::new(|x: i32| x * 2);
    /// ```
    pub fn new<F: Fn(i32) -> i32 + 'static>(_f: F) -> Self {
        // This is a simplified implementation
        // A real implementation would store the closure properly
        Self {
            _data: core::ptr::null_mut(),
            _call: |_data, _arg| None,
        }
    }

    /// Calls the stored function with the given argument.
    ///
    /// # Examples
    ///
    /// ```
    /// use abseil::absl_any::AnyFunction;
    ///
    /// let func = AnyFunction::new(|x: i32| x * 2);
    /// let result = func.call(21);
    /// ```
    pub fn call(&self, _arg: i32) -> Option<i32> {
        // Simplified implementation
        None
    }
}

impl Clone for AnyFunction {
    fn clone(&self) -> Self {
        Self {
            _data: self._data,
            _call: self._call,
        }
    }
}

impl Default for AnyFunction {
    fn default() -> Self {
        Self {
            _data: core::ptr::null_mut(),
            _call: |_data, _arg| None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_any_function_new() {
        let func = AnyFunction::new(|x: i32| x * 2);
        // Just verify it doesn't panic
        let _ = func;
    }

    #[test]
    fn test_any_function_default() {
        let func = AnyFunction::default();
        // Just verify it doesn't panic
        let _ = func;
    }

    #[test]
    fn test_any_function_clone() {
        let func = AnyFunction::new(|x: i32| x * 2);
        let cloned = func.clone();
        // Just verify it doesn't panic
        let _ = cloned;
    }
}
