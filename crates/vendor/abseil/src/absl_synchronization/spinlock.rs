//! Spinlock primitive that uses busy-waiting.

use core::sync::atomic::{AtomicBool, Ordering};

/// A spinlock is a primitive lock that uses busy-waiting.
///
/// This is useful for very short critical sections where the overhead
/// of a mutex might be too high. Spinlocks should only be used when
/// you expect the wait time to be very short.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_synchronization::Spinlock;
///
/// let lock = Spinlock::new();
/// {
///     let _guard = lock.lock();
///     // Critical section
/// }
/// ```
#[derive(Debug)]
pub struct Spinlock {
    locked: AtomicBool,
}

impl Spinlock {
    /// Creates a new unlocked spinlock.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use abseil::absl_synchronization::Spinlock;
    ///
    /// let lock = Spinlock::new();
    /// ```
    #[inline]
    pub const fn new() -> Self {
        Self {
            locked: AtomicBool::new(false),
        }
    }

    /// Attempts to acquire the lock without blocking.
    ///
    /// Returns true if the lock was acquired, false if it was already locked.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use abseil::absl_synchronization::Spinlock;
    ///
    /// let lock = Spinlock::new();
    /// if lock.try_lock() {
    ///     // Lock acquired, do work
    ///     lock.unlock();
    /// }
    /// ```
    #[inline]
    pub fn try_lock(&self) -> bool {
        !self.locked.swap(true, Ordering::Acquire)
    }

    /// Acquires the lock, spinning until it becomes available.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use abseil::absl_synchronization::Spinlock;
    ///
    /// let lock = Spinlock::new();
    /// let guard = lock.lock();
    /// // Critical section
    /// drop(guard); // Lock released automatically
    /// ```
    #[inline]
    pub fn lock(&self) -> SpinlockGuard<'_> {
        while !self.try_lock() {
            core::hint::spin_loop();
        }
        SpinlockGuard { lock: self }
    }

    /// Releases the lock.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use abseil::absl_synchronization::Spinlock;
    ///
    /// let lock = Spinlock::new();
    /// let guard = lock.lock();
    /// guard.release();
    /// ```
    #[inline]
    pub fn unlock(&self) {
        self.locked.store(false, Ordering::Release);
    }

    /// Returns true if the lock is currently held.
    #[inline]
    pub fn is_locked(&self) -> bool {
        self.locked.load(Ordering::Acquire)
    }
}

/// A guard that releases the spinlock when dropped.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_synchronization::Spinlock;
///
/// let lock = Spinlock::new();
/// {
///     let _guard = lock.lock();
///     // Lock is held
/// } // Lock is released automatically
/// ```
#[derive(Debug)]
pub struct SpinlockGuard<'a> {
    lock: &'a Spinlock,
}

impl<'a> SpinlockGuard<'a> {
    /// Releases the lock early.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use abseil::absl_synchronization::Spinlock;
    ///
    /// let lock = Spinlock::new();
    /// let guard = lock.lock();
    /// guard.release();
    /// // Lock is now released
    /// ```
    #[inline]
    pub fn release(mut self) {
        if self.lock.is_locked() {
            self.lock.unlock();
        }
    }
}

impl<'a> Drop for SpinlockGuard<'a> {
    fn drop(&mut self) {
        self.release();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spinlock_basic() {
        let lock = Spinlock::new();
        assert!(!lock.is_locked());

        {
            let _guard = lock.lock();
            assert!(lock.is_locked());
        }

        assert!(!lock.is_locked());
    }

    #[test]
    fn test_spinlock_try_lock() {
        let lock = Spinlock::new();
        assert!(lock.try_lock());

        assert!(!lock.try_lock()); // Already locked

        lock.unlock();
        assert!(lock.try_lock()); // Now available
    }
}
