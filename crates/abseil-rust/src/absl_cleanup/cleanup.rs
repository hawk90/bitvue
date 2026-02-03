//! Cleanup/ScopeGuard utilities.
//!
//! This module provides a `Cleanup` type (similar to Abseil's `absl::Cleanup`)
//! which executes a cleanup function when it goes out of scope.
//!
//! # Thread Safety
//!
//! The `dismissed` flag uses `AtomicBool` for thread-safe access to the
//! `is_dismissed()` method, allowing safe concurrent reads from multiple threads.
//!
//! # Examples
//!
//! ```rust
//! use abseil::Cleanup;
//!
//! fn do_something() -> Result<(), String> {
//!     let resource = acquire_resource();
//!     let _cleanup = Cleanup::new(|| {
//!         release_resource(resource);
//!     });
//!
//!     // Do work with resource...
//!     Ok(()) // cleanup runs automatically here
//! }
//!
//! # fn acquire_resource() -> i32 { 42 }
//! # fn release_resource(_: i32) {}
//! ```

use core::mem::ManuallyDrop;
use core::ops::{Deref, DerefMut};
use core::sync::atomic::{AtomicBool, Ordering};

/// A guard that runs a cleanup function when dropped.
///
/// This is similar to Go's `defer` statement or C++'s `ScopeGuard`.
/// The cleanup function is called when the `Cleanup` guard is dropped,
/// unless it has been explicitly dismissed or released.
///
/// # Thread Safety
///
/// The `dismissed` flag uses `AtomicBool` for safe concurrent access.
/// Multiple threads can safely call `is_dismissed()` concurrently.
///
/// # Examples
///
/// ```rust
/// use abseil::Cleanup;
///
/// {
///     let mut cleaned = false;
///     {
///         let _cleanup = Cleanup::new(|| {
///             cleaned = true;
///         });
///         // Do work...
///     } // cleanup runs here, setting cleaned to true
///     assert!(cleaned);
/// }
/// ```
pub struct Cleanup<F: FnOnce()> {
    /// The cleanup function to execute.
    f: ManuallyDrop<F>,
    /// Whether this cleanup has been dismissed.
    /// Uses AtomicBool for thread-safe reads from is_dismissed().
    dismissed: AtomicBool,
}

impl<F: FnOnce()> Cleanup<F> {
    /// Creates a new `Cleanup` guard with the given cleanup function.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use abseil::Cleanup;
    ///
    /// let cleanup = Cleanup::new(|| {
    ///     println!("Cleaning up!");
    /// });
    /// ```
    pub fn new(f: F) -> Self {
        Self {
            f: ManuallyDrop::new(f),
            dismissed: AtomicBool::new(false),
        }
    }

    /// Creates a new `Cleanup` guard from an already-existing cleanup function.
    ///
    /// This is an alias for `new()` provided for compatibility.
    pub fn make(f: F) -> Self {
        Self::new(f)
    }

    /// Dismisses the cleanup, preventing it from running.
    ///
    /// Once dismissed, the cleanup function will not be called when
    /// the guard is dropped.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use abseil::Cleanup;
    ///
    /// let mut cleaned = false;
    /// {
    ///     let mut cleanup = Cleanup::new(|| {
    ///         cleaned = true;
    ///     });
    ///     cleanup.dismiss();
    /// } // cleanup does NOT run
    /// assert!(!cleaned);
    /// ```
    pub fn dismiss(&mut self) {
        self.dismissed.store(true, Ordering::SeqCst);
    }

    /// Returns true if this cleanup has been dismissed.
    ///
    /// This method is thread-safe and can be called concurrently from
    /// multiple threads.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use abseil::Cleanup;
    ///
    /// let cleanup = Cleanup::new(|| {});
    /// assert!(!cleanup.is_dismissed());
    /// ```
    pub fn is_dismissed(&self) -> bool {
        self.dismissed.load(Ordering::SeqCst)
    }

    /// Releases and returns the cleanup function without running it.
    ///
    /// This consumes the `Cleanup` guard and returns the original function,
    /// allowing you to call it manually or store it elsewhere.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use abseil::Cleanup;
    ///
    /// let cleanup = Cleanup::new(|| println!("Cleaned!"));
    /// let f = cleanup.release();
    /// f(); // Call manually
    /// ```
    pub fn release(mut self) -> F {
        self.dismissed.store(true, Ordering::SeqCst);
        // SAFETY: We've marked this as dismissed, so Drop won't run.
        // We can now extract the function using ManuallyDrop.
        unsafe { ManuallyDrop::take(&mut self.f) }
    }

    /// Executes the cleanup function early and dismisses the guard.
    ///
    /// After calling this, the cleanup function will not run again when
    /// the guard is dropped.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use abseil::Cleanup;
    ///
    /// let mut cleanup = Cleanup::new(|| {
    ///     println!("Cleanup executed!");
    /// });
    /// cleanup.invoke(); // Runs cleanup now
    /// // cleanup will NOT run again when dropped
    /// ```
    pub fn invoke(&mut self) {
        if !self.dismissed.load(Ordering::SeqCst) {
            self.dismissed.store(true, Ordering::SeqCst);
            // SAFETY: We're about to consume the function and dismiss the guard.
            let f = unsafe { ManuallyDrop::take(&mut self.f) };
            f();
        }
    }

    /// Executes the cleanup function early and dismisses the guard.
    ///
    /// This is an alias for `invoke()` provided for compatibility.
    pub fn invoke_and_dismiss(&mut self) {
        self.invoke();
    }
}

impl<F: FnOnce()> Drop for Cleanup<F> {
    fn drop(&mut self) {
        if !self.dismissed.load(Ordering::SeqCst) {
            // SAFETY: We're in drop, and we haven't been dismissed.
            // We can now consume and run the function.
            let f = unsafe { ManuallyDrop::take(&mut self.f) };
            f();
        }
    }
}

/// A cleanup guard that only runs on failure (when an error occurs).
///
/// This guard does NOT run its cleanup function if:
/// - It is explicitly dismissed
/// - It is explicitly committed (success case)
/// - It is dropped without error (needs explicit commit)
///
/// # Thread Safety
///
/// The `committed` flag uses `AtomicBool` for safe concurrent access.
/// Multiple threads can safely call `is_committed()` concurrently.
///
/// # Examples
///
/// ```rust
/// use abseil::FailureCleanup;
///
/// fn operation() -> Result<(), String> {
///     let mut cleanup = FailureCleanup::new(|| {
///         println!("Rolling back due to error");
///     });
///
///     // Do work...
///     if some_error_condition() {
///         return Err("error".to_string()); // cleanup runs here
///     }
///
///     cleanup.commit(); // Mark as successful - cleanup won't run
///     Ok(())
/// }
/// # fn some_error_condition() -> bool { false }
/// ```
pub struct FailureCleanup<F: FnOnce()> {
    /// The cleanup function.
    f: ManuallyDrop<F>,
    /// Whether this cleanup has been committed (success case).
    /// Uses AtomicBool for thread-safe reads from is_committed().
    committed: AtomicBool,
}

impl<F: FnOnce()> FailureCleanup<F> {
    /// Creates a new `FailureCleanup` guard.
    ///
    /// ```rust
    /// use abseil::FailureCleanup;
    ///
    /// let cleanup = FailureCleanup::new(|| {
    ///     println!("Rolling back!");
    /// });
    /// ```
    pub fn new(f: F) -> Self {
        Self {
            f: ManuallyDrop::new(f),
            committed: AtomicBool::new(false),
        }
    }

    /// Marks the operation as successful, preventing cleanup.
    ///
    /// Call this when the operation completes successfully.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use abseil::FailureCleanup;
    ///
    /// let mut cleanup = FailureCleanup::new(|| {
    ///     println!("This won't run");
    /// });
    /// cleanup.commit(); // Success - no cleanup
    /// ```
    pub fn commit(&mut self) {
        self.committed.store(true, Ordering::SeqCst);
    }

    /// Returns true if this cleanup has been committed (successful).
    ///
    /// This method is thread-safe and can be called concurrently from
    /// multiple threads.
    pub fn is_committed(&self) -> bool {
        self.committed.load(Ordering::SeqCst)
    }

    /// Releases and returns the cleanup function.
    pub fn release(mut self) -> F {
        self.committed.store(true, Ordering::SeqCst);
        unsafe { ManuallyDrop::take(&mut self.f) }
    }
}

impl<F: FnOnce()> Drop for FailureCleanup<F> {
    fn drop(&mut self) {
        // Only run cleanup if NOT committed (i.e., on failure/early return)
        if !self.committed.load(Ordering::SeqCst) {
            let f = unsafe { ManuallyDrop::take(&mut self.f) };
            f();
        }
    }
}

/// Creates a cleanup guard from a closure.
///
/// This is a convenience function for creating `Cleanup` guards.
///
/// # Examples
///
/// ```rust
/// use abseil::cleanup;
///
/// let _guard = cleanup(|| {
///     println!("Cleaning up!");
/// });
/// ```
pub fn cleanup<F: FnOnce()>(f: F) -> Cleanup<F> {
    Cleanup::new(f)
}

/// Creates a failure-only cleanup guard from a closure.
///
/// This is a convenience function for creating `FailureCleanup` guards.
///
/// # Examples
///
/// ```rust
/// use abseil::failure_cleanup;
///
/// let mut guard = failure_cleanup(|| {
///     println!("Rolling back!");
/// });
///
/// // On success:
/// guard.commit();
/// ```
pub fn failure_cleanup<F: FnOnce()>(f: F) -> FailureCleanup<F> {
    FailureCleanup::new(f)
}

// Implement Deref for accessing wrapped cleanup if needed
impl<F: FnOnce()> Deref for Cleanup<F> {
    type Target = F;

    fn deref(&self) -> &Self::Target {
        &self.f
    }
}

impl<F: FnOnce()> DerefMut for Cleanup<F> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.f
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
    use std::sync::Arc;

    #[test]
    fn test_cleanup_runs_on_drop() {
        let cleaned = Arc::new(AtomicBool::new(false));
        {
            let cleaned_clone = cleaned.clone();
            let _cleanup = Cleanup::new(|| {
                cleaned_clone.store(true, Ordering::SeqCst);
            });
            assert!(!cleaned.load(Ordering::SeqCst));
        }
        assert!(cleaned.load(Ordering::SeqCst));
    }

    #[test]
    fn test_cleanup_dismiss() {
        let cleaned = Arc::new(AtomicBool::new(false));
        {
            let cleaned_clone = cleaned.clone();
            let mut cleanup = Cleanup::new(|| {
                cleaned_clone.store(true, Ordering::SeqCst);
            });
            cleanup.dismiss();
            assert!(cleanup.is_dismissed());
        }
        assert!(!cleaned.load(Ordering::SeqCst));
    }

    #[test]
    fn test_cleanup_invoke() {
        let count = Arc::new(AtomicUsize::new(0));
        {
            let count_clone = count.clone();
            let mut cleanup = Cleanup::new(|| {
                count_clone.fetch_add(1, Ordering::SeqCst);
            });
            cleanup.invoke();
            assert_eq!(count.load(Ordering::SeqCst), 1);
        } // does not run again
        assert_eq!(count.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn test_cleanup_invoke_and_dismiss() {
        let count = Arc::new(AtomicUsize::new(0));
        {
            let count_clone = count.clone();
            let mut cleanup = Cleanup::new(|| {
                count_clone.fetch_add(1, Ordering::SeqCst);
            });
            cleanup.invoke_and_dismiss();
            assert_eq!(count.load(Ordering::SeqCst), 1);
        }
        assert_eq!(count.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn test_cleanup_release() {
        let count = Arc::new(AtomicUsize::new(0));
        let count_clone = count.clone();
        let cleanup = Cleanup::new(|| {
            count_clone.fetch_add(1, Ordering::SeqCst);
        });
        let f = cleanup.release();
        assert_eq!(count.load(Ordering::SeqCst), 0);
        f();
        assert_eq!(count.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn test_cleanup_make() {
        let cleaned = Arc::new(AtomicBool::new(false));
        {
            let cleaned_clone = cleaned.clone();
            let _cleanup = Cleanup::make(|| {
                cleaned_clone.store(true, Ordering::SeqCst);
            });
        }
        assert!(cleaned.load(Ordering::SeqCst));
    }

    #[test]
    fn test_failure_cleanup_on_drop() {
        let cleaned = Arc::new(AtomicBool::new(false));
        {
            let cleaned_clone = cleaned.clone();
            let _cleanup = FailureCleanup::new(|| {
                cleaned_clone.store(true, Ordering::SeqCst);
            });
        } // Runs on drop (not committed)
        assert!(cleaned.load(Ordering::SeqCst));
    }

    #[test]
    fn test_failure_cleanup_commit() {
        let cleaned = Arc::new(AtomicBool::new(false));
        {
            let cleaned_clone = cleaned.clone();
            let mut cleanup = FailureCleanup::new(|| {
                cleaned_clone.store(true, Ordering::SeqCst);
            });
            cleanup.commit();
            assert!(cleanup.is_committed());
        } // Does NOT run (committed)
        assert!(!cleaned.load(Ordering::SeqCst));
    }

    #[test]
    fn test_failure_cleanup_release() {
        let count = Arc::new(AtomicUsize::new(0));
        let count_clone = count.clone();
        let cleanup = FailureCleanup::new(|| {
            count_clone.fetch_add(1, Ordering::SeqCst);
        });
        let f = cleanup.release();
        assert_eq!(count.load(Ordering::SeqCst), 0);
        f();
        assert_eq!(count.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn test_cleanup_function() {
        let cleaned = Arc::new(AtomicBool::new(false));
        {
            let cleaned_clone = cleaned.clone();
            let _guard = cleanup(|| {
                cleaned_clone.store(true, Ordering::SeqCst);
            });
        }
        assert!(cleaned.load(Ordering::SeqCst));
    }

    #[test]
    fn test_failure_cleanup_function() {
        let cleaned = Arc::new(AtomicBool::new(false));
        {
            let cleaned_clone = cleaned.clone();
            let mut guard = failure_cleanup(|| {
                cleaned_clone.store(true, Ordering::SeqCst);
            });
            guard.commit();
        }
        assert!(!cleaned.load(Ordering::SeqCst));
    }

    #[test]
    fn test_cleanup_with_captured_vars() {
        let vec = Arc::new(std::sync::Mutex::new(vec![1, 2, 3]));
        {
            let vec_clone = vec.clone();
            let _cleanup = Cleanup::new(|| {
                vec_clone.lock().unwrap().push(4);
            });
            assert_eq!(*vec.lock().unwrap(), vec![1, 2, 3]);
        }
        assert_eq!(*vec.lock().unwrap(), vec![1, 2, 3, 4]);
    }

    #[test]
    fn test_multiple_cleanups_run_in_order() {
        let order = Arc::new(std::sync::Mutex::new(Vec::new()));
        {
            let order_clone1 = order.clone();
            let order_clone2 = order.clone();
            let order_clone3 = order.clone();
            let _c1 = Cleanup::new(|| {
                order_clone1.lock().unwrap().push(1);
            });
            let _c2 = Cleanup::new(|| {
                order_clone2.lock().unwrap().push(2);
            });
            let _c3 = Cleanup::new(|| {
                order_clone3.lock().unwrap().push(3);
            });
        }
        // Cleanups run in reverse order (LIFO)
        assert_eq!(*order.lock().unwrap(), vec![3, 2, 1]);
    }

    #[test]
    fn test_cleanup_with_result() {
        fn do_work() -> Result<i32, String> {
            let _cleanup = Cleanup::new(|| {
                // Cleanup runs regardless of success/failure
            });
            Ok(42)
        }
        assert_eq!(do_work(), Ok(42));
    }

    #[test]
    fn test_failure_cleanup_with_error() {
        fn do_work() -> Result<i32, String> {
            let mut cleanup = FailureCleanup::new(|| {
                // Rollback on error
            });
            if true {
                return Err("error".to_string());
            }
            cleanup.commit();
            Ok(42)
        }
        assert_eq!(do_work(), Err("error".to_string()));
    }

    #[test]
    fn test_failure_cleanup_with_success() {
        let cleaned = Arc::new(AtomicBool::new(false));
        fn do_work() -> Result<i32, String> {
            let mut cleanup = FailureCleanup::new(|| {
                // Rollback on error
            });
            cleanup.commit();
            Ok(42)
        }
        {
            let cleaned_clone = cleaned.clone();
            let mut cleanup = FailureCleanup::new(|| {
                cleaned_clone.store(true, Ordering::SeqCst);
            });
            cleanup.commit();
        }
        assert!(!cleaned.load(Ordering::SeqCst));
        assert_eq!(do_work(), Ok(42));
    }

    #[test]
    fn test_cleanup_panic_in_cleanup() {
        // Cleanup should run even if panic occurs (though test can't easily verify this)
        let result = std::panic::catch_unwind(|| {
            let _cleanup = Cleanup::new(|| {
                panic!("cleanup panic");
            });
        });
        assert!(result.is_err());
    }

    #[test]
    fn test_nested_cleanups() {
        let outer = Arc::new(AtomicUsize::new(0));
        let inner = Arc::new(AtomicUsize::new(0));
        {
            let outer_clone = outer.clone();
            let _c1 = Cleanup::new(|| {
                outer_clone.fetch_add(1, Ordering::SeqCst);
            });
            {
                let inner_clone = inner.clone();
                let _c2 = Cleanup::new(|| {
                    inner_clone.fetch_add(1, Ordering::SeqCst);
                });
            }
            assert_eq!(inner.load(Ordering::SeqCst), 1);
            assert_eq!(outer.load(Ordering::SeqCst), 0);
        }
        assert_eq!(inner.load(Ordering::SeqCst), 1);
        assert_eq!(outer.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn test_cleanup_default_not_called() {
        let cleaned = Arc::new(AtomicBool::new(false));
        {
            let cleaned_clone = cleaned.clone();
            let mut cleanup = Cleanup::new(|| {
                cleaned_clone.store(true, Ordering::SeqCst);
            });
            cleanup.dismiss();
        }
        assert!(!cleaned.load(Ordering::SeqCst));
    }

    #[test]
    fn test_cleanup_invoke_twice() {
        let count = Arc::new(AtomicUsize::new(0));
        {
            let count_clone = count.clone();
            let mut cleanup = Cleanup::new(|| {
                count_clone.fetch_add(1, Ordering::SeqCst);
            });
            cleanup.invoke();
            cleanup.invoke(); // Second invoke should do nothing
        }
        assert_eq!(count.load(Ordering::SeqCst), 1);
    }
}
