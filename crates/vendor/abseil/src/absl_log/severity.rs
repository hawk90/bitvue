//! Log severity levels.
//!
//! This module defines the severity levels for logging, similar to
//! Abseil's `absl::LogSeverity` enum.

use core::fmt;

/// Log severity levels.
///
/// These correspond to the severity levels used in Abseil C++:
/// - `kInfo`: Informational messages
/// - `kWarning`: Warning messages
/// - `kError`: Error messages
/// - `kFatal`: Fatal errors (terminate the program)
///
/// # Example
///
/// ```rust
/// use abseil::absl_log::severity::LogSeverity;
///
/// fn log_message(severity: LogSeverity, message: &str) {
///     match severity {
///         LogSeverity::Info => println!("INFO: {}", message),
///         LogSeverity::Warning => println!("WARNING: {}", message),
///         LogSeverity::Error => println!("ERROR: {}", message),
///         LogSeverity::Fatal => {
///             println!("FATAL: {}", message);
///             std::process::exit(1);
///         }
///     }
/// }
/// ```
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
#[repr(i32)]
pub enum LogSeverity {
    /// Informational message.
    ///
    /// This is the default severity for general informational messages.
    Info = 0,

    /// Warning message.
    ///
    /// Used for potentially harmful situations that don't prevent
    /// the program from continuing.
    Warning = 1,

    /// Error message.
    ///
    /// Used for error events that might still allow the application
    /// to continue running.
    Error = 2,

    /// Fatal error.
    ///
    /// Used for critical errors that will cause the application to
    /// terminate after logging the message.
    Fatal = 3,
}

impl LogSeverity {
    /// Returns the string representation of the severity.
    ///
    /// # Example
    ///
    /// ```rust
    /// use abseil::absl_log::severity::LogSeverity;
    ///
    /// assert_eq!(LogSeverity::Info.as_str(), "INFO");
    /// assert_eq!(LogSeverity::Warning.as_str(), "WARNING");
    /// ```
    #[inline]
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            LogSeverity::Info => "INFO",
            LogSeverity::Warning => "WARNING",
            LogSeverity::Error => "ERROR",
            LogSeverity::Fatal => "FATAL",
        }
    }

    /// Returns the numeric value of the severity.
    ///
    /// # Example
    ///
    /// ```rust
    /// use abseil::absl_log::severity::LogSeverity;
    ///
    /// assert_eq!(LogSeverity::Info.as_i32(), 0);
    /// assert_eq!(LogSeverity::Warning.as_i32(), 1);
    /// ```
    #[inline]
    #[must_use]
    pub const fn as_i32(self) -> i32 {
        self as i32
    }

    /// Creates a `LogSeverity` from an i32 value.
    ///
    /// Returns `None` if the value is not a valid severity level.
    ///
    /// # Example
    ///
    /// ```rust
    /// use abseil::absl_log::severity::LogSeverity;
    ///
    /// assert_eq!(LogSeverity::from_i32(0), Some(LogSeverity::Info));
    /// assert_eq!(LogSeverity::from_i32(3), Some(LogSeverity::Fatal));
    /// assert_eq!(LogSeverity::from_i32(99), None);
    /// ```
    #[inline]
    #[must_use]
    pub const fn from_i32(value: i32) -> Option<Self> {
        match value {
            0 => Some(LogSeverity::Info),
            1 => Some(LogSeverity::Warning),
            2 => Some(LogSeverity::Error),
            3 => Some(LogSeverity::Fatal),
            _ => None,
        }
    }

    /// Returns `true` if this severity is at least as severe as the given level.
    ///
    /// # Example
    ///
    /// ```rust
    /// use abseil::absl_log::severity::LogSeverity;
    ///
    /// assert!(LogSeverity::Error.is_at_least(LogSeverity::Warning));
    /// assert!(!LogSeverity::Info.is_at_least(LogSeverity::Error));
    /// ```
    #[inline]
    #[must_use]
    pub const fn is_at_least(self, min_level: LogSeverity) -> bool {
        self as i32 >= min_level as i32
    }
}

impl Default for LogSeverity {
    #[inline]
    fn default() -> Self {
        LogSeverity::Info
    }
}

impl fmt::Display for LogSeverity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Minimum log level.
///
/// Messages below this level will not be logged.
pub type MinLogLevel = LogSeverity;

/// Stacktrace log level.
///
/// If a message is at least this severe, a stacktrace will be included.
pub type StacktraceLogLevel = LogSeverity;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_as_str() {
        assert_eq!(LogSeverity::Info.as_str(), "INFO");
        assert_eq!(LogSeverity::Warning.as_str(), "WARNING");
        assert_eq!(LogSeverity::Error.as_str(), "ERROR");
        assert_eq!(LogSeverity::Fatal.as_str(), "FATAL");
    }

    #[test]
    fn test_as_i32() {
        assert_eq!(LogSeverity::Info.as_i32(), 0);
        assert_eq!(LogSeverity::Warning.as_i32(), 1);
        assert_eq!(LogSeverity::Error.as_i32(), 2);
        assert_eq!(LogSeverity::Fatal.as_i32(), 3);
    }

    #[test]
    fn test_from_i32() {
        assert_eq!(LogSeverity::from_i32(0), Some(LogSeverity::Info));
        assert_eq!(LogSeverity::from_i32(1), Some(LogSeverity::Warning));
        assert_eq!(LogSeverity::from_i32(2), Some(LogSeverity::Error));
        assert_eq!(LogSeverity::from_i32(3), Some(LogSeverity::Fatal));
        assert_eq!(LogSeverity::from_i32(-1), None);
        assert_eq!(LogSeverity::from_i32(4), None);
        assert_eq!(LogSeverity::from_i32(99), None);
    }

    #[test]
    fn test_is_at_least() {
        assert!(LogSeverity::Info.is_at_least(LogSeverity::Info));
        assert!(LogSeverity::Warning.is_at_least(LogSeverity::Info));
        assert!(LogSeverity::Error.is_at_least(LogSeverity::Warning));
        assert!(LogSeverity::Fatal.is_at_least(LogSeverity::Error));

        assert!(!LogSeverity::Info.is_at_least(LogSeverity::Warning));
        assert!(!LogSeverity::Warning.is_at_least(LogSeverity::Error));
        assert!(!LogSeverity::Error.is_at_least(LogSeverity::Fatal));
    }

    #[test]
    fn test_ord() {
        assert!(LogSeverity::Info < LogSeverity::Warning);
        assert!(LogSeverity::Warning < LogSeverity::Error);
        assert!(LogSeverity::Error < LogSeverity::Fatal);
    }

    #[test]
    fn test_default() {
        assert_eq!(LogSeverity::default(), LogSeverity::Info);
    }

    #[test]
    fn test_display() {
        assert_eq!(format!("{}", LogSeverity::Info), "INFO");
        assert_eq!(format!("{}", LogSeverity::Fatal), "FATAL");
    }

    #[test]
    fn test_copy() {
        let s = LogSeverity::Error;
        let s2 = s;
        assert_eq!(s, s2); // LogSeverity is Copy
    }
}
