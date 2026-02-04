//! Convenience macros for creating cleanup guards.

/// Convenience function to create a cleanup stack.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_cleanup::cleanup_stack;
///
/// let mut stack = cleanup_stack!();
/// stack.push(|| println!("Cleanup 1"));
/// ```
#[macro_export]
macro_rules! cleanup_stack {
    () => {
        $crate::absl_cleanup::CleanupStack::new()
    };
    ($($cleanup:expr),+ $(,)?) => {{
        let mut stack = $crate::absl_cleanup::CleanupStack::new();
        $(
            stack.push($cleanup);
        )+
        stack
    }};
}

/// Convenience function to create a cleanup queue.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_cleanup::cleanup_queue;
///
/// let mut queue = cleanup_queue!();
/// queue.push(|| println!("Cleanup 1"));
/// ```
#[macro_export]
macro_rules! cleanup_queue {
    () => {
        $crate::absl_cleanup::CleanupQueue::new()
    };
    ($($cleanup:expr),+ $(,)?) => {{
        let mut queue = $crate::absl_cleanup::CleanupQueue::new();
        $(
            queue.push($cleanup);
        )+
        queue
    }};
}

/// Creates a finally guard.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_cleanup::finally;
///
/// let _guard = finally(|| {
///     println!("This always runs");
/// });
/// ```
#[macro_export]
macro_rules! finally {
    ($cleanup:expr) => {
        $crate::absl_cleanup::FinallyGuard::new($cleanup)
    };
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_cleanup_stack_macro() {
        let _stack = cleanup_stack!();
    }

    #[test]
    fn test_cleanup_stack_macro_with_actions() {
        let _stack = cleanup_stack!(
            || {},
            || {},
        );
    }

    #[test]
    fn test_cleanup_queue_macro() {
        let _queue = cleanup_queue!();
    }

    #[test]
    fn test_cleanup_queue_macro_with_actions() {
        let _queue = cleanup_queue!(
            || {},
            || {},
        );
    }

    #[test]
    fn test_finally_macro() {
        let _guard = finally(|| {});
    }
}
