//! Type marker traits for compile-time type checking.

/// Marker trait for reference types.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_meta::IsReference;
///
/// assert!(IsReference::<&i32>::VALUE);
/// assert!(!IsReference::<i32>::VALUE);
/// assert!(IsReference::<&mut i32>::VALUE);
/// ```
pub trait IsReference {
    /// True if this is a reference type.
    const VALUE: bool = false;
}

impl<T: ?Sized> IsReference for &T {
    const VALUE: bool = true;
}

impl<T: ?Sized> IsReference for &mut T {
    const VALUE: bool = true;
}

/// Marker trait for pointer types.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_meta::IsPointer;
///
/// assert!(IsPointer::<*const i32>::VALUE);
/// assert!(IsPointer::<*mut i32>::VALUE);
/// assert!(!IsPointer::<&i32>::VALUE);
/// assert!(!IsPointer::<i32>::VALUE);
/// ```
pub trait IsPointer {
    /// True if this is a raw pointer type.
    const VALUE: bool = false;
}

impl<T> IsPointer for *const T {
    const VALUE: bool = true;
}

impl<T> IsPointer for *mut T {
    const VALUE: bool = true;
}

/// Marker trait for array types.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_meta::IsArray;
///
/// assert!(IsArray::<[i32; 5]>::VALUE);
/// assert!(IsArray::<[i32]>::VALUE);
/// assert!(!IsArray::<i32>::VALUE);
/// ```
pub trait IsArray {
    /// True if this is an array type.
    const VALUE: bool = false;
}

impl<T, const N: usize> IsArray for [T; N] {
    const VALUE: bool = true;
}

impl<T> IsArray for [T] {
    const VALUE: bool = true;
}

/// Trait for getting the length of a fixed-size array at compile time.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_meta::ArrayLen;
///
/// assert_eq!(ArrayLen::<[i32; 5]>::VALUE, 5);
/// ```
pub trait ArrayLen {
    /// The length of the array.
    const VALUE: usize = 0;
}

impl<T, const N: usize> ArrayLen for [T; N] {
    const VALUE: usize = N;
}

/// Trait for types that are safe to transmute to/from bytes.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_meta::IsPod;
///
/// assert!(IsPod::<i32>::VALUE);
/// assert!(IsPod::<u64>::VALUE);
/// ```
pub trait IsPod {
    /// True if the type is Plain Old Data (safe for byte operations).
    const VALUE: bool = false;
}

macro_rules! impl_is_pod {
    ($($t:ty),*) => {
        $(
            impl IsPod for $t {
                const VALUE: bool = true;
            }
        )*
    };
}

impl_is_pod!(
    u8, u16, u32, u64, u128, usize,
    i8, i16, i32, i64, i128, isize,
    f32, f64,
    bool, char
);

/// Trait for compile-time checking if a type implements Copy.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_meta::HasCopyTrait;
///
/// assert!(HasCopyTrait::<i32>::VALUE);
/// assert!(!HasCopyTrait::<Vec<i32>>::VALUE);
/// ```
pub trait HasCopyTrait {
    /// True if the type implements Copy.
    const VALUE: bool = false;
}

impl<T: Copy> HasCopyTrait for T {
    const VALUE: bool = true;
}

/// Trait for compile-time checking if a type is a primitive.
///
/// Primitives are the built-in scalar types.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_meta::IsPrimitive;
///
/// assert!(IsPrimitive::<i32>::VALUE);
/// assert!(IsPrimitive::<bool>::VALUE);
/// assert!(!IsPrimitive::<String>::VALUE);
/// ```
pub trait IsPrimitive {
    /// True if this is a primitive type.
    const VALUE: bool = false;
}

macro_rules! impl_is_primitive {
    ($($t:ty),*) => {
        $(
            impl IsPrimitive for $t {
                const VALUE: bool = true;
            }
        )*
    };
}

impl_is_primitive!(
    u8, u16, u32, u64, u128, usize,
    i8, i16, i32, i64, i128, isize,
    f32, f64,
    bool, char
);

/// Trait for function types.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_meta::IsFunction;
///
/// fn foo() {}
///
/// assert!(IsFunction::<fn()>::VALUE);
/// assert!(IsFunction::<fn(i32) -> i32>::VALUE);
/// assert!(!IsFunction::<i32>::VALUE);
/// ```
pub trait IsFunction {
    /// True if this is a function pointer type.
    const VALUE: bool = false;
}

impl<A: 'static, R: 'static> IsFunction for fn(A) -> R {
    const VALUE: bool = true;
}

impl<R: 'static> IsFunction for fn() -> R {
    const VALUE: bool = true;
}

impl<A: 'static> IsFunction for fn(A) {
    const VALUE: bool = true;
}

impl IsFunction for fn() {
    const VALUE: bool = true;
}

/// Checks if a type is const-qualified (always false in Rust).
pub trait IsConst {
    const VALUE: bool = false;
}

impl<T> IsConst for T {}

/// Checks if a type is volatile-qualified (always false in Rust).
pub trait IsVolatile {
    const VALUE: bool = false;
}

impl<T> IsVolatile for T {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_reference() {
        assert!(IsReference::<&i32>::VALUE);
        assert!(IsReference::<&mut i32>::VALUE);
        assert!(!IsReference::<i32>::VALUE);
    }

    #[test]
    fn test_is_pointer() {
        assert!(IsPointer::<*const i32>::VALUE);
        assert!(IsPointer::<*mut i32>::VALUE);
        assert!(!IsPointer::<&i32>::VALUE);
        assert!(!IsPointer::<i32>::VALUE);
    }

    #[test]
    fn test_is_array() {
        assert!(IsArray::<[i32; 5]>::VALUE);
        assert!(IsArray::<[i32]>::VALUE);
        assert!(!IsArray::<i32>::VALUE);
    }

    #[test]
    fn test_array_len() {
        assert_eq!(ArrayLen::<[i32; 5]>::VALUE, 5);
        assert_eq!(ArrayLen::<[i32; 100]>::VALUE, 100);
    }

    #[test]
    fn test_is_pod() {
        assert!(IsPod::<i32>::VALUE);
        assert!(IsPod::<u64>::VALUE);
        assert!(IsPod::<bool>::VALUE);
        assert!(IsPod::<f32>::VALUE);
    }

    #[test]
    fn test_has_copy_trait() {
        assert!(HasCopyTrait::<i32>::VALUE);
        assert!(HasCopyTrait::<bool>::VALUE);
    }

    #[test]
    fn test_is_primitive() {
        assert!(IsPrimitive::<i32>::VALUE);
        assert!(IsPrimitive::<bool>::VALUE);
        assert!(IsPrimitive::<f64>::VALUE);
        assert!(IsPrimitive::<char>::VALUE);
    }

    #[test]
    fn test_is_function() {
        fn foo() {}
        fn bar(x: i32) -> i32 { x }

        assert!(IsFunction::<fn()>::VALUE);
        assert!(IsFunction::<fn(i32) -> i32>::VALUE);
        assert!(!IsFunction::<i32>::VALUE);
    }

    #[test]
    fn test_is_const() {
        assert!(!IsConst::<i32>::VALUE);
        assert!(!IsConst::<&i32>::VALUE);
    }

    #[test]
    fn test_is_volatile() {
        assert!(!IsVolatile::<i32>::VALUE);
    }
}
