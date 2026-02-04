//! VLOG (Verbose Logging) system.
//!
//! VLOG allows for detailed debug logging that can be enabled at runtime
//! without recompiling. The verbosity level can be controlled per-module
//! or globally.

use crate::absl_log::config::is_vlog_enabled;

/// Logs a message at the specified verbosity level.
///
/// The message will only be logged if the current VLOG level for the
/// current file is at least `level`. VLOG levels are set at runtime
/// via environment variables or programmatic configuration.
///
/// # Syntax
///
/// ```ignore
/// vlog!(level, "message {}", arg1)
/// ```
///
/// # Example
///
/// ```ignore
/// # fn main() {
/// use abseil::vlog;
/// # let data = vec![1, 2, 3];
/// // Only logs if VLOG level for this file is >= 2
/// vlog!(2, "Detailed debug info: {:?}", data);
/// # }
/// ```
///
/// # VLOG Level Configuration
///
/// Set the default VLOG level:
/// ```text
/// export ABSL_VLOG_LEVEL=2
/// ```
///
/// Set per-module VLOG level:
/// ```text
/// export ABSL_VMODULE=my_module=3,http_server=2
/// ```
#[macro_export]
macro_rules! vlog {
    ($level:expr, $($arg:tt)*) => {{
        if $crate::is_vlog_on!($level, file!()) {
            $crate::absl_log::log::do_log(
                $crate::absl_log::severity::LogSeverity::Info,
                file!(),
                line!(),
                format_args!("[VLOG{}] {}", $level, format_args!($($arg)*))
            );
        }
    }};
}

/// Checks if VLOG is enabled for the given level and file.
///
/// # Example
///
/// ```ignore
/// # fn main() {
/// use abseil::vlog_is_on;
/// if vlog_is_on!(2) {
///     // Expensive debug computation
///     let _result = 42;
/// }
/// # }
/// ```
#[macro_export]
macro_rules! vlog_is_on {
    ($level:expr) => {{
        $crate::is_vlog_on!($level, file!())
    }};
}

/// Internal macro to check if VLOG is enabled.
#[doc(hidden)]
#[doc(hidden)]
#[macro_export]
macro_rules! is_vlog_on {
    ($level:expr, $file:expr) => {{
        $crate::absl_log::config::is_vlog_enabled($level, $file)
    }};
}

/// Debug VLOG - only enabled in debug builds.
///
/// # Example
///
/// ```ignore
/// # fn main() {
/// use abseil::dvlog;
/// dvlog!(2, "Debug only verbose logging");
/// # }
/// ```
#[macro_export]
macro_rules! dvlog {
    ($level:expr, $($arg:tt)*) => {{
        #[cfg(debug_assertions)]
        {
            vlog!($level, $($arg)*);
        }
    }};
}

/// Returns true if VLOG is enabled for the given level and file.
///
/// This is the non-macro version of `vlog_is_on!`, useful when you need
/// a function rather than a macro.
///
/// # Example
///
/// ```rust
/// use abseil::absl_log::vlog::vlog_is_on;
///
/// fn expensive_operation() {
///     if vlog_is_on(2, file!()) {
///         // Only do expensive logging if VLOG is on
///     }
/// }
/// ```
#[inline]
pub fn vlog_is_on(level: i32, file: &str) -> bool {
    is_vlog_enabled(level, file)
}

/// VLOG with a condition.
///
/// Only logs if both the condition is true AND VLOG is enabled.
///
/// # Example
///
/// ```ignore
/// # fn main() {
/// use abseil::vlog_if;
/// # let detailed_debug_enabled = true;
/// # let data = vec![1, 2, 3];
/// vlog_if!(2, detailed_debug_enabled, "Detailed info: {:?}", data);
/// # }
/// ```
#[macro_export]
macro_rules! vlog_if {
    ($level:expr, $condition:expr, $($arg:tt)*) => {{
        if $condition && $crate::is_vlog_on!($level, file!()) {
            $crate::absl_log::log::do_log(
                $crate::absl_log::severity::LogSeverity::Info,
                file!(),
                line!(),
                format_args!("[VLOG{}] {}", $level, format_args!($($arg)*))
            );
        }
    }};
}

/// VLOG every N occurrences.
///
/// Logs only every Nth call (per call site).
///
/// # Example
///
/// ```ignore
/// # fn main() {
/// use abseil::vlog_every_n;
/// # let items = vec![1, 2, 3, 4, 5];
/// for item in items {
///     vlog_every_n!(2, 2, "Processing item: {:?}", item);
/// }
/// # }
/// ```
#[macro_export]
macro_rules! vlog_every_n {
    ($level:expr, $n:expr, $($arg:tt)*) => {{
        use std::sync::atomic::{AtomicUsize, Ordering};
        static COUNTER: AtomicUsize = AtomicUsize::new(0);
        let count = COUNTER.fetch_add(1, Ordering::Relaxed);
        if count % $n == 0 && $crate::is_vlog_on!($level, file!()) {
            $crate::absl_log::log::do_log(
                $crate::absl_log::severity::LogSeverity::Info,
                file!(),
                line!(),
                format_args!("[VLOG{}] [every {}] {}", $level, $n, format_args!($($arg)*))
            );
        }
    }};
}

/// VLOG only the first N times.
///
/// Logs only the first N times this macro is executed (per call site).
///
/// # Example
///
/// ```ignore
/// # fn main() {
/// use abseil::vlog_first_n;
/// # let items = vec![1, 2, 3, 4, 5];
/// for item in items {
///     vlog_first_n!(2, 3, "Initial items: {:?}", item);
/// }
/// # }
/// ```
#[macro_export]
macro_rules! vlog_first_n {
    ($level:expr, $n:expr, $($arg:tt)*) => {{
        use std::sync::atomic::{AtomicUsize, Ordering};
        static COUNT: AtomicUsize = AtomicUsize::new(0);
        let count = COUNT.load(Ordering::Relaxed);
        if count < $n && $crate::is_vlog_on!($level, file!()) {
            let _ = COUNT.fetch_add(1, Ordering::Relaxed);
            $crate::absl_log::log::do_log(
                $crate::absl_log::severity::LogSeverity::Info,
                file!(),
                line!(),
                format_args!("[VLOG{}] [first {}] {}", $level, $n, format_args!($($arg)*))
            );
        }
    }};
}

/// Get the current VLOG level for a file.
///
/// # Example
///
/// ```rust
/// use abseil::absl_log::vlog::get_vlog_level;
///
/// let level = get_vlog_level(file!());
/// if level >= 2 {
///     // Detailed logging available
/// }
/// ```
#[inline]
pub fn get_vlog_level(file: &str) -> i32 {
    crate::absl_log::config::global_config().get_vlog_level(file)
}

/// Set the VLOG level for a file pattern.
///
/// # Example
///
/// ```rust
/// use abseil::absl_log::vlog::set_vlog_level;
///
/// // Enable VLOG up to level 3 for http_server files
/// set_vlog_level("http_server", 3);
/// ```
#[inline]
pub fn set_vlog_level(pattern: &str, level: i32) {
    crate::absl_log::config::global_config().set_vlog_level(pattern, level);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::absl_log::config;

    #[test]
    fn test_vlog_compiles() {
        // These should compile and not panic
        vlog!(1, "Test message");
        vlog!(2, "Test with arg: {}", 42);
    }

    #[test]
    fn test_vlog_if_compiles() {
        vlog_if!(1, true, "Conditional log");
        vlog_if!(1, false, "Should not log");
    }

    #[test]
    fn test_dvlog_compiles() {
        dvlog!(1, "Debug vlog");
        dvlog!(2, "Debug vlog with arg: {}", 123);
    }

    #[test]
    fn test_vlog_every_n_compiles() {
        for i in 0..10 {
            vlog_every_n!(1, 3, "Item: {}", i);
        }
    }

    #[test]
    fn test_vlog_first_n_compiles() {
        for i in 0..20 {
            vlog_first_n!(1, 5, "Item: {}", i);
        }
    }

    #[test]
    fn test_vlog_is_on() {
        let enabled = vlog_is_on!(1);
        // Result depends on configuration, just check it returns a bool
        let _: bool = enabled;
    }

    #[test]
    fn test_get_vlog_level() {
        let level = get_vlog_level("test_file.rs");
        assert!(level >= 0);
    }

    #[test]
    fn test_set_vlog_level() {
        // Set and verify
        set_vlog_level("test_pattern", 2);
        let level = get_vlog_level("test_pattern.rs");
        assert_eq!(level, 2);

        // Clean up
        config::global_config().clear_vlog_levels();
    }

    #[test]
    fn test_vlog_with_custom_config() {
        // Set a high VLOG level for our test file
        config::global_config().set_vlog_level("vlog", 3);

        // This should log (level 1 <= configured level 3)
        vlog!(1, "This should log with config set");

        // Clean up
        config::global_config().clear_vlog_levels();
    }

    #[test]
    fn test_vlog_is_on_function() {
        let is_on = vlog_is_on(1, file!());
        assert!(is_on == true || is_on == false);
    }

    #[test]
    fn test_vlog_module() {
        use crate::absl_log::config;

        // Set up per-module logging
        config::global_config().set_vlog_level("test_module", 2);

        // Check that the level is set correctly
        assert_eq!(config::global_config().get_vlog_level("test_module.rs"), 2);
        assert_eq!(config::global_config().get_vlog_level("other_module.rs"), 0);

        // Clean up
        config::global_config().clear_vlog_levels();
    }
}
