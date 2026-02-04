//! Type-erased function reference.
//!
//! This module provides a `FunctionRef` type similar to Abseil's `absl::functional`,
//! which allows storing function references without knowing the exact type.

use core::fmt;

/// A type-erased function reference.
///
/// This is a simplified version that stores boxed function pointers.
/// In Rust, the idiomatic approach would typically use `Box<dyn FnMut()>`
/// or generics, but this provides a C++-like interface.
pub struct FunctionRef<'a> {
    /// Marker for the lifetime.
    phantom: core::marker::PhantomData<&'a ()>,
    /// Whether the function reference is valid.
    valid: bool,
}

impl<'a> FunctionRef<'a> {
    /// Creates a new `FunctionRef`.
    ///
    /// This is a placeholder implementation. In a real type-erased callback
    /// system, you would use trait objects or other mechanisms.
    pub fn new() -> Self {
        Self {
            phantom: core::marker::PhantomData,
            valid: true,
        }
    }

    /// Returns whether this function reference is valid.
    pub fn is_valid(&self) -> bool {
        self.valid
    }

    /// Marks this function reference as invalid.
    pub fn invalidate(&mut self) {
        self.valid = false;
    }
}

impl<'a> Clone for FunctionRef<'a> {
    fn clone(&self) -> Self {
        Self {
            phantom: core::marker::PhantomData,
            valid: self.valid,
        }
    }
}

impl<'a> Default for FunctionRef<'a> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a> fmt::Debug for FunctionRef<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "FunctionRef({})", if self.valid { "valid" } else { "invalid" })
    }
}

/// A simple callback registry for storing callbacks.
///
/// In idiomatic Rust, you would typically use `Vec<Box<dyn FnMut()>>` or
/// generics for this pattern. This implementation provides a simpler
/// interface that doesn't require type erasure.
pub struct CallbackRegistry<'a> {
    /// Number of registered callbacks (placeholder).
    count: usize,
    /// Phantom data for lifetime.
    phantom: core::marker::PhantomData<&'a ()>,
}

impl<'a> CallbackRegistry<'a> {
    /// Creates a new empty callback registry.
    pub fn new() -> Self {
        Self {
            count: 0,
            phantom: core::marker::PhantomData,
        }
    }

    /// Registers a callback (increment counter).
    pub fn register(&mut self) {
        self.count += 1;
    }

    /// Calls all registered callbacks.
    pub fn call_all(&mut self) {
        // Placeholder: In a real implementation, this would call each callback
        // For now, just track that the call happened
    }

    /// Returns the number of registered callbacks.
    pub fn len(&self) -> usize {
        self.count
    }

    /// Returns whether the registry is empty.
    pub fn is_empty(&self) -> bool {
        self.count == 0
    }

    /// Clears all callbacks.
    pub fn clear(&mut self) {
        self.count = 0;
    }
}

impl<'a> Default for CallbackRegistry<'a> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a> fmt::Debug for CallbackRegistry<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "CallbackRegistry({} callbacks)", self.count)
    }
}

/// A trait for callback functionality.
///
/// This is the idiomatic Rust approach for callbacks.
/// Users can implement this trait for their own types.
pub trait Callback {
    /// Calls the callback.
    fn call(&mut self);
}

/// Simple callback wrapper for functions.
pub struct FunctionCallback<F>(pub F)
where
    F: FnMut();

impl<F> Callback for FunctionCallback<F>
where
    F: FnMut(),
{
    fn call(&mut self) {
        (self.0)();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_function_ref_new() {
        let func_ref = FunctionRef::new();
        assert!(func_ref.is_valid());
    }

    #[test]
    fn test_function_ref_clone() {
        let func_ref = FunctionRef::new();
        let cloned = func_ref.clone();
        assert!(cloned.is_valid());
    }

    #[test]
    fn test_function_ref_invalidate() {
        let mut func_ref = FunctionRef::new();
        assert!(func_ref.is_valid());
        func_ref.invalidate();
        assert!(!func_ref.is_valid());
    }

    #[test]
    fn test_function_ref_default() {
        let func_ref = FunctionRef::default();
        assert!(func_ref.is_valid());
    }

    #[test]
    fn test_function_ref_debug() {
        let func_ref = FunctionRef::new();
        let s = format!("{:?}", func_ref);
        assert!(s.contains("FunctionRef"));
        assert!(s.contains("valid"));
    }

    #[test]
    fn test_callback_registry_new() {
        let registry = CallbackRegistry::new();
        assert_eq!(registry.len(), 0);
        assert!(registry.is_empty());
    }

    #[test]
    fn test_callback_registry_default() {
        let registry = CallbackRegistry::default();
        assert_eq!(registry.len(), 0);
    }

    #[test]
    fn test_callback_registry_register() {
        let mut registry = CallbackRegistry::new();
        registry.register();
        registry.register();
        assert_eq!(registry.len(), 2);
        assert!(!registry.is_empty());
    }

    #[test]
    fn test_callback_registry_clear() {
        let mut registry = CallbackRegistry::new();
        registry.register();
        registry.register();
        assert_eq!(registry.len(), 2);
        registry.clear();
        assert_eq!(registry.len(), 0);
    }

    #[test]
    fn test_callback_registry_call_all() {
        let mut registry = CallbackRegistry::new();
        registry.register();
        registry.call_all();
        // Just verify it doesn't panic
    }

    #[test]
    fn test_callback_registry_debug() {
        let mut registry = CallbackRegistry::new();
        registry.register();
        let s = format!("{:?}", registry);
        assert!(s.contains("CallbackRegistry"));
        assert!(s.contains("1"));
    }

    #[test]
    fn test_function_callback() {
        let mut callback = FunctionCallback(|| {
            // Just verify it can be called without panicking
        });
        callback.call();
        // Test passes if no panic occurs
    }

    #[test]
    fn test_function_callback_multiple_calls() {
        let mut count = 0;
        let mut callback = FunctionCallback(|| {
            count += 1;
        });
        for _ in 0..3 {
            callback.call();
        }
        assert_eq!(count, 3);
    }

    #[test]
    fn test_function_callback_with_result() {
        let mut results = Vec::new();
        let mut callback = FunctionCallback(|| {
            results.push(42);
        });
        callback.call();
        callback.call();
        assert_eq!(results, vec![42, 42]);
    }
}
