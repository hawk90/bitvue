//! Type transformations and type manipulations.

/// Removes reference from a type.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_meta::RemoveReference;
///
/// type T1 = RemoveReference::<i32>;
/// type T2 = RemoveReference::<&i32>;
/// type T3 = RemoveReference::<&mut i32>;
/// ```
pub type RemoveReference<T> = <T as RemoveReferenceImpl>::Output;

pub trait RemoveReferenceImpl {
    type Output;
}

impl<T> RemoveReferenceImpl for T {
    type Output = T;
}

impl<T: ?Sized> RemoveReferenceImpl for &T {
    type Output = T;
}

impl<T: ?Sized> RemoveReferenceImpl for &mut T {
    type Output = T;
}

/// Adds const qualifier to type (conceptual - for type-level operations).
///
/// Note: Rust doesn't have a direct const type qualifier, so this is
/// primarily used for type-level computations and compatibility.
pub type AddConst<T> = T;

/// Removes const qualifier from type (conceptual).
pub type RemoveConst<T> = T;

/// Adds volatile qualifier to type (conceptual).
pub type AddVolatile<T> = T;

/// Removes volatile qualifier from type (conceptual).
pub type RemoveVolatile<T> = T;

/// Gets the underlying type from a reference or pointer.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_meta::RemovePointer;
///
/// type T1 = RemovePointer<i32>;
/// type T2 = RemovePointer<*const i32>;
/// type T3 = RemovePointer<*mut i32>;
/// ```
pub type RemovePointer<T> = <T as RemovePointerImpl>::Output;

pub trait RemovePointerImpl {
    type Output;
}

impl<T> RemovePointerImpl for T {
    type Output = T;
}

impl<T> RemovePointerImpl for *const T {
    type Output = T;
}

impl<T> RemovePointerImpl for *mut T {
    type Output = T;
}

/// Converts type to its const reference version.
pub type AsConstRef<'a, T> = &'a T;

/// Converts type to its mutable reference version.
pub type AsMutRef<'a, T> = &'a mut T;

/// Trait for getting the pointed-to type from a reference.
pub type RemoveConstRef<'a, T> = <&'a T as RemoveRefImpl>::Output;
pub type RemoveMutRef<'a, T> = <&'a mut T as RemoveRefImpl>::Output;

pub trait RemoveRefImpl {
    type Output;
}

impl<'a, T: ?Sized> RemoveRefImpl for &'a T {
    type Output = T;
}

impl<'a, T: ?Sized> RemoveRefImpl for &'a mut T {
    type Output = T;
}

#[cfg(test)]
mod tests {
    use super::*;

    // Tests for RemoveReference
    #[test]
    fn test_remove_reference() {
        type T1 = RemoveReference<i32>;
        type T2 = RemoveReference<&i32>;
        type T3 = RemoveReference<&mut i32>;

        // Verify through size
        assert_eq!(core::mem::size_of::<T1>(), 4);
        assert_eq!(core::mem::size_of::<T2>(), 4);
    }

    // Tests for RemovePointer
    #[test]
    fn test_remove_pointer() {
        type T1 = RemovePointer<i32>;
        type T2 = RemovePointer<*const i32>;
        type T3 = RemovePointer<*mut i32>;

        assert_eq!(core::mem::size_of::<T1>(), 4);
    }

    // Tests for RemoveConstRef
    #[test]
    fn test_remove_const_ref() {
        type T = RemoveConstRef::<i32>;
        assert_eq!(core::mem::size_of::<T>(), 0); // ZST for unsized type
    }

    // Tests for RemoveMutRef
    #[test]
    fn test_remove_mut_ref() {
        type T = RemoveMutRef::<i32>;
        assert_eq!(core::mem::size_of::<T>(), 0); // ZST for unsized type
    }
}
