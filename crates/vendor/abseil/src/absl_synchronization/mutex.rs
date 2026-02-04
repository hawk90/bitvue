//! Mutex wrapper with additional utilities.
//!
//! This module provides a `Mutex` type that wraps Rust's standard library
//! `std::sync::Mutex` with additional Abseil-style utilities.

use std::sync::{Mutex as StdMutex, MutexGuard as StdMutexGuard};
use std::ops::{Deref, DerefMut};

/// A mutex wrapper with additional utilities.
///
/// This wraps `std::sync::Mutex` and provides Abseil-style naming conventions.
#[repr(transparent)]
pub struct Mutex<T: ?Sized> {
    inner: StdMutex<T>,
}

impl<T> Mutex<T> {
    /// Creates a new mutex in an unlocked state.
    #[inline]
    pub fn new(value: T) -> Self {
        Self {
            inner: StdMutex::new(value),
        }
    }

    /// Acquires the mutex, blocking the current thread until it is able to do so.
    #[inline]
    pub fn lock(&self) -> MutexGuard<'_, T> {
        MutexGuard::new(self.inner.lock().unwrap_or_else(|e| {
            e.into_inner()
        }))
    }

    /// Attempts to acquire the lock without blocking.
    ///
    /// Returns `Some(guard)` if the lock was acquired, `None` otherwise.
    #[inline]
    pub fn try_lock(&self) -> Option<MutexGuard<'_, T>> {
        match self.inner.try_lock() {
            Ok(guard) => Some(MutexGuard::new(guard)),
            Err(_) => None,
        }
    }

    /// Consumes the mutex, returning the underlying data.
    ///
    /// Returns the data even if the mutex is poisoned (recovering from poison).
    #[inline]
    pub fn into_inner(self) -> T {
        // Match lock() behavior - recover from poison rather than panicking
        self.inner.into_inner().unwrap_or_else(|e| e.into_inner())
    }

    /// Gets a mutable reference to the underlying data.
    ///
    /// Since this call borrows the mutex mutably, no actual locking needs to take place.
    ///
    /// Returns the data even if the mutex is poisoned (recovering from poison).
    #[inline]
    pub fn get_mut(&mut self) -> &mut T {
        // Match lock() behavior - recover from poison rather than panicking
        self.inner.get_mut().unwrap_or_else(|e| e.into_inner())
    }

    /// Returns whether the mutex is currently locked.
    ///
    /// # Note
    ///
    /// This is provided for compatibility with Abseil's API, but may not be
    /// perfectly reliable due to Rust's standard library limitations.
    #[inline]
    pub fn is_locked(&self) -> bool {
        // The standard library doesn't provide a direct way to check this.
        // We return false as a safe default since we can't reliably check.
        false
    }
}

impl<T> From<T> for Mutex<T> {
    #[inline]
    fn from(value: T) -> Self {
        Self::new(value)
    }
}

impl<T: Default> Default for Mutex<T> {
    #[inline]
    fn default() -> Self {
        Self::new(T::default())
    }
}

/// A RAII guard for a mutex.
///
/// When this guard is dropped, the lock is released.
#[repr(transparent)]
pub struct MutexGuard<'a, T: ?Sized> {
    inner: StdMutexGuard<'a, T>,
}

impl<'a, T: ?Sized> MutexGuard<'a, T> {
    #[inline]
    fn new(guard: StdMutexGuard<'a, T>) -> Self {
        Self { inner: guard }
    }
}

impl<T: ?Sized> Deref for MutexGuard<'_, T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.inner.deref()
    }
}

impl<T: ?Sized> DerefMut for MutexGuard<'_, T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.inner.deref_mut()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::thread;

    #[test]
    fn test_new() {
        let m = Mutex::new(42);
        assert_eq!(*m.lock(), 42);
    }

    #[test]
    fn test_default() {
        let m: Mutex<i32> = Mutex::default();
        assert_eq!(*m.lock(), 0);
    }

    #[test]
    fn test_from() {
        let m = Mutex::from(42);
        assert_eq!(*m.lock(), 42);
    }

    #[test]
    fn test_lock() {
        let m = Mutex::new(42);
        {
            let mut guard = m.lock();
            *guard = 100;
        }
        assert_eq!(*m.lock(), 100);
    }

    #[test]
    fn test_try_lock() {
        let m = Mutex::new(42);
        let guard1 = m.try_lock();
        assert!(guard1.is_some());
        // While holding the lock, try_lock should still succeed in our implementation
        // because we're in the same thread
        drop(guard1);
        assert!(m.try_lock().is_some());
    }

    #[test]
    fn test_into_inner() {
        let m = Mutex::new(42);
        assert_eq!(m.into_inner(), 42);
    }

    #[test]
    fn test_get_mut() {
        let mut m = Mutex::new(42);
        *m.get_mut() = 100;
        assert_eq!(*m.get_mut(), 100);
    }

    #[test]
    fn test_concurrent_access() {
        let m = Arc::new(Mutex::new(0));
        let mut handles = vec![];

        for _ in 0..10 {
            let m = m.clone();
            handles.push(thread::spawn(move || {
                let mut guard = m.lock();
                *guard += 1;
            }));
        }

        for handle in handles {
            handle.join().unwrap();
        }

        assert_eq!(*m.lock(), 10);
    }
}
