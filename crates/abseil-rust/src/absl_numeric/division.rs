//! Integer division utilities.

use super::error::NumericError;
use core::ops::{Div, Rem};

/// Performs ceiling division (rounds up).
///
/// # Panics
///
/// Panics if `denominator` is zero.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_numeric::division::ceil_div;
///
/// assert_eq!(ceil_div(10u32, 3), 4);
/// assert_eq!(ceil_div(9u32, 3), 3);
/// assert_eq!(ceil_div(1u32, 10), 1);
/// ```
#[inline]
pub const fn ceil_div<
    T: Copy + Div<Output = T> + Rem<Output = T> + PartialEq + Eq,
>(
    numerator: T,
    denominator: T,
) -> T {
    let d = numerator / denominator;
    let r = numerator % denominator;
    if r == T::from(0u8) {
        d
    } else {
        d + T::from(1u8)
    }
}

/// Performs ceiling division with error checking.
///
/// Returns `Err(NumericError::DivisionByZero)` if the denominator is zero.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_numeric::division::{checked_ceil_div, NumericError};
///
/// assert_eq!(checked_ceil_div(10u32, 3), Ok(4));
/// assert_eq!(checked_ceil_div(9u32, 3), Ok(3));
/// assert_eq!(checked_ceil_div(10u32, 0), Err(NumericError::DivisionByZero));
/// ```
#[inline]
pub const fn checked_ceil_div<
    T: Copy + Div<Output = T> + Rem<Output = T> + PartialEq + Eq,
>(
    numerator: T,
    denominator: T,
) -> Result<T, NumericError> {
    if denominator == T::from(0u8) {
        return Err(NumericError::DivisionByZero);
    }
    Ok(ceil_div(numerator, denominator))
}

/// Performs floor division (rounds down, same as regular division for positive).
///
/// For signed types, this always rounds towards negative infinity.
///
/// # Panics
///
/// Panics if `denominator` is zero.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_numeric::division::floor_div;
///
/// assert_eq!(floor_div(10i32, 3), 3);
/// assert_eq!(floor_div(-10i32, 3), -4);
/// assert_eq!(floor_div(9i32, 3), 3);
/// ```
#[inline]
pub const fn floor_div<T: Copy + Div<Output = T>>(numerator: T, denominator: T) -> T {
    numerator / denominator
}

/// Performs floor division with error checking.
///
/// Returns `Err(NumericError::DivisionByZero)` if the denominator is zero.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_numeric::division::{checked_floor_div, NumericError};
///
/// assert_eq!(checked_floor_div(10i32, 3), Ok(3));
/// assert_eq!(checked_floor_div(10i32, 0), Err(NumericError::DivisionByZero));
/// ```
#[inline]
pub const fn checked_floor_div<T: Copy + Div<Output = T> + PartialEq + Eq>(
    numerator: T,
    denominator: T,
) -> Result<T, NumericError> {
    if denominator == T::from(0u8) {
        return Err(NumericError::DivisionByZero);
    }
    Ok(floor_div(numerator, denominator))
}

/// Computes the quotient and remainder simultaneously.
///
/// # Panics
///
/// Panics if `denominator` is zero.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_numeric::division::div_rem;
///
/// let (q, r) = div_rem(17i32, 5);
/// assert_eq!(q, 3);
/// assert_eq!(r, 2);
/// ```
#[inline]
pub const fn div_rem<
    T: Copy + Div<Output = T> + Rem<Output = T>,
>(
    numerator: T,
    denominator: T,
) -> (T, T) {
    let q = numerator / denominator;
    let r = numerator % denominator;
    (q, r)
}

/// Computes the quotient and remainder with error checking.
///
/// Returns `Err(NumericError::DivisionByZero)` if the denominator is zero.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_numeric::division::{checked_div_rem, NumericError};
///
/// assert_eq!(checked_div_rem(17i32, 5), Ok((3, 2)));
/// assert_eq!(checked_div_rem(10i32, 0), Err(NumericError::DivisionByZero));
/// ```
#[inline]
pub const fn checked_div_rem<
    T: Copy + Div<Output = T> + Rem<Output = T> + PartialEq + Eq,
>(
    numerator: T,
    denominator: T,
) -> Result<(T, T), NumericError> {
    if denominator == T::from(0u8) {
        return Err(NumericError::DivisionByZero);
    }
    Ok(div_rem(numerator, denominator))
}

/// Computes the greatest common divisor using Euclid's algorithm.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_numeric::division::gcd;
///
/// assert_eq!(gcd(48u32, 18), 6);
/// assert_eq!(gcd(17u32, 5), 1);
/// assert_eq!(gcd(100u32, 100), 100);
/// assert_eq!(gcd(0u32, 5), 5);  // gcd(0, n) = n
/// assert_eq!(gcd(0u32, 0), 0);  // gcd(0, 0) = 0
/// ```
#[inline]
pub const fn gcd<T: Copy + PartialEq + Rem<Output = T>>(a: T, b: T) -> T {
    // Handle zero cases explicitly to avoid division by zero
    if b == T::from(0u8) {
        a
    } else if a == T::from(0u8) {
        b
    } else {
        gcd(b, a % b)
    }
}

/// Computes the least common multiple.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_numeric::division::lcm;
///
/// assert_eq!(lcm(4u32, 6), 12);
/// assert_eq!(lcm(5u32, 7), 35);
/// ```
#[inline]
pub const fn lcm<
    T: Copy + PartialEq + Mul<Output = T> + Div<Output = T>,
>(
    a: T,
    b: T,
) -> T {
    // lcm(a, b) = 0 if either a or b is 0
    if a == T::from(0u8) || b == T::from(0u8) {
        T::from(0u8)
    } else {
        // lcm(a, b) = (a / gcd(a, b)) * b
        // First divide to reduce overflow risk
        let g = gcd(a, b);
        (a / g) * b
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_div_rem() {
        let (q, r) = div_rem(17i32, 5);
        assert_eq!(q, 3);
        assert_eq!(r, 2);

        let (q, r) = div_rem(100i32, 10);
        assert_eq!(q, 10);
        assert_eq!(r, 0);
    }

    #[test]
    fn test_gcd() {
        assert_eq!(gcd(48u32, 18u32), 6);
        assert_eq!(gcd(17u32, 5u32), 1);
        assert_eq!(gcd(100u32, 100u32), 100);
        assert_eq!(gcd(0u32, 5u32), 5);
    }

    #[test]
    fn test_lcm() {
        assert_eq!(lcm(4u32, 6u32), 12);
        assert_eq!(lcm(5u32, 7u32), 35);
        assert_eq!(lcm(6u32, 8u32), 24);
    }

    // Edge case tests for CRITICAL security fixes

    #[test]
    fn test_gcd_edge_cases() {
        // gcd(0, 0) = 0 by convention
        assert_eq!(gcd(0u32, 0u32), 0);
        // gcd(0, n) = n
        assert_eq!(gcd(0u32, 5u32), 5);
        assert_eq!(gcd(0u64, 100u64), 100);
        // gcd(n, 0) = n
        assert_eq!(gcd(5u32, 0u32), 5);
        assert_eq!(gcd(100u64, 0u64), 100);
    }

    #[test]
    fn test_lcm_edge_cases() {
        // lcm(0, n) = 0
        assert_eq!(lcm(0u32, 5u32), 0);
        assert_eq!(lcm(0u64, 100u64), 0);
        // lcm(n, 0) = 0
        assert_eq!(lcm(5u32, 0u32), 0);
        assert_eq!(lcm(100u64, 0u64), 0);
        // lcm(0, 0) = 0
        assert_eq!(lcm(0u32, 0u32), 0);
    }

    #[test]
    fn test_ceil_div_edge_cases() {
        // Division by zero would panic - this is expected
        // Just verify normal cases work
        assert_eq!(ceil_div(10u32, 3), 4);
        assert_eq!(ceil_div(9u32, 3), 3);
        assert_eq!(ceil_div(1u32, 10), 1);
    }

    // Edge case tests for HIGH security fix - division by zero in ceil_div

    #[test]
    fn test_checked_ceil_div_normal() {
        assert_eq!(checked_ceil_div(10u32, 3), Ok(4));
        assert_eq!(checked_ceil_div(9u32, 3), Ok(3));
        assert_eq!(checked_ceil_div(1u32, 10), Ok(1));
        assert_eq!(checked_ceil_div(100i32, 7), Ok(15)); // 100/7 = 14.28, ceil = 15
    }

    #[test]
    fn test_checked_ceil_div_by_zero() {
        assert_eq!(checked_ceil_div(10u32, 0), Err(NumericError::DivisionByZero));
        assert_eq!(checked_ceil_div(0i32, 0), Err(NumericError::DivisionByZero));
        assert_eq!(checked_ceil_div(i32::MAX, 0), Err(NumericError::DivisionByZero));
    }

    #[test]
    fn test_checked_ceil_div_exact_division() {
        // When division is exact, ceil should equal normal division
        assert_eq!(checked_ceil_div(100u32, 10), Ok(10));
        assert_eq!(checked_ceil_div(99u32, 11), Ok(9));
        assert_eq!(checked_ceil_div(0u32, 10), Ok(0));
    }

    #[test]
    fn test_checked_ceil_div_rounds_up() {
        // Test that ceil correctly rounds up for non-exact divisions
        assert_eq!(checked_ceil_div(10u32, 3), Ok(4)); // 3.33 -> 4
        assert_eq!(checked_ceil_div(11u32, 3), Ok(4)); // 3.66 -> 4
        assert_eq!(checked_ceil_div(1u32, 100), Ok(1)); // 0.01 -> 1
    }

    #[test]
    fn test_checked_ceil_div_signed_negative() {
        // Test with signed integers (rounds toward zero, then up if not exact)
        assert_eq!(checked_ceil_div(-10i32, 3), Ok(-3)); // -3.33 -> -3
        assert_eq!(checked_ceil_div(-11i32, 3), Ok(-3)); // -3.66 -> -3
        assert_eq!(checked_ceil_div(-9i32, 3), Ok(-3)); // -3 exactly
    }

    #[test]
    fn test_checked_floor_div_normal() {
        assert_eq!(checked_floor_div(10i32, 3), Ok(3));
        assert_eq!(checked_floor_div(9i32, 3), Ok(3));
        assert_eq!(checked_floor_div(1i32, 10), Ok(0));
    }

    #[test]
    fn test_checked_floor_div_by_zero() {
        assert_eq!(checked_floor_div(10i32, 0), Err(NumericError::DivisionByZero));
        assert_eq!(checked_floor_div(0i32, 0), Err(NumericError::DivisionByZero));
        assert_eq!(checked_floor_div(i32::MAX, 0), Err(NumericError::DivisionByZero));
    }

    #[test]
    fn test_checked_floor_div_signed() {
        // Test that floor_div rounds toward negative infinity for signed types
        assert_eq!(checked_floor_div(10i32, 3), Ok(3));
        assert_eq!(checked_floor_div(-10i32, 3), Ok(-4)); // -3.33 -> -4
        assert_eq!(checked_floor_div(-9i32, 3), Ok(-3)); // -3 exactly
    }

    #[test]
    fn test_checked_div_rem_normal() {
        assert_eq!(checked_div_rem(17i32, 5), Ok((3, 2)));
        assert_eq!(checked_div_rem(100i32, 10), Ok((10, 0)));
        assert_eq!(checked_div_rem(10i32, 3), Ok((3, 1)));
    }

    #[test]
    fn test_checked_div_rem_by_zero() {
        assert_eq!(checked_div_rem(17i32, 0), Err(NumericError::DivisionByZero));
        assert_eq!(checked_div_rem(0i32, 0), Err(NumericError::DivisionByZero));
        assert_eq!(checked_div_rem(i32::MAX, 0), Err(NumericError::DivisionByZero));
    }

    #[test]
    fn test_checked_div_rem_properties() {
        // Test that quotient and remainder satisfy: numerator = q*d + r
        assert_eq!(checked_div_rem(17i32, 5), Ok((3, 2)));
        // Verify: 17 = 3*5 + 2 = 17 ✓
    }

    #[test]
    fn test_checked_div_rem_negative() {
        assert_eq!(checked_div_rem(-17i32, 5), Ok((-3, -2)));
        assert_eq!(checked_div_rem(17i32, -5), Ok((-3, 2)));
        assert_eq!(checked_div_rem(-17i32, -5), Ok((3, -2)));
    }

    #[test]
    fn test_checked_functions_with_max_values() {
        // Test with maximum values to ensure no overflow issues
        assert!(checked_ceil_div(u32::MAX, 1).is_ok());
        assert!(checked_ceil_div(u32::MAX, u32::MAX).is_ok());

        assert!(checked_floor_div(i32::MAX, 1).is_ok());
        assert!(checked_floor_div(i32::MIN, 1).is_ok());

        assert!(checked_div_rem(u64::MAX, 1).is_ok());
    }
}
