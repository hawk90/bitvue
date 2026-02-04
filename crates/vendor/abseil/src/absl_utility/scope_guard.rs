//! Scope guard utilities.

use alloc::vec::Vec;
use core::mem::ManuallyDrop;

/// A scope guard that runs a function when dropped.
///
/// This is similar to Abseil's `Cleanup` or C++'s `ScopeGuard`.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_utility::scope_guard::ScopeGuard;
///
/// let mut resource_acquired = false;
/// let _guard = ScopeGuard::new(|| {
///     resource_acquired = false;
/// });
/// resource_acquired = true;
/// // _guard drops here, setting resource_acquired back to false
/// ```
pub struct ScopeGuard<F: FnOnce()> {
    cleanup: Option<ManuallyDrop<F>>,
}

impl<F: FnOnce()> ScopeGuard<F> {
    /// Creates a new scope guard with the given cleanup function.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use abseil::absl_utility::scope_guard::ScopeGuard;
    ///
    /// let _guard = ScopeGuard::new(|| {
    ///     println!("Cleaning up!");
    /// });
    /// ```
    #[inline]
    pub fn new(cleanup: F) -> Self {
        ScopeGuard {
            cleanup: Some(ManuallyDrop::new(cleanup)),
        }
    }

    /// Disarms the guard, preventing the cleanup function from running.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use abseil::absl_utility::scope_guard::ScopeGuard;
    ///
    /// let mut guard = ScopeGuard::new(|| {
    ///     println!("This won't run");
    /// });
    /// guard.disarm();
    /// // Cleanup won't run when guard is dropped
    /// ```
    #[inline]
    pub fn disarm(&mut self) {
        self.cleanup.take();
    }
}

impl<F: FnOnce()> Drop for ScopeGuard<F> {
    fn drop(&mut self) {
        if let Some(cleanup) = self.cleanup.take() {
            // SAFETY: We take() the cleanup, so it only runs once
            unsafe { ManuallyDrop::into_inner(cleanup)() };
        }
    }
}

/// Creates a scope guard that runs a function when dropped.
///
/// This is a convenience function for creating `ScopeGuard`.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_utility::scope_guard::on_scope_exit;
///
/// let mut state = 0;
/// {
///     let _guard = on_scope_exit(|| state = 42);
///     state = 100;
/// }
/// assert_eq!(state, 42);  // Set by the guard
/// ```
#[inline]
pub fn on_scope_exit<F: FnOnce()>(cleanup: F) -> ScopeGuard<F> {
    ScopeGuard::new(cleanup)
}

/// A guard that runs a function on scope exit, but can be deferred.
///
/// This is similar to Abseil's `Cleanup` with defer capability.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_utility::scope_guard::DeferGuard;
///
/// let mut guard = DeferGuard::new(|| {
///     println!("Deferred cleanup!");
/// });
/// guard.run_now();  // Runs the cleanup early
/// ```
pub struct DeferGuard<F: FnOnce()> {
    cleanup: Option<ManuallyDrop<F>>,
}

impl<F: FnOnce()> DeferGuard<F> {
    /// Creates a new defer guard.
    #[inline]
    pub fn new(cleanup: F) -> Self {
        DeferGuard {
            cleanup: Some(ManuallyDrop::new(cleanup)),
        }
    }

    /// Runs the cleanup function immediately and disarms the guard.
    #[inline]
    pub fn run_now(&mut self) {
        if let Some(cleanup) = self.cleanup.take() {
            unsafe { ManuallyDrop::into_inner(cleanup)() };
        }
    }
}

impl<F: FnOnce()> Drop for DeferGuard<F> {
    fn drop(&mut self) {
        if let Some(cleanup) = self.cleanup.take() {
            unsafe { ManuallyDrop::into_inner(cleanup)() };
        }
    }
}

/// A guard that runs cleanup functions in reverse order on drop.
///
/// This is useful for managing multiple resources that need cleanup in LIFO order.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_utility::scope_guard::CleanupStack;
///
/// let mut stack = CleanupStack::new();
/// stack.push(|| println!("Cleanup 3"));
/// stack.push(|| println!("Cleanup 2"));
/// stack.push(|| println!("Cleanup 1"));
/// // Drops in reverse order: 1, 2, 3
/// ```
pub struct CleanupStack {
    cleanups: Vec<Box<dyn FnOnce()>>,
}

impl CleanupStack {
    /// Creates a new empty cleanup stack.
    #[inline]
    pub fn new() -> Self {
        CleanupStack {
            cleanups: Vec::new(),
        }
    }

    /// Pushes a cleanup function onto the stack.
    #[inline]
    pub fn push<F: FnOnce() + 'static>(&mut self, cleanup: F) {
        self.cleanups.push(Box::new(cleanup));
    }

    /// Runs all cleanups immediately and clears the stack.
    #[inline]
    pub fn run_all(&mut self) {
        while let Some(cleanup) = self.cleanups.pop() {
            cleanup();
        }
    }
}

impl Default for CleanupStack {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for CleanupStack {
    fn drop(&mut self) {
        self.run_all();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cleanup_stack() {
        let mut order = Vec::new();
        {
            let mut stack = CleanupStack::new();
            stack.push(|| order.push(3));
            stack.push(|| order.push(2));
            stack.push(|| order.push(1));
        }
        // LIFO order
        assert_eq!(order, vec![1, 2, 3]);
    }
}
