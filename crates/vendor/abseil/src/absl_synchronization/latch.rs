//! Latch - blocks until a specified count reaches zero.

use core::sync::atomic::{AtomicUsize, Ordering};
use core::time::Duration;

/// A latch that blocks until a specified count reaches zero.
///
/// Unlike a blocking counter, a latch can only be counted down once.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_synchronization::Latch;
///
/// let latch = Latch::new(3);
///
/// // In threads:
/// // latch.count_down(); // Called when each thread completes
///
/// // Wait for all threads:
/// latch.wait(); // Blocks until count reaches 0
/// ```
#[derive(Debug)]
pub struct Latch {
    count: AtomicUsize,
}

impl Latch {
    /// Creates a new latch with the specified count.
    ///
    /// # Panics
    ///
    /// Panics if `count` is 0.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use abseil::absl_synchronization::Latch;
    ///
    /// let latch = Latch::new(3);
    /// ```
    pub fn new(count: usize) -> Self {
        assert!(count > 0, "Latch::new: count must be greater than 0");
        Self {
            count: AtomicUsize::new(count),
        }
    }

    /// Decrements the count by one.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use abseil::absl_synchronization::Latch;
    ///
    /// let latch = Latch::new(3);
    /// latch.count_down();
    /// ```
    #[inline]
    pub fn count_down(&self) {
        self.count.fetch_sub(1, Ordering::Release);
    }

    /// Blocks until the count reaches zero.
    ///
    /// Uses exponential backoff to avoid CPU exhaustion while waiting.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use abseil::absl_synchronization::Latch;
    ///
    /// let latch = Latch::new(3);
    /// // In each thread: latch.count_down();
    /// latch.wait(); // Blocks until count reaches 0
    /// ```
    pub fn wait(&self) {
        let mut backoff = 1;
        while self.count.load(Ordering::Acquire) > 0 {
            if backoff <= 64 {
                for _ in 0..backoff {
                    if self.count.load(Ordering::Acquire) == 0 {
                        return;
                    }
                    core::hint::spin_loop();
                }
                backoff *= 2;
            } else {
                // After sufficient spinning, yield to other threads
                #[cfg(feature = "std")]
                std::thread::yield_now();
                // Small sleep to prevent busy-waiting
                #[cfg(feature = "std")]
                std::thread::sleep(Duration::from_micros(100));
            }
        }
    }

    /// Blocks until the count reaches zero or the timeout expires.
    ///
    /// Returns `true` if the count reached zero, `false` if timeout occurred.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use abseil::absl_synchronization::Latch;
    /// use core::time::Duration;
    ///
    /// let latch = Latch::new(3);
    /// // In each thread: latch.count_down();
    /// let result = latch.wait_timeout(Duration::from_millis(100));
    /// ```
    pub fn wait_timeout(&self, timeout: Duration) -> bool {
        let start = #[cfg(feature = "std")]
        {
            std::time::Instant::now()
        };
        #[cfg(not(feature = "std"))]
        let mut spins = 0usize;

        loop {
            if self.count.load(Ordering::Acquire) == 0 {
                return true;
            }

            #[cfg(feature = "std")]
            if start.elapsed() >= timeout {
                return false;
            }

            #[cfg(not(feature = "std"))]
            {
                spins += 1;
                if spins > timeout.as_millis() as usize * 10 {
                    return false;
                }
            }

            core::hint::spin_loop();
        }
    }

    /// Returns the current count.
    #[inline]
    pub fn count(&self) -> usize {
        self.count.load(Ordering::Acquire)
    }

    /// Returns true if the count has reached zero.
    #[inline]
    pub fn is_ready(&self) -> bool {
        self.count() == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_latch() {
        let latch = Latch::new(3);
        assert_eq!(latch.count(), 3);
        assert!(!latch.is_ready());

        latch.count_down();
        assert_eq!(latch.count(), 2);

        latch.count_down();
        assert_eq!(latch.count(), 1);

        latch.count_down();
        assert_eq!(latch.count(), 0);
        assert!(latch.is_ready());
    }

    #[test]
    fn test_wait_already_ready() {
        let latch = Latch::new(1);
        latch.count_down();
        latch.wait(); // Should return immediately
        assert!(latch.is_ready());
    }

    #[test]
    fn test_wait_timeout_not_ready() {
        let latch = Latch::new(3);
        latch.count_down();
        latch.count_down();
        // Still has count 1, should timeout
        #[cfg(feature = "std")]
        {
            let result = latch.wait_timeout(Duration::from_millis(10));
            assert!(!result);
        }
    }

    #[test]
    fn test_wait_timeout_ready() {
        let latch = Latch::new(2);
        latch.count_down();
        latch.count_down();
        // Count is now 0, should return true immediately
        let result = latch.wait_timeout(Duration::from_millis(10));
        assert!(result);
    }
}
