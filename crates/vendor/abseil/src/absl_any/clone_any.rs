//! CloneAny - A type-erased container that supports cloning.

use core::any::{Any as CoreAny, TypeId};
use crate::absl_any::any_box::AnyBox;

/// A wrapper that allows cloning through type erasure.
///
/// This requires the wrapped type to implement `Clone`.
///
/// # Examples
///
/// ```
/// use abseil::absl_any::CloneAny;
///
/// let value: CloneAny = CloneAny::new(42i32);
/// let cloned = value.clone();
/// assert!(cloned.is::<i32>());
/// ```
#[derive(Clone)]
pub struct CloneAny {
    inner: AnyBox,
    clone_fn: unsafe fn(*const ()) -> Box<dyn CoreAny>,
}

impl CloneAny {
    /// Creates a new `CloneAny` containing the given value.
    ///
    /// # Examples
    ///
    /// ```
    /// use abseil::absl_any::CloneAny;
    ///
    /// let value: CloneAny = CloneAny::new(42i32);
    /// ```
    pub fn new<T: Clone + 'static>(value: T) -> Self {
        unsafe fn clone_box<T: Clone>(t: *const ()) -> Box<dyn CoreAny> {
            let t: &T = &*(t as *const T);
            Box::new(t.clone())
        }

        Self {
            inner: AnyBox::new(value),
            clone_fn: clone_box::<T>,
        }
    }

    /// Returns true if the contained type is `T`.
    pub fn is<T: 'static>(&self) -> bool {
        self.inner.is::<T>()
    }

    /// Returns a reference to the contained value if it is of type `T`.
    pub fn downcast_ref<T: 'static>(&self) -> Option<&T> {
        self.inner.downcast_ref::<T>()
    }

    /// Returns a mutable reference to the contained value if it is of type `T`.
    pub fn downcast_mut<T: 'static>(&mut self) -> Option<&mut T> {
        self.inner.downcast_mut::<T>()
    }

    /// Returns the `TypeId` of the contained value.
    pub fn type_id(&self) -> TypeId {
        self.inner.type_id()
    }

    /// Returns the name of the contained type.
    pub fn type_name(&self) -> &'static str {
        self.inner.type_name()
    }
}

impl core::fmt::Debug for CloneAny {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("CloneAny")
            .field("type", &self.inner.type_name())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clone_any_new() {
        let value: CloneAny = CloneAny::new(42i32);
        assert!(value.is::<i32>());
    }

    #[test]
    fn test_clone_any_clone() {
        let value: CloneAny = CloneAny::new(42i32);
        let cloned = value.clone();
        assert!(cloned.is::<i32>());
    }

    #[test]
    fn test_clone_any_downcast_ref() {
        let value: CloneAny = CloneAny::new(42i32);
        assert_eq!(*value.downcast_ref::<i32>().unwrap(), 42);
    }

    #[test]
    fn test_clone_any_downcast_mut() {
        let mut value: CloneAny = CloneAny::new(42i32);
        if let Some(v) = value.downcast_mut::<i32>() {
            *v = 100;
        }
        assert_eq!(*value.downcast_ref::<i32>().unwrap(), 100);
    }

    #[test]
    fn test_clone_any_type_id() {
        let value: CloneAny = CloneAny::new(42i32);
        assert_eq!(value.type_id(), TypeId::of::<i32>());
    }

    #[test]
    fn test_clone_any_type_name() {
        let value: CloneAny = CloneAny::new(42i32);
        assert!(value.type_name().contains("i32"));
    }
}
