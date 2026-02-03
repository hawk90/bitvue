//! Type introspection traits for compile-time type information.

/// Trait for types that can be compared for equality at compile time.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_meta::TypeEq;
///
/// assert_eq!(TypeEq::<i32, i32>::VALUE, true);
/// assert_eq!(TypeEq::<i32, u32>::VALUE, false);
/// ```
pub trait TypeEq<T> {
    /// True if Self and T are the same type.
    const VALUE: bool;
}

// Every type is equal to itself
impl<T> TypeEq<T> for T {
    const VALUE: bool = true;
}

/// Trait for getting the size of a type at compile time.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_meta::SizeOf;
///
/// assert_eq!(SizeOf::<i32>::VALUE, 4);
/// assert_eq!(SizeOf::<i64>::VALUE, 8);
/// ```
pub trait SizeOf {
    /// The size of the type in bytes.
    const VALUE: usize;
}

impl<T> SizeOf for T {
    const VALUE: usize = core::mem::size_of::<T>();
}

/// Trait for getting the alignment of a type at compile time.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_meta::AlignOf;
///
/// assert_eq!(AlignOf::<i32>::VALUE, 4);
/// assert_eq!(AlignOf::<i64>::VALUE, 8);
/// ```
pub trait AlignOf {
    /// The alignment of the type in bytes.
    const VALUE: usize;
}

impl<T> AlignOf for T {
    const VALUE: usize = core::mem::align_of::<T>();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_size_of() {
        assert_eq!(SizeOf::<i32>::VALUE, 4);
        assert_eq!(SizeOf::<i64>::VALUE, 8);
        assert_eq!(SizeOf::<u8>::VALUE, 1);
        assert_eq!(SizeOf::<bool>::VALUE, 1);
    }

    #[test]
    fn test_align_of() {
        assert_eq!(AlignOf::<i32>::VALUE, 4);
        assert_eq!(AlignOf::<i64>::VALUE, 8);
        assert_eq!(AlignOf::<u8>::VALUE, 1);
    }
}
