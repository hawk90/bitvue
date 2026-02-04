//! Semaphore - a synchronization primitive for controlling concurrent access.

use core::sync::atomic::{AtomicIsize, AtomicUsize, Ordering};
use core::time::Duration;

/// A semaphore that controls concurrent access to a resource.
///
/// A semaphore maintains a set of permits. Threads can acquire permits
/// (decreasing the available count) and release permits (increasing the count).
///
/// # Examples
///
/// ```
/// use abseil::absl_synchronization::Semaphore;
///
/// let sem = Semaphore::new(3); // Max 3 concurrent accesses
///
/// // Acquire a permit
/// let _permit = sem.acquire();
/// // Do work...
/// // Permit is released when dropped
/// ```
pub struct Semaphore {
    permits: AtomicIsize,
}

impl Semaphore {
    /// Creates a new semaphore with the given number of permits.
    ///
    /// # Examples
    ///
    /// ```
    /// use abseil::absl_synchronization::Semaphore;
    ///
    /// let sem = Semaphore::new(5);
    /// ```
    pub const fn new(permits: isize) -> Self {
        Self {
            permits: AtomicIsize::new(permits),
        }
    }

    /// Acquires a permit, blocking until one is available.
    ///
    /// Uses exponential backoff to avoid CPU exhaustion while waiting.
    ///
    /// Returns a permit that will be released when dropped.
    ///
    /// # Examples
    ///
    /// ```
    /// use abseil::absl_synchronization::Semaphore;
    ///
    /// let sem = Semaphore::new(2);
    /// let _permit = sem.acquire();
    /// ```
    pub fn acquire(&self) -> SemaphorePermit<'_> {
        let mut backoff = 1;
        loop {
            let mut current = self.permits.load(Ordering::Acquire);
            if current > 0 {
                match self.permits.compare_exchange_weak(
                    current,
                    current - 1,
                    Ordering::AcqRel,
                ) {
                    Ok(_) => return SemaphorePermit { sem: self },
                    Err(_) => {}
                }
            }

            // Exponential backoff
            if backoff <= 64 {
                for _ in 0..backoff {
                    // Try again inside the backoff loop
                    let current = self.permits.load(Ordering::Acquire);
                    if current > 0 {
                        match self.permits.compare_exchange_weak(
                            current,
                            current - 1,
                            Ordering::AcqRel,
                        ) {
                            Ok(_) => return SemaphorePermit { sem: self },
                            Err(_) => {}
                        }
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

    /// Tries to acquire a permit with a timeout.
    ///
    /// Returns `Some(permit)` if acquired, `None` if timeout occurred.
    ///
    /// # Examples
    ///
    /// ```
    /// use abseil::absl_synchronization::Semaphore;
    /// use core::time::Duration;
    ///
    /// let sem = Semaphore::new(0);
    /// assert!(sem.acquire_timeout(Duration::from_millis(10)).is_none());
    /// ```
    pub fn acquire_timeout(&self, timeout: Duration) -> Option<SemaphorePermit<'_>> {
        let start = #[cfg(feature = "std")]
        {
            std::time::Instant::now()
        };
        #[cfg(not(feature = "std"))]
        let mut spins = 0usize;

        loop {
            let current = self.permits.load(Ordering::Acquire);
            if current > 0 {
                match self.permits.compare_exchange_weak(
                    current,
                    current - 1,
                    Ordering::AcqRel,
                ) {
                    Ok(_) => return Some(SemaphorePermit { sem: self }),
                    Err(_) => {}
                }
            }

            // Check timeout
            #[cfg(feature = "std")]
            if start.elapsed() >= timeout {
                return None;
            }
            #[cfg(not(feature = "std"))]
            {
                spins += 1;
                if spins > timeout.as_millis() as usize * 10 {
                    return None;
                }
            }

            core::hint::spin_loop();
        }
    }

    /// Tries to acquire a permit without blocking.
    ///
    /// Returns None if no permits are available.
    ///
    /// # Examples
    ///
    /// ```
    /// use abseil::absl_synchronization::Semaphore;
    ///
    /// let sem = Semaphore::new(0);
    /// assert!(sem.try_acquire().is_none());
    /// ```
    pub fn try_acquire(&self) -> Option<SemaphorePermit<'_>> {
        loop {
            let current = self.permits.load(Ordering::Acquire);
            if current <= 0 {
                return None;
            }
            match self.permits.compare_exchange_weak(
                current,
                current - 1,
                Ordering::AcqRel,
            ) {
                Ok(_) => return Some(SemaphorePermit { sem: self }),
                Err(_) => {}
            }
        }
    }

    /// Releases a permit, increasing the available count.
    ///
    /// # Examples
    ///
    /// ```
    /// use abseil::absl_synchronization::Semaphore;
    ///
    /// let sem = Semaphore::new(0);
    /// sem.release();
    /// assert_eq!(sem.available_permits(), 1);
    /// ```
    pub fn release(&self) {
        self.permits.fetch_add(1, Ordering::Release);
    }

    /// Returns the number of available permits.
    ///
    /// # Examples
    ///
    /// ```
    /// use abseil::absl_synchronization::Semaphore;
    ///
    /// let sem = Semaphore::new(5);
    /// assert_eq!(sem.available_permits(), 5);
    /// ```
    pub fn available_permits(&self) -> isize {
        self.permits.load(Ordering::Acquire)
    }

    /// Drains all available permits.
    ///
    /// # Examples
    ///
    /// ```
    /// use abseil::absl_synchronization::Semaphore;
    ///
    /// let sem = Semaphore::new(5);
    /// sem.drain();
    /// assert_eq!(sem.available_permits(), 0);
    /// ```
    pub fn drain(&self) {
        self.permits.store(0, Ordering::Release);
    }

    /// Adds the specified number of permits.
    ///
    /// # Examples
    ///
    /// ```
    /// use abseil::absl_synchronization::Semaphore;
    ///
    /// let sem = Semaphore::new(2);
    /// sem.add_permits(3);
    /// assert_eq!(sem.available_permits(), 5);
    /// ```
    ///
    /// # Note
    ///
    /// This method uses saturating arithmetic to prevent overflow.
    /// If adding permits would overflow, the count is capped at `isize::MAX`.
    pub fn add_permits(&self, n: isize) {
        // Use saturating arithmetic to prevent overflow
        let mut current = self.permits.load(Ordering::Acquire);
        loop {
            let new = current.saturating_add(n);
            match self.permits.compare_exchange_weak(current, new, Ordering::AcqRel, Ordering::Acquire) {
                Ok(_) => break,
                Err(actual) => current = actual,
            }
        }
    }

    /// Reduces the available permits by the specified amount.
    ///
    /// # Examples
    ///
    /// ```
    /// use abseil::absl_synchronization::Semaphore;
    ///
    /// let sem = Semaphore::new(5);
    /// sem.reduce_permits(2);
    /// assert_eq!(sem.available_permits(), 3);
    /// ```
    pub fn reduce_permits(&self, n: isize) -> isize {
        let mut current = self.permits.load(Ordering::Acquire);
        loop {
            let new = current.saturating_sub(n);
            match self.permits.compare_exchange_weak(current, new, Ordering::AcqRel) {
                Ok(_) => return new,
                Err(_) => current = self.permits.load(Ordering::Acquire),
            }
        }
    }
}

/// RAII guard that releases a semaphore permit when dropped.
pub struct SemaphorePermit<'a> {
    sem: &'a Semaphore,
}

impl<'a> Drop for SemaphorePermit<'a> {
    fn drop(&mut self) {
        self.sem.release();
    }
}

/// A counting semaphore that tracks the number of waiters.
///
/// This extends the basic semaphore with the ability to track how many
/// threads are waiting for permits.
#[derive(Debug)]
pub struct CountingSemaphore {
    inner: Semaphore,
    waiters: AtomicUsize,
}

impl CountingSemaphore {
    /// Creates a new counting semaphore.
    pub const fn new(permits: usize) -> Self {
        Self {
            inner: Semaphore::new(permits as isize),
            waiters: AtomicUsize::new(0),
        }
    }

    /// Acquires a permit.
    pub fn acquire(&self) -> SemaphorePermit<'_> {
        self.waiters.fetch_add(1, Ordering::Relaxed);
        let permit = self.inner.acquire();
        self.waiters.fetch_sub(1, Ordering::Relaxed);
        permit
    }

    /// Tries to acquire a permit with a timeout.
    pub fn acquire_timeout(&self, timeout: Duration) -> Option<SemaphorePermit<'_>> {
        self.waiters.fetch_add(1, Ordering::Relaxed);
        let permit = self.inner.acquire_timeout(timeout);
        if permit.is_some() {
            self.waiters.fetch_sub(1, Ordering::Relaxed);
        } else {
            self.waiters.fetch_sub(1, Ordering::Relaxed);
        }
        permit
    }

    /// Returns the number of available permits.
    pub fn available_permits(&self) -> usize {
        self.inner.available_permits() as usize
    }

    /// Returns the number of waiters.
    pub fn waiter_count(&self) -> usize {
        self.waiters.load(Ordering::Relaxed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_semaphore_acquire() {
        let sem = Semaphore::new(2);

        let _p1 = sem.acquire();
        let _p2 = sem.acquire();

        assert_eq!(sem.available_permits(), 0);
    }

    #[test]
    fn test_semaphore_release() {
        let sem = Semaphore::new(0);
        sem.release();
        assert_eq!(sem.available_permits(), 1);
    }

    #[test]
    fn test_semaphore_drop() {
        let sem = Semaphore::new(1);
        {
            let _permit = sem.acquire();
            assert_eq!(sem.available_permits(), 0);
        }
        assert_eq!(sem.available_permits(), 1);
    }

    #[test]
    fn test_try_acquire() {
        let sem = Semaphore::new(1);
        assert!(sem.try_acquire().is_some());
        assert!(sem.try_acquire().is_none());
    }

    #[test]
    fn test_add_permits() {
        let sem = Semaphore::new(2);
        sem.add_permits(3);
        assert_eq!(sem.available_permits(), 5);
    }

    #[test]
    fn test_reduce_permits() {
        let sem = Semaphore::new(5);
        let remaining = sem.reduce_permits(2);
        assert_eq!(remaining, 3);
    }

    #[test]
    fn test_counting_semaphore() {
        let sem = CountingSemaphore::new(2);
        assert_eq!(sem.available_permits(), 2);
        assert_eq!(sem.waiter_count(), 0);

        let _p = sem.acquire();
        assert_eq!(sem.waiter_count(), 0); // Acquired immediately
        assert_eq!(sem.available_permits(), 1);
    }

    #[test]
    #[cfg(feature = "std")]
    fn test_acquire_timeout_success() {
        let sem = Semaphore::new(1);
        let result = sem.acquire_timeout(Duration::from_millis(10));
        assert!(result.is_some());
    }

    #[test]
    #[cfg(feature = "std")]
    fn test_acquire_timeout_fail() {
        let sem = Semaphore::new(0);
        let result = sem.acquire_timeout(Duration::from_millis(10));
        assert!(result.is_none());
    }

    #[test]
    fn test_acquire_timeout_immediate() {
        let sem = Semaphore::new(1);
        // Already has permit, should succeed immediately
        let result = sem.acquire_timeout(Duration::from_millis(10));
        assert!(result.is_some());
    }
}
