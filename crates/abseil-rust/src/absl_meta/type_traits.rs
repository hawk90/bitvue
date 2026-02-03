//! Type trait utilities.
//!
//! Provides compile-time type information and transformations.


/// A type identity trait - used to pass types as values.
///
/// This is useful for generic programming where you need to pass
/// a type rather than a value.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_meta::type_traits::TypeIdentity;
///
/// fn get_type<T>() -> TypeIdentity<T> {
///     TypeIdentity::default()
/// }
///
/// let _id: TypeIdentity<i32> = TypeIdentity::default();
/// ```
pub struct TypeIdentity<T>(core::marker::PhantomData<T>);

impl<T> Clone for TypeIdentity<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for TypeIdentity<T> {}

impl<T> Default for TypeIdentity<T> {
    fn default() -> Self {
        TypeIdentity(core::marker::PhantomData)
    }
}

/// Trait for types that have a signed representation.
pub trait IsSigned: private::Sealed {
    /// Returns true if the type is signed.
    const IS_SIGNED: bool;
}

/// Trait for types that have an unsigned representation.
pub trait IsUnsigned: private::Sealed {
    /// Returns true if the type is unsigned.
    const IS_UNSIGNED: bool;
}

/// Trait for integral types.
pub trait IsIntegral: private::Sealed {
    /// Returns true if the type is integral.
    const IS_INTEGRAL: bool;
}

/// Trait for floating-point types.
pub trait IsFloatingPoint: private::Sealed {
    /// Returns true if the type is floating-point.
    const IS_FLOATING_POINT: bool;
}

/// Trait for arithmetic types (integral or floating-point).
pub trait IsArithmetic: private::Sealed {
    /// Returns true if the type is arithmetic.
    const IS_ARITHMETIC: bool;
}

// Implement IsSigned
impl IsSigned for i8 { const IS_SIGNED: bool = true; }
impl IsSigned for i16 { const IS_SIGNED: bool = true; }
impl IsSigned for i32 { const IS_SIGNED: bool = true; }
impl IsSigned for i64 { const IS_SIGNED: bool = true; }
impl IsSigned for i128 { const IS_SIGNED: bool = true; }
impl IsSigned for isize { const IS_SIGNED: bool = true; }
impl IsSigned for f32 { const IS_SIGNED: bool = true; }
impl IsSigned for f64 { const IS_SIGNED: bool = true; }
impl IsSigned for u8 { const IS_SIGNED: bool = false; }
impl IsSigned for u16 { const IS_SIGNED: bool = false; }
impl IsSigned for u32 { const IS_SIGNED: bool = false; }
impl IsSigned for u64 { const IS_SIGNED: bool = false; }
impl IsSigned for u128 { const IS_SIGNED: bool = false; }
impl IsSigned for usize { const IS_SIGNED: bool = false; }
impl IsSigned for bool { const IS_SIGNED: bool = false; }

// Implement IsUnsigned
impl IsUnsigned for u8 { const IS_UNSIGNED: bool = true; }
impl IsUnsigned for u16 { const IS_UNSIGNED: bool = true; }
impl IsUnsigned for u32 { const IS_UNSIGNED: bool = true; }
impl IsUnsigned for u64 { const IS_UNSIGNED: bool = true; }
impl IsUnsigned for u128 { const IS_UNSIGNED: bool = true; }
impl IsUnsigned for usize { const IS_UNSIGNED: bool = true; }
impl IsUnsigned for bool { const IS_UNSIGNED: bool = true; }
impl IsUnsigned for i8 { const IS_UNSIGNED: bool = false; }
impl IsUnsigned for i16 { const IS_UNSIGNED: bool = false; }
impl IsUnsigned for i32 { const IS_UNSIGNED: bool = false; }
impl IsUnsigned for i64 { const IS_UNSIGNED: bool = false; }
impl IsUnsigned for i128 { const IS_UNSIGNED: bool = false; }
impl IsUnsigned for isize { const IS_UNSIGNED: bool = false; }
impl IsUnsigned for f32 { const IS_UNSIGNED: bool = false; }
impl IsUnsigned for f64 { const IS_UNSIGNED: bool = false; }

// Implement IsIntegral
impl IsIntegral for i8 { const IS_INTEGRAL: bool = true; }
impl IsIntegral for i16 { const IS_INTEGRAL: bool = true; }
impl IsIntegral for i32 { const IS_INTEGRAL: bool = true; }
impl IsIntegral for i64 { const IS_INTEGRAL: bool = true; }
impl IsIntegral for i128 { const IS_INTEGRAL: bool = true; }
impl IsIntegral for isize { const IS_INTEGRAL: bool = true; }
impl IsIntegral for u8 { const IS_INTEGRAL: bool = true; }
impl IsIntegral for u16 { const IS_INTEGRAL: bool = true; }
impl IsIntegral for u32 { const IS_INTEGRAL: bool = true; }
impl IsIntegral for u64 { const IS_INTEGRAL: bool = true; }
impl IsIntegral for u128 { const IS_INTEGRAL: bool = true; }
impl IsIntegral for usize { const IS_INTEGRAL: bool = true; }
impl IsIntegral for f32 { const IS_INTEGRAL: bool = false; }
impl IsIntegral for f64 { const IS_INTEGRAL: bool = false; }
impl IsIntegral for bool { const IS_INTEGRAL: bool = false; }

// Implement IsFloatingPoint
impl IsFloatingPoint for f32 { const IS_FLOATING_POINT: bool = true; }
impl IsFloatingPoint for f64 { const IS_FLOATING_POINT: bool = true; }
impl IsFloatingPoint for i8 { const IS_FLOATING_POINT: bool = false; }
impl IsFloatingPoint for i16 { const IS_FLOATING_POINT: bool = false; }
impl IsFloatingPoint for i32 { const IS_FLOATING_POINT: bool = false; }
impl IsFloatingPoint for i64 { const IS_FLOATING_POINT: bool = false; }
impl IsFloatingPoint for i128 { const IS_FLOATING_POINT: bool = false; }
impl IsFloatingPoint for isize { const IS_FLOATING_POINT: bool = false; }
impl IsFloatingPoint for u8 { const IS_FLOATING_POINT: bool = false; }
impl IsFloatingPoint for u16 { const IS_FLOATING_POINT: bool = false; }
impl IsFloatingPoint for u32 { const IS_FLOATING_POINT: bool = false; }
impl IsFloatingPoint for u64 { const IS_FLOATING_POINT: bool = false; }
impl IsFloatingPoint for u128 { const IS_FLOATING_POINT: bool = false; }
impl IsFloatingPoint for usize { const IS_FLOATING_POINT: bool = false; }
impl IsFloatingPoint for bool { const IS_FLOATING_POINT: bool = false; }

// Implement IsArithmetic
impl IsArithmetic for i8 { const IS_ARITHMETIC: bool = true; }
impl IsArithmetic for i16 { const IS_ARITHMETIC: bool = true; }
impl IsArithmetic for i32 { const IS_ARITHMETIC: bool = true; }
impl IsArithmetic for i64 { const IS_ARITHMETIC: bool = true; }
impl IsArithmetic for i128 { const IS_ARITHMETIC: bool = true; }
impl IsArithmetic for isize { const IS_ARITHMETIC: bool = true; }
impl IsArithmetic for u8 { const IS_ARITHMETIC: bool = true; }
impl IsArithmetic for u16 { const IS_ARITHMETIC: bool = true; }
impl IsArithmetic for u32 { const IS_ARITHMETIC: bool = true; }
impl IsArithmetic for u64 { const IS_ARITHMETIC: bool = true; }
impl IsArithmetic for u128 { const IS_ARITHMETIC: bool = true; }
impl IsArithmetic for usize { const IS_ARITHMETIC: bool = true; }
impl IsArithmetic for f32 { const IS_ARITHMETIC: bool = true; }
impl IsArithmetic for f64 { const IS_ARITHMETIC: bool = true; }
impl IsArithmetic for bool { const IS_ARITHMETIC: bool = false; }

/// Returns true if the type is a signed integer or floating-point type.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_meta::type_traits::is_signed;
///
/// assert!(is_signed::<i8>());
/// assert!(is_signed::<i32>());
/// assert!(is_signed::<f32>());
/// assert!(!is_signed::<u32>());
/// ```
#[inline]
pub const fn is_signed<T>() -> bool
where
    T: IsSigned,
{
    T::IS_SIGNED
}

/// Returns true if the type is an unsigned integer type.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_meta::type_traits::is_unsigned;
///
/// assert!(is_unsigned::<u8>());
/// assert!(is_unsigned::<u32>());
/// assert!(!is_unsigned::<i32>());
/// ```
#[inline]
pub const fn is_unsigned<T>() -> bool
where
    T: IsUnsigned,
{
    T::IS_UNSIGNED
}

/// Returns true if T is an integer type.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_meta::type_traits::is_integral;
///
/// assert!(is_integral::<i32>());
/// assert!(is_integral::<u64>());
/// assert!(!is_integral::<f32>());
/// ```
#[inline]
pub const fn is_integral<T>() -> bool
where
    T: IsIntegral,
{
    T::IS_INTEGRAL
}

/// Returns true if T is a floating-point type.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_meta::type_traits::is_floating_point;
///
/// assert!(is_floating_point::<f32>());
/// assert!(is_floating_point::<f64>());
/// assert!(!is_floating_point::<i32>());
/// ```
#[inline]
pub const fn is_floating_point<T>() -> bool
where
    T: IsFloatingPoint,
{
    T::IS_FLOATING_POINT
}

/// Returns true if T is an arithmetic type (integer or floating-point).
///
/// # Examples
///
/// ```rust
/// use abseil::absl_meta::type_traits::is_arithmetic;
///
/// assert!(is_arithmetic::<i32>());
/// assert!(is_arithmetic::<f64>());
/// assert!(!is_arithmetic::<bool>());
/// ```
#[inline]
pub const fn is_arithmetic<T>() -> bool
where
    T: IsArithmetic,
{
    T::IS_ARITHMETIC
}

mod private {
    pub trait Sealed {}

    impl Sealed for i8 {}
    impl Sealed for i16 {}
    impl Sealed for i32 {}
    impl Sealed for i64 {}
    impl Sealed for i128 {}
    impl Sealed for isize {}
    impl Sealed for u8 {}
    impl Sealed for u16 {}
    impl Sealed for u32 {}
    impl Sealed for u64 {}
    impl Sealed for u128 {}
    impl Sealed for usize {}
    impl Sealed for bool {}
    impl Sealed for f32 {}
    impl Sealed for f64 {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_signed() {
        assert!(is_signed::<i8>());
        assert!(is_signed::<i16>());
        assert!(is_signed::<i32>());
        assert!(is_signed::<i64>());
        assert!(is_signed::<i128>());
        assert!(is_signed::<isize>());
        assert!(is_signed::<f32>());
        assert!(is_signed::<f64>());

        assert!(!is_signed::<u8>());
        assert!(!is_signed::<u16>());
        assert!(!is_signed::<u32>());
        assert!(!is_signed::<u64>());
        assert!(!is_signed::<u128>());
        assert!(!is_signed::<usize>());
        assert!(!is_signed::<bool>());
    }

    #[test]
    fn test_is_unsigned() {
        assert!(is_unsigned::<u8>());
        assert!(is_unsigned::<u16>());
        assert!(is_unsigned::<u32>());
        assert!(is_unsigned::<u64>());
        assert!(is_unsigned::<u128>());
        assert!(is_unsigned::<usize>());
        assert!(is_unsigned::<bool>());

        assert!(!is_unsigned::<i8>());
        assert!(!is_unsigned::<i16>());
        assert!(!is_unsigned::<i32>());
        assert!(!is_unsigned::<i64>());
        assert!(!is_unsigned::<f32>());
        assert!(!is_unsigned::<f64>());
    }

    #[test]
    fn test_is_integral() {
        assert!(is_integral::<i8>());
        assert!(is_integral::<i16>());
        assert!(is_integral::<i32>());
        assert!(is_integral::<i64>());
        assert!(is_integral::<i128>());
        assert!(is_integral::<isize>());
        assert!(is_integral::<u8>());
        assert!(is_integral::<u16>());
        assert!(is_integral::<u32>());
        assert!(is_integral::<u64>());
        assert!(is_integral::<u128>());
        assert!(is_integral::<usize>());

        assert!(!is_integral::<f32>());
        assert!(!is_integral::<f64>());
        assert!(!is_integral::<bool>());
    }

    #[test]
    fn test_is_floating_point() {
        assert!(is_floating_point::<f32>());
        assert!(is_floating_point::<f64>());

        assert!(!is_floating_point::<i32>());
        assert!(!is_floating_point::<u32>());
        assert!(!is_floating_point::<bool>());
    }

    #[test]
    fn test_is_arithmetic() {
        assert!(is_arithmetic::<i8>());
        assert!(is_arithmetic::<i16>());
        assert!(is_arithmetic::<i32>());
        assert!(is_arithmetic::<i64>());
        assert!(is_arithmetic::<i128>());
        assert!(is_arithmetic::<isize>());
        assert!(is_arithmetic::<u8>());
        assert!(is_arithmetic::<u16>());
        assert!(is_arithmetic::<u32>());
        assert!(is_arithmetic::<u64>());
        assert!(is_arithmetic::<u128>());
        assert!(is_arithmetic::<usize>());
        assert!(is_arithmetic::<f32>());
        assert!(is_arithmetic::<f64>());

        assert!(!is_arithmetic::<bool>());
    }

    #[test]
    fn test_type_identity() {
        let _id: TypeIdentity<i32> = TypeIdentity::default();
        let _id2: TypeIdentity<f64> = TypeIdentity::default();
        assert_eq!(core::mem::size_of::<TypeIdentity<i32>>(), 0);
    }
}
