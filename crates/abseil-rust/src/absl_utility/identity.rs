//! Identity utilities.

use alloc::rc::Rc as StdRc;
use core::cell::{Ref, RefCell, RefMut};

/// A wrapper type that explicitly marks its contents as "owned".
///
/// This can be useful for API design to distinguish between borrowed and owned data.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Owned<T>(pub T);

impl<T> Owned<T> {
    /// Creates a new Owned wrapper.
    #[inline]
    pub const fn new(value: T) -> Self {
        Owned(value)
    }

    /// Extracts the inner value.
    #[inline]
    pub const fn into_inner(self) -> T {
        self.0
    }

    /// Gets a reference to the inner value.
    #[inline]
    pub const fn get(&self) -> &T {
        &self.0
    }

    /// Gets a mutable reference to the inner value.
    #[inline]
    pub fn get_mut(&mut self) -> &mut T {
        &mut self.0
    }
}

/// A wrapper type that explicitly marks its contents as "borrowed".
///
/// This can be useful for API design to distinguish between borrowed and owned data.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Borrowed<'a, T>(pub &'a T);

impl<'a, T> Borrowed<'a, T> {
    /// Creates a new Borrowed wrapper.
    #[inline]
    pub const fn new(value: &'a T) -> Self {
        Borrowed(value)
    }

    /// Extracts the inner reference.
    #[inline]
    pub const fn into_inner(self) -> &'a T {
        self.0
    }

    /// Gets the inner reference.
    #[inline]
    pub const fn get(&self) -> &'a T {
        self.0
    }
}

/// A wrapper that provides shared ownership through reference counting.
///
/// Note: This is a simplified placeholder. For production use,
/// consider using `std::rc::Rc` or `alloc::rc::Rc`.
#[derive(Clone, Debug)]
pub struct Rc<T> {
    inner: StdRc<T>,
}

impl<T> Rc<T> {
    /// Creates a new Rc.
    #[inline]
    pub fn new(value: T) -> Self {
        Rc {
            inner: StdRc::new(value),
        }
    }

    /// Gets the number of strong references to this Rc.
    #[inline]
    pub fn strong_count(&self) -> usize {
        StdRc::strong_count(&self.inner)
    }

    /// Gets a reference to the inner value.
    #[inline]
    pub fn get(&self) -> &T {
        &self.inner
    }
}

impl<T: Clone> Rc<T> {
    /// Returns a mutable reference to the inner value, cloning if necessary.
    ///
    /// This is similar to `Rc::make_mut` but returns a reference instead.
    #[inline]
    pub fn get_mut(&mut self) -> &mut T {
        StdRc::make_mut(&mut self.inner)
    }
}

/// A wrapper that provides shared mutable ownership through reference counting.
///
/// Note: This is a simplified placeholder. For production use,
/// consider using `std::rc::Rc` with `std::cell::RefCell`.
#[derive(Clone, Debug)]
pub struct RcRefCell<T> {
    inner: StdRc<RefCell<T>>,
}

impl<T> RcRefCell<T> {
    /// Creates a new RcRefCell.
    #[inline]
    pub fn new(value: T) -> Self {
        RcRefCell {
            inner: StdRc::new(RefCell::new(value)),
        }
    }

    /// Gets the number of strong references to this RcRefCell.
    #[inline]
    pub fn strong_count(&self) -> usize {
        StdRc::strong_count(&self.inner)
    }

    /// Borrows the inner value.
    #[inline]
    pub fn borrow(&self) -> Ref<'_, T> {
        self.inner.borrow()
    }

    /// Mutably borrows the inner value.
    #[inline]
    pub fn borrow_mut(&self) -> RefMut<'_, T> {
        self.inner.borrow_mut()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_owned_borrowed() {
        let owned = Owned::new(42);
        assert_eq!(owned.get(), &42);
        assert_eq!(owned.into_inner(), 42);

        let value = 42;
        let borrowed = Borrowed::new(&value);
        assert_eq!(borrowed.get(), &42);
    }

    #[test]
    fn test_rc() {
        let rc = Rc::new(vec![1, 2, 3]);
        assert_eq!(rc.strong_count(), 1);
        let rc2 = rc.clone();
        assert_eq!(rc2.strong_count(), 2);
    }

    #[test]
    fn test_rc_ref_cell() {
        let cell = RcRefCell::new(42);
        assert_eq!(*cell.borrow(), 42);
        *cell.borrow_mut() = 100;
        assert_eq!(*cell.borrow(), 100);
    }
}
