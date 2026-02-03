//! ScopeGuard for RAII cleanup - ensures cleanup code runs even when panicking.

use core::cell::Cell;

/// An RAII guard that runs a function when dropped.
///
/// This is useful for ensuring cleanup code runs even when panicking.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_synchronization::ScopeGuard;
///
/// let cleaned = false;
/// {
///     let _guard = ScopeGuard::new(|| {
///         // Cleanup code here
///     });
///     // Do work...
/// } // Cleanup runs here even on panic
/// ```
#[derive(Debug)]
pub struct ScopeGuard<F: FnOnce()>
where
    F: FnOnce(),
{
    cleanup: Option<core::mem::ManuallyDrop<F>>,
    dismissed: Cell<bool>,
}

impl<F: FnOnce()> ScopeGuard<F> {
    /// Creates a new ScopeGuard with the given cleanup function.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use abseil::absl_synchronization::ScopeGuard;
    ///
    /// let guard = ScopeGuard::new(|| {
    ///     println!("Cleaning up!");
    /// });
    /// ```
    pub fn new(f: F) -> Self {
        Self {
            cleanup: Some(core::mem::ManuallyDrop::new(f)),
            dismissed: Cell::new(false),
        }
    }

    /// Dismisses the guard, preventing the cleanup function from running.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use abseil::absl_synchronization::ScopeGuard;
    ///
    /// let guard = ScopeGuard::new(|| println!("Cleanup"));
    /// guard.dismiss(); // Cleanup won't run
    /// ```
    pub fn dismiss(&self) {
        self.dismissed.set(true);
    }

    /// Returns true if this guard has been dismissed.
    pub fn is_dismissed(&self) -> bool {
        self.dismissed.get()
    }

    /// Executes the cleanup function early.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use abseil::absl_synchronization::ScopeGuard;
    ///
    /// let mut guard = ScopeGuard::new(|| println!("Cleanup"));
    /// guard.invoke(); // Runs cleanup now
    /// ```
    pub fn invoke(&mut self) {
        if !self.dismissed.get() {
            self.dismissed.set(true);
            unsafe {
                let f = core::mem::ManuallyDrop::take(&mut self.cleanup);
                f();
            }
        }
    }
}

impl<F: FnOnce()> Drop for ScopeGuard<F> {
    fn drop(&mut self) {
        if !self.dismissed.get() {
            unsafe {
                let f = core::mem::ManuallyDrop::take(&mut self.cleanup);
                f();
            }
        }
    }
}

/// Creates a scope guard that runs the given function when dropped.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_synchronization::scope_guard;
///
/// {
///     let _guard = scope_guard(|| {
///         println!("Cleanup!");
///     });
///     // Do work...
/// } // Cleanup runs here automatically
/// ```
#[inline]
pub fn scope_guard<F: FnOnce()>(f: F) -> ScopeGuard<F> {
    ScopeGuard::new(f)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scope_guard() {
        let mut cleaned = false;
        {
            let _guard = scope_guard(|| {
                cleaned = true;
            });
            assert!(!cleaned);
        }
        assert!(cleaned);
    }

    #[test]
    fn test_scope_guard_dismiss() {
        let cleaned = false;
        {
            let guard = scope_guard(|| {
                cleaned = true;
            });
            guard.dismiss();
        }
        assert!(!cleaned);
    }

    #[test]
    fn test_scope_guard_invoke() {
        let mut cleaned = false;
        {
            let mut guard = scope_guard(|| {
                cleaned = true;
            });
            guard.invoke();
            assert!(cleaned);
        }
        assert!(cleaned);
    }
}
