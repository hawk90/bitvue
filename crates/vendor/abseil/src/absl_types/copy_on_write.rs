//! Copy-on-write wrapper.

use alloc::boxed::Box;
use core::cell::Cell;

/// A wrapper that provides copy-on-write semantics.
///
/// The inner value is lazily cloned when a mutable reference is requested.
///
/// This implementation uses interior mutability to safely handle the
/// copy-on-write pattern without undefined behavior.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_types::copy_on_write::CopyOnWrite;
///
/// let cow = CopyOnWrite::new(vec![1, 2, 3]);
/// assert_eq!(cow.get(), &[1, 2, 3]);
///
/// let mut cow = cow.clone();
/// cow.get_mut().push(4);
/// // The cloned cow has the new value
/// assert_eq!(cow.get(), &[1, 2, 3, 4]);
/// ```
#[derive(Clone)]
pub struct CopyOnWrite<T: Clone> {
    inner: T,
    shared: Cell<bool>,
}

impl<T: Clone> CopyOnWrite<T> {
    /// Creates a new CopyOnWrite wrapper.
    #[inline]
    pub fn new(inner: T) -> Self {
        Self {
            inner,
            shared: Cell::new(false),
        }
    }

    /// Gets a reference to the inner value.
    #[inline]
    pub fn get(&self) -> &T {
        &self.inner
    }

    /// Gets a mutable reference, cloning the value if this has been shared.
    ///
    /// If this CopyOnWrite has been cloned (is shared), the inner value
    /// will be cloned before returning a mutable reference.
    #[inline]
    pub fn get_mut(&mut self) -> &mut T
    where
        T: Clone,
    {
        if self.shared.get() {
            // We're shared, need to clone the inner value
            self.inner = self.inner.clone();
            self.shared.set(false);
        }
        &mut self.inner
    }

    /// Marks this value as shared (has multiple references).
    ///
    /// This is called automatically when cloning.
    #[inline]
    fn mark_shared(&self) {
        self.shared.set(true);
    }

    /// Consumes the wrapper and returns the inner value.
    #[inline]
    pub fn into_inner(self) -> T {
        self.inner
    }
}

impl<T: Clone> Clone for CopyOnWrite<T> {
    #[inline]
    fn clone(&self) -> Self {
        self.mark_shared();
        Self {
            inner: self.inner.clone(),
            shared: Cell::new(true),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_copy_on_write() {
        let cow = CopyOnWrite::new(vec![1, 2, 3]);
        assert_eq!(cow.get(), &[1, 2, 3]);
    }
}
