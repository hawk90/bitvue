//! One-time function invocation utilities.
//!
//! This module provides `OnceFlag` and `call_once` similar to Abseil's `absl::base_once`
//! and Rust's `std::sync::Once`, but with API compatibility closer to Abseil's C++ version.

use core::sync::atomic::{AtomicBool, Ordering};
use core::cell::Cell;

/// A flag that can be used to perform a one-time initialization.
///
/// Similar to `absl::OnceFlag` in C++ Abseil and `std::sync::Once` in Rust,
/// but with a simpler API that matches Abseil's patterns.
///
/// # Example
///
/// ```rust
/// use abseil::absl_base::call_once::{call_once, OnceFlag};
///
/// static INIT: OnceFlag = OnceFlag::new();
///
/// fn main() {
///     call_once(&INIT, || {
///         println!("This will only run once");
///     });
///
///     // Subsequent calls won't execute the closure
///     call_once(&INIT, || {
///         println!("This won't print");
///     });
/// }
/// ```
#[repr(transparent)]
pub struct OnceFlag(AtomicBool);

unsafe impl Send for OnceFlag {}
unsafe impl Sync for OnceFlag {}

impl OnceFlag {
    /// Creates a new `OnceFlag` that has not been called.
    ///
    /// # Example
    ///
    /// ```rust
    /// use abseil::absl_base::call_once::OnceFlag;
    ///
    /// let flag = OnceFlag::new();
    /// ```
    #[inline]
    #[must_use]
    pub const fn new() -> Self {
        Self(AtomicBool::new(false))
    }

    /// Creates a new `OnceFlag` that has already been called.
    ///
    /// This is useful for creating pre-initialized flags.
    ///
    /// # Example
    ///
    /// ```rust
    /// use abseil::absl_base::call_once::OnceFlag;
    ///
    /// let flag = OnceFlag::called();
    /// assert!(abseil::absl_base::call_once::is_done(&flag));
    /// ```
    #[inline]
    #[must_use]
    pub const fn called() -> Self {
        Self(AtomicBool::new(true))
    }
}

impl Default for OnceFlag {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

/// Calls `f` if `flag` has not been called yet.
///
/// If `flag` has already been called, `f` will not be executed.
/// This function is thread-safe: multiple threads can call `call_once`
/// with the same flag simultaneously, and `f` is guaranteed to be
/// called exactly once.
///
/// # Arguments
///
/// * `flag` - A reference to the `OnceFlag`
/// * `f` - A closure to execute exactly once
///
/// # Example
///
/// ```rust
/// use abseil::absl_base::call_once::{call_once, OnceFlag};
///
/// static INIT: OnceFlag = OnceFlag::new();
/// static mut COUNTER: i32 = 0;
///
/// // From multiple threads, this is safe
/// call_once(&INIT, || {
///     unsafe { COUNTER += 1; }
/// });
///
/// assert_eq!(unsafe { COUNTER }, 1);
/// ```
#[inline]
pub fn call_once<F>(flag: &OnceFlag, f: F)
where
    F: FnOnce(),
{
    if !flag.0.load(Ordering::Acquire) {
        call_once_impl(flag, f);
    }
}

/// Internal implementation of call_once.
/// Uses a CAS (Compare-And-Swap) loop to ensure exactly one thread executes the closure.
/// Uses exponential backoff to prevent busy-wait DoS.
#[inline(never)]
#[cold]
fn call_once_impl<F>(flag: &OnceFlag, f: F)
where
    F: FnOnce(),
{
    // Try to acquire the right to run initialization
    if !flag.0.compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire).is_err() {
        // We won the race, run the initialization
        f();
    } else {
        // Someone else is running or has run the initialization
        // Wait for the flag to be set with exponential backoff to prevent DoS
        let mut spins = 0;
        const MAX_SPINS: u32 = 100;

        while !flag.0.load(Ordering::Acquire) {
            if spins < MAX_SPINS {
                // Spin for a bit first (fast path when init is quick)
                for _ in 0..(1 << spins.min(8)) {
                    core::hint::spin_loop();
                }
                spins += 1;
            } else {
                // After too many spins, yield the thread
                // This prevents CPU-bound busy-wait loops
                core::hint::spin_loop();
                // In a full implementation, we would use thread::sleep here,
                // but we're in a no_std compatible context
            }
        }
    }
}

/// Returns `true` if the `OnceFlag` has been called.
///
/// # Example
///
/// ```rust
/// use abseil::absl_base::call_once::{call_once, is_done, OnceFlag};
///
/// let flag = OnceFlag::new();
/// assert!(!is_done(&flag));
///
/// call_once(&flag, || {});
/// assert!(is_done(&flag));
/// ```
#[inline]
#[must_use]
pub fn is_done(flag: &OnceFlag) -> bool {
    flag.0.load(Ordering::Acquire)
}

/// Thread-local storage for per-thread initialization state.
///
/// This provides a way to have thread-local one-time initialization,
/// similar to `thread_local!` but with an API closer to Abseil's patterns.
#[repr(transparent)]
pub struct ThreadOnceFlag(Cell<bool>);

impl ThreadOnceFlag {
    /// Creates a new thread-local `OnceFlag`.
    #[inline]
    #[must_use]
    pub const fn new() -> Self {
        Self(Cell::new(false))
    }

    /// Creates a pre-called thread-local flag.
    #[inline]
    #[must_use]
    pub const fn called() -> Self {
        Self(Cell::new(true))
    }
}

impl Default for ThreadOnceFlag {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

/// Calls `f` if the thread-local `flag` has not been called in this thread.
///
/// Unlike `call_once`, this is per-thread rather than global.
/// Each thread gets its own initialization.
///
/// # Example
///
/// ```rust
/// use abseil::absl_base::call_once::{call_thread_once, ThreadOnceFlag};
///
/// thread_local! {
///     static LOCAL_INIT: ThreadOnceFlag = ThreadOnceFlag::new();
/// }
///
/// // In each thread, access the thread_local and call once
/// LOCAL_INIT.with(|flag| {
///     call_thread_once(flag, || {
///         println!("Thread-local init");
///     });
/// });
/// ```
#[inline]
pub fn call_thread_once<F>(flag: &ThreadOnceFlag, f: F)
where
    F: FnOnce(),
{
    if !flag.0.get() {
        flag.0.set(true);
        f();
    }
}

/// Returns `true` if the thread-local flag has been called in this thread.
#[inline]
#[must_use]
pub fn is_thread_done(flag: &ThreadOnceFlag) -> bool {
    flag.0.get()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_once_flag_basic() {
        let flag = OnceFlag::new();
        assert!(!is_done(&flag));

        let mut called = false;
        call_once(&flag, || {
            called = true;
        });
        assert!(called);
        assert!(is_done(&flag));
    }

    #[test]
    fn test_once_flag_only_called_once() {
        let flag = OnceFlag::new();
        let mut counter = 0;

        call_once(&flag, || {
            counter += 1;
        });
        assert_eq!(counter, 1);

        call_once(&flag, || {
            counter += 1;
        });
        assert_eq!(counter, 1); // Still 1, second call didn't execute
    }

    #[test]
    fn test_once_flag_called_state() {
        let flag = OnceFlag::called();
        assert!(is_done(&flag));
    }

    #[test]
    fn test_default() {
        let flag = OnceFlag::default();
        assert!(!is_done(&flag));
    }

    #[test]
    fn test_thread_once_flag() {
        let flag = ThreadOnceFlag::new();
        assert!(!is_thread_done(&flag));

        call_thread_once(&flag, || {
            // This should execute
        });
        assert!(is_thread_done(&flag));

        let mut second_call = false;
        call_thread_once(&flag, || {
            second_call = true;
        });
        assert!(!second_call); // Second call didn't execute
    }

    #[test]
    fn test_thread_once_flag_called_state() {
        let flag = ThreadOnceFlag::called();
        assert!(is_thread_done(&flag));
    }

    #[test]
    fn test_static_once_flag() {
        static FLAG: OnceFlag = OnceFlag::new();
        static mut COUNTER: i32 = 0;

        call_once(&FLAG, || {
            unsafe { COUNTER += 10; }
        });

        call_once(&FLAG, || {
            unsafe { COUNTER += 20; }
        });

        assert_eq!(unsafe { COUNTER }, 10); // Only first call executed
    }

    #[test]
    fn test_concurrent_call_once() {
        use std::sync::Arc;
        use std::sync::atomic::{AtomicI32, Ordering};

        let flag = Arc::new(OnceFlag::new());
        let counter = Arc::new(AtomicI32::new(0));
        let mut handles = vec![];

        for _ in 0..10 {
            let flag_clone = Arc::clone(&flag);
            let counter_clone = Arc::clone(&counter);

            handles.push(std::thread::spawn(move || {
                call_once(&flag_clone, || {
                    counter_clone.fetch_add(1, Ordering::Relaxed);
                });
            }));
        }

        for handle in handles {
            handle.join().unwrap();
        }

        // Despite 10 threads racing, the closure should only run once
        assert_eq!(counter.load(Ordering::Relaxed), 1);
        assert!(is_done(&flag));
    }
}
