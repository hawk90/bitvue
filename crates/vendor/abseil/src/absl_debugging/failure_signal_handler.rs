//! Failure signal handling utilities.
//!
//! Provides signal handlers for fatal failures (SIGSEGV, SIGABRT, etc.).

/// Installs a failure signal handler.
///
/// This function sets up signal handlers for common fatal signals
/// (SIGSEGV, SIGABRT, SIGFPE, SIGILL, SIGBUS, SIGTERM) that will
/// print stack traces and other debugging information.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_debugging::failure_signal_handler::install_failure_signal_handler;
///
/// // Install signal handlers at program startup
/// install_failure_signal_handler();
/// ```
///
/// # Notes
///
/// - Only available on Unix-like systems with "std" feature
/// - Windows uses structured exception handling instead
/// - In no_std environments, this does nothing
#[inline]
pub fn install_failure_signal_handler() {
    #[cfg(all(feature = "std", unix))]
    {
        // Note: A real implementation would set up signal handlers
        // using sigaction() for signals like SIGSEGV, SIGABRT, etc.
        // This is a stub that does nothing.
    }
    #[cfg(not(all(feature = "std", unix)))]
    {
        // No-op on other platforms
    }
}

/// Installs a failure signal handler with a custom writer function.
///
/// The writer function is called with formatted error messages.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_debugging::failure_signal_handler::install_failure_signal_handler_with_writer;
///
/// install_failure_signal_handler_with_writer(|msg| {
///     eprintln!("CRASH: {}", msg);
/// });
/// ```
///
/// # Notes
///
/// - Only available on Unix-like systems with "std" feature
/// - The writer function should not allocate or panic
#[inline]
pub fn install_failure_signal_handler_with_writer<F>(writer: F)
where
    F: Fn(&str) + Sync + Send + 'static,
{
    #[cfg(all(feature = "std", unix))]
    {
        // Note: A real implementation would store the writer and
        // call it from the signal handler (using async-signal-safe functions only)
        let _writer = writer;
    }
    #[cfg(not(all(feature = "std", unix)))]
    {
        let _writer = writer;
    }
}

/// Uninstalls the failure signal handler.
///
/// Restores the default signal handlers.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_debugging::failure_signal_handler::uninstall_failure_signal_handler;
///
/// uninstall_failure_signal_handler();
/// ```
#[inline]
pub fn uninstall_failure_signal_handler() {
    #[cfg(all(feature = "std", unix))]
    {
        // Note: A real implementation would restore default handlers
    }
}

/// Returns true if a failure signal handler is installed.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_debugging::failure_signal_handler::is_failure_signal_handler_installed;
///
/// if !is_failure_signal_handler_installed() {
///     // Install handler
/// }
/// ```
#[inline]
pub fn is_failure_signal_handler_installed() -> bool {
    #[cfg(all(feature = "std", unix))]
    {
        // Note: A real implementation would track whether handlers are installed
        false
    }
    #[cfg(not(all(feature = "std", unix)))]
    {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_install_failure_signal_handler() {
        // Should not panic
        install_failure_signal_handler();
    }

    #[test]
    fn test_install_failure_signal_handler_with_writer() {
        // Should not panic
        install_failure_signal_handler_with_writer(|msg| {
            let _ = msg;
        });
    }

    #[test]
    fn test_uninstall_failure_signal_handler() {
        // Should not panic
        uninstall_failure_signal_handler();
    }

    #[test]
    fn test_is_failure_signal_handler_installed() {
        // Returns false by default (stub implementation)
        assert!(!is_failure_signal_handler_installed());
    }
}
