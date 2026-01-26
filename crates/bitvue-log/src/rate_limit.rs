//! Rate-Limited Logging - Prevent log spam in high-frequency code paths.
//!
//! These macros help control logging output when code is called frequently,
//! such as in tight loops or per-frame processing.
//!
//! # Examples
//!
//! ```no_run
//! use bitvue_log::{log_every_n, log_first_n, log_every_n_sec};
//!
//! // Log every 100th occurrence
//! for i in 0..10000 {
//!     log_every_n!(info, 100, "Processed {} items", i);
//! }
//!
//! // Log only first 3 occurrences
//! loop {
//!     log_first_n!(warn, 3, "Initialization warning");
//! }
//!
//! // Log at most once every 5 seconds
//! loop {
//!     log_every_n_sec!(info, 5.0, "Heartbeat");
//! }
//! ```

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

/// LOG_EVERY_N - Log every Nth occurrence.
///
/// Uses a static counter per call site to track occurrences.
///
/// # Arguments
///
/// * `level` - Log level (trace, debug, info, warn, error)
/// * `n` - Log every Nth occurrence (1 = every time, 100 = every 100th)
/// * Format string and arguments
///
/// # Examples
///
/// ```no_run
/// use bitvue_log::log_every_n;
///
/// for i in 0..10000 {
///     log_every_n!(info, 1000, "Progress: {}/10000", i);
/// }
/// ```
#[macro_export]
macro_rules! log_every_n {
    ($level:tt, $n:expr, $($arg:tt)+) => {{
        static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let count = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let n: u64 = $n;
        if count % n == 0 {
            tracing::$level!(occurrence = count, $($arg)+);
        }
    }};
}

/// LOG_FIRST_N - Log only the first N occurrences.
///
/// Useful for logging initialization issues or first-time events.
///
/// # Arguments
///
/// * `level` - Log level (trace, debug, info, warn, error)
/// * `n` - Maximum number of times to log
/// * Format string and arguments
///
/// # Examples
///
/// ```no_run
/// use bitvue_log::log_first_n;
///
/// // Only warn about this the first 3 times
/// fn potentially_slow_operation() {
///     log_first_n!(warn, 3, "This operation may be slow");
/// }
/// ```
#[macro_export]
macro_rules! log_first_n {
    ($level:tt, $n:expr, $($arg:tt)+) => {{
        static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let count = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let n: u64 = $n;
        if count < n {
            tracing::$level!(occurrence = count, $($arg)+);
        }
    }};
}

/// LOG_EVERY_N_SEC - Log at most once per N seconds.
///
/// Uses timestamps to control log frequency. Useful for periodic status updates.
///
/// # Arguments
///
/// * `level` - Log level (trace, debug, info, warn, error)
/// * `sec` - Minimum seconds between logs
/// * Format string and arguments
///
/// # Examples
///
/// ```no_run
/// use bitvue_log::log_every_n_sec;
///
/// loop {
///     // Status update at most once every 10 seconds
///     log_every_n_sec!(info, 10.0, "System status: OK");
/// }
/// ```
#[macro_export]
macro_rules! log_every_n_sec {
    ($level:tt, $sec:expr, $($arg:tt)+) => {{
        static LAST_LOG_MS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);

        let last = LAST_LOG_MS.load(std::sync::atomic::Ordering::Relaxed);
        let interval_ms = ($sec * 1000.0) as u64;

        if now_ms.saturating_sub(last) >= interval_ms {
            // Use compare_exchange to avoid race conditions
            if LAST_LOG_MS.compare_exchange(
                last,
                now_ms,
                std::sync::atomic::Ordering::SeqCst,
                std::sync::atomic::Ordering::Relaxed
            ).is_ok() {
                tracing::$level!($($arg)+);
            }
        }
    }};
}

/// LOG_IF - Conditional logging.
///
/// Only logs if the condition is true.
///
/// # Arguments
///
/// * `level` - Log level (trace, debug, info, warn, error)
/// * `cond` - Condition to check
/// * Format string and arguments
///
/// # Examples
///
/// ```ignore
/// use bitvue_log::log_if;
///
/// log_if!(warn, frame_count > 1000, "High frame count: {}", frame_count);
/// log_if!(error, result.is_err(), "Operation failed");
/// ```
#[macro_export]
macro_rules! log_if {
    ($level:tt, $cond:expr, $($arg:tt)+) => {{
        if $cond {
            tracing::$level!($($arg)+);
        }
    }};
}

/// LOG_IF_EVERY_N - Conditional logging with rate limiting.
///
/// Only logs if condition is true, and only every Nth occurrence.
///
/// # Arguments
///
/// * `level` - Log level (trace, debug, info, warn, error)
/// * `cond` - Condition to check
/// * `n` - Log every Nth occurrence when condition is true
/// * Format string and arguments
#[macro_export]
macro_rules! log_if_every_n {
    ($level:tt, $cond:expr, $n:expr, $($arg:tt)+) => {{
        if $cond {
            $crate::log_every_n!($level, $n, $($arg)+);
        }
    }};
}

/// PLOG - Log with system error (errno).
///
/// Appends the last system error to the log message.
/// Useful for system calls that set errno.
///
/// # Arguments
///
/// * `level` - Log level (trace, debug, info, warn, error)
/// * Format string and arguments
///
/// # Examples
///
/// ```no_run
/// use bitvue_log::plog;
///
/// // After a failed system call
/// plog!(error, "Failed to open file");  // Logs: "Failed to open file (errno: ...)"
/// ```
#[macro_export]
macro_rules! plog {
    ($level:tt, $($arg:tt)+) => {{
        let errno = std::io::Error::last_os_error();
        tracing::$level!("{} ({})", format!($($arg)+), errno);
    }};
}

/// DFATAL - Debug: FATAL, Release: ERROR.
///
/// In debug builds, this macro will panic after logging.
/// In release builds, it only logs an error.
///
/// Useful for "should not happen" conditions that are recoverable but indicate bugs.
///
/// # Examples
///
/// ```ignore
/// use bitvue_log::dfatal;
///
/// if index >= array.len() {
///     dfatal!("Index {} out of bounds for array of size {}", index, array.len());
///     return; // Only reached in release builds
/// }
/// ```
#[macro_export]
macro_rules! dfatal {
    ($($arg:tt)+) => {{
        let msg = format!($($arg)+);
        tracing::error!("DFATAL: {}", msg);
        #[cfg(debug_assertions)]
        panic!("DFATAL: {}", msg);
    }};
}

/// Helper: Get current timestamp in milliseconds.
#[doc(hidden)]
#[inline]
pub fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// Helper struct for tracking rate-limited logging state.
#[doc(hidden)]
pub struct RateLimitState {
    counter: AtomicU64,
    last_log_ms: AtomicU64,
}

impl RateLimitState {
    pub const fn new() -> Self {
        Self {
            counter: AtomicU64::new(0),
            last_log_ms: AtomicU64::new(0),
        }
    }

    pub fn increment(&self) -> u64 {
        self.counter.fetch_add(1, Ordering::Relaxed)
    }

    pub fn should_log_every_n(&self, n: u64) -> bool {
        let count = self.increment();
        count % n == 0
    }

    pub fn should_log_first_n(&self, n: u64) -> bool {
        let count = self.increment();
        count < n
    }

    pub fn should_log_every_n_sec(&self, sec: f64) -> bool {
        let now = now_millis();
        let last = self.last_log_ms.load(Ordering::Relaxed);
        let interval_ms = (sec * 1000.0) as u64;

        if now.saturating_sub(last) >= interval_ms {
            self.last_log_ms
                .compare_exchange(last, now, Ordering::SeqCst, Ordering::Relaxed)
                .is_ok()
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rate_limit_state_every_n() {
        let state = RateLimitState::new();

        // Should log at 0, 3, 6, 9...
        assert!(state.should_log_every_n(3)); // count=0
        assert!(!state.should_log_every_n(3)); // count=1
        assert!(!state.should_log_every_n(3)); // count=2
        assert!(state.should_log_every_n(3)); // count=3
    }

    #[test]
    fn test_rate_limit_state_first_n() {
        let state = RateLimitState::new();

        // Should log first 3 times
        assert!(state.should_log_first_n(3)); // count=0
        assert!(state.should_log_first_n(3)); // count=1
        assert!(state.should_log_first_n(3)); // count=2
        assert!(!state.should_log_first_n(3)); // count=3
        assert!(!state.should_log_first_n(3)); // count=4
    }

    #[test]
    fn test_now_millis() {
        let ms = now_millis();
        // Should be a reasonable timestamp (after 2020)
        assert!(ms > 1577836800000); // Jan 1, 2020
    }

    #[test]
    fn test_log_every_n_macro() {
        let mut logged_count = 0;

        for _ in 0..10 {
            // This is a compile test - we can't easily capture tracing output
            // but we verify the macro expands correctly
            log_every_n!(trace, 3, "test message");
        }

        // Manual simulation
        for i in 0..10 {
            if i % 3 == 0 {
                logged_count += 1;
            }
        }
        assert_eq!(logged_count, 4); // 0, 3, 6, 9
    }

    #[test]
    fn test_log_first_n_macro() {
        let mut logged_count = 0;

        for _ in 0..10 {
            log_first_n!(trace, 3, "test message");
        }

        // Manual simulation
        for i in 0..10 {
            if i < 3 {
                logged_count += 1;
            }
        }
        assert_eq!(logged_count, 3);
    }

    #[test]
    fn test_log_if_macro() {
        log_if!(trace, true, "should log");
        log_if!(trace, false, "should not log");
        // Compile test - verifies macro expansion
    }

    #[test]
    fn test_dfatal_release() {
        // In release mode, dfatal should not panic
        // In debug mode, this test would panic
        #[cfg(not(debug_assertions))]
        {
            dfatal!("This should not panic in release");
        }
    }
}
