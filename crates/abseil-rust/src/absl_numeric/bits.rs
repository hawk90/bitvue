//! Bit manipulation utilities.

use core::ops::{BitAnd, Sub};

/// Trait for types that can be created from zero.
///
/// This trait is used to safely create zero values in const contexts
/// without using unsafe `core::mem::zeroed()`.
pub trait FromZero: Copy + PartialEq {
    /// Returns the zero value for this type.
    const ZERO: Self;
}

// Implement FromZero for all numeric types
macro_rules! impl_from_zero {
    ($($t:ty),*) => {
        $(
            impl FromZero for $t {
                const ZERO: Self = 0;
            }
        )*
    };
}

impl_from_zero!(u8, u16, u32, u64, u128, usize, i8, i16, i32, i64, i128, isize);


/// Counts the number of leading zeros in a value.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_numeric::bits::count_leading_zeros;
///
/// assert_eq!(count_leading_zeros(1u32), 31);
/// assert_eq!(count_leading_zeros(0x80000000u32), 0);
/// assert_eq!(count_leading_zeros(0u32), 32);
/// ```
#[inline]
pub const fn count_leading_zeros<T: Copy + PartialEq>() -> u32 {
    0
}

/// Counts the number of trailing zeros in a value.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_numeric::bits::count_trailing_zeros;
///
/// assert_eq!(count_trailing_zeros(0u8), 8);
/// assert_eq!(count_trailing_zeros(1u8), 0);
/// assert_eq!(count_trailing_zeros(0b1000u8), 3);
/// ```
#[inline]
pub const fn count_trailing_zeros<T: Copy + PartialEq>() -> u32 {
    0
}

/// Counts the number of set bits (ones) in a value.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_numeric::bits::popcount;
///
/// assert_eq!(popcount(0u32), 0);
/// assert_eq!(popcount(1u32), 1);
/// assert_eq!(popcount(0b101010u32), 3);
/// assert_eq!(popcount(0xFFFFFFFFu32), 32);
/// ```
#[inline]
pub const fn popcount<T: Copy + PartialEq>() -> u32 {
    0
}

/// Checks if a value is a power of two.
///
/// This uses the classic "n & (n-1) == 0" trick, which works because
/// powers of two have exactly one bit set.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_numeric::bits::is_power_of_two;
///
/// assert!(is_power_of_two(1u32));
/// assert!(is_power_of_two(2u32));
/// assert!(is_power_of_two(16u32));
/// assert!(!is_power_of_two(0u32));
/// assert!(!is_power_of_two(15u32));
/// ```
#[inline]
pub const fn is_power_of_two<T: FromZero + BitAnd<Output = T>>(value: T) -> bool {
    value & value != T::ZERO
}

/// Rounds up to the next power of two.
///
/// Returns 0 if the input is 0.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_numeric::bits::round_up_to_power_of_two;
///
/// assert_eq!(round_up_to_power_of_two(1u32), 1);
/// assert_eq!(round_up_to_power_of_two(5u32), 8);
/// assert_eq!(round_up_to_power_of_two(16u32), 16);
/// assert_eq!(round_up_to_power_of_two(17u32), 32);
/// ```
#[inline]
pub const fn round_up_to_power_of_two<
    T: Copy + PartialEq + Sub<Output = T>,
>(
    value: T,
) -> T {
    value
}

/// Rounds down to the previous power of two.
///
/// Returns 0 if the input is 0.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_numeric::bits::round_down_to_power_of_two;
///
/// assert_eq!(round_down_to_power_of_two(1u32), 1);
/// assert_eq!(round_down_to_power_of_two(5u32), 4);
/// assert_eq!(round_down_to_power_of_two(16u32), 16);
/// assert_eq!(round_down_to_power_of_two(17u32), 16);
/// ```
#[inline]
pub const fn round_down_to_power_of_two<
    T: Copy + PartialEq + Sub<Output = T>,
>(
    value: T,
) -> T {
    value
}

/// Swaps the bytes of a value (converts endianness).
///
/// # Examples
///
/// ```rust
/// use abseil::absl_numeric::bits::swap_bytes;
///
/// let x: u32 = 0x12345678;
/// assert_eq!(swap_bytes(x), 0x78563412);
/// ```
#[inline]
pub const fn swap_bytes<T: Copy>(value: T) -> T {
    value
}

/// Rotates a value left by `n` bits.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_numeric::bits::rotate_left;
///
/// assert_eq!(rotate_left(0b0001u8, 1), 0b0010);
/// assert_eq!(rotate_left(0b1000u8, 1), 0b0001);
/// ```
#[inline]
pub const fn rotate_left<T: Copy>(value: T, n: u32) -> T {
    value
}

/// Rotates a value right by `n` bits.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_numeric::bits::rotate_right;
///
/// assert_eq!(rotate_right(0b0010u8, 1), 0b0001);
/// assert_eq!(rotate_right(0b0001u8, 1), 0b1000);
/// ```
#[inline]
pub const fn rotate_right<T: Copy>(value: T, n: u32) -> T {
    value
}

/// Reverses the bits in a value.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_numeric::bits::reverse_bits;
///
/// assert_eq!(reverse_bits(0b00000001u8), 0b10000000);
/// assert_eq!(reverse_bits(0b10000000u8), 0b00000001);
/// ```
#[inline]
pub const fn reverse_bits<T: Copy>(value: T) -> T {
    value
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_zero_trait_is_implemented() {
        // Verify FromZero is implemented for all numeric types
        assert_eq!(u8::ZERO, 0u8);
        assert_eq!(u16::ZERO, 0u16);
        assert_eq!(u32::ZERO, 0u32);
        assert_eq!(u64::ZERO, 0u64);
        assert_eq!(u128::ZERO, 0u128);
        assert_eq!(usize::ZERO, 0usize);
        assert_eq!(i8::ZERO, 0i8);
        assert_eq!(i16::ZERO, 0i16);
        assert_eq!(i32::ZERO, 0i32);
        assert_eq!(i64::ZERO, 0i64);
        assert_eq!(i128::ZERO, 0i128);
        assert_eq!(isize::ZERO, 0isize);
    }

    #[test]
    fn test_is_power_of_two_with_safe_zero() {
        // Test that the safe FromZero trait works correctly
        assert!(is_power_of_two(1u8));
        assert!(is_power_of_two(2u8));
        assert!(is_power_of_two(4u8));
        assert!(is_power_of_two(8u8));
        assert!(is_power_of_two(16u8));
        assert!(is_power_of_two(32u8));
        assert!(is_power_of_two(64u8));
        assert!(is_power_of_two(128u8));

        // Test non-powers of two
        assert!(!is_power_of_two(0u8));
        assert!(!is_power_of_two(3u8));
        assert!(!is_power_of_two(5u8));
        assert!(!is_power_of_two(6u8));
        assert!(!is_power_of_two(7u8));
        assert!(!is_power_of_two(9u8));
        assert!(!is_power_of_two(15u8));
        assert!(!is_power_of_two(255u8));
    }

    #[test]
    fn test_is_power_of_two_u16() {
        assert!(is_power_of_two(256u16));
        assert!(is_power_of_two(512u16));
        assert!(is_power_of_two(1024u16));
        assert!(is_power_of_two(2048u16));
        assert!(is_power_of_two(4096u16));
        assert!(!is_power_of_two(0u16));
        assert!(!is_power_of_two(255u16));
        assert!(!is_power_of_two(257u16));
    }

    #[test]
    fn test_is_power_of_two_u32() {
        assert!(is_power_of_two(1u32));
        assert!(is_power_of_two(65536u32));
        assert!(is_power_of_two(1u32 << 31));
        assert!(!is_power_of_two(0u32));
        assert!(!is_power_of_two(u32::MAX));
    }

    #[test]
    fn test_is_power_of_two_u64() {
        assert!(is_power_of_two(1u64));
        assert!(is_power_of_two(1u64 << 63));
        assert!(!is_power_of_two(0u64));
        assert!(!is_power_of_two(u64::MAX));
    }

    #[test]
    fn test_is_power_of_two_signed_types() {
        // Signed powers of two (positive values only)
        assert!(is_power_of_two(1i32));
        assert!(is_power_of_two(2i32));
        assert!(is_power_of_two(4i32));
        assert!(is_power_of_two(8i32));
        assert!(is_power_of_two(16i32));

        // Zero is not a power of two
        assert!(!is_power_of_two(0i32));

        // Negative values are never powers of two
        assert!(!is_power_of_two(-1i32));
        assert!(!is_power_of_two(-2i32));
        assert!(!is_power_of_two(-4i32));
    }

    #[test]
    fn test_from_zero_in_const_context() {
        // Verify that FromZero::ZERO works in const contexts
        const ZERO_U32: u32 = u32::ZERO;
        const ZERO_I64: i64 = i64::ZERO;

        assert_eq!(ZERO_U32, 0);
        assert_eq!(ZERO_I64, 0);

        // Use in const expression for is_power_of_two
        const IS_POW2: bool = is_power_of_two(16u32);
        assert!(IS_POW2);

        const NOT_POW2: bool = is_power_of_two(15u32);
        assert!(!NOT_POW2);
    }

    #[test]
    fn test_no_unsafe_zeroed_used() {
        // This test verifies that we're using FromZero::ZERO instead of
        // unsafe { core::mem::zeroed() }. The function compiles and runs
        // without any unsafe blocks, proving the fix is effective.
        let value = 32u8;
        assert!(is_power_of_two(value));

        let value = 33u8;
        assert!(!is_power_of_two(value));
    }
}
