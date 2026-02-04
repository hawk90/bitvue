//! Typed index types.

use core::ops::{Add, Sub, AddAssign, SubAssign};

/// A typed index type.
///
/// This wraps a usize with a type parameter to prevent mixing indices
/// of different types.
///
/// # Examples
///
/// ```rust
//! use abseil::absl_types::TypedIndex;
//!
//! struct EntityId;
//! struct ItemId;
//!
//! let entity_index: TypedIndex<EntityId> = TypedIndex::new(0);
//! let item_index: TypedIndex<ItemId> = TypedIndex::new(0);
//!
//! // These won't compile - different types:
//! // assert_eq!(entity_index, item_index);
//! ```
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TypedIndex<T, I = usize>(pub I);

impl<T, I: Clone + Copy> TypedIndex<T, I> {
    /// Creates a new typed index.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use abseil::absl_types::TypedIndex;
    ///
    /// struct EntityId;
    /// let index: TypedIndex<EntityId> = TypedIndex::new(5);
    /// assert_eq!(index.get(), 5);
    /// ```
    #[inline]
    pub const fn new(index: I) -> Self {
        Self(index)
    }

    /// Returns the raw index value.
    #[inline]
    pub const fn get(&self) -> I {
        self.0
    }

    /// Creates a typed index from a raw value.
    ///
    /// # Safety
    ///
    /// The caller must ensure the index is valid for the type.
    #[inline]
    pub const unsafe fn from_raw_unchecked(index: I) -> Self {
        Self(index)
    }
}

impl<T, I: AddAssign + Add<Output = I>> Add for TypedIndex<T, I> {
    type Output = TypedIndex<T, I>;

    #[inline]
    fn add(self, rhs: Self) -> Self::Output {
        Self(self.0 + rhs.0)
    }
}

impl<T, I: SubAssign + Sub<Output = I>> Sub for TypedIndex<T, I> {
    type Output = TypedIndex<T, I>;

    #[inline]
    fn sub(self, rhs: Self) -> Self::Output {
        Self(self.0 - rhs.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_typed_index() {
        struct EntityId;
        struct ItemId;

        let entity: TypedIndex<EntityId> = TypedIndex::new(5);
        let item: TypedIndex<ItemId> = TypedIndex::new(5);

        assert_eq!(entity.get(), 5);
        assert_eq!(item.get(), 5);
        assert_ne!(entity, item);
    }
}
