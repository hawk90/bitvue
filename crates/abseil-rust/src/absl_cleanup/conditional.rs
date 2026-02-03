//! ConditionalCleanup - cleanup that runs based on a condition.

use core::cell::Cell;
use core::mem::ManuallyDrop;

/// A cleanup guard that runs based on a condition.
///
/// The cleanup function receives a boolean indicating whether the condition
/// was met when the guard was dropped.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_cleanup::ConditionalCleanup;
///
/// fn operation() {
///     let mut success = false;
///     let cleanup = ConditionalCleanup::new(|success_flag| {
///         if !success_flag {
///             println!("Operation failed, rolling back");
///         }
///     });
///
///     // Do work...
///     success = true;
///     cleanup.update_condition(success);
/// }
/// ```
pub struct ConditionalCleanup<F: FnOnce(bool)> {
    cleanup_fn: ManuallyDrop<F>,
    condition: Cell<bool>,
}

impl<F: FnOnce(bool)> ConditionalCleanup<F> {
    /// Creates a new conditional cleanup with an initial condition value.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use abseil::absl_cleanup::ConditionalCleanup;
    ///
    /// let cleanup = ConditionalCleanup::new(false, |success| {
    ///     println!("Operation success: {}", success);
    /// });
    /// ```
    pub fn new(initial_condition: bool, f: F) -> Self {
        Self {
            cleanup_fn: ManuallyDrop::new(f),
            condition: Cell::new(initial_condition),
        }
    }

    /// Updates the condition value.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use abseil::absl_cleanup::ConditionalCleanup;
    ///
    /// let mut cleanup = ConditionalCleanup::new(false, |s| println!("{}", s));
    /// cleanup.update_condition(true); // Mark as success
    /// ```
    pub fn update_condition(&mut self, condition: bool) {
        self.condition.set(condition);
    }

    /// Returns the current condition value.
    pub fn condition(&self) -> bool {
        self.condition.get()
    }
}

impl<F: FnOnce(bool)> Drop for ConditionalCleanup<F> {
    fn drop(&mut self) {
        let condition = self.condition.get();
        let f = unsafe { ManuallyDrop::take(&mut self.cleanup_fn) };
        f(condition);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_conditional_cleanup_condition() {
        let received = alloc::sync::Arc::new(core::sync::atomic::AtomicBool::new(false));
        let received_clone = received.clone();

        let cleanup = ConditionalCleanup::new(false, |success| {
            received_clone.store(success, core::sync::atomic::Ordering::SeqCst);
        });
        assert!(!cleanup.condition());

        drop(cleanup);
        assert!(received.load(core::sync::atomic::Ordering::SeqCst));
    }

    #[test]
    fn test_conditional_cleanup_update() {
        let cleanup = ConditionalCleanup::new(false, |_| {});
        assert!(!cleanup.condition());
        cleanup.update_condition(true);
        assert!(cleanup.condition());
    }
}
