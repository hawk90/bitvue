//! Downcast utilities for type-erased values.

use core::any::{Any as CoreAny};

/// Attempts to downcast a reference to `dyn Any` to a specific type.
///
/// # Examples
///
/// ```
/// use abseil::absl_any::downcast_ref;
/// use core::any::Any;
///
/// let value: &dyn Any = &42i32;
/// assert_eq!(*downcast_ref::<i32>(value).unwrap(), 42);
/// ```
pub fn downcast_ref<T: 'static>(value: &dyn CoreAny) -> Option<&T> {
    value.downcast_ref::<T>()
}

/// Attempts to downcast a mutable reference to `dyn Any` to a specific type.
///
/// # Examples
///
/// ```
/// use abseil::absl_any::downcast_mut;
/// use core::any::Any;
///
/// let mut value: Box<dyn Any> = Box::new(42i32);
/// if let Some(v) = downcast_mut::<i32>(&mut *value) {
///     *v = 100;
/// }
/// ```
pub fn downcast_mut<T: 'static>(value: &mut dyn CoreAny) -> Option<&mut T> {
    value.downcast_mut::<T>()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_downcast_ref_function() {
        let value: &dyn CoreAny = &42i32;
        assert_eq!(*downcast_ref::<i32>(value).unwrap(), 42);
        assert!(downcast_ref::<i64>(value).is_none());
    }

    #[test]
    fn test_downcast_mut_function() {
        let mut value: Box<dyn CoreAny> = Box::new(42i32);
        if let Some(v) = downcast_mut::<i32>(&mut *value) {
            *v = 100;
        }
        assert_eq!(*downcast_ref::<i32>(&*value).unwrap(), 100);
    }
}
