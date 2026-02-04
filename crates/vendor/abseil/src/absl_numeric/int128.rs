//! 128-bit integer types.
//!
//! This module provides signed and unsigned 128-bit integer types with
//! additional utilities beyond Rust's built-in `i128` and `u128`.
//!
//! # Example
//!
//! ```rust
//! use abseil::absl_numeric::int128::{int128, uint128};
//!
//! let a = int128::from(42i64);
//! let b = uint128::from(42u64);
//! ```

use core::cmp::Ordering;
use core::fmt;
use core::ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Rem, RemAssign, Sub, SubAssign};

/// Error type for division operations.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DivisionError {
    /// Division by zero.
    DivisionByZero,
}

impl fmt::Display for DivisionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DivisionError::DivisionByZero => write!(f, "Division by zero"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for DivisionError {}

/// Signed 128-bit integer wrapper.
///
/// This type wraps Rust's built-in `i128` and provides additional
/// utility functions compatible with Abseil's `absl::int128`.
#[repr(transparent)]
#[derive(Clone, Copy, Default, Hash, PartialEq, Eq)]
#[allow(non_camel_case_types)]
pub struct int128(pub i128);

/// Unsigned 128-bit integer wrapper.
///
/// This type wraps Rust's built-in `u128` and provides additional
/// utility functions compatible with Abseil's `absl::uint128`.
#[repr(transparent)]
#[derive(Clone, Copy, Default, Hash, PartialEq, Eq)]
#[allow(non_camel_case_types)]
pub struct uint128(pub u128);

// ============================================================================
// int128 Implementation
// ============================================================================

impl int128 {
    /// The minimum value of `int128`.
    pub const MIN: int128 = int128(i128::MIN);

    /// The maximum value of `int128`.
    pub const MAX: int128 = int128(i128::MAX);

    /// Creates an `int128` from an `i64`.
    #[inline]
    pub const fn from(value: i64) -> Self {
        Self(value as i128)
    }

    /// Creates an `int128` from an `i32`.
    #[inline]
    pub const fn from_i32(value: i32) -> Self {
        Self(value as i128)
    }

    /// Creates an `int128` from an `i16`.
    #[inline]
    pub const fn from_i16(value: i16) -> Self {
        Self(value as i128)
    }

    /// Creates an `int128` from an `i8`.
    #[inline]
    pub const fn from_i8(value: i8) -> Self {
        Self(value as i128)
    }

    /// Creates an `int128` from a `u32`.
    #[inline]
    pub const fn from_u32(value: u32) -> Self {
        Self(value as i128)
    }

    /// Creates an `int128` from a `u16`.
    #[inline]
    pub const fn from_u16(value: u16) -> Self {
        Self(value as i128)
    }

    /// Creates an `int128` from a `u8`.
    #[inline]
    pub const fn from_u8(value: u8) -> Self {
        Self(value as i128)
    }

    /// Returns the value as `i64` if it fits, otherwise `None`.
    #[inline]
    pub const fn as_i64(&self) -> Option<i64> {
        if self.0 >= i64::MIN as i128 && self.0 <= i64::MAX as i128 {
            Some(self.0 as i64)
        } else {
            None
        }
    }

    /// Returns the value as `u64` if it fits in the unsigned range, otherwise `None`.
    #[inline]
    pub const fn as_u64(&self) -> Option<u64> {
        if self.0 >= 0 && self.0 <= u64::MAX as i128 {
            Some(self.0 as u64)
        } else {
            None
        }
    }

    /// Returns `true` if the value is negative.
    #[inline]
    pub const fn is_negative(&self) -> bool {
        self.0 < 0
    }

    /// Returns the absolute value.
    ///
    /// ⚠️ **WARNING**: This method will panic for `i128::MIN` because
    /// the absolute value of `i128::MIN` cannot be represented in `i128`.
    /// Consider using `checked_abs()` instead for safe handling.
    #[inline]
    pub fn abs(&self) -> Self {
        Self(self.0.abs())
    }

    /// Returns the absolute value, or `None` if it would overflow.
    ///
    /// This method safely handles the case where `self` is `i128::MIN`,
    /// which cannot be represented as a positive `i128` value.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use abseil::absl_numeric::int128::int128;
    ///
    /// assert_eq!(int128::from(-42).checked_abs(), Some(int128(42)));
    /// assert_eq!(int128::from(42).checked_abs(), Some(int128(42)));
    /// // i128::MIN cannot be represented as positive i128
    /// assert_eq!(int128(i128::MIN).checked_abs(), None);
    /// ```
    #[inline]
    pub const fn checked_abs(&self) -> Option<Self> {
        // i128::MIN is -170141183460469231731687303715884105728
        // Its absolute value is 170141183460469231731687303715884105728
        // which is i128::MAX + 1, so it overflows
        match self.0.checked_abs() {
            Some(abs) => Some(Self(abs)),
            None => None,
        }
    }

    /// Computes the absolute difference between two `int128` values.
    #[inline]
    pub fn abs_diff(&self, other: Self) -> uint128 {
        if self.0 < other.0 {
            uint128((other.0 - self.0) as u128)
        } else {
            uint128((self.0 - other.0) as u128)
        }
    }

    /// Returns the number of significant bits in the binary representation.
    #[inline]
    pub fn bit_width(&self) -> u32 {
        if self.0 >= 0 {
            (128 - self.0.leading_zeros()) as u32
        } else {
            128 - self.0.wrapping_abs().leading_zeros() as u32 + 1
        }
    }

    /// Returns `true` if the value is a power of 2.
    #[inline]
    pub fn is_power_of_two(&self) -> bool {
        self.0 > 0 && (self.0 & (self.0 - 1)) == 0
    }

    /// Checked division returning `Result` with error on division by zero.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use abseil::absl_numeric::int128::{int128, DivisionError};
    ///
    /// assert_eq!(int128(100).checked_div(int128(5)), Ok(int128(20)));
    /// assert_eq!(int128(100).checked_div(int128(0)), Err(DivisionError::DivisionByZero));
    /// ```
    #[inline]
    pub const fn checked_div(&self, other: Self) -> Result<Self, DivisionError> {
        if other.0 == 0 {
            Err(DivisionError::DivisionByZero)
        } else {
            Ok(Self(self.0 / other.0))
        }
    }

    /// Checked remainder returning `Result` with error on division by zero.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use abseil::absl_numeric::int128::{int128, DivisionError};
    ///
    /// assert_eq!(int128(100).checked_rem(int128(30)), Ok(int128(10)));
    /// assert_eq!(int128(100).checked_rem(int128(0)), Err(DivisionError::DivisionByZero));
    /// ```
    #[inline]
    pub const fn checked_rem(&self, other: Self) -> Result<Self, DivisionError> {
        if other.0 == 0 {
            Err(DivisionError::DivisionByZero)
        } else {
            Ok(Self(self.0 % other.0))
        }
    }
}

// ============================================================================
// uint128 Implementation
// ============================================================================

impl uint128 {
    /// The minimum value of `uint128`.
    pub const MIN: uint128 = uint128(0);

    /// The maximum value of `uint128`.
    pub const MAX: uint128 = uint128(u128::MAX);

    /// Creates a `uint128` from a `u64`.
    #[inline]
    pub const fn from(value: u64) -> Self {
        Self(value as u128)
    }

    /// Creates a `uint128` from a `u32`.
    #[inline]
    pub const fn from_u32(value: u32) -> Self {
        Self(value as u128)
    }

    /// Creates a `uint128` from a `u16`.
    #[inline]
    pub const fn from_u16(value: u16) -> Self {
        Self(value as u128)
    }

    /// Creates a `uint128` from a `u8`.
    #[inline]
    pub const fn from_u8(value: u8) -> Self {
        Self(value as u128)
    }

    /// Creates a `uint128` from an `i64` (returns 0 for negative values).
    #[inline]
    pub const fn from_i64(value: i64) -> Self {
        if value < 0 {
            Self(0)
        } else {
            Self(value as u128)
        }
    }

    /// Returns the value as `u64` if it fits, otherwise `None`.
    #[inline]
    pub const fn as_u64(&self) -> Option<u64> {
        if self.0 <= u64::MAX as u128 {
            Some(self.0 as u64)
        } else {
            None
        }
    }

    /// Returns the value as `i64` if it fits in the signed range, otherwise `None`.
    #[inline]
    pub const fn as_i64(&self) -> Option<i64> {
        if self.0 <= i64::MAX as u128 {
            Some(self.0 as i64)
        } else {
            None
        }
    }

    /// Returns the number of significant bits in the binary representation.
    #[inline]
    pub fn bit_width(&self) -> u32 {
        128 - self.0.leading_zeros() as u32
    }

    /// Returns `true` if the value is a power of 2.
    #[inline]
    pub fn is_power_of_two(&self) -> bool {
        self.0 > 0 && (self.0 & (self.0 - 1)) == 0
    }

    /// Computes the absolute difference between two `uint128` values.
    #[inline]
    pub fn abs_diff(&self, other: Self) -> Self {
        if self.0 < other.0 {
            Self(other.0 - self.0)
        } else {
            Self(self.0 - other.0)
        }
    }

    /// Checked division returning `Result` with error on division by zero.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use abseil::absl_numeric::int128::{uint128, DivisionError};
    ///
    /// assert_eq!(uint128(100).checked_div(uint128(5)), Ok(uint128(20)));
    /// assert_eq!(uint128(100).checked_div(uint128(0)), Err(DivisionError::DivisionByZero));
    /// ```
    #[inline]
    pub const fn checked_div(&self, other: Self) -> Result<Self, DivisionError> {
        if other.0 == 0 {
            Err(DivisionError::DivisionByZero)
        } else {
            Ok(Self(self.0 / other.0))
        }
    }

    /// Checked remainder returning `Result` with error on division by zero.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use abseil::absl_numeric::int128::{uint128, DivisionError};
    ///
    /// assert_eq!(uint128(100).checked_rem(uint128(30)), Ok(uint128(10)));
    /// assert_eq!(uint128(100).checked_rem(uint128(0)), Err(DivisionError::DivisionByZero));
    /// ```
    #[inline]
    pub const fn checked_rem(&self, other: Self) -> Result<Self, DivisionError> {
        if other.0 == 0 {
            Err(DivisionError::DivisionByZero)
        } else {
            Ok(Self(self.0 % other.0))
        }
    }
}

// ============================================================================
// Format Implementations
// ============================================================================

impl fmt::Display for int128 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl fmt::Debug for int128 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "int128({})", self.0)
    }
}

impl fmt::LowerHex for int128 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:x}", self.0)
    }
}

impl fmt::UpperHex for int128 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:X}", self.0)
    }
}

impl fmt::Binary for int128 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:b}", self.0)
    }
}

impl fmt::Octal for int128 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:o}", self.0)
    }
}

impl fmt::Display for uint128 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl fmt::Debug for uint128 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "uint128({})", self.0)
    }
}

impl fmt::LowerHex for uint128 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:x}", self.0)
    }
}

impl fmt::UpperHex for uint128 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:X}", self.0)
    }
}

impl fmt::Binary for uint128 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:b}", self.0)
    }
}

impl fmt::Octal for uint128 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:o}", self.0)
    }
}

// ============================================================================
// Operator Implementations
// ============================================================================

impl Add for int128 {
    type Output = Self;
    fn add(self, other: Self) -> Self::Output {
        Self(self.0 + other.0)
    }
}

impl AddAssign for int128 {
    fn add_assign(&mut self, other: Self) {
        self.0 += other.0;
    }
}

impl Sub for int128 {
    type Output = Self;
    fn sub(self, other: Self) -> Self::Output {
        Self(self.0 - other.0)
    }
}

impl SubAssign for int128 {
    fn sub_assign(&mut self, other: Self) {
        self.0 -= other.0;
    }
}

impl Mul for int128 {
    type Output = Self;
    fn mul(self, other: Self) -> Self::Output {
        Self(self.0 * other.0)
    }
}

impl MulAssign for int128 {
    fn mul_assign(&mut self, other: Self) {
        self.0 *= other.0;
    }
}

impl Div for int128 {
    type Output = Self;
    fn div(self, other: Self) -> Self::Output {
        Self(self.0 / other.0)
    }
}

impl DivAssign for int128 {
    fn div_assign(&mut self, other: Self) {
        self.0 /= other.0;
    }
}

impl Rem for int128 {
    type Output = Self;
    fn rem(self, other: Self) -> Self::Output {
        Self(self.0 % other.0)
    }
}

impl RemAssign for int128 {
    fn rem_assign(&mut self, other: Self) {
        self.0 %= other.0;
    }
}

impl PartialOrd for int128 {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for int128 {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.cmp(&other.0)
    }
}

impl Add for uint128 {
    type Output = Self;
    fn add(self, other: Self) -> Self::Output {
        Self(self.0 + other.0)
    }
}

impl AddAssign for uint128 {
    fn add_assign(&mut self, other: Self) {
        self.0 += other.0;
    }
}

impl Sub for uint128 {
    type Output = Self;
    fn sub(self, other: Self) -> Self::Output {
        Self(self.0 - other.0)
    }
}

impl SubAssign for uint128 {
    fn sub_assign(&mut self, other: Self) {
        self.0 -= other.0;
    }
}

impl Mul for uint128 {
    type Output = Self;
    fn mul(self, other: Self) -> Self::Output {
        Self(self.0 * other.0)
    }
}

impl MulAssign for uint128 {
    fn mul_assign(&mut self, other: Self) {
        self.0 *= other.0;
    }
}

impl Div for uint128 {
    type Output = Self;
    fn div(self, other: Self) -> Self::Output {
        Self(self.0 / other.0)
    }
}

impl DivAssign for uint128 {
    fn div_assign(&mut self, other: Self) {
        self.0 /= other.0;
    }
}

impl Rem for uint128 {
    type Output = Self;
    fn rem(self, other: Self) -> Self::Output {
        Self(self.0 % other.0)
    }
}

impl RemAssign for uint128 {
    fn rem_assign(&mut self, other: Self) {
        self.0 %= other.0;
    }
}

impl PartialOrd for uint128 {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for uint128 {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.cmp(&other.0)
    }
}

// ============================================================================
// From Implementations
// ============================================================================

impl From<i8> for int128 {
    fn from(value: i8) -> Self {
        Self(value as i128)
    }
}

impl From<i16> for int128 {
    fn from(value: i16) -> Self {
        Self(value as i128)
    }
}

impl From<i32> for int128 {
    fn from(value: i32) -> Self {
        Self(value as i128)
    }
}

impl From<i64> for int128 {
    fn from(value: i64) -> Self {
        Self(value as i128)
    }
}

impl From<u8> for int128 {
    fn from(value: u8) -> Self {
        Self(value as i128)
    }
}

impl From<u16> for int128 {
    fn from(value: u16) -> Self {
        Self(value as i128)
    }
}

impl From<u32> for int128 {
    fn from(value: u32) -> Self {
        Self(value as i128)
    }
}

impl From<u64> for int128 {
    fn from(value: u64) -> Self {
        Self(value as i128)
    }
}

impl From<u8> for uint128 {
    fn from(value: u8) -> Self {
        Self(value as u128)
    }
}

impl From<u16> for uint128 {
    fn from(value: u16) -> Self {
        Self(value as u128)
    }
}

impl From<u32> for uint128 {
    fn from(value: u32) -> Self {
        Self(value as u128)
    }
}

impl From<u64> for uint128 {
    fn from(value: u64) -> Self {
        Self(value as u128)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_int128_from() {
        let a = int128::from(42i64);
        assert_eq!(a.0, 42);

        let b = int128::from_i32(-100);
        assert_eq!(b.0, -100);
    }

    #[test]
    fn test_int128_conversions() {
        let val = int128::from(1000i64);
        assert_eq!(val.as_i64(), Some(1000));
        assert_eq!(val.as_u64(), Some(1000));

        let big = int128::from(i64::MAX);
        assert_eq!(big.as_i64(), Some(i64::MAX));
        // i64::MAX fits in u64
        assert_eq!(big.as_u64(), Some(i64::MAX as u64));

        // Negative values don't fit in u64
        let negative = int128::from(-1i64);
        assert!(negative.as_u64().is_none());
    }

    #[test]
    fn test_int128_operations() {
        let a = int128::from(10i64);
        let b = int128::from(5i64);

        assert_eq!((a + b).0, 15);
        assert_eq!((a - b).0, 5);
        assert_eq!((a * b).0, 50);
        assert_eq!((a / b).0, 2);
        assert_eq!((a % b).0, 0);
    }

    #[test]
    fn test_int128_negative() {
        let a = int128::from(-42i64);
        assert!(a.is_negative());
        assert_eq!(a.abs().0, 42);
    }

    #[test]
    fn test_int128_bit_width() {
        assert_eq!(int128(0).bit_width(), 0);
        assert_eq!(int128(1).bit_width(), 1);
        assert_eq!(int128(255).bit_width(), 8);
        assert_eq!(int128(256).bit_width(), 9);
    }

    #[test]
    fn test_int128_power_of_two() {
        assert!(int128(1).is_power_of_two());
        assert!(int128(2).is_power_of_two());
        assert!(int128(256).is_power_of_two());
        assert!(!int128(3).is_power_of_two());
        assert!(!int128(0).is_power_of_two());
    }

    #[test]
    fn test_uint128_from() {
        let a = uint128::from(42u64);
        assert_eq!(a.0, 42);

        let b = uint128::from_u32(100);
        assert_eq!(b.0, 100);
    }

    #[test]
    fn test_uint128_conversions() {
        let val = uint128::from(1000u64);
        assert_eq!(val.as_u64(), Some(1000));
        assert_eq!(val.as_i64(), Some(1000));

        let big = uint128::from(u64::MAX);
        assert_eq!(big.as_u64(), Some(u64::MAX));
        assert!(big.as_i64().is_none());
    }

    #[test]
    fn test_uint128_operations() {
        let a = uint128::from(10u64);
        let b = uint128::from(5u64);

        assert_eq!((a + b).0, 15);
        assert_eq!((a - b).0, 5);
        assert_eq!((a * b).0, 50);
        assert_eq!((a / b).0, 2);
        assert_eq!((a % b).0, 0);
    }

    #[test]
    fn test_uint128_bit_width() {
        assert_eq!(uint128(0).bit_width(), 0);
        assert_eq!(uint128(1).bit_width(), 1);
        assert_eq!(uint128(255).bit_width(), 8);
        assert_eq!(uint128(256).bit_width(), 9);
    }

    #[test]
    fn test_uint128_power_of_two() {
        assert!(uint128(1).is_power_of_two());
        assert!(uint128(2).is_power_of_two());
        assert!(uint128(256).is_power_of_two());
        assert!(!uint128(3).is_power_of_two());
        assert!(!uint128(0).is_power_of_two());
    }

    #[test]
    fn test_int128_abs_diff() {
        let a = int128::from(100i64);
        let b = int128::from(50i64);
        assert_eq!(a.abs_diff(b).0, 50);

        let c = int128::from(50i64);
        let d = int128::from(100i64);
        assert_eq!(c.abs_diff(d).0, 50);
    }

    #[test]
    fn test_display() {
        assert_eq!(format!("{}", int128(42)), "42");
        assert_eq!(format!("{}", uint128(42)), "42");
        assert_eq!(format!("{:x}", int128(255)), "ff");
        assert_eq!(format!("{:X}", uint128(255)), "FF");
        assert_eq!(format!("{:b}", int128(5)), "101");
    }

    // Edge case tests for MEDIUM security fix

    #[test]
    fn test_int128_checked_abs_normal() {
        assert_eq!(int128(-42).checked_abs(), Some(int128(42)));
        assert_eq!(int128(42).checked_abs(), Some(int128(42)));
        assert_eq!(int128(0).checked_abs(), Some(int128(0)));
    }

    #[test]
    fn test_int128_checked_abs_min_overflow() {
        // i128::MIN cannot be represented as positive i128
        // The absolute value would be i128::MAX + 1, which overflows
        let min = int128(i128::MIN);
        assert_eq!(min.checked_abs(), None);
    }

    #[test]
    fn test_int128_checked_div_normal() {
        assert_eq!(int128(100).checked_div(int128(5)), Ok(int128(20)));
        assert_eq!(int128(-100).checked_div(int128(5)), Ok(int128(-20)));
        assert_eq!(int128(100).checked_div(int128(-5)), Ok(int128(-20)));
        assert_eq!(int128(-100).checked_div(int128(-5)), Ok(int128(20)));
    }

    #[test]
    fn test_int128_checked_div_by_zero() {
        assert_eq!(
            int128(100).checked_div(int128(0)),
            Err(DivisionError::DivisionByZero)
        );
        assert_eq!(
            int128(0).checked_div(int128(0)),
            Err(DivisionError::DivisionByZero)
        );
        assert_eq!(
            int128(i128::MIN).checked_div(int128(0)),
            Err(DivisionError::DivisionByZero)
        );
    }

    #[test]
    fn test_int128_checked_rem_normal() {
        assert_eq!(int128(100).checked_rem(int128(30)), Ok(int128(10)));
        assert_eq!(int128(-100).checked_rem(int128(30)), Ok(int128(-10)));
        assert_eq!(int128(100).checked_rem(int128(-30)), Ok(int128(10)));
    }

    #[test]
    fn test_int128_checked_rem_by_zero() {
        assert_eq!(
            int128(100).checked_rem(int128(0)),
            Err(DivisionError::DivisionByZero)
        );
        assert_eq!(
            int128(0).checked_rem(int128(0)),
            Err(DivisionError::DivisionByZero)
        );
        assert_eq!(
            int128(i128::MIN).checked_rem(int128(0)),
            Err(DivisionError::DivisionByZero)
        );
    }

    #[test]
    fn test_uint128_checked_div_normal() {
        assert_eq!(uint128(100).checked_div(uint128(5)), Ok(uint128(20)));
        assert_eq!(uint128(0).checked_div(uint128(5)), Ok(uint128(0)));
    }

    #[test]
    fn test_uint128_checked_div_by_zero() {
        assert_eq!(
            uint128(100).checked_div(uint128(0)),
            Err(DivisionError::DivisionByZero)
        );
        assert_eq!(
            uint128(0).checked_div(uint128(0)),
            Err(DivisionError::DivisionByZero)
        );
        assert_eq!(
            uint128(u128::MAX).checked_div(uint128(0)),
            Err(DivisionError::DivisionByZero)
        );
    }

    #[test]
    fn test_uint128_checked_rem_normal() {
        assert_eq!(uint128(100).checked_rem(uint128(30)), Ok(uint128(10)));
        assert_eq!(uint128(0).checked_rem(uint128(30)), Ok(uint128(0)));
    }

    #[test]
    fn test_uint128_checked_rem_by_zero() {
        assert_eq!(
            uint128(100).checked_rem(uint128(0)),
            Err(DivisionError::DivisionByZero)
        );
        assert_eq!(
            uint128(0).checked_rem(uint128(0)),
            Err(DivisionError::DivisionByZero)
        );
        assert_eq!(
            uint128(u128::MAX).checked_rem(uint128(0)),
            Err(DivisionError::DivisionByZero)
        );
    }

    #[test]
    fn test_int128_checked_operations_with_max_min() {
        let max = int128(i128::MAX);
        let min = int128(i128::MIN);
        let one = int128(1);

        // checked_div with extreme values
        assert_eq!(min.checked_div(one), Ok(min));
        assert_eq!(max.checked_div(one), Ok(max));

        // checked_rem with extreme values
        assert_eq!(min.checked_rem(one), Ok(int128(0)));
        assert_eq!(max.checked_rem(one), Ok(int128(0)));
    }

    #[test]
    fn test_int128_checked_div_rounding() {
        // Test integer division behavior (rounds toward zero)
        assert_eq!(int128(7).checked_div(int128(3)), Ok(int128(2)));
        assert_eq!(int128(-7).checked_div(int128(3)), Ok(int128(-2)));
        assert_eq!(int128(7).checked_div(int128(-3)), Ok(int128(-2)));
        assert_eq!(int128(-7).checked_div(int128(-3)), Ok(int128(2)));
    }

    #[test]
    fn test_uint128_checked_div_large_values() {
        // Test with large u128 values
        let large = uint128(u128::MAX);
        let half = uint128(u128::MAX / 2);
        assert_eq!(large.checked_div(uint128(2)), Ok(uint128(u128::MAX / 2)));
        assert_eq!(large.checked_rem(uint128(2)), Ok(uint128(1)));
    }

    #[test]
    fn test_checked_abs_preserves_sign() {
        // Verify checked_abs returns positive values correctly
        assert_eq!(int128(-1).checked_abs(), Some(int128(1)));
        assert_eq!(int128(i128::MAX).checked_abs(), Some(int128(i128::MAX)));
        assert_eq!(int128(i128::MIN + 1).checked_abs(), Some(int128(i128::MAX)));
    }
}
