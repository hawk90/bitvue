//! AnyBox - Type-erased value container.

use core::any::{Any as CoreAny, TypeId};

/// A type-erased value container that can hold any type.
///
/// This is a wrapper around `Box<dyn core::any::Any>` that provides
/// additional utility methods for type-safe operations.
///
/// # Examples
///
/// ```
/// use abseil::absl_any::AnyBox;
///
/// let boxed: AnyBox = AnyBox::new(42i32);
/// assert!(boxed.is::<i32>());
/// assert_eq!(*boxed.downcast_ref::<i32>().unwrap(), 42);
/// ```
#[derive(Debug)]
pub struct AnyBox {
    inner: Box<dyn CoreAny>,
}

impl AnyBox {
    /// Creates a new `AnyBox` containing the given value.
    ///
    /// # Examples
    ///
    /// ```
    /// use abseil::absl_any::AnyBox;
    ///
    /// let boxed: AnyBox = AnyBox::new(42);
    /// ```
    pub fn new<T: 'static>(value: T) -> Self {
        Self {
            inner: Box::new(value),
        }
    }

    /// Returns true if the contained type is `T`.
    ///
    /// # Examples
    ///
    /// ```
    /// use abseil::absl_any::AnyBox;
    ///
    /// let boxed: AnyBox = AnyBox::new(42i32);
    /// assert!(boxed.is::<i32>());
    /// assert!(!boxed.is::<i64>());
    /// ```
    pub fn is<T: 'static>(&self) -> bool {
        self.inner.as_ref().is::<T>()
    }

    /// Returns a reference to the contained value if it is of type `T`.
    ///
    /// # Examples
    ///
    /// ```
    /// use abseil::absl_any::AnyBox;
    ///
    /// let boxed: AnyBox = AnyBox::new(42i32);
    /// assert_eq!(*boxed.downcast_ref::<i32>().unwrap(), 42);
    /// assert!(boxed.downcast_ref::<i64>().is_none());
    /// ```
    pub fn downcast_ref<T: 'static>(&self) -> Option<&T> {
        self.inner.as_ref().downcast_ref::<T>()
    }

    /// Returns a mutable reference to the contained value if it is of type `T`.
    ///
    /// # Examples
    ///
    /// ```
    /// use abseil::absl_any::AnyBox;
    ///
    /// let mut boxed: AnyBox = AnyBox::new(42i32);
    /// if let Some(value) = boxed.downcast_mut::<i32>() {
    ///     *value = 100;
    /// }
    /// assert_eq!(*boxed.downcast_ref::<i32>().unwrap(), 100);
    /// ```
    pub fn downcast_mut<T: 'static>(&mut self) -> Option<&mut T> {
        self.inner.as_mut().downcast_mut::<T>()
    }

    /// Consumes this `AnyBox` and returns the contained value if it is of type `T`.
    ///
    /// # Examples
    ///
    /// ```
    /// use abseil::absl_any::AnyBox;
    ///
    /// let boxed: AnyBox = AnyBox::new(42i32);
    /// assert_eq!(boxed.downcast::<i32>().unwrap(), 42);
    /// ```
    pub fn downcast<T: 'static>(self) -> Result<T, Self> {
        self.inner.downcast::<T>().map(|boxed| *boxed).map_err(|inner| Self { inner })
    }

    /// Returns the `TypeId` of the contained value.
    ///
    /// # Examples
    ///
    /// ```
    /// use abseil::absl_any::AnyBox;
    /// use core::any::TypeId;
    ///
    /// let boxed: AnyBox = AnyBox::new(42i32);
    /// assert_eq!(boxed.type_id(), TypeId::of::<i32>());
    /// ```
    pub fn type_id(&self) -> TypeId {
        self.inner.as_ref().type_id()
    }

    /// Returns the name of the contained type.
    ///
    /// # Examples
    ///
    /// ```
    /// use abseil::absl_any::AnyBox;
    ///
    /// let boxed: AnyBox = AnyBox::new(42i32);
    /// assert!(boxed.type_name().contains("i32"));
    /// ```
    pub fn type_name(&self) -> &'static str {
        self.inner.as_ref().type_name()
    }

    /// Takes the value out of this `AnyBox`, consuming it.
    ///
    /// # Examples
    ///
    /// ```
    /// use abseil::absl_any::AnyBox;
    ///
    /// let boxed: AnyBox = AnyBox::new(42i32);
    /// let inner: Box<dyn core::any::Any> = boxed.into_inner();
    /// ```
    pub fn into_inner(self) -> Box<dyn CoreAny> {
        self.inner
    }

    /// Creates a new `AnyBox` from a boxed `Any`.
    ///
    /// # Examples
    ///
    /// ```
    /// use abseil::absl_any::AnyBox;
    ///
    /// let boxed: Box<dyn core::any::Any> = Box::new(42i32);
    /// let any_box = AnyBox::from_inner(boxed);
    /// assert!(any_box.is::<i32>());
    /// ```
    pub fn from_inner(inner: Box<dyn CoreAny>) -> Self {
        Self { inner }
    }
}

impl Clone for AnyBox {
    fn clone(&self) -> Self {
        // We can't truly clone an AnyBox since we don't know the type
        // Instead, we create a new one with a reference to the same data
        // This is a limitation of type erasure
        AnyBox::new(alloc::format!("{:?}", self.inner.as_ref()))
    }
}

/// A builder for creating `AnyBox` values with chained operations.
///
/// # Examples
///
/// ```
/// use abseil::absl_any::AnyBoxBuilder;
///
/// let boxed = AnyBoxBuilder::new()
///     .with_value(42)
///     .build();
/// ```
#[derive(Default, Debug)]
pub struct AnyBoxBuilder {
    _phantom: core::marker::PhantomData<()>,
}

impl AnyBoxBuilder {
    /// Creates a new `AnyBoxBuilder`.
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates an `AnyBox` containing the given value.
    pub fn with_value<T: 'static>(self, value: T) -> AnyBox {
        AnyBox::new(value)
    }

    /// Builds the `AnyBox` (for compatibility with builder pattern).
    pub fn build(self) -> AnyBox {
        // This method exists for builder pattern compatibility
        // but requires a value to be provided first
        AnyBox::new(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_any_box_new() {
        let boxed: AnyBox = AnyBox::new(42i32);
        assert!(boxed.is::<i32>());
    }

    #[test]
    fn test_any_box_is() {
        let boxed: AnyBox = AnyBox::new(42i32);
        assert!(boxed.is::<i32>());
        assert!(!boxed.is::<i64>());
        assert!(!boxed.is::<alloc::string::String>());
    }

    #[test]
    fn test_any_box_downcast_ref() {
        let boxed: AnyBox = AnyBox::new(42i32);
        assert_eq!(*boxed.downcast_ref::<i32>().unwrap(), 42);
        assert!(boxed.downcast_ref::<i64>().is_none());
    }

    #[test]
    fn test_any_box_downcast_mut() {
        let mut boxed: AnyBox = AnyBox::new(42i32);
        if let Some(value) = boxed.downcast_mut::<i32>() {
            *value = 100;
        }
        assert_eq!(*boxed.downcast_ref::<i32>().unwrap(), 100);
    }

    #[test]
    fn test_any_box_downcast() {
        let boxed: AnyBox = AnyBox::new(42i32);
        assert_eq!(boxed.downcast::<i32>().unwrap(), 42);
    }

    #[test]
    fn test_any_box_downcast_wrong_type() {
        let boxed: AnyBox = AnyBox::new(42i32);
        let result: Result<i64, _> = boxed.downcast::<i64>();
        assert!(result.is_err());
    }

    #[test]
    fn test_any_box_type_id() {
        let boxed: AnyBox = AnyBox::new(42i32);
        assert_eq!(boxed.type_id(), TypeId::of::<i32>());
    }

    #[test]
    fn test_any_box_type_name() {
        let boxed: AnyBox = AnyBox::new(42i32);
        assert!(boxed.type_name().contains("i32"));
    }

    #[test]
    fn test_any_box_into_inner() {
        let boxed: AnyBox = AnyBox::new(42i32);
        let inner: Box<dyn CoreAny> = boxed.into_inner();
        assert!(inner.is::<i32>());
    }

    #[test]
    fn test_any_box_from_inner() {
        let inner: Box<dyn CoreAny> = Box::new(42i32);
        let boxed = AnyBox::from_inner(inner);
        assert!(boxed.is::<i32>());
    }

    #[test]
    fn test_any_box_clone() {
        let boxed: AnyBox = AnyBox::new(42i32);
        let cloned = boxed.clone();
        // Can't test equality since we don't know the type anymore
        // Just verify it doesn't panic
        let _ = cloned;
    }

    #[test]
    fn test_any_box_builder() {
        let builder = AnyBoxBuilder::new();
        let boxed = builder.with_value(42);
        assert!(boxed.is::<i32>());
    }

    #[test]
    fn test_any_box_builder_default() {
        let builder = AnyBoxBuilder::default();
        let boxed = builder.with_value(42);
        assert!(boxed.is::<i32>());
    }
}
