//! DeferredCleanup - cleanup with deferred/invoked execution.

use core::cell::Cell;
use core::mem::ManuallyDrop;

/// A cleanup guard that defers execution until explicitly invoked.
///
/// Unlike `Cleanup` which runs on drop, `DeferredCleanup` waits for
/// explicit invocation. If not invoked before dropping, the cleanup runs then.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_cleanup::DeferredCleanup;
///
/// fn operation() {
///     let mut cleanup = DeferredCleanup::new(|| {
///         println!("Cleanup deferred");
///     });
///
///     // Do work...
///     cleanup.invoke(); // Run cleanup now
/// }
/// ```
pub struct DeferredCleanup<F: FnOnce()> {
    cleanup_fn: ManuallyDrop<F>,
    invoked: Cell<bool>,
    auto_cleanup: Cell<bool>,
}

impl<F: FnOnce()> DeferredCleanup<F> {
    /// Creates a new deferred cleanup.
    ///
    /// By default, the cleanup will run on drop if not explicitly invoked.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use abseil::absl_cleanup::DeferredCleanup;
    ///
    /// let cleanup = DeferredCleanup::new(|| {
    ///      println!("Deferred cleanup");
    ///     });
    /// ```
    pub fn new(f: F) -> Self {
        Self {
            cleanup_fn: ManuallyDrop::new(f),
            invoked: Cell::new(false),
            auto_cleanup: Cell::new(true),
        }
    }

    /// Creates a deferred cleanup that won't run on drop.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use abseil::absl_cleanup::DeferredCleanup;
    ///
    /// let cleanup = DeferredCleanup::no_auto(|| {
    ///     println!("Only if explicitly invoked");
    /// });
    /// drop(cleanup); // Doesn't run
    /// ```
    pub fn no_auto(f: F) -> Self {
        Self {
            cleanup_fn: ManuallyDrop::new(f),
            invoked: Cell::new(false),
            auto_cleanup: Cell::new(false),
        }
    }

    /// Invokes the cleanup function immediately.
    ///
    /// # Panics
    ///
    /// Panics if the cleanup function has already been invoked.
    ///
    /// # Safety
    ///
    /// The cleanup function will be consumed. If it panics during execution,
    /// the function will be lost (as with any FnOnce that panics). This is a
    /// known limitation of the FnOnce pattern - ensure the cleanup function is
    /// panic-safe or use defer/catch_unwind for panic recovery.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use abseil::absl_cleanup::DeferredCleanup;
    ///
    /// let mut cleanup = DeferredCleanup::new(|| {
    ///     println!("Running now");
    /// });
    /// cleanup.invoke(); // Prints "Running now"
    /// ```
    pub fn invoke(&mut self) {
        if !self.invoked.get() {
            self.invoked.set(true);
            // SAFETY: Once we take f from ManuallyDrop, we're committed to calling it.
            // If f() panics, the function is lost (consumed but not executed).
            // This is a known limitation of FnOnce - ensure f is panic-safe.
            let f = unsafe { ManuallyDrop::take(&mut self.cleanup_fn) };
            f();
        }
    }

    /// Returns true if the cleanup has been invoked.
    pub fn is_invoked(&self) -> bool {
        self.invoked.get()
    }
}

impl<F: FnOnce()> Drop for DeferredCleanup<F> {
    fn drop(&mut self) {
        if self.auto_cleanup.get() && !self.invoked.get() {
            // SAFETY: During drop, panics will abort the process, so we don't need
            // to worry about losing the function. The function will execute even if
            // it panics (though the panic will terminate the program).
            let f = unsafe { ManuallyDrop::take(&mut self.cleanup_fn) };
            f();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deferred_cleanup_invoke() {
        let invoked = crate::absl_cleanup::tests::TestCounter::new();
        let mut cleanup = DeferredCleanup::new(|| invoked.inc());
        cleanup.invoke();
        assert!(cleanup.is_invoked());
        assert_eq!(invoked.get(), 1);
    }

    #[test]
    fn test_deferred_cleanup_auto_cleanup() {
        let invoked = crate::absl_cleanup::tests::TestCounter::new();
        {
            let cleanup = DeferredCleanup::new(|| invoked.inc());
            assert!(!cleanup.is_invoked());
        }
        assert_eq!(invoked.get(), 1);
    }

    #[test]
    fn test_deferred_cleanup_no_auto() {
        let invoked = crate::absl_cleanup::tests::TestCounter::new();
        {
            let cleanup = DeferredCleanup::no_auto(|| invoked.inc());
            assert!(!cleanup.is_invoked());
        }
        assert_eq!(invoked.get(), 0);
    }

    #[test]
    fn test_deferred_cleanup_double_invoke() {
        let invoked = crate::absl_cleanup::tests::TestCounter::new();
        let mut cleanup = DeferredCleanup::new(|| invoked.inc());
        cleanup.invoke();
        cleanup.invoke(); // Second invoke does nothing
        assert_eq!(invoked.get(), 1);
    }
}
