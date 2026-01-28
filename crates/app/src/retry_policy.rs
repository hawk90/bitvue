//! Retry Policy - Configurable retry logic for async workers
//!
//! Provides exponential backoff and configurable retry attempts for
//! transient failures in I/O operations.

use std::thread;
use std::time::Duration;

/// Retry policy configuration
#[derive(Debug, Clone)]
pub struct RetryPolicy {
    /// Maximum number of retry attempts (0 = no retries)
    pub max_attempts: u32,
    /// Initial delay between retries (doubles each attempt)
    pub initial_delay_ms: u64,
    /// Maximum delay between retries
    pub max_delay_ms: u64,
}

impl RetryPolicy {
    /// No retries - fail immediately
    pub fn none() -> Self {
        Self {
            max_attempts: 0,
            initial_delay_ms: 0,
            max_delay_ms: 0,
        }
    }

    /// Standard retry policy: 3 attempts with exponential backoff
    /// Delays: 100ms, 200ms, 400ms
    pub fn standard() -> Self {
        Self {
            max_attempts: 3,
            initial_delay_ms: 100,
            max_delay_ms: 1000,
        }
    }

    /// Aggressive retry policy: 5 attempts with longer backoff
    /// Delays: 200ms, 400ms, 800ms, 1000ms, 1000ms
    pub fn aggressive() -> Self {
        Self {
            max_attempts: 5,
            initial_delay_ms: 200,
            max_delay_ms: 1000,
        }
    }

    /// Custom retry policy
    pub fn custom(max_attempts: u32, initial_delay_ms: u64, max_delay_ms: u64) -> Self {
        Self {
            max_attempts,
            initial_delay_ms,
            max_delay_ms,
        }
    }

    /// Execute an operation with retry logic
    ///
    /// Returns Ok(T) on success, Err(E) if all retries exhausted
    pub fn execute<T, E, F>(&self, mut operation: F) -> Result<T, E>
    where
        F: FnMut() -> Result<T, E>,
    {
        let mut attempts = 0;
        let mut delay_ms = self.initial_delay_ms;

        loop {
            match operation() {
                Ok(result) => {
                    if attempts > 0 {
                        tracing::info!("Operation succeeded after {} retries", attempts);
                    }
                    return Ok(result);
                }
                Err(err) => {
                    attempts += 1;
                    if attempts > self.max_attempts {
                        tracing::warn!("Operation failed after {} attempts", self.max_attempts);
                        return Err(err);
                    }

                    tracing::debug!(
                        "Retry attempt {}/{}, waiting {}ms",
                        attempts,
                        self.max_attempts,
                        delay_ms
                    );

                    thread::sleep(Duration::from_millis(delay_ms));

                    // Exponential backoff: double the delay, capped at max
                    delay_ms = (delay_ms * 2).min(self.max_delay_ms);
                }
            }
        }
    }

    /// Execute an operation with retry logic, passing attempt number to operation
    ///
    /// Useful when the operation needs to know which attempt it is
    pub fn execute_with_attempt<T, E, F>(&self, mut operation: F) -> Result<T, E>
    where
        F: FnMut(u32) -> Result<T, E>,
    {
        let mut attempts = 0;
        let mut delay_ms = self.initial_delay_ms;

        loop {
            match operation(attempts) {
                Ok(result) => {
                    if attempts > 0 {
                        tracing::info!("Operation succeeded after {} retries", attempts);
                    }
                    return Ok(result);
                }
                Err(err) => {
                    attempts += 1;
                    if attempts > self.max_attempts {
                        tracing::warn!("Operation failed after {} attempts", self.max_attempts);
                        return Err(err);
                    }

                    tracing::debug!(
                        "Retry attempt {}/{}, waiting {}ms",
                        attempts,
                        self.max_attempts,
                        delay_ms
                    );

                    thread::sleep(Duration::from_millis(delay_ms));
                    delay_ms = (delay_ms * 2).min(self.max_delay_ms);
                }
            }
        }
    }

    /// Check if a specific error should be retried
    ///
    /// Currently retries all errors, but can be customized per error type
    pub fn should_retry<E>(&self, _error: &E) -> bool {
        true // Retry all errors by default
    }
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self::standard()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::Arc;

    #[test]
    fn test_none_policy() {
        let policy = RetryPolicy::none();
        let counter = Arc::new(AtomicU32::new(0));
        let counter_clone = Arc::clone(&counter);

        let result: Result<(), &str> = policy.execute(|| {
            counter_clone.fetch_add(1, Ordering::SeqCst);
            Err("always fails")
        });

        assert!(result.is_err());
        assert_eq!(counter.load(Ordering::SeqCst), 1); // Only one attempt
    }

    #[test]
    fn test_standard_policy_success_first_try() {
        let policy = RetryPolicy::standard();
        let counter = Arc::new(AtomicU32::new(0));
        let counter_clone = Arc::clone(&counter);

        let result: Result<u32, &str> = policy.execute(|| {
            counter_clone.fetch_add(1, Ordering::SeqCst);
            Ok(42)
        });

        assert_eq!(result.unwrap(), 42);
        assert_eq!(counter.load(Ordering::SeqCst), 1); // Only one attempt needed
    }

    #[test]
    fn test_standard_policy_success_after_retries() {
        let policy = RetryPolicy::standard();
        let counter = Arc::new(AtomicU32::new(0));
        let counter_clone = Arc::clone(&counter);

        let result: Result<u32, &str> = policy.execute(|| {
            let count = counter_clone.fetch_add(1, Ordering::SeqCst);
            if count < 2 {
                Err("transient failure")
            } else {
                Ok(42)
            }
        });

        assert_eq!(result.unwrap(), 42);
        assert_eq!(counter.load(Ordering::SeqCst), 3); // Succeeded on 3rd attempt
    }

    #[test]
    fn test_standard_policy_all_retries_exhausted() {
        let policy = RetryPolicy::standard();
        let counter = Arc::new(AtomicU32::new(0));
        let counter_clone = Arc::clone(&counter);

        let result: Result<(), &str> = policy.execute(|| {
            counter_clone.fetch_add(1, Ordering::SeqCst);
            Err("always fails")
        });

        assert!(result.is_err());
        assert_eq!(counter.load(Ordering::SeqCst), 4); // Initial + 3 retries
    }

    #[test]
    fn test_execute_with_attempt() {
        let policy = RetryPolicy::standard();
        let attempts_seen = Arc::new(AtomicU32::new(0));
        let attempts_clone = Arc::clone(&attempts_seen);

        let result: Result<u32, &str> = policy.execute_with_attempt(|attempt| {
            attempts_clone.store(attempt, Ordering::SeqCst);
            if attempt < 2 {
                Err("not yet")
            } else {
                Ok(attempt)
            }
        });

        assert_eq!(result.unwrap(), 2);
        assert_eq!(attempts_seen.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn test_aggressive_policy() {
        let policy = RetryPolicy::aggressive();
        assert_eq!(policy.max_attempts, 5);
        assert_eq!(policy.initial_delay_ms, 200);
        assert_eq!(policy.max_delay_ms, 1000);
    }

    #[test]
    fn test_custom_policy() {
        let policy = RetryPolicy::custom(10, 50, 500);
        assert_eq!(policy.max_attempts, 10);
        assert_eq!(policy.initial_delay_ms, 50);
        assert_eq!(policy.max_delay_ms, 500);
    }

    #[test]
    fn test_exponential_backoff() {
        let policy = RetryPolicy::custom(3, 100, 500);
        let counter = Arc::new(AtomicU32::new(0));
        let counter_clone = Arc::clone(&counter);

        let start = std::time::Instant::now();
        let _result: Result<(), &str> = policy.execute(|| {
            counter_clone.fetch_add(1, Ordering::SeqCst);
            Err("always fails")
        });

        let elapsed = start.elapsed();

        // Should have delays: 100ms, 200ms, 400ms = 700ms total (approximately)
        assert!(elapsed >= Duration::from_millis(650)); // Allow some variance
        assert!(elapsed < Duration::from_millis(850));
    }

    #[test]
    fn test_max_delay_cap() {
        let policy = RetryPolicy::custom(4, 100, 300);
        let counter = Arc::new(AtomicU32::new(0));
        let counter_clone = Arc::clone(&counter);

        let start = std::time::Instant::now();
        let _result: Result<(), &str> = policy.execute(|| {
            counter_clone.fetch_add(1, Ordering::SeqCst);
            Err("always fails")
        });

        let elapsed = start.elapsed();

        // Delays: 100ms, 200ms, 300ms (capped), 300ms (capped) = 900ms total
        assert!(elapsed >= Duration::from_millis(850));
        assert!(elapsed < Duration::from_millis(1050));
    }

    #[test]
    fn test_should_retry() {
        let policy = RetryPolicy::standard();
        assert!(policy.should_retry(&"any error"));
        assert!(policy.should_retry(&42));
    }
}
