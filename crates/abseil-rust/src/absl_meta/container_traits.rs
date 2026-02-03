//! Container type traits for detecting standard library container types.

use super::type_constants::{Bool, Void};

/// Trait for tuple types.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_meta::IsTuple;
///
/// assert!(IsTuple::<(i32, i32)>::VALUE);
/// assert!(IsTuple::<()>::VALUE);
/// assert!(!IsTuple::<i32>::VALUE);
/// ```
pub trait IsTuple {
    /// True if this is a tuple type.
    const VALUE: bool = false;
    /// The number of elements in the tuple.
    const COUNT: usize = 0;
}

impl<T> IsTuple for (T,) {
    const VALUE: bool = true;
    const COUNT: usize = 1;
}

impl<T1, T2> IsTuple for (T1, T2) {
    const VALUE: bool = true;
    const COUNT: usize = 2;
}

impl<T1, T2, T3> IsTuple for (T1, T2, T3) {
    const VALUE: bool = true;
    const COUNT: usize = 3;
}

impl<T1, T2, T3, T4> IsTuple for (T1, T2, T3, T4) {
    const VALUE: bool = true;
    const COUNT: usize = 4;
}

impl<T1, T2, T3, T4, T5> IsTuple for (T1, T2, T3, T4, T5) {
    const VALUE: bool = true;
    const COUNT: usize = 5;
}

impl IsTuple for () {
    const VALUE: bool = true;
    const COUNT: usize = 0;
}

/// Trait for Option types.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_meta::IsOption;
///
/// assert!(IsOption::<Option<i32>>::VALUE);
/// assert!(!IsOption::<i32>::VALUE);
/// ```
pub trait IsOption {
    /// True if this is an Option type.
    const VALUE: bool = false;
}

impl<T> IsOption for Option<T> {
    const VALUE: bool = true;
}

/// Trait for Result types.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_meta::IsResult;
///
/// assert!(IsResult::<Result<i32, String>>::VALUE);
/// assert!(!IsResult::<i32>::VALUE);
/// ```
pub trait IsResult {
    /// True if this is a Result type.
    const VALUE: bool = false;
}

impl<T, E> IsResult for Result<T, E> {
    const VALUE: bool = true;
}

/// Trait for slice types.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_meta::IsSlice;
///
/// assert!(IsSlice::<[i32]>::VALUE);
/// assert!(!IsSlice::<i32>::VALUE);
/// ```
pub trait IsSlice {
    /// True if this is a slice type.
    const VALUE: bool = false;
}

impl<T> IsSlice for [T] {
    const VALUE: bool = true;
}

/// Trait for checking if a type is an enum.
///
/// Note: This is a basic implementation. Full enum detection requires
/// more complex type inspection.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_meta::IsEnum;
///
/// assert!(IsEnum::<Option<i32>>::VALUE);
/// ```
pub trait IsEnum {
    /// True if this is an enum type.
    const VALUE: bool = false;
}

impl<T> IsEnum for Option<T> {
    const VALUE: bool = true;
}

impl<T, E> IsEnum for Result<T, E> {
    const VALUE: bool = true;
}

impl IsEnum for Bool<true> {
    const VALUE: bool = true;
}

impl IsEnum for Bool<false> {
    const VALUE: bool = true;
}

impl IsEnum for Void {
    const VALUE: bool = true;
}

/// Trait for checking if a type is a union.
///
/// Note: Rust doesn't have traditional unions like C, but has
/// `union` types. This trait detects those.
pub trait IsUnion {
    /// True if this is a union type.
    const VALUE: bool = false;
}

/// Trait for getting the value type from an Option.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_meta::OptionValue;
///
/// type T = OptionValue::<Option<i32>>;
/// // T is i32
/// ```
pub type OptionValue<T> = <T as OptionValueImpl>::Output;

pub trait OptionValueImpl {
    type Output;
}

impl<T> OptionValueImpl for Option<T> {
    type Output = T;
}

/// Trait for getting the success type from a Result.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_meta::ResultOk;
///
/// type T = ResultOk::<Result<i32, String>>;
/// // T is i32
/// ```
pub type ResultOk<T> = <T as ResultOkImpl>::Output;

pub trait ResultOkImpl {
    type Output;
}

impl<T, E> ResultOkImpl for Result<T, E> {
    type Output = T;
}

/// Trait for getting the error type from a Result.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_meta::ResultErr;
///
/// type T = ResultErr::<Result<i32, String>>;
/// // T is String
/// ```
pub type ResultErr<T> = <T as ResultErrImpl>::Output;

pub trait ResultErrImpl {
    type Output;
}

impl<T, E> ResultErrImpl for Result<T, E> {
    type Output = E;
}

#[cfg(test)]
mod tests {
    use super::*;

    // Tests for IsTuple
    #[test]
    fn test_is_tuple() {
        assert!(IsTuple::<(i32, i32)>::VALUE);
        assert!(IsTuple::<()>::VALUE);
        assert!(IsTuple::<(i32, i32, i32, i32, i32)>::VALUE);
        assert!(!IsTuple::<i32>::VALUE);
        assert_eq!(IsTuple::<(i32, i32)>::COUNT, 2);
        assert_eq!(IsTuple::<()>::COUNT, 0);
        assert_eq!(IsTuple::<(i32, i32, i32)>::COUNT, 3);
    }

    // Tests for IsOption
    #[test]
    fn test_is_option() {
        assert!(IsOption::<Option<i32>>::VALUE);
        assert!(IsOption::<Option<&str>>::VALUE);
        assert!(!IsOption::<i32>::VALUE);
    }

    // Tests for IsResult
    #[test]
    fn test_is_result() {
        assert!(IsResult::<Result<i32, String>>::VALUE);
        assert!(!IsResult::<i32>::VALUE);
    }

    // Tests for IsSlice
    #[test]
    fn test_is_slice() {
        assert!(IsSlice::<[i32]>::VALUE);
        assert!(IsSlice::<[u8]>::VALUE);
        assert!(!IsSlice::<i32>::VALUE);
    }

    // Tests for IsEnum
    #[test]
    fn test_is_enum() {
        assert!(IsEnum::<Option<i32>>::VALUE);
        assert!(IsEnum::<Result<i32, String>>::VALUE);
        assert!(!IsEnum::<i32>::VALUE);
    }

    // Tests for OptionValue
    #[test]
    fn test_option_value() {
        type T = OptionValue::<Option<i32>>;
        // T should be i32
        assert_eq!(core::mem::size_of::<T>(), 4);
    }

    // Tests for ResultOk
    #[test]
    fn test_result_ok() {
        type T = ResultOk::<Result<i32, String>>;
        assert_eq!(core::mem::size_of::<T>(), 4);
    }

    // Tests for ResultErr
    #[test]
    fn test_result_err() {
        type T = ResultErr::<Result<i32, String>>;
        // String doesn't have a fixed size, but we can check it compiles
        let _: T = String::new();
    }
}
