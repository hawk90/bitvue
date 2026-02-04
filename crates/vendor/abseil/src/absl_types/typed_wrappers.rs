//! Typed boolean and integer wrappers.

use core::marker::PhantomData;
use core::ops::{Add, Sub};

/// A type-safe boolean wrapper.
///
/// This prevents mixing boolean values of different semantic types.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_types::typed_wrappers::Bool;
///
/// struct IsVisible;
/// struct IsEnabled;
///
/// let visible: Bool<IsVisible> = Bool::new(true);
/// let enabled: Bool<IsEnabled> = Bool::new(false);
/// ```
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Bool<T> {
    value: bool,
    _marker: PhantomData<T>,
}

impl<T> Bool<T> {
    /// Creates a new typed boolean.
    #[inline]
    pub const fn new(value: bool) -> Self {
        Self {
            value,
            _marker: PhantomData,
        }
    }

    /// Returns the wrapped boolean value.
    #[inline]
    pub const fn get(&self) -> bool {
        self.value
    }

    /// Creates a true value.
    #[inline]
    pub const fn true_() -> Self {
        Self::new(true)
    }

    /// Creates a false value.
    #[inline]
    pub const fn false_() -> Self {
        Self::new(false)
    }
}

/// A type-safe integer wrapper.
///
/// This prevents mixing integer values of different semantic types.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_types::typed_wrappers::Int;
///
/// struct UserId;
/// struct ItemId;
///
/// let user_id: Int<UserId, i32> = Int::new(42);
/// let item_id: Int<ItemId, i32> = Int::new(42);
/// ```
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Int<T, I = i32>(pub I);

impl<T, I: Clone + Copy> Int<T, I> {
    /// Creates a new typed integer.
    #[inline]
    pub const fn new(value: I) -> Self {
        Self(value)
    }

    /// Returns the wrapped integer value.
    #[inline]
    pub const fn get(&self) -> I {
        self.0
    }
}

impl<T, I: Add<Output = I>> Add for Int<T, I> {
    type Output = Int<T, I>;

    #[inline]
    fn add(self, rhs: Self) -> Self::Output {
        Self(self.0 + rhs.0)
    }
}

impl<T, I: Sub<Output = I>> Sub for Int<T, I> {
    type Output = Int<T, I>;

    #[inline]
    fn sub(self, rhs: Self) -> Self::Output {
        Self(self.0 - rhs.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_typed_bool() {
        struct IsVisible;
        let visible: Bool<IsVisible> = Bool::new(true);
        assert!(visible.get());
        assert!(Bool::<IsVisible>::true_().get());
        assert!(!Bool::<IsVisible>::false_().get());
    }

    #[test]
    fn test_typed_int() {
        struct UserId;
        struct ItemId;

        let user_id: Int<UserId, i32> = Int::new(42);
        let item_id: Int<ItemId, i32> = Int::new(42);

        assert_eq!(user_id.get(), 42);
        assert_ne!(user_id, item_id);
    }

    #[test]
    fn test_typed_int_add() {
        struct Count;
        let a: Int<Count, i32> = Int::new(5);
        let b: Int<Count, i32> = Int::new(3);
        let sum = a + b;
        assert_eq!(sum.get(), 8);
    }
}
