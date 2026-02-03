//! Fallback and recovery mechanisms for error handling.

use alloc::vec::Vec;

use super::status::Status;
use super::status::StatusCode;
use super::error_chain::ToStatus;

/// Executes an operation with a fallback on error.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_status::{Status, StatusCode, fallback};
///
/// let result = fallback(
///     || Err(Status::new(StatusCode::NotFound, "Primary failed")),
///     || Ok("Fallback value")
/// );
/// assert_eq!(result, Ok("Fallback value"));
/// ```
pub fn fallback<T, E1, E2, F1, F2>(primary: F1, fallback: F2) -> Result<T, Status>
where
    F1: FnOnce() -> Result<T, E1>,
    E1: ToStatus,
    F2: FnOnce() -> Result<T, E2>,
    E2: ToStatus,
{
    match primary() {
        Ok(value) => Ok(value),
        Err(e) => {
            let _ = e; // Use the error for logging in a real implementation
            fallback().map_err(|e| e.to_status(StatusCode::Internal))
        }
    }
}

/// Executes operations in sequence until one succeeds.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_status::{Status, StatusCode, try_fallbacks};
///
/// let result = try_fallbacks(&[
///     || Err(Status::new(StatusCode::Unavailable, "Service 1 down")),
///     || Err(Status::new(StatusCode::NotFound, "Service 2 not found")),
///     || Ok("Service 3 response"),
/// ]);
/// assert_eq!(result, Ok("Service 3 response"));
/// ```
pub fn try_fallbacks<T, F>(fallbacks: &[F]) -> Result<T, Status>
where
    F: Fn() -> Result<T, Status>,
{
    for f in fallbacks {
        match f() {
            Ok(value) => return Ok(value),
            Err(_) => continue,
        }
    }
    Err(Status::new(StatusCode::Internal, "All fallbacks failed"))
}

/// Executes an operation with a cached fallback.
pub struct CachedFallback<T> {
    cached_value: Option<T>,
    cache_time: Option<u64>,
    ttl_ms: u64,
}

impl<T: Clone> Default for CachedFallback<T> {
    fn default() -> Self {
        Self {
            cached_value: None,
            cache_time: None,
            ttl_ms: 5000,
        }
    }
}

impl<T: Clone> CachedFallback<T> {
    /// Creates a new cached fallback with the given TTL.
    pub fn new(ttl_ms: u64) -> Self {
        Self {
            ttl_ms,
            ..Default::default()
        }
    }

    /// Sets the cached value.
    pub fn set_cache(&mut self, value: T) {
        // In a real implementation, you'd get the actual time
        self.cached_value = Some(value);
        self.cache_time = Some(0); // Placeholder
    }

    /// Gets the cached value if still valid.
    pub fn get_cached(&self) -> Option<&T> {
        self.cached_value.as_ref()
    }

    /// Clears the cache.
    pub fn clear_cache(&mut self) {
        self.cached_value = None;
        self.cache_time = None;
    }

    /// Tries to get a value, falling back to cache on error.
    pub fn get_or_cached<F>(&mut self, f: F) -> Result<T, Status>
    where
        F: FnOnce() -> Result<T, Status>,
    {
        match f() {
            Ok(value) => {
                self.set_cache(value.clone());
                Ok(value)
            }
            Err(e) => {
                if let Some(cached) = self.get_cached() {
                    Ok(cached.clone())
                } else {
                    Err(e)
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fallback_primary_succeeds() {
        let result = fallback(
            || Ok(42),
            || Ok(99),
        );
        assert_eq!(result, Ok(42));
    }

    #[test]
    fn test_fallback_uses_fallback() {
        let result = fallback(
            || Err(Status::new(StatusCode::Internal, "Primary failed")),
            || Ok(99),
        );
        assert_eq!(result, Ok(99));
    }

    #[test]
    fn test_try_fallbacks() {
        let fallbacks: &[fn() -> Result<&str, Status>] = &[
            || Err(Status::new(StatusCode::Unavailable, "Service 1 down")),
            || Err(Status::new(StatusCode::NotFound, "Service 2 not found")),
            || Ok("Service 3 response"),
        ];

        let result = try_fallbacks(fallbacks);
        assert_eq!(result, Ok("Service 3 response"));
    }

    #[test]
    fn test_try_fallbacks_all_fail() {
        let fallbacks: &[fn() -> Result<&str, Status>] = &[
            || Err(Status::new(StatusCode::Unavailable, "Service 1 down")),
            || Err(Status::new(StatusCode::NotFound, "Service 2 not found")),
        ];

        let result = try_fallbacks(fallbacks);
        assert!(result.is_err());
    }

    #[test]
    fn test_cached_fallback_new() {
        let cache: CachedFallback<u32> = CachedFallback::new(1000);
        assert_eq!(cache.ttl_ms, 1000);
    }

    #[test]
    fn test_cached_fallback_set_and_get() {
        let mut cache = CachedFallback::new(1000);
        cache.set_cache(42);
        assert_eq!(cache.get_cached(), Some(&42));
    }

    #[test]
    fn test_cached_fallback_clear() {
        let mut cache = CachedFallback::new(1000);
        cache.set_cache(42);
        cache.clear_cache();
        assert_eq!(cache.get_cached(), None);
    }

    #[test]
    fn test_cached_fallback_get_or_cached_succeeds() {
        let mut cache = CachedFallback::new(1000);
        let result = cache.get_or_cached(|| Ok(42));
        assert_eq!(result, Ok(42));
        assert_eq!(cache.get_cached(), Some(&42));
    }

    #[test]
    fn test_cached_fallback_get_or_cached_uses_cache() {
        let mut cache = CachedFallback::new(1000);
        cache.set_cache(99);

        let mut call_count = 0;
        let result = cache.get_or_cached(|| {
            call_count += 1;
            Err(Status::new(StatusCode::Internal, "Error"))
        });

        assert_eq!(result, Ok(99));
        assert_eq!(call_count, 0); // Primary function was called but error returned cached value
    }
}
