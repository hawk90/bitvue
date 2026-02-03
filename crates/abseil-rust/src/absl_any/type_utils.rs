//! Type utility functions.

use core::any::TypeId;

/// Returns the type name of the given value.
///
/// # Examples
///
/// ```
/// use abseil::absl_any::type_name_of;
///
/// assert_eq!(type_name_of(&42), "i32");
/// assert!(type_name_of(&"hello").contains("str"));
/// ```
pub fn type_name_of<T: ?Sized>(_value: &T) -> &'static str {
    core::any::type_name::<T>()
}

/// Returns the `TypeId` of the given type.
///
/// # Examples
///
/// ```
/// use abseil::absl_any::type_id_of;
/// use core::any::TypeId;
///
/// assert_eq!(type_id_of::<i32>(), TypeId::of::<i32>());
/// ```
pub fn type_id_of<T: 'static>() -> TypeId {
    TypeId::of::<T>()
}

/// Checks if two values have the same type.
///
/// # Examples
///
/// ```
/// use abseil::absl_any::is_same_type;
///
/// assert!(is_same_type(&42i32, &100i32));
/// assert!(!is_same_type(&42i32, &"hello"));
/// ```
pub fn is_same_type<T: ?Sized, U: ?Sized>(_a: &T, _b: &U) -> bool {
    TypeId::of::<T>() == TypeId::of::<U>()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_type_name_of() {
        assert_eq!(type_name_of(&42), "i32");
        assert!(type_name_of(&"hello").contains("str"));
        assert!(type_name_of(&(1, 2, 3)).contains("tuple"));
    }

    #[test]
    fn test_type_id_of() {
        assert_eq!(type_id_of::<i32>(), TypeId::of::<i32>());
        assert_ne!(type_id_of::<i32>(), TypeId::of::<i64>());
    }

    #[test]
    fn test_is_same_type() {
        assert!(is_same_type(&42i32, &100i32));
        assert!(!is_same_type(&42i32, &"hello"));
        assert!(is_same_type(&"hello", &"world"));
    }
}
