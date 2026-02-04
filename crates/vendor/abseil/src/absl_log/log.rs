//! LOG macro for logging at different severity levels.
//!
//! This module provides the LOG macro family similar to Abseil's logging:
//! - LOG(severity, ...)
//! - LOG_IF(condition, severity, ...)
//! - QLOG(quiet severity, ...)

use crate::absl_log::severity::LogSeverity;
use std::fmt;
use std::io::{self, Write};
use std::fmt::Write as FmtWrite;

/// Logs a message at the specified severity level.
///
/// # Syntax
///
/// ```ignore
/// LOG!(INFO, "message {}", arg1)
/// LOG!(WARNING, "message {}", arg1)
/// LOG!(ERROR, "message {}", arg1)
/// LOG!(FATAL, "message {}", arg1)
/// ```
///
/// # Example
///
/// ```ignore
/// # #![allow(unused_variables)]
/// # fn main() {
/// use abseil::LOG;
/// LOG!(INFO, "Starting application");
/// # let value = 42;
/// LOG!(WARNING, "This is a warning: {}", value);
/// # let error = "test error";
/// LOG!(ERROR, "An error occurred: {}", error);
/// # }
/// ```
///
/// # Note
///
/// FATAL severity will terminate the program after logging the message.
#[macro_export]
macro_rules! LOG {
    (INFO, $($arg:tt)*) => {
        $crate::log_impl!(
            $crate::absl_log::severity::LogSeverity::Info,
            $($arg)*
        )
    };
    (WARNING, $($arg:tt)*) => {
        $crate::log_impl!(
            $crate::absl_log::severity::LogSeverity::Warning,
            $($arg)*
        )
    };
    (ERROR, $($arg:tt)*) => {
        $crate::log_impl!(
            $crate::absl_log::severity::LogSeverity::Error,
            $($arg)*
        )
    };
    (FATAL, $($arg:tt)*) => {
        $crate::log_impl!(
            $crate::absl_log::severity::LogSeverity::Fatal,
            $($arg)*
        )
    };
}

/// Internal implementation of the LOG macro.
#[doc(hidden)]
#[macro_export]
macro_rules! log_impl {
    ($severity:expr, $($arg:tt)*) => {{
        if $crate::absl_log::config::is_logging_enabled($severity) {
            $crate::absl_log::log::do_log(
                $severity,
                file!(),
                line!(),
                format_args!($($arg)*)
            );
        }
    }};
}

/// Logs a message if the condition is true.
///
/// # Syntax
///
/// ```ignore
/// LOG_IF(condition, INFO, "message {}", arg1)
/// ```
///
/// # Example
///
/// ```ignore
/// # fn main() {
/// use abseil::LOG_IF;
/// # let verbose = true;
/// # let data = 42;
/// LOG_IF(verbose, INFO, "Detailed info: {}", data);
/// # let value = -5;
/// LOG_IF(value < 0, WARNING, "Negative value: {}", value);
/// # }
/// ```
#[macro_export]
macro_rules! LOG_IF {
    ($condition:expr, INFO, $($arg:tt)*) => {
        $crate::log_if_impl!(
            $condition,
            $crate::absl_log::severity::LogSeverity::Info,
            $($arg)*
        )
    };
    ($condition:expr, WARNING, $($arg:tt)*) => {
        $crate::log_if_impl!(
            $condition,
            $crate::absl_log::severity::LogSeverity::Warning,
            $($arg)*
        )
    };
    ($condition:expr, ERROR, $($arg:tt)*) => {
        $crate::log_if_impl!(
            $condition,
            $crate::absl_log::severity::LogSeverity::Error,
            $($arg)*
        )
    };
    ($condition:expr, FATAL, $($arg:tt)*) => {
        $crate::log_if_impl!(
            $condition,
            $crate::absl_log::severity::LogSeverity::Fatal,
            $($arg)*
        )
    };
}

/// Internal implementation of the LOG_IF macro.
#[doc(hidden)]
#[macro_export]
macro_rules! log_if_impl {
    ($condition:expr, $severity:expr, $($arg:tt)*) => {{
        if $condition && $crate::absl_log::config::is_logging_enabled($severity) {
            $crate::absl_log::log::do_log(
                $severity,
                file!(),
                line!(),
                format_args!($($arg)*)
            );
        }
    }};
}

/// Quiet log - logs only if verbosity is enabled.
///
/// This is similar to LOG but with slightly different semantics for
/// determining whether to log.
///
/// # Example
///
/// ```ignore
/// # fn main() {
/// use abseil::QLOG;
/// QLOG!(INFO, "This message may not appear");
/// # }
/// ```
#[macro_export]
macro_rules! QLOG {
    ($severity:ident, $($arg:tt)*) => {
        LOG!($severity, $($arg)*)
    };
}

/// Internal function that actually performs the logging.
///
/// This function formats the log message and writes it to stderr.
pub fn do_log(severity: LogSeverity, file: &str, line: u32, args: fmt::Arguments<'_>) {
    // Format: [SEVERITY] file:line: message
    let mut output = String::new();
    let _ = write!(output, "[{}] {}:{}: {}", severity.as_str(), file, line, args);

    // Write to stderr (thread-safe in Rust)
    let stderr = io::stderr();
    let mut stderr_lock = stderr.lock();

    let _ = writeln!(stderr_lock, "{}", output);

    // Terminate on fatal
    if severity == LogSeverity::Fatal {
        #[cfg(feature = "std")]
        {
            stderr_lock.flush().unwrap();
            std::process::exit(1);
        }
        #[cfg(not(feature = "std"))]
        {
            // In no_std, we can't exit, so just loop
            loop {
                core::hint::spin_loop();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_log_info() {
        // Should not panic
        LOG!(INFO, "Test info message");
        LOG!(INFO, "Test with arg: {}", 42);
    }

    #[test]
    fn test_log_warning() {
        LOG!(WARNING, "Test warning message");
    }

    #[test]
    fn test_log_error() {
        LOG!(ERROR, "Test error message: {}", "error details");
    }

    #[test]
    fn test_log_if_true() {
        LOG_IF!(true, INFO, "This should log");
        LOG_IF!(true, WARNING, "This should also log: {}", 123);
    }

    #[test]
    fn test_log_if_false() {
        // These should not log (condition is false)
        LOG_IF!(false, INFO, "This should not log");
    }

    #[test]
    fn test_log_if_with_expression() {
        let value = 42;
        LOG_IF!(value > 10, INFO, "Value is large: {}", value);
        LOG_IF!(value < 10, WARNING, "Value is small"); // Won't log
    }

    #[test]
    fn test_qlog() {
        QLOG!(INFO, "Quiet log message");
        QLOG!(WARNING, "Quiet warning");
    }
}
