//! FinallyGuard - try-finally pattern guard.

use core::mem::ManuallyDrop;

/// A try-finally style guard that runs cleanup regardless of success/failure.
///
/// This is similar to Java's try-finally or Python's try-finally blocks.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_cleanup::FinallyGuard;
///
/// fn operation() -> Result<(), String> {
///     let finally = FinallyGuard::new(|| {
///         println!("This always runs");
///     });
///
///     // Do work...
///     Err("error".to_string()) // finally still runs
/// }
/// ```
pub struct FinallyGuard<F: FnOnce()> {
    cleanup_fn: ManuallyDrop<F>,
}

impl<F: FnOnce()> FinallyGuard<F> {
    /// Creates a new finally guard.
    ///
    /// The cleanup will always run when the guard is dropped.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use abseil::absl_cleanup::FinallyGuard;
    ///
    /// let guard = FinallyGuard::new(|| {
    ///     println!("Finally!");
    /// });
    /// ```
    pub fn new(f: F) -> Self {
        Self {
            cleanup_fn: ManuallyDrop::new(f),
        }
    }

    /// Executes the cleanup immediately.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use abseil::absl_cleanup::FinallyGuard;
    ///
    /// let mut guard = FinallyGuard::new(|| {
    ///     println!("Running now");
    /// });
    /// guard.execute();
    /// ```
    pub fn execute(&mut self) {
        let f = unsafe { ManuallyDrop::take(&mut self.cleanup_fn) };
        f();
    }
}

impl<F: FnOnce()> Drop for FinallyGuard<F> {
    fn drop(&mut self) {
        let f = unsafe { ManuallyDrop::take(&mut self.cleanup_fn) };
        f();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_finally_guard_always_runs() {
        let ran = crate::absl_cleanup::tests::TestCounter::new();
        {
            let _guard = FinallyGuard::new(|| ran.inc());
            // panic!("Error!"); // Even with panic, finally runs (but test would fail)
        }
        assert_eq!(ran.get(), 1);
    }

    #[test]
    fn test_finally_guard_execute() {
        let ran = crate::absl_cleanup::tests::TestCounter::new();
        let mut guard = FinallyGuard::new(|| ran.inc());
        guard.execute();
        assert_eq!(ran.get(), 1);
    }

    #[test]
    fn test_finally_macro() {
        let _guard = crate::absl_cleanup::finally(|| {});
    }
}
