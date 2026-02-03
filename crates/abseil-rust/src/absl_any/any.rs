//! Type erasure wrapper.
//!
//! Provides a type-safe wrapper for values of unknown types.

use core::any::{Any as CoreAny, TypeId};

/// A type-safe wrapper for values of unknown types.
///
/// This is a wrapper around `core::any::Any` with additional convenience methods.
/// It allows you to store and work with values of a single unknown type.
///
/// # Examples
///
/// ```rust
/// use abseil::Any;
///
/// // Store an i32
/// let any_value = Any::new(42i32);
/// assert!(any_value.is::<i32>());
/// assert_eq!(*any_value.downcast_ref::<i32>().unwrap(), 42);
///
/// // Store a String
/// let any_value2 = Any::new("hello".to_string());
/// assert!(any_value2.is::<String>());
/// ```
pub struct Any {
    inner: Box<dyn CoreAny>,
    type_name: &'static str,
}

impl Any {
    /// Creates a new `Any` holding the given value.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use abseil::Any;
    ///
    /// let any_value = Any::new(42i32);
    /// ```
    #[inline]
    pub fn new<T: 'static>(value: T) -> Self {
        Any {
            inner: Box::new(value),
            type_name: core::any::type_name::<T>(),
        }
    }

    /// Returns true if the contained value is of type `T`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use abseil::Any;
    ///
    /// let any_value = Any::new(42i32);
    /// assert!(any_value.is::<i32>());
    /// assert!(!any_value.is::<i64>());
    /// ```
    #[inline]
    pub fn is<T: 'static>(&self) -> bool {
        self.inner.as_ref().is::<T>()
    }

    /// Returns a reference to the contained value if it is of type `T`.
    ///
    /// Returns `None` if the contained value is not of type `T`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use abseil::Any;
    ///
    /// let any_value = Any::new(42i32);
    /// assert_eq!(*any_value.downcast_ref::<i32>().unwrap(), 42);
    /// assert!(any_value.downcast_ref::<i64>().is_none());
    /// ```
    #[inline]
    pub fn downcast_ref<T: 'static>(&self) -> Option<&T> {
        self.inner.as_ref().downcast_ref::<T>()
    }

    /// Returns a mutable reference to the contained value if it is of type `T`.
    ///
    /// Returns `None` if the contained value is not of type `T`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use abseil::Any;
    ///
    /// let mut any_value = Any::new(42i32);
    /// *any_value.downcast_mut::<i32>().unwrap() = 100;
    /// assert_eq!(*any_value.downcast_ref::<i32>().unwrap(), 100);
    /// ```
    #[inline]
    pub fn downcast_mut<T: 'static>(&mut self) -> Option<&mut T> {
        self.inner.as_mut().downcast_mut::<T>()
    }

    /// Returns the contained value if it is of type `T`, consuming the `Any`.
    ///
    /// Returns `None` if the contained value is not of type `T`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use abseil::Any;
    ///
    /// let any_value = Any::new(42i32);
    /// assert_eq!(any_value.downcast::<i32>().unwrap(), 42);
    /// ```
    #[inline]
    pub fn downcast<T: 'static>(self) -> Result<T, Self> {
        self.inner.downcast::<T>().map(|boxed| *boxed).map_err(|boxed| Any {
            inner: boxed,
            type_name: self.type_name,
        })
    }

    /// Returns the `TypeId` of the contained value.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use abseil::Any;
    /// use core::any::TypeId;
    ///
    /// let any_value = Any::new(42i32);
    /// assert_eq!(any_value.type_id(), TypeId::of::<i32>());
    /// ```
    #[inline]
    pub fn type_id(&self) -> TypeId {
        self.inner.as_ref().type_id()
    }

    /// Returns the type name of the contained value.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use abseil::Any;
    ///
    /// let any_value = Any::new(42i32);
    /// assert_eq!(any_value.type_name(), "i32");
    /// ```
    #[inline]
    pub fn type_name(&self) -> &'static str {
        self.type_name
    }
}

impl core::fmt::Debug for Any {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_tuple("Any")
            .field(&self.type_name())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_any_new() {
        let any_value = Any::new(42i32);
        assert!(any_value.is::<i32>());
    }

    #[test]
    fn test_any_is() {
        let any_value = Any::new(42i32);
        assert!(any_value.is::<i32>());
        assert!(!any_value.is::<i64>());
        assert!(!any_value.is::<String>());
    }

    #[test]
    fn test_any_downcast_ref() {
        let any_value = Any::new(42i32);
        assert_eq!(*any_value.downcast_ref::<i32>().unwrap(), 42);
        assert!(any_value.downcast_ref::<i64>().is_none());
    }

    #[test]
    fn test_any_downcast_mut() {
        let mut any_value = Any::new(42i32);
        *any_value.downcast_mut::<i32>().unwrap() = 100;
        assert_eq!(*any_value.downcast_ref::<i32>().unwrap(), 100);
    }

    #[test]
    fn test_any_downcast() {
        let any_value = Any::new(42i32);
        assert_eq!(any_value.downcast::<i32>().unwrap(), 42);

        let any_value2 = Any::new(42i32);
        assert!(any_value2.downcast::<i64>().is_err());
    }

    #[test]
    fn test_any_type_id() {
        use core::any::TypeId;
        let any_value = Any::new(42i32);
        assert_eq!(any_value.type_id(), TypeId::of::<i32>());
    }

    #[test]
    fn test_any_type_name() {
        let any_value = Any::new(42i32);
        assert_eq!(any_value.type_name(), "i32");

        let any_value2 = Any::new("hello");
        assert_eq!(any_value2.type_name(), "&str");
    }

    #[test]
    fn test_any_string() {
        let any_value = Any::new("hello".to_string());
        assert!(any_value.is::<String>());
        assert_eq!(*any_value.downcast_ref::<String>().unwrap(), "hello");
    }

    #[test]
    fn test_any_debug() {
        let any_value = Any::new(42i32);
        assert_eq!(format!("{:?}", any_value), "Any(\"i32\")");
    }

    #[test]
    fn test_any_with_vec() {
        let any_value = Any::new(vec![1, 2, 3]);
        assert!(any_value.is::<Vec<i32>>());
        assert_eq!(*any_value.downcast_ref::<Vec<i32>>().unwrap(), vec![1, 2, 3]);
    }
}
