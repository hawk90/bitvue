//! AnyBuilder - Comprehensive builder for type-erased containers.

use crate::absl_any::{any_box::AnyBox, clone_any::CloneAny};

/// A comprehensive builder for creating type-erased containers.
#[derive(Debug, Default)]
pub struct AnyBuilder {
    _phantom: core::marker::PhantomData<()>,
}

impl AnyBuilder {
    /// Creates a new AnyBuilder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a cloneable AnyBox.
    ///
    /// # Examples
    ///
    /// ```
    /// use abseil::absl_any::AnyBuilder;
    ///
    /// let boxed = AnyBuilder::new().cloneable(42i32);
    /// let cloned = boxed.clone();
    /// assert!(cloned.is::<i32>());
    /// ```
    pub fn cloneable<T: Clone + 'static>(self, value: T) -> CloneAny {
        CloneAny::new(value)
    }

    /// Creates a regular AnyBox.
    pub fn boxed<T: 'static>(self, value: T) -> AnyBox {
        AnyBox::new(value)
    }

    /// Creates a value with metadata.
    ///
    /// # Examples
    ///
    /// ```
    /// use abseil::absl_any::AnyBuilder;
    ///
    /// let (boxed, meta) = AnyBuilder::new()
    ///     .with_metadata(42i32, "answer");
    /// ```
    pub fn with_metadata<T: 'static>(self, value: T, _metadata: &str) -> (AnyBox, alloc::string::String) {
        (AnyBox::new(value), _metadata.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_any_builder_cloneable() {
        let builder = AnyBuilder::new();
        let cloned = builder.cloneable(42i32);
        assert!(cloned.is::<i32>());
    }

    #[test]
    fn test_any_builder_boxed() {
        let builder = AnyBuilder::new();
        let boxed = builder.boxed(42i32);
        assert!(boxed.is::<i32>());
    }

    #[test]
    fn test_any_builder_with_metadata() {
        let builder = AnyBuilder::new();
        let (boxed, meta) = builder.with_metadata(42i32, "answer");
        assert!(boxed.is::<i32>());
        assert_eq!(meta, "answer");
    }
}
