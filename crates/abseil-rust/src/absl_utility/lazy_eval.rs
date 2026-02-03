//! Lazy evaluation utilities.

use core::ops::Deref;

/// A lazily computed value (thread-safe).
///
/// The value is computed on first access and then cached.
/// This implementation uses `std::sync::OnceLock` for thread safety.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_utility::lazy_eval::Lazy;
///
/// let lazy = Lazy::new(|| {
///     println!("Computing...");
///     42
/// });
/// assert_eq!(*lazy, 42);
/// assert_eq!(*lazy, 42);  // Uses cached value
/// ```
///
/// # Thread Safety
///
/// This implementation is thread-safe. Multiple threads can access
/// the Lazy value simultaneously, and the initialization function
/// will only be called once.
pub struct Lazy<T, F: FnOnce() -> T> {
    state: std::sync::OnceLock<T>,
    init: core::cell::UnsafeCell<Option<F>>,
}

// Safety: The init field is only accessed during initialization,
// which is protected by OnceLock's internal synchronization.
// After initialization, init is never accessed again.
unsafe impl<T: Send, F: Send + FnOnce() -> T> Send for Lazy<T, F> {}
unsafe impl<T: Sync, F: Sync + FnOnce() -> T> Sync for Lazy<T, F> {}

impl<T, F: FnOnce() -> T> Lazy<T, F> {
    /// Creates a new lazy value with the given initialization function.
    #[inline]
    pub fn new(init: F) -> Self {
        Lazy {
            state: std::sync::OnceLock::new(),
            init: core::cell::UnsafeCell::new(Some(init)),
        }
    }

    /// Forces evaluation and returns a reference to the value.
    ///
    /// This method is thread-safe. Multiple threads calling this
    /// simultaneously will block until the value is initialized,
    /// then all will see the same value.
    #[inline]
    fn force(&self) -> &T {
        self.state.get_or_init(|| unsafe {
            // Safety: We only access init when the OnceLock is not yet initialized.
            // OnceLock::get_or_init ensures this is called at most once.
            // After taking the init function, we never access init again.
            let init = self.init.get().take().unwrap();
            init()
        })
    }
}

impl<T, F: FnOnce() -> T> Deref for Lazy<T, F> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.force()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::thread;

    #[test]
    fn test_lazy_basic() {
        let lazy = Lazy::new(|| {
            println!("Computing...");
            42
        });
        assert_eq!(*lazy, 42);
        assert_eq!(*lazy, 42);  // Uses cached value
    }

    #[test]
    fn test_lazy_expensive_computation() {
        let lazy = Lazy::new(|| {
            // Simulate expensive computation
            let mut sum = 0;
            for i in 1..=1000 {
                sum += i;
            }
            sum
        });
        assert_eq!(*lazy, 500500);
        assert_eq!(*lazy, 500500);  // Cached
    }

    #[test]
    fn test_lazy_with_closure() {
        let multiplier = 7;
        let lazy = Lazy::new(|| {
            multiplier * 6
        });
        assert_eq!(*lazy, 42);
    }

    #[test]
    fn test_lazy_thread_safety() {
        let lazy = Arc::new(Lazy::new(|| {
            // Simulate expensive computation
            std::thread::sleep(std::time::Duration::from_millis(10));
            42
        }));

        // Spawn multiple threads that all access the lazy value
        let handles: Vec<_> = (0..10)
            .map(|_| {
                let lazy = Arc::clone(&lazy);
                thread::spawn(move || {
                    *lazy
                })
            })
            .collect();

        // All threads should get the same value
        for handle in handles {
            assert_eq!(handle.join().unwrap(), 42);
        }
    }

    #[test]
    fn test_lazy_only_initializes_once() {
        use std::sync::atomic::{AtomicUsize, Ordering};

        let counter = Arc::new(AtomicUsize::new(0));
        let lazy = Arc::new(Lazy::new(|| {
            counter.fetch_add(1, Ordering::SeqCst);
            42
        }));

        // Spawn multiple threads
        let handles: Vec<_> = (0..10)
            .map(|_| {
                let lazy = Arc::clone(&lazy);
                thread::spawn(move || {
                    *lazy
                })
            })
            .collect();

        // Wait for all threads
        for handle in handles {
            handle.join().unwrap();
        }

        // The initialization function should only be called once
        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn test_lazy_different_types() {
        let lazy_string = Lazy::new(|| String::from("Hello, World!"));
        assert_eq!(*lazy_string, "Hello, World!");

        let lazy_vec = Lazy::new(|| vec![1, 2, 3, 4, 5]);
        assert_eq!(*lazy_vec, vec![1, 2, 3, 4, 5]);

        let lazy_option = Lazy::new(|| Some(42));
        assert_eq!(*lazy_option, Some(42));
    }

    #[test]
    fn test_lazy_send_sync() {
        // Verify that Lazy<T, F> implements Send and Sync when appropriate
        fn is_send<T: Send>() {}
        fn is_sync<T: Sync>() {}

        is_send::<Lazy<i32, fn() -> i32>>();
        is_sync::<Lazy<i32, fn() -> i32>>();

        // Arc<Lazy<...>> should be Send + Sync
        is_send::<Arc<Lazy<i32, fn() -> i32>>>();
        is_sync::<Arc<Lazy<i32, fn() -> i32>>>();
    }

    #[test]
    fn test_lazy_deref_multiple_times() {
        let lazy = Lazy::new(|| vec![1, 2, 3]);

        // Multiple dereferences should return the same reference
        let ref1 = &*lazy;
        let ref2 = &*lazy;

        // They should point to the same memory
        assert!(std::ptr::eq(ref1, ref2));
    }
}
