//! Lazy evaluation wrapper.

use core::marker::PhantomData;

/// A lazy evaluation wrapper.
///
/// The value is computed only when first accessed.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_types::lazy::Lazy;
///
/// let lazy = Lazy::new(|| {
///     println!("Computing...");
///     42
/// });
/// assert_eq!(*lazy.get(), 42);
/// assert_eq!(*lazy.get(), 42); // Not computed again
/// ```
#[derive(Clone)]
pub struct Lazy<T, F = fn() -> T>
where
    F: FnOnce() -> T,
{
    value: Option<T>,
    init: F,
}

impl<T, F> Lazy<T, F>
where
    F: FnOnce() -> T,
{
    /// Creates a new lazy value with the given initialization function.
    #[inline]
    pub const fn new(init: F) -> Self {
        Self {
            value: None,
            init,
        }
    }

    /// Gets a reference to the value, computing it if necessary.
    #[inline]
    pub fn get(&mut self) -> &T {
        if self.value.is_none() {
            let init = core::mem::replace(&mut self.init, || panic!("Lazy already initialized"));
            self.value = Some(init());
        }
        self.value.as_ref().unwrap()
    }

    /// Forces evaluation of the lazy value.
    #[inline]
    pub fn force(&mut self) -> &T {
        self.get()
    }

    /// Returns true if the value has been computed.
    #[inline]
    pub fn is_initialized(&self) -> bool {
        self.value.is_some()
    }
}

impl<T: Copy, F: FnOnce() -> T> Lazy<T, F>
where
    F: FnOnce() -> T,
{
    /// Gets the value, computing if necessary and returning a copy.
    #[inline]
    pub fn get_copy(&mut self) -> T {
        *self.get()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lazy() {
        let mut lazy = Lazy::new(|| 42);
        assert!(!lazy.is_initialized());
        assert_eq!(lazy.get(), &42);
        assert!(lazy.is_initialized());
    }
}
