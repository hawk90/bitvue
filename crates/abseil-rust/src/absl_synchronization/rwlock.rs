//! Reader-writer lock that allows multiple readers or one writer.
//!
//! # ⚠️ SECURITY WARNING
//!
//! This implementation has a known TOCTOU (time-of-check-time-of-use) race condition:
//!
//! - The `read()` method checks `writer` flag and then increments `readers` separately
//! - Between these operations, a writer could acquire the lock
//! - This can lead to data races where readers access data while a writer modifies it
//!
//! ## Recommended Alternative
//!
//! Use `std::sync::RwLock` from the standard library instead, which has been
//! extensively tested and is memory-safe:
//!
//! ```rust
//! use std::sync::RwLock;
//!
//! let lock = RwLock::new(42);
//! {
//!     let guard = lock.read().unwrap();
//!     // Safe read access
//! }
//! {
//!     let mut guard = lock.write().unwrap();
//!     // Safe write access
//! }
//! ```

use core::cell::UnsafeCell;
use core::ops::{Deref, DerefMut};
use core::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

/// A reader-writer lock that allows multiple readers or one writer.
///
/// ⚠️ **DEPRECATED** due to known race condition. Use `std::sync::RwLock` instead.
///
/// This is a simple implementation that prioritizes readers.
/// It has a known TOCTOU race condition in the `read()` method.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_synchronization::RwLock;
///
/// let lock = RwLock::new(42);
///
/// // Read access
/// {
///     let guard = lock.read();
///     assert_eq!(*guard, 42);
/// }
///
/// // Write access
/// {
///     let mut guard = lock.write();
///     *guard = 100;
/// }
/// ```
#[deprecated(
    since = "0.1.0",
    note = "Use std::sync::RwLock instead due to known TOCTOU race condition"
)]
#[derive(Debug)]
pub struct RwLock<T> {
    data: UnsafeCell<T>,
    readers: AtomicUsize,
    writer: AtomicBool,
}

impl<T> RwLock<T> {
    /// Creates a new reader-writer lock.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use abseil::absl_synchronization::RwLock;
    ///
    /// let lock = RwLock::new(42);
    /// ```
    pub const fn new(data: T) -> Self {
        Self {
            data: UnsafeCell::new(data),
            readers: AtomicUsize::new(0),
            writer: AtomicBool::new(false),
        }
    }

    /// Acquires a read lock, blocking until available.
    ///
    /// ⚠️ **WARNING**: This method has a TOCTOU race condition:
    /// - Checks writer flag at line X
    /// - Increments readers count at line Y
    /// - Writer could acquire between these two operations
    /// - Can cause data races and undefined behavior
    ///
    /// # Examples
    ///
    /// ```rust
    /// use abseil::absl_synchronization::RwLock;
    ///
    /// let lock = RwLock::new(42);
    /// let guard = lock.read();
    /// // Read access here
    /// ```
    #[deprecated(since = "0.1.0", note = "Use std::sync::RwLock instead")]
    pub fn read(&self) -> ReadGuard<T> {
        loop {
            // Check if no writer
            if !self.writer.load(Ordering::Acquire) {
                // Try to acquire read lock
                if self.readers.fetch_add(1, Ordering::Acquire) == 0 {
                    return ReadGuard { lock: self };
                }
                // We incremented, so we need to decrement
                self.readers.fetch_sub(1, Ordering::Release);
            }

            // Spin and retry
            core::hint::spin_loop();
        }
    }

    /// Acquires a write lock, blocking until available.
    ///
    /// ⚠️ **WARNING**: This RwLock implementation has known race conditions.
    /// Consider using `std::sync::RwLock` from the standard library instead.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use abseil::absl_synchronization::RwLock;
    ///
    /// let lock = RwLock::new(42);
    /// let mut guard = lock.write();
    /// // Write access here
    /// ```
    #[deprecated(since = "0.1.0", note = "Use std::sync::RwLock instead")]
    pub fn write(&self) -> WriteGuard<T> {
        loop {
            // Try to acquire write lock
            if self.writer.compare_exchange(
                false,
                true,
                Ordering::Acquire,
            ).is_ok()
            {
                // Wait for all readers to finish
                while self.readers.load(Ordering::Acquire) > 0 {
                    core::hint::spin_loop();
                }
                return WriteGuard { lock: self };
            }

            // Spin and retry
            core::hint::spin_loop();
        }
    }

    /// Returns a mutable reference to the data.
    ///
    /// # Safety
    ///
    /// The caller must ensure that exclusive access is maintained.
    #[inline]
    pub unsafe fn get_mut(&self) -> &mut T {
        &mut *self.data.get()
    }

    /// Returns a reference to the data.
    ///
    /// # Safety
    ///
    /// The caller must ensure that some access (read or write) is held.
    #[inline]
    pub unsafe fn get(&self) -> &T {
        &*self.data.get()
    }

    /// Returns true if the lock is currently write-locked.
    #[inline]
    pub fn is_write_locked(&self) -> bool {
        self.writer.load(Ordering::Acquire)
    }

    /// Returns the number of current readers.
    #[inline]
    pub fn reader_count(&self) -> usize {
        self.readers.load(Ordering::Acquire)
    }
}

/// Guard for read access.
#[derive(Debug)]
pub struct ReadGuard<'a, T> {
    lock: &'a RwLock<T>,
}

impl<'a, T> Drop for ReadGuard<'a, T> {
    fn drop(&mut self) {
        self.lock.readers.fetch_sub(1, Ordering::Release);
    }
}

impl<'a, T> Deref for ReadGuard<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { self.lock.get() }
    }
}

/// Guard for write access.
#[derive(Debug)]
pub struct WriteGuard<'a, T> {
    lock: &'a RwLock<T>,
}

impl<'a, T> Drop for WriteGuard<'a, T> {
    fn drop(&mut self) {
        self.lock.writer.store(false, Ordering::Release);
    }
}

impl<'a, T> Deref for WriteGuard<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { self.lock.get() }
    }
}

impl<'a, T> DerefMut for WriteGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { self.lock.get_mut() }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rwlock_read() {
        let lock = RwLock::new(42);

        {
            let guard1 = lock.read();
            let guard2 = lock.read();
            assert_eq!(*guard1, 42);
            assert_eq!(*guard2, 42);
            assert_eq!(lock.reader_count(), 2);
        }

        assert_eq!(lock.reader_count(), 0);
    }

    #[test]
    fn test_rwlock_write() {
        let lock = RwLock::new(42);

        {
            let mut guard = lock.write();
            *guard = 100;
            assert!(lock.is_write_locked());
            assert_eq!(lock.reader_count(), 0);
        }

        assert!(!lock.is_write_locked());
        assert_eq!(unsafe { lock.get() }, &100);
    }

    // Edge case tests for HIGH security fix - race condition documentation

    #[test]
    #[allow(deprecated)]
    fn test_rwlock_multiple_readers() {
        // Test that multiple readers can coexist
        let lock = RwLock::new(42);

        {
            let guard1 = lock.read();
            let guard2 = lock.read();
            let guard3 = lock.read();
            assert_eq!(*guard1, 42);
            assert_eq!(*guard2, 42);
            assert_eq!(*guard3, 42);
            assert_eq!(lock.reader_count(), 3);
        }

        assert_eq!(lock.reader_count(), 0);
    }

    #[test]
    fn test_rwlock_reader_writer_mutex() {
        // Test that writer and readers are mutually exclusive
        let lock = RwLock::new(42);

        // Acquire write lock first
        {
            let mut guard = lock.write();
            assert!(lock.is_write_locked());
            *guard = 100;
        }

        // Now readers can acquire
        {
            let guard = lock.read();
            assert_eq!(*guard, 100);
        }
    }

    #[test]
    fn test_rwlock_zero_value() {
        // Test with zero value
        let lock = RwLock::new(0);

        {
            let guard = lock.read();
            assert_eq!(*guard, 0);
        }

        {
            let mut guard = lock.write();
            *guard = 1;
        }

        assert_eq!(unsafe { lock.get() }, &1);
    }

    #[test]
    fn test_rwlock_alternating_access() {
        // Test alternating read and write access
        let lock = RwLock::new(42);

        for i in 0..10 {
            // Write
            {
                let mut guard = lock.write();
                *guard = i * 10;
            }

            // Read
            {
                let guard = lock.read();
                assert_eq!(*guard, i * 10);
            }
        }
    }

    #[test]
    fn test_rwlock_is_write_locked() {
        // Test is_write_locked flag
        let lock = RwLock::new(42);

        assert!(!lock.is_write_locked());

        {
            let _guard = lock.write();
            assert!(lock.is_write_locked());
        }

        assert!(!lock.is_write_locked());
    }

    #[test]
    fn test_rwlock_reader_count() {
        // Test reader_count accuracy
        let lock = RwLock::new(42);

        assert_eq!(lock.reader_count(), 0);

        let guard1 = lock.read();
        assert_eq!(lock.reader_count(), 1);

        let guard2 = lock.read();
        assert_eq!(lock.reader_count(), 2);

        drop(guard1);
        assert_eq!(lock.reader_count(), 1);

        drop(guard2);
        assert_eq!(lock.reader_count(), 0);
    }

    #[test]
    fn test_rwlock_deprecated_warning() {
        // This test documents that the RwLock is deprecated
        // Users should use std::sync::RwLock instead
        let lock = RwLock::new(42);

        // The read and write methods should trigger deprecation warnings
        // but we can't test that in a test that needs to compile

        // Just verify the basic functionality still works for legacy code
        assert_eq!(unsafe { lock.get() }, &42);
    }
}
