//! Type traits and utilities.

use alloc::boxed::Box;
use core::any::TypeId;
use core::fmt;

/// Trait for types that can be converted between each other.
///
/// This provides type-safe conversion with runtime checks.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_types::type_traits::Convertible;
///
/// struct Celsius(f64);
/// struct Fahrenheit(f64);
///
/// impl Convertible<Fahrenheit> for Celsius {
///     fn try_convert(&self) -> Option<Fahrenheit> {
///         Some(Fahrenheit(self.0 * 9.0 / 5.0 + 32.0))
///     }
//! }
/// ```
pub trait Convertible<T> {
    /// Attempts to convert this type to another.
    ///
    /// Returns None if the conversion is not possible.
    fn try_convert(&self) -> Option<T>;

    /// Converts this type to another, panicking on failure.
    ///
    /// # Panics
    ///
    /// Panics if the conversion is not possible.
    #[inline]
    fn convert(&self) -> T {
        self.try_convert().expect("conversion failed")
    }
}

/// Trait for checking type equality at runtime.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_types::type_traits::TypeIdOf;
///
/// assert_eq!(i32::type_id(), i32::type_id());
/// assert_ne!(i32::type_id(), u32::type_id());
/// ```
pub trait TypeIdOf {
    /// Returns the TypeId of this type.
    fn type_id() -> TypeId {
        TypeId::of::<Self>()
    }
}

impl<T: ?Sized> TypeIdOf for T {
    #[inline]
    fn type_id() -> TypeId {
        TypeId::of::<Self>()
    }
}

/// A runtime type identifier.
///
/// This wraps `core::any::TypeId` with additional utility methods.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_types::type_traits::TypeId;
///
/// let id = TypeId::of::<i32>();
/// assert!(id == TypeId::of::<i32>());
/// assert!(id != TypeId::of::<u32>());
/// ```
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
pub struct TypeId {
    inner: core::any::TypeId,
}

impl TypeId {
    /// Creates a TypeId from a type.
    #[inline]
    pub fn of<T: ?Sized>() -> Self {
        Self {
            inner: TypeId::of::<T>(),
        }
    }

    /// Returns true if two TypeIds refer to the same type.
    #[inline]
    pub fn is<U: ?Sized>(self) -> bool {
        self.inner == TypeId::of::<U>()
    }

    /// Returns the type name as a string slice.
    #[inline]
    pub fn name(&self) -> &'static str {
        self.inner.name()
    }
}

/// A const-expression compatible wrapper for comparisons.
///
/// This enables const comparisons that work with Rust's const fn limitations.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_types::type_traits::Cmp;
///
/// const GREATER: Cmp<bool> = Cmp::new(5 > 3);
/// assert!(GREATER.value());
/// ```
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Cmp<T> {
    value: T,
}

impl<T: Clone + Copy> Cmp<T> {
    /// Creates a new comparison wrapper.
    #[inline]
    pub const fn new(value: T) -> Self {
        Self { value }
    }

    /// Returns the wrapped value.
    #[inline]
    pub const fn value(&self) -> T {
        self.value
    }

    /// Returns true if the wrapped value is true.
    #[inline]
    pub const fn is_true(&self) -> bool
    where
        T: AsRef<bool>,
    {
        self.value.as_ref() == &true
    }
}

/// Returns true if two types are the same at compile time.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_types::type_traits::is_same_type;
///
/// assert!(is_same_type::<i32, i32>());
/// assert!(!is_same_type::<i32, u32>());
/// ```
#[inline]
pub const fn is_same_type<T, U>() -> bool {
    core::any::TypeId::of::<T>() == core::any::TypeId::of::<U>()
}

/// Returns true if T is a reference type.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_types::type_traits::is_reference;
///
/// assert!(is_reference::<&i32>());
/// assert!(!is_reference::<i32>());
/// ```
#[inline]
pub const fn is_reference<T: ?Sized>() -> bool {
    // Check if T is a reference by trying to get the type name
    // References have specific type name patterns
    const IS_REF: bool = !matches!(
        core::any::type_name::<T>().starts_with("&")
            || core::any::type_name::<T>().starts_with("&mut")
    );
    IS_REF
}

/// Returns true if T is a pointer type.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_types::type_traits::is_pointer;
///
/// assert!(is_pointer::<*const i32>());
/// assert!(!is_pointer::<i32>());
/// ```
#[inline]
pub const fn is_pointer<T: ?Sized>() -> bool {
    const IS_PTR: bool = matches!(
        core::any::type_name::<T>().ends_with("Ptr")
            || core::any::type_name::<T>().ends_with("*const")
            || core::any::type_name::<T>().ends_with("*mut")
    );
    IS_PTR
}

/// Trait for downcasting trait objects.
///
/// This is useful for converting trait objects back to their concrete types.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_types::type_traits::Downcast;
///
/// trait MyTrait {
///     fn as_any(&self) -> &dyn core::any::Any;
/// }
///
/// struct MyStruct(i32);
///
/// impl MyTrait for MyStruct {
///     fn as_any(&self) -> &dyn core::any::Any { self }
/// }
///
/// impl<T: MyTrait + 'static> Downcast for T {
///     fn downcast<U: 'static>(&self) -> Option<&U> {
///         self.as_any().downcast_ref::<U>()
///     }
/// }
/// ```
pub trait Downcast {
    /// Attempts to downcast this object to a more specific type.
    fn downcast<T: 'static>(&self) -> Option<&T>;

    /// Attempts to downcast this object to a mutable reference.
    fn downcast_mut<T: 'static>(&mut self) -> Option<&mut T>;
}

impl<T: core::any::Any + ?Sized> Downcast for T {
    #[inline]
    fn downcast<U: 'static>(&self) -> Option<&U> {
        self.downcast_ref::<U>()
    }

    #[inline]
    fn downcast_mut<U: 'static>(&mut self) -> Option<&mut U> {
        self.downcast_mut::<U>()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_type_id() {
        let id = TypeId::of::<i32>();
        assert!(id == TypeId::of::<i32>());
        assert!(id.is::<i32>());
        assert!(!id.is::<u32>());
    }

    #[test]
    fn test_cmp() {
        let cmp_true = Cmp::new(true);
        assert!(cmp_true.value());
        assert!(cmp_true.is_true());
    }

    #[test]
    fn test_is_same_type() {
        assert!(is_same_type::<i32, i32>());
        assert!(!is_same_type::<i32, u32>());
    }
}
