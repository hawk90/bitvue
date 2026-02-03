//! Conversion utilities for type-erased values.

use crate::absl_any::{any_box::AnyBox, clone_any::CloneAny};

/// Converts a value to an `AnyBox`.
///
/// # Examples
///
/// ```
/// use abseil::absl_any::to_any_box;
///
/// let boxed = to_any_box(42i32);
/// assert!(boxed.is::<i32>());
/// ```
pub fn to_any_box<T: 'static>(value: T) -> AnyBox {
    AnyBox::new(value)
}

/// Converts a value to a `CloneAny`.
///
/// # Examples
///
/// ```
/// use abseil::absl_any::to_clone_any;
///
/// let cloned = to_clone_any(42i32);
/// assert!(cloned.is::<i32>());
/// ```
pub fn to_clone_any<T: Clone + 'static>(value: T) -> CloneAny {
    CloneAny::new(value)
}

/// Attempts to extract a value from an `AnyBox`.
///
/// # Examples
///
/// ```
/// use abseil::absl_any::{from_any_box, AnyBox};
///
/// let boxed = AnyBox::new(42i32);
/// assert_eq!(from_any_box::<i32>(boxed), Some(42));
/// ```
pub fn from_any_box<T: 'static>(boxed: AnyBox) -> Option<T> {
    boxed.downcast::<T>().ok()
}

/// A trait for types that can be converted to/from type-erased form.
///
/// # Examples
///
/// ```
/// use abseil::absl_any::TypeErasable;
///
/// struct MyStruct(i32);
///
/// impl TypeErasable for MyStruct {
///     type AnyType = AnyBox;
///
///     fn into_any(self) -> AnyBox {
///         AnyBox::new(self)
///     }
///
///     fn from_any(any: AnyBox) -> Option<Self> {
///         any.downcast::<Self>().ok()
///     }
/// }
/// ```
pub trait TypeErasable {
    /// The type-erased container type.
    type AnyType;

    /// Converts this value to its type-erased form.
    fn into_any(self) -> Self::AnyType;

    /// Attempts to convert from the type-erased form.
    fn from_any(any: Self::AnyType) -> Option<Self>
    where
        Self: Sized;
}

impl<T: 'static> TypeErasable for T {
    type AnyType = AnyBox;

    fn into_any(self) -> AnyBox {
        AnyBox::new(self)
    }

    fn from_any(any: AnyBox) -> Option<Self> {
        any.downcast::<T>().ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_to_any_box() {
        let boxed = to_any_box(42i32);
        assert!(boxed.is::<i32>());
    }

    #[test]
    fn test_to_clone_any() {
        let cloned = to_clone_any(42i32);
        assert!(cloned.is::<i32>());
    }

    #[test]
    fn test_from_any_box() {
        let boxed = AnyBox::new(42i32);
        assert_eq!(from_any_box::<i32>(boxed), Some(42));
    }

    #[test]
    fn test_from_any_box_wrong_type() {
        let boxed = AnyBox::new(42i32);
        assert!(from_any_box::<i64>(boxed).is_none());
    }

    #[test]
    fn test_type_erasable_int() {
        let value: i32 = 42;
        let any = value.into_any();
        assert!(any.is::<i32>());
        assert_eq!(i32::from_any(any), Some(42));
    }

    #[test]
    fn test_type_erasable_string() {
        let value: alloc::string::String = "hello".to_string();
        let any = value.into_any();
        assert!(any.is::<alloc::string::String>());
        assert_eq!(alloc::string::String::from_any(any), Some("hello".to_string()));
    }
}
