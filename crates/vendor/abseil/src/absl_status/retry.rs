//! Retry policies and backoff strategies for transient error handling.

use alloc::vec::Vec;

use super::categories::StatusCode;
use super::status::Status;

/// Backoff strategy for retries.
#[derive(Clone, Debug)]
pub enum BackoffStrategy {
    /// No delay between retries.
    Immediate,
    /// Fixed delay between retries.
    Fixed {
        /// Delay in milliseconds.
        delay_ms: u64,
    },
    /// Exponential backoff with optional jitter.
    Exponential {
        /// Initial delay in milliseconds.
        initial_delay_ms: u64,
        /// Maximum delay in milliseconds.
        max_delay_ms: u64,
        /// Multiplier for each retry.
        multiplier: f64,
        /// Whether to add jitter.
        jitter: bool,
    },
    /// Linear backoff.
    Linear {
        /// Initial delay in milliseconds.
        initial_delay_ms: u64,
        /// Increment in milliseconds per retry.
        increment_ms: u64,
        /// Maximum delay in milliseconds.
        max_delay_ms: u64,
    },
}

impl Default for BackoffStrategy {
    fn default() -> Self {
        BackoffStrategy::Exponential {
            initial_delay_ms: 100,
            max_delay_ms: 5000,
            multiplier: 2.0,
            jitter: true,
        }
    }
}

impl BackoffStrategy {
    /// Calculates the delay for a given retry attempt (0-indexed).
    pub fn delay_ms(&self, attempt: u32) -> u64 {
        match self {
            BackoffStrategy::Immediate => 0,
            BackoffStrategy::Fixed { delay_ms } => *delay_ms,
            BackoffStrategy::Exponential {
                initial_delay_ms,
                max_delay_ms,
                multiplier,
                jitter,
            } => {
                let delay = (*initial_delay_ms as f64 * multiplier.powi(attempt as i32)).ceil() as u64;
                let delay = delay.min(*max_delay_ms);
                if *jitter {
                    // Add up to 25% jitter
                    let jitter_amount = delay / 4;
                    let random_add = ((attempt as u64).wrapping_mul(17) % jitter_amount.wrapping_add(1));
                    delay + random_add
                } else {
                    delay
                }
            }
            BackoffStrategy::Linear {
                initial_delay_ms,
                increment_ms,
                max_delay_ms,
            } => {
                let delay = *initial_delay_ms + (*increment_ms * attempt as u64);
                delay.min(*max_delay_ms)
            }
        }
    }

    /// Creates a fixed delay backoff strategy.
    pub fn fixed(delay_ms: u64) -> Self {
        BackoffStrategy::Fixed { delay_ms }
    }

    /// Creates an exponential backoff strategy.
    pub fn exponential(initial_delay_ms: u64, max_delay_ms: u64, multiplier: f64) -> Self {
        BackoffStrategy::Exponential {
            initial_delay_ms,
            max_delay_ms,
            multiplier,
            jitter: false,
        }
    }

    /// Creates an exponential backoff strategy with jitter.
    pub fn exponential_with_jitter(initial_delay_ms: u64, max_delay_ms: u64, multiplier: f64) -> Self {
        BackoffStrategy::Exponential {
            initial_delay_ms,
            max_delay_ms,
            multiplier,
            jitter: true,
        }
    }

    /// Creates a linear backoff strategy.
    pub fn linear(initial_delay_ms: u64, increment_ms: u64, max_delay_ms: u64) -> Self {
        BackoffStrategy::Linear {
            initial_delay_ms,
            increment_ms,
            max_delay_ms,
        }
    }
}

/// Policy for retrying operations.
#[derive(Clone, Debug)]
pub struct RetryPolicy {
    /// Maximum number of retry attempts (0 = no retry).
    pub max_attempts: u32,
    /// Backoff strategy for retries.
    pub backoff: BackoffStrategy,
    /// Which status codes should be retried.
    pub retry_on_codes: Vec<StatusCode>,
    /// Whether to retry on all transient errors.
    pub retry_on_transient: bool,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        RetryPolicy {
            max_attempts: 3,
            backoff: BackoffStrategy::default(),
            retry_on_codes: vec![StatusCode::Unavailable, StatusCode::DeadlineExceeded],
            retry_on_transient: true,
        }
    }
}

impl RetryPolicy {
    /// Creates a new retry policy with the given maximum attempts.
    pub fn new(max_attempts: u32) -> Self {
        Self {
            max_attempts,
            ..Default::default()
        }
    }

    /// Sets the backoff strategy.
    pub fn with_backoff(mut self, backoff: BackoffStrategy) -> Self {
        self.backoff = backoff;
        self
    }

    /// Adds a status code to the retry list.
    pub fn retry_on_code(mut self, code: StatusCode) -> Self {
        self.retry_on_codes.push(code);
        self
    }

    /// Sets whether to retry on transient errors.
    pub fn retry_on_transient(mut self, retry: bool) -> Self {
        self.retry_on_transient = retry;
        self
    }

    /// Returns true if the given status should be retried.
    pub fn should_retry(&self, status: &Status) -> bool {
        if status.is_ok() {
            return false;
        }
        if self.retry_on_codes.contains(&status.code()) {
            return true;
        }
        if self.retry_on_transient && status.is_transient() {
            return true;
        }
        false
    }

    /// Returns the delay in milliseconds before the next retry.
    pub fn delay_before_retry(&self, attempt: u32) -> u64 {
        self.backoff.delay_ms(attempt)
    }
}

/// Result of a retry operation.
#[derive(Clone, Debug)]
pub enum RetryResult<T> {
    /// Operation succeeded.
    Success(T),
    /// Operation failed after all retries.
    Failed(Status),
    /// Operation failed with a permanent error (not retried).
    PermanentError(Status),
}

impl<T> RetryResult<T> {
    /// Returns true if the operation succeeded.
    pub fn is_success(&self) -> bool {
        matches!(self, RetryResult::Success(_))
    }

    /// Returns true if the operation failed.
    pub fn is_failure(&self) -> bool {
        !self.is_success()
    }

    /// Maps the success value if present.
    pub fn map<U, F>(self, f: F) -> RetryResult<U>
    where
        F: FnOnce(T) -> U,
    {
        match self {
            RetryResult::Success(v) => RetryResult::Success(f(v)),
            RetryResult::Failed(s) => RetryResult::Failed(s),
            RetryResult::PermanentError(s) => RetryResult::PermanentError(s),
        }
    }

    /// Converts to a Result.
    pub fn to_result(self) -> Result<T, Status> {
        match self {
            RetryResult::Success(v) => Ok(v),
            RetryResult::Failed(s) => Err(s),
            RetryResult::PermanentError(s) => Err(s),
        }
    }
}

/// Executes an operation with retry according to the policy.
///
/// Note: In a no_std environment without async, this function
/// describes the retry logic but cannot actually sleep between retries.
/// Use the returned delay values to implement actual delays.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_status::{Status, StatusCode, retry_sync, RetryPolicy};
///
/// let mut attempts = 0;
/// let policy = RetryPolicy::new(3);
///
/// let result = retry_sync(|| {
///     attempts += 1;
///     if attempts < 3 {
///         Err(Status::new(StatusCode::Unavailable, "Not ready"))
///     } else {
///         Ok(attempts)
///     }
/// }, &policy);
/// ```
pub fn retry_sync<T, F>(mut operation: F, policy: &RetryPolicy) -> RetryResult<T>
where
    F: FnMut() -> Result<T, Status>,
{
    let mut attempt = 0;

    loop {
        match operation() {
            Ok(value) => return RetryResult::Success(value),
            Err(status) => {
                if attempt >= policy.max_attempts {
                    return RetryResult::Failed(status);
                }
                if !policy.should_retry(&status) {
                    return RetryResult::PermanentError(status);
                }
                // In a real implementation, you would sleep here using:
                // let delay = policy.delay_before_retry(attempt);
                // sleep(Duration::from_millis(delay));
                attempt += 1;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // BackoffStrategy tests
    #[test]
    fn test_backoff_immediate() {
        let strategy = BackoffStrategy::Immediate;
        assert_eq!(strategy.delay_ms(0), 0);
        assert_eq!(strategy.delay_ms(5), 0);
    }

    #[test]
    fn test_backoff_fixed() {
        let strategy = BackoffStrategy::Fixed { delay_ms: 100 };
        assert_eq!(strategy.delay_ms(0), 100);
        assert_eq!(strategy.delay_ms(10), 100);
    }

    #[test]
    fn test_backoff_exponential() {
        let strategy = BackoffStrategy::Exponential {
            initial_delay_ms: 100,
            max_delay_ms: 1000,
            multiplier: 2.0,
            jitter: false,
        };
        assert_eq!(strategy.delay_ms(0), 100);
        assert_eq!(strategy.delay_ms(1), 200);
        assert_eq!(strategy.delay_ms(2), 400);
        assert_eq!(strategy.delay_ms(3), 800);
        assert_eq!(strategy.delay_ms(4), 1000); // Capped at max
    }

    #[test]
    fn test_backoff_linear() {
        let strategy = BackoffStrategy::Linear {
            initial_delay_ms: 100,
            increment_ms: 50,
            max_delay_ms: 300,
        };
        assert_eq!(strategy.delay_ms(0), 100);
        assert_eq!(strategy.delay_ms(1), 150);
        assert_eq!(strategy.delay_ms(2), 200);
        assert_eq!(strategy.delay_ms(5), 300); // Capped at max
    }

    #[test]
    fn test_backoff_helpers() {
        let strategy = BackoffStrategy::fixed(100);
        assert_eq!(strategy.delay_ms(0), 100);

        let strategy = BackoffStrategy::exponential(100, 1000, 2.0);
        assert_eq!(strategy.delay_ms(0), 100);

        let strategy = BackoffStrategy::exponential_with_jitter(100, 1000, 2.0);
        assert!(strategy.delay_ms(0) >= 100);
    }

    // RetryPolicy tests
    #[test]
    fn test_retry_policy_default() {
        let policy = RetryPolicy::default();
        assert_eq!(policy.max_attempts, 3);
        assert!(policy.retry_on_transient);
    }

    #[test]
    fn test_retry_policy_new() {
        let policy = RetryPolicy::new(5);
        assert_eq!(policy.max_attempts, 5);
    }

    #[test]
    fn test_retry_policy_should_retry_transient() {
        let policy = RetryPolicy::new(3).retry_on_transient(true);
        let status = Status::new(StatusCode::Unavailable, "Service down");
        assert!(policy.should_retry(&status));
    }

    #[test]
    fn test_retry_policy_should_retry_specific_code() {
        let policy = RetryPolicy::new(3).retry_on_code(StatusCode::NotFound);
        let status = Status::new(StatusCode::NotFound, "Not found");
        assert!(policy.should_retry(&status));
    }

    #[test]
    fn test_retry_policy_should_not_retry_permanent() {
        let policy = RetryPolicy::new(3);
        let status = Status::new(StatusCode::InvalidArgument, "Bad input");
        assert!(!policy.should_retry(&status));
    }

    #[test]
    fn test_retry_policy_delay_before_retry() {
        let policy = RetryPolicy::new(3).with_backoff(BackoffStrategy::Fixed { delay_ms: 200 });
        assert_eq!(policy.delay_before_retry(0), 200);
        assert_eq!(policy.delay_before_retry(1), 200);
    }

    // RetryResult tests
    #[test]
    fn test_retry_result_is_success() {
        let result: RetryResult<u32> = RetryResult::Success(42);
        assert!(result.is_success());
        assert!(!result.is_failure());
    }

    #[test]
    fn test_retry_result_is_failure() {
        let result: RetryResult<u32> = RetryResult::Failed(Status::new(StatusCode::Internal, "Error"));
        assert!(!result.is_success());
        assert!(result.is_failure());
    }

    #[test]
    fn test_retry_result_map() {
        let result: RetryResult<u32> = RetryResult::Success(42);
        let mapped = result.map(|v| v * 2);
        assert!(matches!(mapped, RetryResult::Success(84)));
    }

    #[test]
    fn test_retry_result_to_result() {
        let result: RetryResult<u32> = RetryResult::Success(42);
        assert_eq!(result.to_result(), Ok(42));

        let result: RetryResult<u32> = RetryResult::Failed(Status::new(StatusCode::Internal, "Error"));
        assert!(result.to_result().is_err());
    }

    // retry_sync tests
    #[test]
    fn test_retry_sync_immediate_success() {
        let policy = RetryPolicy::new(3);
        let mut call_count = 0;
        let result = retry_sync(
            || {
                call_count += 1;
                Ok(42)
            },
            &policy,
        );
        assert!(result.is_success());
        assert_eq!(call_count, 1);
    }

    #[test]
    fn test_retry_sync_eventual_success() {
        let policy = RetryPolicy::new(5);
        let mut attempt = 0;
        let result = retry_sync(
            || {
                attempt += 1;
                if attempt < 3 {
                    Err(Status::new(StatusCode::Unavailable, "Not ready"))
                } else {
                    Ok(attempt)
                }
            },
            &policy,
        );
        assert!(result.is_success());
    }

    #[test]
    fn test_retry_sync_permanent_error() {
        let policy = RetryPolicy::new(5);
        let result = retry_sync(
            || Err(Status::new(StatusCode::NotFound, "Not found")),
            &policy,
        );
        assert!(matches!(result, RetryResult::PermanentError(_)));
    }

    #[test]
    fn test_retry_sync_max_attempts() {
        let policy = RetryPolicy::new(2);
        let mut attempts = 0;
        let result = retry_sync(
            || {
                attempts += 1;
                Err(Status::new(StatusCode::Unavailable, "Always fails"))
            },
            &policy,
        );
        assert!(result.is_failure());
        assert_eq!(attempts, 3); // Initial + 2 retries
    }
}
