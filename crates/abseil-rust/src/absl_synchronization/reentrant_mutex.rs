//! Reentrant mutex that allows the same thread to lock multiple times.

use core::cell::Cell;
use core::sync::atomic::Ordering;

#[cfg(feature = "std")]
use std::thread;

/// Returns a pseudo-unique thread ID.
///
/// This uses platform-specific APIs to get a unique thread ID.
#[inline]
#[cfg(feature = "std")]
fn current_thread_id() -> usize {
    // Use thread ID for std builds
    let thread_id = thread::current().id();
    // Convert ThreadId to usize using its NonZeroU64 representation
    // ThreadId is internally a NonZeroU64, so we extract it
    unsafe {
        *((&thread_id as *const thread::ThreadId) as *const u64) as usize
    }
}

/// Returns a pseudo-unique thread ID for no_std.
///
/// WARNING: In no_std mode without proper thread support, this uses
/// a fallback that may not provide true thread uniqueness. For
/// production use with no_std, implement proper thread ID detection
/// for your platform.
#[inline]
#[cfg(not(feature = "std"))]
fn current_thread_id() -> usize {
    // For no_std, use address of a thread-local static as a proxy
    // This is not guaranteed to be unique across threads in all scenarios
    thread_local! {
        static THREAD_ID_KEY: u8 = 0;
    }
    THREAD_ID_KEY.with(|key| key as *const u8 as usize)
}

/// A reentrant mutex lock.
///
/// Unlike the regular Mutex, this allows the same thread to lock
/// the mutex multiple times (deadlock-safe). Each lock must be matched
/// with an unlock, but the thread can unlock in any order.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_synchronization::ReentrantMutex;
///
/// let mutex = ReentrantMutex::new(42);
/// {
///     let guard1 = mutex.lock();
///     let guard2 = mutex.lock(); // Same thread can lock again
///     // Critical section
/// } // Both guards are dropped here, unlocking twice
/// ```
#[derive(Debug)]
pub struct ReentrantMutex<T> {
    data: core::cell::UnsafeCell<T>,
    owner: Cell<usize>,
    lock_count: Cell<usize>,
}

impl<T> ReentrantMutex<T> {
    /// Creates a new reentrant mutex.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use abseil::absl_synchronization::ReentrantMutex;
    ///
    /// let mutex = ReentrantMutex::new(42);
    /// ```
    pub const fn new(data: T) -> Self {
        Self {
            data: core::cell::UnsafeCell::new(data),
            owner: Cell::new(usize::MAX),
            lock_count: Cell::new(0),
        }
    }

    /// Acquires the mutex, blocking until it becomes available.
    ///
    /// If the current thread already holds the lock, the lock count is
    /// incremented (reentrant).
    ///
    /// # Examples
    ///
    /// ```rust
    /// use abseil::absl_synchronization::ReentrantMutex;
    ///
    /// let mutex = ReentrantMutex::new(42);
    /// let guard = mutex.lock();
    /// // Critical section
    /// ```
    pub fn lock(&self) -> ReentrantMutexGuard<T> {
        let thread_id = current_thread_id();
        let mut spin_count = 0;

        loop {
            if self.owner.get() == thread_id {
                // Already holding the lock
                let count = self.lock_count.get();
                self.lock_count.set(count + 1);
                return ReentrantMutexGuard { mutex: self };
            }

            // Try to acquire the lock
            if self.owner.compare_exchange(
                usize::MAX,
                thread_id,
                Ordering::Acquire,
            ).is_ok() {
                // Successfully acquired
                self.lock_count.set(1);
                return ReentrantMutexGuard { mutex: self };
            }

            // Spin and retry
            spin_count += 1;
            if spin_count >= 100 {
                // Yield occasionally to be nice to the scheduler
                spin_count = 0;
            }
            core::hint::spin_loop();
        }
    }

    /// Attempts to acquire the mutex without blocking.
    ///
    /// Returns None if the mutex is held by a different thread.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use abseil::absl_synchronization::ReentrantMutex;
    ///
    /// let mutex = ReentrantMutex::new(42);
    /// if let Some(guard) = mutex.try_lock() {
    ///     // Lock acquired
    /// }
    /// ```
    pub fn try_lock(&self) -> Option<ReentrantMutexGuard<T>> {
        let thread_id = current_thread_id();

        if self.owner.get() == thread_id {
            // Already holding the lock
            let count = self.lock_count.get();
            self.lock_count.set(count + 1);
            return Some(ReentrantMutexGuard { mutex: self });
        }

        // Try to acquire the lock
        if self.owner.compare_exchange(
            usize::MAX,
            thread_id,
            Ordering::Acquire,
        ).is_err() {
            return None;
        }

        // Successfully acquired
        self.lock_count.set(1);
        Some(ReentrantMutexGuard { mutex: self })
    }

    /// Returns a mutable reference to the data.
    ///
    /// # Safety
    ///
    /// The caller must ensure that the mutex is held.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use abseil::absl_synchronization::ReentrantMutex;
    ///
    /// let mutex = ReentrantMutex::new(42);
    /// let guard = mutex.lock();
    /// let data = unsafe { mutex.get_mut(&guard) };
    /// *data = 100;
    /// ```
    #[inline]
    pub unsafe fn get_mut(&self, _guard: &ReentrantMutexGuard<T>) -> &mut T {
        &mut *self.data.get()
    }

    /// Returns a reference to the data.
    ///
    /// # Safety
    ///
    /// The caller must ensure that the mutex is held.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use abseil::absl_synchronization::ReentrantMutex;
    ///
    /// let mutex = ReentrantMutex::new(42);
    /// let guard = mutex.lock();
    /// let data = unsafe { mutex.get(&guard) };
    /// assert_eq!(*data, 42);
    /// ```
    #[inline]
    pub unsafe fn get(&self, _guard: &ReentrantMutexGuard<T>) -> &T {
        &*self.data.get()
    }

    /// Returns true if the mutex is currently held.
    #[inline]
    pub fn is_locked(&self) -> bool {
        self.owner.get() != usize::MAX
    }

    /// Returns the lock count (0 if unlocked, >0 if locked).
    #[inline]
    pub fn lock_count(&self) -> usize {
        self.lock_count.get()
    }
}

/// Guard for ReentrantMutex.
///
/// Automatically releases the lock (or decrements the lock count) when dropped.
#[derive(Debug)]
pub struct ReentrantMutexGuard<'a, T> {
    mutex: &'a ReentrantMutex<T>,
}

impl<'a, T> ReentrantMutexGuard<'a, T> {
    /// Releases the lock early.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use abseil::absl_synchronization::ReentrantMutex;
    ///
    /// let mutex = ReentrantMutex::new(42);
    /// let guard = mutex.lock();
    /// guard.release();
    /// ```
    pub fn release(self) {
        drop(self);
    }
}

impl<'a, T> Drop for ReentrantMutexGuard<'a, T> {
    fn drop(&mut self) {
        let thread_id = current_thread_id();
        if self.mutex.owner.get() == thread_id {
            let count = self.mutex.lock_count.get();
            if count == 1 {
                // Last unlock
                self.mutex.owner.set(usize::MAX);
                self.mutex.lock_count.set(0);
            } else {
                self.mutex.lock_count.set(count - 1);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reentrant_mutex() {
        let mutex = ReentrantMutex::new(42);

        {
            let guard1 = mutex.lock();
            let guard2 = mutex.lock();
            // Both guards hold the lock
        }
        // Lock released
    }

    #[test]
    fn test_reentrant_mutex_try_lock() {
        let mutex = ReentrantMutex::new(42);

        {
            let guard1 = mutex.try_lock().unwrap();
            assert!(mutex.try_lock().is_some()); // Can lock again
            let guard2 = mutex.try_lock().unwrap();
            // Reentrant lock works
        }
    }
}
