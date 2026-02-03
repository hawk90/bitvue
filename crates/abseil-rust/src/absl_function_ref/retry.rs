//! Retry logic utilities - RetryConfig, retry, retry_n

/// Configuration for retry behavior.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RetryConfig {
    pub max_attempts: usize,
    pub base_delay_ms: u64,
    pub max_delay_ms: u64,
    pub backoff_multiplier: f64,
}

impl Default for RetryConfig {
    #[inline]
    fn default() -> Self {
        Self {
            max_attempts: 3,
            base_delay_ms: 100,
            max_delay_ms: 5000,
            backoff_multiplier: 2.0,
        }
    }
}

impl RetryConfig {
    /// Creates a new retry config.
    #[inline]
    pub const fn new() -> Self {
        Self {
            max_attempts: 3,
            base_delay_ms: 100,
            max_delay_ms: 5000,
            backoff_multiplier: 2.0,
        }
    }

    /// Sets the maximum number of attempts.
    #[inline]
    pub const fn max_attempts(mut self, attempts: usize) -> Self {
        self.max_attempts = attempts;
        self
    }

    /// Sets the base delay in milliseconds.
    #[inline]
    pub const fn base_delay(mut self, delay: u64) -> Self {
        self.base_delay_ms = delay;
        self
    }
}

/// Retries a function until it succeeds or max attempts is reached.
///
/// Note: This is a simplified version. A production version would use
/// std::thread::sleep for delays.
#[inline]
pub fn retry<T, E, F>(mut f: F, _config: RetryConfig) -> Result<T, E>
where
    F: FnMut() -> Result<T, E>,
{
    let mut attempts = 0;
    let max_attempts = _config.max_attempts;

    loop {
        match f() {
            Ok(result) => return Ok(result),
            Err(e) if attempts >= max_attempts - 1 => return Err(e),
            Err(_) => {
                attempts += 1;
                // In a real implementation, we'd sleep here
            }
        }
    }
}

/// Retries a function a fixed number of times.
#[inline]
pub fn retry_n<T, E, F>(mut f: F, n: usize) -> Result<T, E>
where
    F: FnMut() -> Result<T, E>,
{
    for _ in 0..n {
        match f() {
            Ok(result) => return Ok(result),
            Err(_) => continue,
        }
    }
    f()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_retry_n() {
        let mut attempts = 0;
        let result = retry_n(
            || {
                attempts += 1;
                if attempts < 3 {
                    Err("not yet")
                } else {
                    Ok(42)
                }
            },
            5,
        );

        assert_eq!(result, Ok(42));
        assert_eq!(attempts, 3);
    }

    #[test]
    fn test_retry_n_fail() {
        let result = retry_n(|| Err::<i32, _>("failed"), 3);
        assert_eq!(result, Err("failed"));
    }

    #[test]
    fn test_retry_config() {
        let config = RetryConfig::new()
            .max_attempts(5)
            .base_delay(200);

        assert_eq!(config.max_attempts, 5);
        assert_eq!(config.base_delay_ms, 200);
    }

    #[test]
    fn test_retry_config_default() {
        let config = RetryConfig::default();
        assert_eq!(config.max_attempts, 3);
        assert_eq!(config.base_delay_ms, 100);
    }
}
