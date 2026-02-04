//! Lazy evaluation utilities - Lazy, lazy
//!
//! # Thread Safety
//!
//! The `Lazy` type uses `RefCell` for interior mutability and is **not**
//! thread-safe. Do not share instances of `Lazy` across threads.
//! For thread-safe lazy initialization, use `std::sync::OnceLock` instead.

use core::cell::RefCell;

/// A lazily evaluated value.
///
/// The value is computed only once, on first access.
///
/// # Thread Safety
///
/// This type is **not thread-safe** and must not be shared across threads.
/// It explicitly implements `!Sync` and `!Send` to prevent accidental
/// multi-threaded use.
#[derive(Clone, Debug)]
pub struct Lazy<T, F>
where
    F: FnOnce() -> T,
{
    value: RefCell<Option<T>>,
    init: RefCell<Option<F>>,
}

// Explicitly mark Lazy as not thread-safe
impl<T, F> !Sync for Lazy<T, F> where F: FnOnce() -> T {}
impl<T, F> !Send for Lazy<T, F> where F: FnOnce() -> T {}

impl<T, F> Lazy<T, F>
where
    T: Clone,
    F: FnOnce() -> T,
{
    /// Creates a new lazy value.
    #[inline]
    pub fn new(init: F) -> Self {
        Self {
            value: RefCell::new(None),
            init: RefCell::new(Some(init)),
        }
    }

    /// Gets the value, computing it if necessary.
    #[inline]
    pub fn get(&self) -> T {
        if self.value.borrow().is_none() {
            let init = self.init.borrow_mut().take()
                .expect("Lazy value already initialized twice");
            *self.value.borrow_mut() = Some(init());
        }
        self.value.borrow().as_ref()
            .expect("Lazy value not initialized")
            .clone()
    }

    /// Forces evaluation and returns the value.
    #[inline]
    pub fn force(&self) -> T {
        self.get()
    }
}

/// Creates a lazy value.
#[inline]
pub fn lazy<T, F>(init: F) -> Lazy<T, F>
where
    T: Clone,
    F: FnOnce() -> T,
{
    Lazy::new(init)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lazy() {
        let call_count = RefCell::new(0);
        let lazy_val = lazy(|| {
            *call_count.borrow_mut() += 1;
            42
        });

        assert_eq!(*call_count.borrow(), 0);
        assert_eq!(lazy_val.get(), 42);
        assert_eq!(*call_count.borrow(), 1);
        assert_eq!(lazy_val.get(), 42);
        assert_eq!(*call_count.borrow(), 1); // Not called again
    }

    // Compile-time test to verify Lazy is not Send/Sync
    // This will fail to compile if Lazy is Send or Sync
    fn _assert_lazy_not_send<T: !Send>() {}
    fn _assert_lazy_not_sync<T: !Sync>() {}

    #[test]
    fn verify_lazy_not_thread_safe() {
        // These will fail to compile if Lazy implements Send or Sync
        _assert_lazy_not_send::<Lazy<i32, fn() -> i32>>();
        _assert_lazy_not_sync::<Lazy<i32, fn() -> i32>>();
    }
}
