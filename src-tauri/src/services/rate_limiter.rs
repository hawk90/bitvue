//! Rate Limiter Service
//!
//! Prevents denial-of-service through resource exhaustion by limiting
//! the frequency of expensive operations like frame decoding and
//! quality metric calculations.

use std::sync::Mutex;
use std::time::{Duration, Instant};

/// Configuration for rate limiting
#[derive(Debug, Clone)]
pub struct RateLimitConfig {
    /// Minimum interval between requests (e.g., 50ms = 20 requests/sec)
    pub min_interval: Duration,
    /// Maximum number of requests in a burst (0 = no limit)
    pub burst_limit: usize,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            // Default: 20 requests per second (50ms between requests)
            min_interval: Duration::from_millis(50),
            burst_limit: 10,
        }
    }
}

/// Rate limiter for expensive operations
pub struct RateLimiter {
    config: RateLimitConfig,
    last_request: Mutex<Instant>,
    burst_count: Mutex<usize>,
    burst_window_start: Mutex<Instant>,
}

impl RateLimiter {
    /// Create a new rate limiter with default config
    pub fn new() -> Self {
        Self::with_config(RateLimitConfig::default())
    }

    /// Create a new rate limiter with custom config
    pub fn with_config(config: RateLimitConfig) -> Self {
        let now = Instant::now();
        Self {
            config,
            last_request: Mutex::new(now.checked_sub(Duration::from_secs(1)).unwrap_or(now)),
            burst_count: Mutex::new(0),
            burst_window_start: Mutex::new(now),
        }
    }

    /// Check if a request should be allowed
    /// Returns Ok(()) if allowed, Err with elapsed time if rate limited
    pub fn check_rate_limit(&self) -> Result<(), Duration> {
        let now = Instant::now();

        // Check minimum interval
        {
            let mut last_request = self.last_request.lock()
                .map_err(|_| Duration::from_secs(1))?;
            let elapsed = now.duration_since(*last_request);

            if elapsed < self.config.min_interval {
                let wait_time = self.config.min_interval.saturating_sub(elapsed);
                return Err(wait_time);
            }

            *last_request = now;
        }

        // Check burst limit
        if self.config.burst_limit > 0 {
            let mut burst_count = self.burst_count.lock()
                .map_err(|_| Duration::from_secs(1))?;
            let mut burst_window_start = self.burst_window_start.lock()
                .map_err(|_| Duration::from_secs(1))?;

            // Reset burst counter if window has expired (1 second)
            let window_duration = Duration::from_secs(1);
            if now.duration_since(*burst_window_start) > window_duration {
                *burst_count = 0;
                *burst_window_start = now;
            }

            if *burst_count >= self.config.burst_limit {
                let wait_time = window_duration.saturating_sub(now.duration_since(*burst_window_start));
                return Err(wait_time);
            }

            *burst_count += 1;
        }

        Ok(())
    }

    /// Reset the burst counter (useful after a long idle period)
    #[allow(dead_code)]
    pub fn reset(&self) {
        // Mutex poisoning is unlikely here but handle gracefully
        if let Ok(mut burst_count) = self.burst_count.lock() {
            *burst_count = 0;
        }
        if let Ok(mut burst_window_start) = self.burst_window_start.lock() {
            *burst_window_start = Instant::now();
        }
    }

    /// Get current configuration
    #[allow(dead_code)]
    pub fn config(&self) -> &RateLimitConfig {
        &self.config
    }
}

impl Default for RateLimiter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rate_limit_enforces_minimum_interval() {
        let limiter = RateLimiter::with_config(RateLimitConfig {
            min_interval: Duration::from_millis(100),
            burst_limit: 0,
        });

        // First request should succeed
        assert!(limiter.check_rate_limit().is_ok());

        // Immediate second request should fail
        let result = limiter.check_rate_limit();
        assert!(result.is_err());
        let wait_time = result.unwrap_err();
        assert!(wait_time.as_millis() > 0 && wait_time.as_millis() <= 100);
    }

    #[test]
    fn test_rate_limit_allows_after_interval() {
        let limiter = RateLimiter::with_config(RateLimitConfig {
            min_interval: Duration::from_millis(50),
            burst_limit: 0,
        });

        assert!(limiter.check_rate_limit().is_ok());
        std::thread::sleep(Duration::from_millis(60));
        assert!(limiter.check_rate_limit().is_ok());
    }

    #[test]
    fn test_burst_limit() {
        let limiter = RateLimiter::with_config(RateLimitConfig {
            min_interval: Duration::from_millis(10), // Very short interval
            burst_limit: 3, // Allow 3 requests in burst
        });

        // First 3 requests should succeed
        assert!(limiter.check_rate_limit().is_ok());
        assert!(limiter.check_rate_limit().is_ok());
        assert!(limiter.check_rate_limit().is_ok());

        // 4th request should be rate limited
        assert!(limiter.check_rate_limit().is_err());
    }
}
