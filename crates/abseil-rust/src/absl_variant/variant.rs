//! Variant type implementation.
//!
//! Provides a type-safe wrapper for sum types (enums).
//!
//! This module provides compatibility helpers for working with Rust's
//! enum types in a way similar to C++'s std::variant.

use core::any::TypeId;

/// Error returned when trying to get the wrong variant type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VariantMatchError {
    /// The type ID that was requested.
    pub requested: &'static str,
    /// The type ID that was actually present.
    pub actual: &'static str,
}

impl core::fmt::Display for VariantMatchError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "Variant type mismatch: requested {}, found {}",
            self.requested, self.actual
        )
    }
}

#[cfg(feature = "std")]
impl std::error::Error for VariantMatchError {}

/// Marker trait for variant types.
pub trait VariantType: 'static {}

impl<T: 'static> VariantType for T {}

/// A type-safe wrapper for a single value type.
///
/// This is a simple wrapper that provides type-safe access. For multi-type
/// variants, Rust's native enum system is recommended:
///
/// ```rust
/// enum MyVariant {
///     Integer(i32),
///     Float(f64),
///     Text(String),
/// }
/// ```
///
/// # Examples
///
/// ```rust
/// use abseil::Variant;
///
/// let v: Variant<i32> = Variant::new(42);
/// assert!(v.is::<i32>());
/// assert_eq!(*v.get::<i32>().unwrap(), 42);
/// ```
#[derive(Clone, Copy, Default)]
pub struct Variant<T: VariantType> {
    value: T,
}

impl<T: VariantType> Variant<T> {
    /// Creates a new Variant holding the given value.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use abseil::Variant;
    ///
    /// let v: Variant<i32> = Variant::new(42);
    /// ```
    #[inline]
    pub fn new(value: T) -> Self {
        Variant { value }
    }

    /// Returns true if the variant currently holds type U.
    ///
    /// For single-type variants, this always returns true if U equals T.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use abseil::Variant;
    ///
    /// let v: Variant<i32> = Variant::new(42);
    /// assert!(v.is::<i32>());
    /// assert!(!v.is::<f64>());
    /// ```
    #[inline]
    pub fn is<U: VariantType>(&self) -> bool {
        TypeId::of::<U>() == TypeId::of::<T>()
    }

    /// Returns a reference to the contained value if it is of type U.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use abseil::Variant;
    ///
    /// let v: Variant<i32> = Variant::new(42);
    /// assert_eq!(*v.get::<i32>().unwrap(), 42);
    /// ```
    #[inline]
    pub fn get<U: VariantType>(&self) -> Option<&T> {
        if self.is::<U>() {
            Some(&self.value)
        } else {
            None
        }
    }

    /// Returns a mutable reference to the contained value if it is of type U.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use abseil::Variant;
    ///
    /// let mut v: Variant<i32> = Variant::new(42);
    /// *v.get_mut::<i32>().unwrap() = 100;
    /// assert_eq!(*v.get::<i32>().unwrap(), 100);
    /// ```
    #[inline]
    pub fn get_mut<U: VariantType>(&mut self) -> Option<&mut T> {
        if self.is::<U>() {
            Some(&mut self.value)
        } else {
            None
        }
    }

    /// Returns the contained value if it is of type U, consuming the variant.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use abseil::Variant;
    ///
    /// let v: Variant<i32> = Variant::new(42);
    /// let value = v.take::<i32>().unwrap();
    /// assert_eq!(value, 42);
    /// ```
    #[inline]
    pub fn take<U: VariantType>(self) -> Option<T> {
        if self.is::<U>() {
            Some(self.value)
        } else {
            None
        }
    }

    /// Returns a reference to the contained value.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use abseil::Variant;
    ///
    /// let v: Variant<i32> = Variant::new(42);
    /// assert_eq!(*v.as_ref(), 42);
    /// ```
    #[inline]
    pub fn as_ref(&self) -> &T {
        &self.value
    }

    /// Returns a mutable reference to the contained value.
    #[inline]
    pub fn as_mut(&mut self) -> &mut T {
        &mut self.value
    }

    /// Consumes the variant and returns the contained value.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use abseil::Variant;
    ///
    /// let v: Variant<i32> = Variant::new(42);
    /// let value = v.into_inner();
    /// assert_eq!(value, 42);
    /// ```
    #[inline]
    pub fn into_inner(self) -> T {
        self.value
    }
}

// Implement From<T> for Variant<T>
impl<T: VariantType> From<T> for Variant<T> {
    #[inline]
    fn from(value: T) -> Self {
        Variant::new(value)
    }
}

// Implement AsRef<T> for Variant<T>
impl<T: VariantType> AsRef<T> for Variant<T> {
    #[inline]
    fn as_ref(&self) -> &T {
        &self.value
    }
}

// Implement AsMut<T> for Variant<T>
impl<T: VariantType> AsMut<T> for Variant<T> {
    #[inline]
    fn as_mut(&mut self) -> &mut T {
        &mut self.value
    }
}

// Implement PartialEq for Variant when T is PartialEq
impl<T: PartialEq + VariantType> PartialEq for Variant<T> {
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value
    }
}

// Implement Eq for Variant when T is Eq
impl<T: Eq + VariantType> Eq for Variant<T> {}

// Implement PartialOrd for Variant when T is PartialOrd
impl<T: PartialOrd + VariantType> PartialOrd for Variant<T> {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        self.value.partial_cmp(&other.value)
    }
}

// Implement Ord for Variant when T is Ord
impl<T: Ord + VariantType> Ord for Variant<T> {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        self.value.cmp(&other.value)
    }
}

// Implement core::fmt::Debug for Variant
impl<T: core::fmt::Debug + VariantType> core::fmt::Debug for Variant<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "Variant({:?})", self.value)
    }
}

// Implement core::fmt::Display for Variant
impl<T: core::fmt::Display + VariantType> core::fmt::Display for Variant<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.value)
    }
}

// Implement Hash for Variant when T is Hash
#[cfg(feature = "std")]
impl<T: std::hash::Hash + VariantType> std::hash::Hash for Variant<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.value.hash(state);
    }
}

/// Convenience macro for creating variants.
///
/// # Examples
///
/// ```rust
/// use abseil::{variant, Variant};
///
/// let v: Variant<i32> = variant!(42);
/// ```
#[macro_export]
macro_rules! variant {
    ($value:expr) => {
        $crate::absl_variant::variant::Variant::new($value)
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_variant_new() {
        let v: Variant<i32> = Variant::new(42);
        assert!(v.is::<i32>());
        assert!(!v.is::<f64>());
    }

    #[test]
    fn test_variant_get() {
        let v: Variant<i32> = Variant::new(42);
        assert_eq!(*v.get::<i32>().unwrap(), 42);
        assert!(v.get::<f64>().is_none());
    }

    #[test]
    fn test_variant_get_mut() {
        let mut v: Variant<i32> = Variant::new(42);
        *v.get_mut::<i32>().unwrap() = 100;
        assert_eq!(*v.get::<i32>().unwrap(), 100);
    }

    #[test]
    fn test_variant_take() {
        let v: Variant<i32> = Variant::new(42);
        let value = v.take::<i32>().unwrap();
        assert_eq!(value, 42);
    }

    #[test]
    fn test_variant_as_ref() {
        let v: Variant<i32> = Variant::new(42);
        assert_eq!(*v.as_ref(), 42);
    }

    #[test]
    fn test_variant_as_mut() {
        let mut v: Variant<i32> = Variant::new(42);
        *v.as_mut() = 100;
        assert_eq!(*v.as_ref(), 100);
    }

    #[test]
    fn test_variant_into_inner() {
        let v: Variant<i32> = Variant::new(42);
        let value = v.into_inner();
        assert_eq!(value, 42);
    }

    #[test]
    fn test_variant_from() {
        let v: Variant<i32> = Variant::from(42);
        assert_eq!(*v.as_ref(), 42);
    }

    #[test]
    fn test_variant_equality() {
        let v1: Variant<i32> = Variant::new(42);
        let v2: Variant<i32> = Variant::new(42);
        let v3: Variant<i32> = Variant::new(100);

        assert_eq!(v1, v2);
        assert_ne!(v1, v3);
    }

    #[test]
    fn test_variant_ordering() {
        let v1: Variant<i32> = Variant::new(1);
        let v2: Variant<i32> = Variant::new(2);

        assert!(v1 < v2);
        assert!(v2 > v1);
    }

    #[test]
    fn test_variant_default() {
        let v: Variant<i32> = Variant::default();
        assert_eq!(*v.as_ref(), 0);
    }

    #[test]
    fn test_variant_clone() {
        let v1: Variant<i32> = Variant::new(42);
        let v2 = v1;
        assert_eq!(v1, v2);
    }

    #[test]
    fn test_variant_debug() {
        let v: Variant<i32> = Variant::new(42);
        assert_eq!(format!("{:?}", v), "Variant(42)");
    }

    #[test]
    fn test_variant_display() {
        let v: Variant<i32> = Variant::new(42);
        assert_eq!(format!("{}", v), "42");
    }

    #[cfg(feature = "std")]
    #[test]
    fn test_variant_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(Variant::<i32>::new(42));
        set.insert(Variant::<i32>::new(42));
        assert_eq!(set.len(), 1);
    }

    #[test]
    fn test_variant_macro() {
        let v: Variant<i32> = variant!(42);
        assert_eq!(*v.as_ref(), 42);
    }
}
