//! RollbackGuard - transaction-style rollback on failure.

use core::cell::Cell;
use core::mem::ManuallyDrop;

/// A rollback guard for transaction-style operations.
///
/// This guard runs its cleanup function only if it's not explicitly committed,
/// making it ideal for rolling back transactions on failure.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_cleanup::RollbackGuard;
///
/// fn transaction() -> Result<(), String> {
///     let mut rollback = RollbackGuard::new(|| {
///         println!("Rolling back transaction");
///     });
///
///     // Do work...
///     if error_occurred() {
///         return Err("error".to_string()); // rollback runs automatically
///     }
///
///     rollback.commit(); // Success - prevent rollback
///     Ok(())
/// }
/// # fn error_occurred() -> bool { false }
/// ```
pub struct RollbackGuard<F: FnOnce()> {
    cleanup_fn: ManuallyDrop<F>,
    committed: Cell<bool>,
}

impl<F: FnOnce()> RollbackGuard<F> {
    /// Creates a new rollback guard.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use abseil::absl_cleanup::RollbackGuard;
    ///
    /// let guard = RollbackGuard::new(|| {
    ///     println!("Rolling back!");
    /// });
    /// ```
    pub fn new(f: F) -> Self {
        Self {
            cleanup_fn: ManuallyDrop::new(f),
            committed: Cell::new(false),
        }
    }

    /// Commits the operation, preventing rollback.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use abseil::absl_cleanup::RollbackGuard;
    ///
    /// let mut guard = RollbackGuard::new(|| {});
    /// guard.commit(); // Success - no rollback
    /// ```
    pub fn commit(&mut self) {
        self.committed.set(true);
    }

    /// Returns true if this guard has been committed.
    pub fn is_committed(&self) -> bool {
        self.committed.get()
    }

    /// Forces the rollback to execute immediately.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use abseil::absl_cleanup::RollbackGuard;
    ///
    /// let mut guard = RollbackGuard::new(|| {
    ///     println!("Explicit rollback");
    /// });
    /// guard.rollback(); // Runs rollback now
    /// ```
    pub fn rollback(&mut self) {
        if !self.committed.get() {
            self.committed.set(true);
            let f = unsafe { ManuallyDrop::take(&mut self.cleanup_fn) };
            f();
        }
    }
}

impl<F: FnOnce()> Drop for RollbackGuard<F> {
    fn drop(&mut self) {
        if !self.committed.get() {
            let f = unsafe { ManuallyDrop::take(&mut self.cleanup_fn) };
            f();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rollback_guard_rollback_on_drop() {
        let rolled_back = crate::absl_cleanup::tests::TestCounter::new();
        {
            let _guard = RollbackGuard::new(|| rolled_back.inc());
        }
        assert_eq!(rolled_back.get(), 1);
    }

    #[test]
    fn test_rollback_guard_commit() {
        let rolled_back = crate::absl_cleanup::tests::TestCounter::new();
        {
            let mut guard = RollbackGuard::new(|| rolled_back.inc());
            guard.commit();
            assert!(guard.is_committed());
        }
        assert_eq!(rolled_back.get(), 0);
    }

    #[test]
    fn test_rollback_guard_early_rollback() {
        let rolled_back = crate::absl_cleanup::tests::TestCounter::new();
        let mut guard = RollbackGuard::new(|| rolled_back.inc());
        guard.rollback();
        assert_eq!(rolled_back.get(), 1);
        assert!(guard.is_committed());
    }

    #[test]
    fn test_rollback_guard_in_result() {
        fn operation() -> Result<(), &'static str> {
            let _guard = RollbackGuard::new(|| {
                // Rollback logic here
            });
            Err("error")
        }

        assert!(operation().is_err());
    }
}
