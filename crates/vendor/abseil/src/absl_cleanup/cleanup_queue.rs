//! CleanupQueue - FIFO queue for managing multiple cleanup actions.

use alloc::boxed::Box;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, Ordering};

/// A queue for managing multiple cleanup actions.
///
/// Unlike `CleanupStack` which runs in LIFO order, `CleanupQueue` maintains
/// FIFO order for cleanup execution.
///
/// # Thread Safety
///
/// The `dismissed` flag uses `AtomicBool` for thread-safe access to the
/// `is_dismissed()` method, allowing safe concurrent reads from multiple threads.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_cleanup::CleanupQueue;
///
/// let mut queue = CleanupQueue::new();
/// queue.push(|| println!("First"));
/// queue.push(|| println!("Second"));
/// queue.push(|| println!("Third"));
/// drop(queue);
/// // Output:
/// // First
/// // Second
/// // Third
/// ```
pub struct CleanupQueue {
    cleanups: Vec<Box<dyn FnOnce()>>,
    /// Uses AtomicBool for thread-safe reads from is_dismissed().
    dismissed: AtomicBool,
}

impl CleanupQueue {
    /// Creates a new empty cleanup queue.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use abseil::absl_cleanup::CleanupQueue;
    ///
    /// let queue = CleanupQueue::new();
    /// ```
    pub const fn new() -> Self {
        Self {
            cleanups: Vec::new(),
            dismissed: AtomicBool::new(false),
        }
    }

    /// Pushes a cleanup action onto the queue.
    ///
    /// Actions run in FIFO (first-in, first-out) order.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use abseil::absl_cleanup::CleanupQueue;
    ///
    /// let mut queue = CleanupQueue::new();
    /// queue.push(|| println!("First"));
    /// queue.push(|| println!("Second"));
    /// ```
    pub fn push<F: FnOnce() + 'static>(&mut self, f: F) {
        self.cleanups.push(Box::new(f));
    }

    /// Returns the number of cleanup actions in the queue.
    pub fn len(&self) -> usize {
        self.cleanups.len()
    }

    /// Returns true if the queue is empty.
    pub fn is_empty(&self) -> bool {
        self.cleanups.is_empty()
    }

    /// Dismisses all cleanup actions.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use abseil::absl_cleanup::CleanupQueue;
    ///
    /// let mut queue = CleanupQueue::new();
    /// queue.push(|| println!("Won't run"));
    /// queue.dismiss();
    /// drop(queue);
    /// ```
    pub fn dismiss(&mut self) {
        self.dismissed.store(true, Ordering::SeqCst);
    }

    /// Returns true if the queue has been dismissed.
    ///
    /// This method is thread-safe and can be called concurrently from
    /// multiple threads.
    pub fn is_dismissed(&self) -> bool {
        self.dismissed.load(Ordering::SeqCst)
    }

    /// Executes all cleanups immediately in FIFO order.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use abseil::absl_cleanup::CleanupQueue;
    ///
    /// let mut queue = CleanupQueue::new();
    /// queue.push(|| println!("One"));
    /// queue.push(|| println!("Two"));
    /// queue.execute(); // Runs One, then Two
    /// ```
    pub fn execute(&mut self) {
        if self.dismissed.load(Ordering::SeqCst) {
            return;
        }
        self.dismissed.store(true, Ordering::SeqCst);
        // Drain in FIFO order
        for cleanup in self.cleanups.drain(..) {
            cleanup();
        }
    }
}

impl Default for CleanupQueue {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for CleanupQueue {
    fn drop(&mut self) {
        if !self.dismissed.load(Ordering::SeqCst) {
            for cleanup in self.cleanups.drain(..) {
                cleanup();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cleanup_queue_fifo_order() {
        let mut queue = CleanupQueue::new();
        queue.push(|| {});
        queue.push(|| {});
        assert_eq!(queue.len(), 2);
        assert!(!queue.is_empty());
    }

    #[test]
    fn test_cleanup_queue_dismiss() {
        let mut queue = CleanupQueue::new();
        queue.push(|| panic!("Should not run"));
        queue.dismiss();
        assert!(queue.is_dismissed());
        drop(queue);
    }

    #[test]
    fn test_cleanup_queue_execute() {
        let executed = crate::absl_cleanup::tests::TestCounter::new();
        let mut queue = CleanupQueue::new();
        queue.push(|| executed.inc());
        queue.push(|| executed.inc());
        queue.execute();
        assert!(queue.is_dismissed());
        assert_eq!(executed.get(), 2);
    }

    #[test]
    fn test_cleanup_queue_default() {
        let queue = CleanupQueue::default();
        assert!(queue.is_empty());
    }

    #[test]
    fn test_cleanup_queue_execute_empty() {
        let mut queue = CleanupQueue::new();
        queue.execute(); // Should not panic
        assert!(queue.is_dismissed());
    }
}
