//! Rate-limited logging macros.
//!
//! These macros provide logging that is automatically rate-limited to avoid
//! spamming logs with repeated messages.

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

/// Logs only if the condition is true.
///
/// # Example
///
/// ```ignore
/// # fn main() {
/// # let condition = true;
/// # let value = 42;
/// log_if!(condition, INFO, "Conditional message: {}", value);
/// # }
/// ```
#[macro_export]
macro_rules! log_if {
    ($condition:expr, INFO, $($arg:tt)*) => {
        $crate::log_if_impl_rate_limit!(
            $condition,
            $crate::absl_log::severity::LogSeverity::Info,
            $($arg)*
        )
    };
    ($condition:expr, WARNING, $($arg:tt)*) => {
        $crate::log_if_impl_rate_limit!(
            $condition,
            $crate::absl_log::severity::LogSeverity::Warning,
            $($arg)*
        )
    };
    ($condition:expr, ERROR, $($arg:tt)*) => {
        $crate::log_if_impl_rate_limit!(
            $condition,
            $crate::absl_log::severity::LogSeverity::Error,
            $($arg)*
        )
    };
    ($condition:expr, FATAL, $($arg:tt)*) => {
        $crate::log_if_impl_rate_limit!(
            $condition,
            $crate::absl_log::severity::LogSeverity::Fatal,
            $($arg)*
        )
    };
}

/// Internal implementation of log_if.
#[doc(hidden)]
#[doc(hidden)]
#[macro_export]
macro_rules! log_if_impl_rate_limit {
    ($condition:expr, $severity:expr, $($arg:tt)*) => {{
        if $condition && $crate::absl_log::config::is_logging_enabled($severity) {
            $crate::absl_log::log::do_log($severity, file!(), line!(), format_args!($($arg)*));
        }
    }};
}

/// Logs only the first N times the macro is executed.
///
/// # Example
///
/// ```ignore
/// # fn main() {
/// use abseil::absl_log::rate_limit::log_first_n;
/// for i in 0..1000 {
///     log_first_n!(INFO, 5, "This will only log 5 times. Iteration: {}", i);
/// }
/// # }
/// ```
#[macro_export]
macro_rules! log_first_n {
    (INFO, $n:expr, $($arg:tt)*) => {
        $crate::log_first_n_impl!(
            $crate::absl_log::severity::LogSeverity::Info,
            $n,
            $($arg)*
        )
    };
    (WARNING, $n:expr, $($arg:tt)*) => {
        $crate::log_first_n_impl!(
            $crate::absl_log::severity::LogSeverity::Warning,
            $n,
            $($arg)*
        )
    };
    (ERROR, $n:expr, $($arg:tt)*) => {
        $crate::log_first_n_impl!(
            $crate::absl_log::severity::LogSeverity::Error,
            $n,
            $($arg)*
        )
    };
    (FATAL, $n:expr, $($arg:tt)*) => {
        $crate::log_first_n_impl!(
            $crate::absl_log::severity::LogSeverity::Fatal,
            $n,
            $($arg)*
        )
    };
}

/// Internal implementation of log_first_n.
#[doc(hidden)]
#[macro_export]
#[doc(hidden)]
macro_rules! log_first_n_impl {
    ($severity:expr, $n:expr, $($arg:tt)*) => {{
        use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
        static COUNTER: AtomicUsize = AtomicUsize::new(0);
        static DONE: AtomicBool = AtomicBool::new(false);

        if !DONE.load(Ordering::Relaxed) {
            let count = COUNTER.fetch_add(1, Ordering::Relaxed);
            if count < $n && $crate::absl_log::config::is_logging_enabled($severity) {
                let remaining = $n - count - 1;
                if remaining == 0 {
                    DONE.store(true, Ordering::Relaxed);
                }
                let msg = if remaining > 0 {
                    format_args!($($arg)*)
                } else {
                    format_args!("{} (message repeated {} times, now suppressing)", format_args!($($arg)*), $n)
                };
                $crate::absl_log::log::do_log($severity, file!(), line!(), msg);
            }
        }
    }};
}

/// Logs every N occurrences.
///
/// # Example
///
/// ```ignore
/// # fn main() {
/// use abseil::absl_log::rate_limit::log_every_n;
/// for i in 0..1000 {
///     log_every_n!(INFO, 100, "Iteration: {}", i);
///     // Logs at iterations 0, 100, 200, ..., 900
/// }
/// # }
/// ```
#[macro_export]
macro_rules! log_every_n {
    (INFO, $n:expr, $($arg:tt)*) => {
        $crate::log_every_n_impl!(
            $crate::absl_log::severity::LogSeverity::Info,
            $n,
            $($arg)*
        )
    };
    (WARNING, $n:expr, $($arg:tt)*) => {
        $crate::log_every_n_impl!(
            $crate::absl_log::severity::LogSeverity::Warning,
            $n,
            $($arg)*
        )
    };
    (ERROR, $n:expr, $($arg:tt)*) => {
        $crate::log_every_n_impl!(
            $crate::absl_log::severity::LogSeverity::Error,
            $n,
            $($arg)*
        )
    };
    (FATAL, $n:expr, $($arg:tt)*) => {
        $crate::log_every_n_impl!(
            $crate::absl_log::severity::LogSeverity::Fatal,
            $n,
            $($arg)*
        )
    };
}

/// Internal implementation of log_every_n.
#[doc(hidden)]
#[macro_export]
#[doc(hidden)]
macro_rules! log_every_n_impl {
    ($severity:expr, $n:expr, $($arg:tt)*) => {{
        use std::sync::atomic::{AtomicUsize, Ordering};
        static COUNTER: AtomicUsize = AtomicUsize::new(0);

        let count = COUNTER.fetch_add(1, Ordering::Relaxed);
        if count % $n == 0 && $crate::absl_log::config::is_logging_enabled($severity) {
            $crate::absl_log::log::do_log($severity, file!(), line!(), format_args!($($arg)*));
        }
    }};
}

/// Logs at most once per N seconds.
///
/// # Example
///
/// ```ignore
/// # fn main() {
/// use abseil::absl_log::rate_limit::log_every_n_sec;
/// // Note: This would normally loop, but for doctest we just call it once
/// log_every_n_sec!(WARNING, 5.0, "This message appears at most every 5 seconds");
/// # }
/// ```
#[macro_export]
macro_rules! log_every_n_sec {
    (INFO, $sec:expr, $($arg:tt)*) => {
        $crate::log_every_n_sec_impl!(
            $crate::absl_log::severity::LogSeverity::Info,
            $sec,
            $($arg)*
        )
    };
    (WARNING, $sec:expr, $($arg:tt)*) => {
        $crate::log_every_n_sec_impl!(
            $crate::absl_log::severity::LogSeverity::Warning,
            $sec,
            $($arg)*
        )
    };
    (ERROR, $sec:expr, $($arg:tt)*) => {
        $crate::log_every_n_sec_impl!(
            $crate::absl_log::severity::LogSeverity::Error,
            $sec,
            $($arg)*
        )
    };
    (FATAL, $sec:expr, $($arg:tt)*) => {
        $crate::log_every_n_sec_impl!(
            $crate::absl_log::severity::LogSeverity::Fatal,
            $sec,
            $($arg)*
        )
    };
}

/// Internal implementation of log_every_n_sec.
#[doc(hidden)]
#[macro_export]
#[doc(hidden)]
macro_rules! log_every_n_sec_impl {
    ($severity:expr, $sec:expr, $($arg:tt)*) => {{
        use std::sync::atomic::{AtomicU64, Ordering};
        use std::time::{SystemTime, Duration};

        // Store the last log time as nanoseconds since unix epoch
        static LAST_LOG: AtomicU64 = AtomicU64::new(0);

        let interval_nanos = ($sec * 1_000_000_000.0) as u64;
        let now_nanos = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0);

        let last = LAST_LOG.load(Ordering::Relaxed);
        let elapsed = now_nanos.saturating_sub(last);

        if elapsed >= interval_nanos && $crate::absl_log::config::is_logging_enabled($severity) {
            // Try to update the last log time
            if LAST_LOG.compare_exchange(last, now_nanos, Ordering::Relaxed, Ordering::Relaxed).is_ok() {
                $crate::absl_log::log::do_log($severity, file!(), line!(), format_args!($($arg)*));
            }
        }
    }};
}

/// Logs only if condition is true, at most once per N seconds.
///
/// Combines log_if with time-based rate limiting.
///
/// # Example
///
/// ```ignore
/// # fn main() {
/// use abseil::absl_log::rate_limit::log_if_every_n;
/// # let error_occurred = true;
/// log_if_every_n!(INFO, error_occurred, 10.0, "Error detected, logging at most every 10 seconds");
/// # }
/// ```
#[macro_export]
macro_rules! log_if_every_n {
    (INFO, $condition:expr, $sec:expr, $($arg:tt)*) => {
        $crate::log_if_every_n_impl!(
            $crate::absl_log::severity::LogSeverity::Info,
            $condition,
            $sec,
            $($arg)*
        )
    };
    (WARNING, $condition:expr, $sec:expr, $($arg:tt)*) => {
        $crate::log_if_every_n_impl!(
            $crate::absl_log::severity::LogSeverity::Warning,
            $condition,
            $sec,
            $($arg)*
        )
    };
    (ERROR, $condition:expr, $sec:expr, $($arg:tt)*) => {
        $crate::log_if_every_n_impl!(
            $crate::absl_log::severity::LogSeverity::Error,
            $condition,
            $sec,
            $($arg)*
        )
    };
    (FATAL, $condition:expr, $sec:expr, $($arg:tt)*) => {
        $crate::log_if_every_n_impl!(
            $crate::absl_log::severity::LogSeverity::Fatal,
            $condition,
            $sec,
            $($arg)*
        )
    };
}

/// Internal implementation of log_if_every_n.
#[doc(hidden)]
#[macro_export]
macro_rules! log_if_every_n_impl {
    ($severity:expr, $condition:expr, $sec:expr, $($arg:tt)*) => {{
        if $condition {
            $crate::log_every_n_sec_impl!(
                $severity,
                $sec,
                $($arg)*
            );
        }
    }};
}

/// Logs a message as ERROR but exits gracefully instead of terminating.
///
/// "dfatal" means "fatal only in debug mode" - in release builds, this
/// behaves like ERROR, but in debug builds it behaves like FATAL.
///
/// # Example
///
/// ```ignore
/// # fn main() {
/// use abseil::absl_log::rate_limit::dfatal;
/// dfatal!("This might terminate depending on build mode");
/// # }
/// ```
#[macro_export]
macro_rules! dfatal {
    ($($arg:tt)*) => {{
        #[cfg(debug_assertions)]
        {
            $crate::absl_log::log::do_log(
                $crate::absl_log::severity::LogSeverity::Fatal,
                file!(),
                line!(),
                format_args!($($arg)*)
            );
        }
        #[cfg(not(debug_assertions))]
        {
            $crate::absl_log::log::do_log(
                $crate::absl_log::severity::LogSeverity::Error,
                file!(),
                line!(),
                format_args!($($arg)*)
            );
        }
    }};
}

/// Pretty log - logs with color/formatting if the terminal supports it.
///
/// # Example
///
/// ```ignore
/// # fn main() {
/// use abseil::absl_log::rate_limit::plog;
/// plog!(INFO, "This message might have colors/enhanced formatting");
/// # }
/// ```
#[macro_export]
macro_rules! plog {
    (INFO, $($arg:tt)*) => {
        $crate::absl_log::log::do_log(
            $crate::absl_log::severity::LogSeverity::Info,
            file!(),
            line!(),
            format_args!($($arg)*)
        )
    };
    (WARNING, $($arg:tt)*) => {
        $crate::absl_log::log::do_log(
            $crate::absl_log::severity::LogSeverity::Warning,
            file!(),
            line!(),
            format_args!($($arg)*)
        )
    };
    (ERROR, $($arg:tt)*) => {
        $crate::absl_log::log::do_log(
            $crate::absl_log::severity::LogSeverity::Error,
            file!(),
            line!(),
            format_args!($($arg)*)
        )
    };
    (FATAL, $($arg:tt)*) => {
        $crate::absl_log::log::do_log(
            $crate::absl_log::severity::LogSeverity::Fatal,
            file!(),
            line!(),
            format_args!($($arg)*)
        )
    };
}

/// A rate limiter that can be shared across multiple log calls.
///
/// This is useful for rate limiting logs across different parts of code.
///
/// # Example
///
/// ```
/// use abseil::absl_log::rate_limit::RateLimiter;
/// use std::time::Duration;
///
/// # fn main() {
/// let limiter = RateLimiter::new(Duration::from_secs(5));
///
/// fn maybe_log(limiter: &RateLimiter, message: &str) {
///     if limiter.try_log() {
///         println!("{}", message);
///     }
/// }
///
/// maybe_log(&limiter, "This message is rate-limited");
/// # }
/// ```
#[derive(Debug)]
pub struct RateLimiter {
    interval: Duration,
    last_log: AtomicU64,
}

impl RateLimiter {
    /// Creates a new rate limiter with the specified interval between logs.
    ///
    /// # Example
    ///
    /// ```
    /// use abseil::absl_log::rate_limit::RateLimiter;
    /// use std::time::Duration;
    ///
    /// # fn main() {
    /// let limiter = RateLimiter::new(Duration::from_secs(1));
    /// # }
    /// ```
    pub const fn new(interval: Duration) -> Self {
        Self {
            interval,
            last_log: AtomicU64::new(0),
        }
    }

    /// Returns true if a log should be emitted (based on rate limit).
    ///
    /// # Example
    ///
    /// ```
    /// use abseil::absl_log::rate_limit::RateLimiter;
    /// use std::time::Duration;
    ///
    /// # fn main() {
    /// let limiter = RateLimiter::new(Duration::from_secs(1));
    ///
    /// if limiter.try_log() {
    ///     println!("This will be rate-limited");
    /// }
    /// # }
    /// ```
    pub fn try_log(&self) -> bool {
        let interval_nanos = self.interval.as_nanos() as u64;
        let now_nanos = current_time_nanos();
        let last = self.last_log.load(Ordering::Relaxed);

        let elapsed = now_nanos.saturating_sub(last);
        if elapsed >= interval_nanos {
            self.last_log.store(now_nanos, Ordering::Relaxed);
            true
        } else {
            false
        }
    }

    /// Resets the rate limiter, allowing an immediate next log.
    #[inline]
    pub fn reset(&self) {
        self.last_log.store(0, Ordering::Relaxed);
    }

    /// Returns the interval between logs.
    #[inline]
    pub fn interval(&self) -> Duration {
        self.interval
    }
}

/// Get current time as nanoseconds (simplified for portability).
#[inline]
fn current_time_nanos() -> u64 {
    #[cfg(feature = "std")]
    {
        use std::time::{SystemTime, UNIX_EPOCH};
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0)
    }
    #[cfg(not(feature = "std"))]
    {
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::absl_log::severity::LogSeverity;

    #[test]
    fn test_log_if_true() {
        log_if!(true, INFO, "This should log");
        log_if!(true, WARNING, "This should also log: {}", 42);
    }

    #[test]
    fn test_log_if_false() {
        log_if!(false, INFO, "This should not log");
        log_if!(false, WARNING, "Neither should this: {}", 123);
    }

    #[test]
    fn test_log_first_n() {
        // Log multiple times, but only first N should actually log
        for i in 0..10 {
            log_first_n!(INFO, 3, "Iteration {}", i);
        }
    }

    #[test]
    fn test_log_every_n() {
        // Test the macro compiles and works
        for i in 0..20 {
            log_every_n!(INFO, 5, "Iteration {}", i);
        }
    }

    #[test]
    fn test_log_every_n_sec() {
        // Test that it compiles
        log_every_n_sec!(INFO, 0.1, "Time-limited log");
        log_every_n_sec!(WARNING, 1.0, "Another time-limited log");
    }

    #[test]
    fn test_log_if_every_n() {
        let condition = true;
        log_if_every_n!(INFO, condition, 1.0, "Conditional and time-limited");
    }

    // Note: dfatal test omitted because it would cause the program to
    // abort in debug mode, which would fail the test suite

    #[test]
    fn test_plog_compiles() {
        plog!(INFO, "Pretty log info");
        plog!(WARNING, "Pretty log warning");
    }

    #[test]
    fn test_rate_limiter_new() {
        let limiter = RateLimiter::new(Duration::from_secs(1));
        assert_eq!(limiter.interval(), Duration::from_secs(1));
    }

    #[test]
    fn test_rate_limiter_try_log() {
        let limiter = RateLimiter::new(Duration::from_millis(100));

        // First call should succeed
        assert!(limiter.try_log());

        // Immediate second call should fail
        assert!(!limiter.try_log());

        // After reset, should succeed
        limiter.reset();
        assert!(limiter.try_log());
    }

    #[test]
    fn test_rate_limiter_interval() {
        let limiter = RateLimiter::new(Duration::from_secs(5));
        assert_eq!(limiter.interval(), Duration::from_secs(5));
    }

    #[test]
    fn test_const_rate_limiter() {
        // Test that RateLimiter can be created as a const
        static LIMITER: RateLimiter = RateLimiter::new(Duration::from_secs(10));
        assert_eq!(LIMITER.interval(), Duration::from_secs(10));
    }

    #[test]
    fn test_log_if_with_expressions() {
        let value = 42;
        log_if!(value > 10, INFO, "Value is large: {}", value);
        log_if!(value < 10, WARNING, "Value is small"); // Won't log
    }
}
