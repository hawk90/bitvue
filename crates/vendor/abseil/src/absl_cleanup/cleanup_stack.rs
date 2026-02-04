//! CleanupStack - LIFO stack of cleanup actions.

use alloc::boxed::Box;
use alloc::vec::Vec;
use core::cell::Cell;

/// A stack of cleanup actions that run in LIFO order when dropped.
///
/// This allows you to push multiple cleanup actions onto a stack
/// and have them all execute in reverse order when the stack is dropped.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_cleanup::CleanupStack;
///
/// let mut stack = CleanupStack::new();
/// stack.push(|| println!("Cleanup 1"));
/// stack.push(|| println!("Cleanup 2"));
/// stack.push(|| println!("Cleanup 3"));
/// drop(stack);
/// // Output:
/// // Cleanup 3
/// // Cleanup 2
/// // Cleanup 1
/// ```
pub struct CleanupStack {
    cleanups: Vec<Box<dyn FnOnce()>>,
    dismissed: Cell<bool>,
}

impl CleanupStack {
    /// Creates a new empty cleanup stack.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use abseil::absl_cleanup::CleanupStack;
    ///
    /// let stack = CleanupStack::new();
    /// ```
    pub const fn new() -> Self {
        Self {
            cleanups: Vec::new(),
            dismissed: Cell::new(false),
        }
    }

    /// Pushes a cleanup action onto the stack.
    ///
    /// Cleanup actions run in LIFO (last-in, first-out) order.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use abseil::absl_cleanup::CleanupStack;
    ///
    /// let mut stack = CleanupStack::new();
    /// stack.push(|| println!("First"));
    /// stack.push(|| println!("Second"));
    /// ```
    pub fn push<F: FnOnce() + 'static>(&mut self, f: F) {
        self.cleanups.push(Box::new(f));
    }

    /// Returns the number of cleanup actions on the stack.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use abseil::absl_cleanup::CleanupStack;
    ///
    /// let mut stack = CleanupStack::new();
    /// assert_eq!(stack.len(), 0);
    /// stack.push(|| {});
    /// assert_eq!(stack.len(), 1);
    /// ```
    pub fn len(&self) -> usize {
        self.cleanups.len()
    }

    /// Returns true if the stack has no cleanup actions.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use abseil::absl_cleanup::CleanupStack;
    ///
    /// let stack = CleanupStack::new();
    /// assert!(stack.is_empty());
    /// ```
    pub fn is_empty(&self) -> bool {
        self.cleanups.is_empty()
    }

    /// Dismisses all cleanup actions on the stack.
    ///
    /// After calling this, no cleanup actions will run when the stack is dropped.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use abseil::absl_cleanup::CleanupStack;
    ///
    /// let mut stack = CleanupStack::new();
    /// stack.push(|| println!("Won't run"));
    /// stack.dismiss();
    /// drop(stack); // Nothing prints
    /// ```
    pub fn dismiss(&mut self) {
        self.dismissed.set(true);
    }

    /// Returns true if the stack has been dismissed.
    pub fn is_dismissed(&self) -> bool {
        self.dismissed.get()
    }

    /// Executes all cleanup actions immediately and clears the stack.
    ///
    /// After calling this, the cleanup actions will not run again when dropped.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use abseil::absl_cleanup::CleanupStack;
    ///
    /// let mut stack = CleanupStack::new();
    /// stack.push(|| println!("Run now"));
    /// stack.execute(); // Prints "Run now"
    /// drop(stack); // Nothing prints
    /// ```
    pub fn execute(&mut self) {
        if self.dismissed.get() {
            return;
        }
        self.dismissed.set(true);
        while let Some(cleanup) = self.cleanups.pop() {
            cleanup();
        }
    }
}

impl Default for CleanupStack {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for CleanupStack {
    fn drop(&mut self) {
        if !self.dismissed.get() {
            while let Some(cleanup) = self.cleanups.pop() {
                cleanup();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cleanup_stack_push_and_len() {
        let mut stack = CleanupStack::new();
        assert_eq!(stack.len(), 0);
        assert!(stack.is_empty());

        stack.push(|| {});
        assert_eq!(stack.len(), 1);
        assert!(!stack.is_empty());

        stack.push(|| {});
        assert_eq!(stack.len(), 2);
    }

    #[test]
    fn test_cleanup_stack_dismiss() {
        let mut stack = CleanupStack::new();
        stack.push(|| panic!("Should not run"));
        stack.dismiss();
        assert!(stack.is_dismissed());
        drop(stack);
        // If we get here, dismissal worked
    }

    #[test]
    fn test_cleanup_stack_execute() {
        let mut stack = CleanupStack::new();
        stack.push(|| {});
        stack.push(|| {});
        stack.execute();
        assert!(stack.is_dismissed());
        assert_eq!(stack.len(), 0);
    }

    #[test]
    fn test_cleanup_stack_default() {
        let stack = CleanupStack::default();
        assert_eq!(stack.len(), 0);
    }

    #[test]
    fn test_cleanup_stack_execute_empty() {
        let mut stack = CleanupStack::new();
        stack.execute(); // Should not panic
        assert!(stack.is_dismissed());
    }
}
