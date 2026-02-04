//! Bit manipulation utilities.
//!
//! This module provides bit manipulation utilities similar to Abseil's `absl/numeric/bits`.
//!
//! # Examples
//!
//! ```rust
//! use abseil::{count_trailing_zeros, count_leading_zeros, popcount};
//!
//! let n: u32 = 0b0001_0110_0000;
//! assert_eq!(count_trailing_zeros(n), 5);
//! assert_eq!(count_leading_zeros(n), 23);
//! assert_eq!(popcount(n), 3);
//! ```

use core::mem;

/// Counts the number of 1-bits (population count) in an integer.
///
/// This is also known as the Hamming weight or popcount.
///
/// # Examples
///
/// ```rust
/// use abseil::popcount;
///
/// assert_eq!(popcount(0u32), 0);
/// assert_eq!(popcount(1u32), 1);
/// assert_eq!(popcount(0b1010u32), 2);
/// assert_eq!(popcount(0xFFFFFFFFu32), 32);
/// ```
#[inline]
pub fn popcount<T: Popcount>(x: T) -> u32 {
    x.popcount()
}

/// Counts the number of trailing zeros in an integer.
///
/// Returns the number of consecutive 0-bits starting from the least significant bit.
///
/// # Examples
///
/// ```rust
/// use abseil::count_trailing_zeros;
///
/// assert_eq!(count_trailing_zeros(1u32), 0);
/// assert_eq!(count_trailing_zeros(0b1000u32), 3);
/// assert_eq!(count_trailing_zeros(0u32), 32);
/// ```
#[inline]
pub fn count_trailing_zeros<T: CountZeros>(x: T) -> u32 {
    x.count_trailing_zeros()
}

/// Counts the number of leading zeros in an integer.
///
/// Returns the number of consecutive 0-bits starting from the most significant bit.
///
/// # Examples
///
/// ```rust
/// use abseil::count_leading_zeros;
///
/// assert_eq!(count_leading_zeros(1u32), 31);
/// assert_eq!(count_leading_zeros(0x8000_0000u32), 0);
/// assert_eq!(count_leading_zeros(0u32), 32);
/// ```
#[inline]
pub fn count_leading_zeros<T: CountZeros>(x: T) -> u32 {
    x.count_leading_zeros()
}

/// Returns the number of bits required to represent the given value.
///
/// For zero, returns 0.
///
/// # Examples
///
/// ```rust
/// use abseil::bit_width;
///
/// assert_eq!(bit_width(0u32), 0);
/// assert_eq!(bit_width(1u32), 1);
/// assert_eq!(bit_width(5u32), 3);  // 5 = 0b101
/// assert_eq!(bit_width(0xFFu32), 8);
/// ```
#[inline]
pub fn bit_width<T: BitWidth>(x: T) -> u32 {
    x.bit_width()
}

/// Returns the highest bit set in the given value.
///
/// If the value is zero, returns zero.
///
/// # Examples
///
/// ```rust
/// use abseil::highest_bit;
///
/// assert_eq!(highest_bit(0u32), 0);
/// assert_eq!(highest_bit(1u32), 1);
/// assert_eq!(highest_bit(5u32), 4);  // 5 = 0b101
/// assert_eq!(highest_bit(0xFFu32), 128);
/// ```
#[inline]
pub fn highest_bit<T: HighestBit>(x: T) -> T {
    x.highest_bit()
}

/// Returns the lowest bit set in the given value.
///
/// If the value is zero, returns zero.
///
/// # Examples
///
/// ```rust
/// use abseil::lowest_bit;
///
/// assert_eq!(lowest_bit(0u32), 0);
/// assert_eq!(lowest_bit(1u32), 1);
/// assert_eq!(lowest_bit(0b1000u32), 8);
/// assert_eq!(lowest_bit(0xFFu32), 1);
/// ```
#[inline]
pub fn lowest_bit<T: LowestBit>(x: T) -> T {
    x.lowest_bit()
}

/// Checks if a value is a power of two.
///
/// Zero is not considered a power of two.
///
/// # Examples
///
/// ```rust
/// use abseil::is_power_of_two;
///
/// assert!(!is_power_of_two(0u32));
/// assert!(is_power_of_two(1u32));
/// assert!(is_power_of_two(2u32));
/// assert!(is_power_of_two(32u32));
/// assert!(!is_power_of_two(3u32));
/// assert!(!is_power_of_two(5u32));
/// ```
#[inline]
pub fn is_power_of_two<T: IsPowerOfTwo>(x: T) -> bool {
    x.is_power_of_two()
}

/// Returns the next power of two greater than or equal to the given value.
///
/// If the value is larger than the maximum representable value, returns zero.
///
/// # Examples
///
/// ```rust
/// use abseil::next_power_of_two;
///
/// assert_eq!(next_power_of_two(0u32), 1);
/// assert_eq!(next_power_of_two(1u32), 1);
/// assert_eq!(next_power_of_two(5u32), 8);
/// assert_eq!(next_power_of_two(16u32), 16);
/// assert_eq!(next_power_of_two(17u32), 32);
/// ```
#[inline]
pub fn next_power_of_two<T: NextPowerOfTwo>(x: T) -> T {
    x.next_power_of_two()
}

/// Returns the previous power of two less than or equal to the given value.
///
/// If the value is zero, returns zero.
///
/// # Examples
///
/// ```rust
/// use abseil::prev_power_of_two;
///
/// assert_eq!(prev_power_of_two(0u32), 0);
/// assert_eq!(prev_power_of_two(1u32), 1);
/// assert_eq!(prev_power_of_two(5u32), 4);
/// assert_eq!(prev_power_of_two(16u32), 16);
/// assert_eq!(prev_power_of_two(17u32), 16);
/// ```
#[inline]
pub fn prev_power_of_two<T: PrevPowerOfTwo>(x: T) -> T {
    x.prev_power_of_two()
}

/// Rotates the bits to the left.
///
/// # Examples
///
/// ```rust
/// use abseil::rotate_left;
///
/// let n: u32 = 0xF0000000;
/// assert_eq!(rotate_left(n, 4), 15);
/// assert_eq!(rotate_left(n, 32), n); // Full rotation
/// ```
#[inline]
pub fn rotate_left<T: Rotate>(x: T, n: u32) -> T {
    x.rotate_left(n)
}

/// Rotates the bits to the right.
///
/// # Examples
///
/// ```rust
/// use abseil::rotate_right;
///
/// let n: u32 = 15;
/// assert_eq!(rotate_right(n, 4), 0xF0000000);
/// assert_eq!(rotate_right(n, 32), n); // Full rotation
/// ```
#[inline]
pub fn rotate_right<T: Rotate>(x: T, n: u32) -> T {
    x.rotate_right(n)
}

/// Reverses the bytes of an integer.
///
/// # Examples
///
/// ```rust
/// use abseil::reverse_bytes;
///
/// let n: u32 = 0x12345678;
/// assert_eq!(reverse_bytes(n), 0x78563412);
/// ```
#[inline]
pub fn reverse_bytes<T: ReverseBytes>(x: T) -> T {
    x.reverse_bytes()
}

/// Reverses the bits of an integer.
///
/// # Examples
///
/// ```rust
/// use abseil::reverse_bits;
///
/// let n: u8 = 0b1111_0000;
/// assert_eq!(reverse_bits(n), 0b0000_1111);
/// ```
#[inline]
pub fn reverse_bits<T: ReverseBits>(x: T) -> T {
    x.reverse_bits()
}

// Trait definitions for bit operations

/// Trait for types that support popcount (population count).
pub trait Popcount: sealed::Sealed {
    /// Counts the number of 1-bits.
    fn popcount(self) -> u32;
}

/// Trait for types that support counting zeros.
pub trait CountZeros: sealed::Sealed {
    /// Counts trailing zeros.
    fn count_trailing_zeros(self) -> u32;
    /// Counts leading zeros.
    fn count_leading_zeros(self) -> u32;
}

/// Trait for types that can calculate bit width.
pub trait BitWidth: sealed::Sealed {
    /// Returns the number of bits required to represent the value.
    fn bit_width(self) -> u32;
}

/// Trait for types that can find the highest set bit.
pub trait HighestBit: sealed::Sealed {
    /// Returns the highest bit set (zero if input is zero).
    fn highest_bit(self) -> Self;
}

/// Trait for types that can find the lowest set bit.
pub trait LowestBit: sealed::Sealed {
    /// Returns the lowest bit set (zero if input is zero).
    fn lowest_bit(self) -> Self;
}

/// Trait for types that can check power of two.
pub trait IsPowerOfTwo: sealed::Sealed {
    /// Returns true if the value is a power of two.
    fn is_power_of_two(self) -> bool;
}

/// Trait for types that can compute next power of two.
pub trait NextPowerOfTwo: sealed::Sealed {
    /// Returns the next power of two >= the value.
    fn next_power_of_two(self) -> Self;
}

/// Trait for types that can compute previous power of two.
pub trait PrevPowerOfTwo: sealed::Sealed {
    /// Returns the previous power of two <= the value.
    fn prev_power_of_two(self) -> Self;
}

/// Trait for types that support rotation.
pub trait Rotate: sealed::Sealed {
    /// Rotates bits left.
    fn rotate_left(self, n: u32) -> Self;
    /// Rotates bits right.
    fn rotate_right(self, n: u32) -> Self;
}

/// Trait for types that support byte reversal.
pub trait ReverseBytes: sealed::Sealed {
    /// Reverses the bytes.
    fn reverse_bytes(self) -> Self;
}

/// Trait for types that support bit reversal.
pub trait ReverseBits: sealed::Sealed {
    /// Reverses the bits.
    fn reverse_bits(self) -> Self;
}

mod sealed {
    pub trait Sealed {}
}

// Implement for unsigned integer types

macro_rules! impl_unsigned {
    ($($ty:ty),*) => {
        $(
            impl sealed::Sealed for $ty {}

            impl Popcount for $ty {
                #[inline]
                fn popcount(self) -> u32 {
                    self.count_ones()
                }
            }

            impl CountZeros for $ty {
                #[inline]
                fn count_trailing_zeros(self) -> u32 {
                    self.trailing_zeros()
                }

                #[inline]
                fn count_leading_zeros(self) -> u32 {
                    self.leading_zeros()
                }
            }

            impl BitWidth for $ty {
                #[inline]
                fn bit_width(self) -> u32 {
                    mem::size_of::<$ty>() as u32 * 8 - self.leading_zeros()
                }
            }

            impl HighestBit for $ty {
                #[inline]
                fn highest_bit(self) -> Self {
                    if self == 0 {
                        0
                    } else {
                        1 << (BitWidth::bit_width(self) - 1)
                    }
                }
            }

            impl LowestBit for $ty {
                #[inline]
                fn lowest_bit(self) -> Self {
                    if self == 0 {
                        0
                    } else {
                        1 << self.trailing_zeros()
                    }
                }
            }

            impl IsPowerOfTwo for $ty {
                #[inline]
                fn is_power_of_two(self) -> bool {
                    self != 0 && (self & (self - 1)) == 0
                }
            }

            impl NextPowerOfTwo for $ty {
                #[inline]
                fn next_power_of_two(self) -> Self {
                    self.next_power_of_two()
                }
            }

            impl PrevPowerOfTwo for $ty {
                #[inline]
                fn prev_power_of_two(self) -> Self {
                    if self == 0 {
                        0
                    } else {
                        1 << (BitWidth::bit_width(self) - 1)
                    }
                }
            }

            impl Rotate for $ty {
                #[inline]
                fn rotate_left(self, n: u32) -> Self {
                    self.rotate_left(n)
                }

                #[inline]
                fn rotate_right(self, n: u32) -> Self {
                    self.rotate_right(n)
                }
            }

            impl ReverseBytes for $ty {
                #[inline]
                fn reverse_bytes(self) -> Self {
                    self.swap_bytes()
                }
            }

            impl ReverseBits for $ty {
                #[inline]
                fn reverse_bits(self) -> Self {
                    self.reverse_bits()
                }
            }
        )*
    };
}

impl_unsigned!(u8, u16, u32, u64, usize);

// Implement for signed integer types

macro_rules! impl_signed {
    ($($ity:ty => $uty:ty),*) => {
        $(
            impl sealed::Sealed for $ity {}

            impl Popcount for $ity {
                #[inline]
                fn popcount(self) -> u32 {
                    (self as $uty).popcount()
                }
            }

            impl CountZeros for $ity {
                #[inline]
                fn count_trailing_zeros(self) -> u32 {
                    (self as $uty).trailing_zeros()
                }

                #[inline]
                fn count_leading_zeros(self) -> u32 {
                    (self as $uty).leading_zeros()
                }
            }

            impl BitWidth for $ity {
                #[inline]
                fn bit_width(self) -> u32 {
                    BitWidth::bit_width(self as $uty)
                }
            }

            impl IsPowerOfTwo for $ity {
                #[inline]
                fn is_power_of_two(self) -> bool {
                    self > 0 && (self as $uty).is_power_of_two()
                }
            }

            impl Rotate for $ity {
                #[inline]
                fn rotate_left(self, n: u32) -> Self {
                    (self as $uty).rotate_left(n) as $ity
                }

                #[inline]
                fn rotate_right(self, n: u32) -> Self {
                    (self as $uty).rotate_right(n) as $ity
                }
            }

            impl ReverseBytes for $ity {
                #[inline]
                fn reverse_bytes(self) -> Self {
                    (self as $uty).swap_bytes() as $ity
                }
            }

            impl ReverseBits for $ity {
                #[inline]
                fn reverse_bits(self) -> Self {
                    (self as $uty).reverse_bits() as $ity
                }
            }
        )*
    };
}

impl_signed!(i8 => u8, i16 => u16, i32 => u32, i64 => u64, isize => usize);

#[cfg(test)]
mod tests {
    use super::*;

    // Popcount tests

    #[test]
    fn test_popcount_u8() {
        assert_eq!(popcount(0u8), 0);
        assert_eq!(popcount(1u8), 1);
        assert_eq!(popcount(0b1010u8), 2);
        assert_eq!(popcount(0xFFu8), 8);
    }

    #[test]
    fn test_popcount_u32() {
        assert_eq!(popcount(0u32), 0);
        assert_eq!(popcount(1u32), 1);
        assert_eq!(popcount(0b1010u32), 2);
        assert_eq!(popcount(0xFFFFFFFFu32), 32);
    }

    #[test]
    fn test_popcount_u64() {
        assert_eq!(popcount(0u64), 0);
        assert_eq!(popcount(1u64), 1);
        assert_eq!(popcount(0b1010u64), 2);
        assert_eq!(popcount(0xFFFFFFFFFFFFFFFFu64), 64);
    }

    // Count zeros tests

    #[test]
    fn test_count_trailing_zeros_u32() {
        assert_eq!(count_trailing_zeros(1u32), 0);
        assert_eq!(count_trailing_zeros(0b1000u32), 3);
        assert_eq!(count_trailing_zeros(0u32), 32);
    }

    #[test]
    fn test_count_leading_zeros_u32() {
        assert_eq!(count_leading_zeros(1u32), 31);
        assert_eq!(count_leading_zeros(0x8000_0000u32), 0);
        assert_eq!(count_leading_zeros(0u32), 32);
    }

    #[test]
    fn test_count_trailing_zeros_u64() {
        assert_eq!(count_trailing_zeros(1u64), 0);
        assert_eq!(count_trailing_zeros(0b1000u64), 3);
        assert_eq!(count_trailing_zeros(0u64), 64);
    }

    #[test]
    fn test_count_leading_zeros_u64() {
        assert_eq!(count_leading_zeros(1u64), 63);
        assert_eq!(count_leading_zeros(0x8000_0000_0000_0000u64), 0);
        assert_eq!(count_leading_zeros(0u64), 64);
    }

    // Bit width tests

    #[test]
    fn test_bit_width() {
        assert_eq!(bit_width(0u32), 0);
        assert_eq!(bit_width(1u32), 1);
        assert_eq!(bit_width(5u32), 3);  // 5 = 0b101
        assert_eq!(bit_width(0xFFu32), 8);
        assert_eq!(bit_width(0xFFFFu32), 16);
        assert_eq!(bit_width(0xFFFFFFFFu32), 32);
    }

    // Highest bit tests

    #[test]
    fn test_highest_bit() {
        assert_eq!(highest_bit(0u32), 0);
        assert_eq!(highest_bit(1u32), 1);
        assert_eq!(highest_bit(5u32), 4);  // 5 = 0b101
        assert_eq!(highest_bit(0xFFu32), 128);
        assert_eq!(highest_bit(0x8000_0000u32), 0x8000_0000);
    }

    // Lowest bit tests

    #[test]
    fn test_lowest_bit() {
        assert_eq!(lowest_bit(0u32), 0);
        assert_eq!(lowest_bit(1u32), 1);
        assert_eq!(lowest_bit(0b1000u32), 8);
        assert_eq!(lowest_bit(0xFFu32), 1);
        assert_eq!(lowest_bit(0x8000_0000u32), 0x8000_0000);
    }

    // Power of two tests

    #[test]
    fn test_is_power_of_two() {
        assert!(!is_power_of_two(0u32));
        assert!(is_power_of_two(1u32));
        assert!(is_power_of_two(2u32));
        assert!(is_power_of_two(4u32));
        assert!(is_power_of_two(32u32));
        assert!(!is_power_of_two(3u32));
        assert!(!is_power_of_two(5u32));
        assert!(!is_power_of_two(6u32));
    }

    #[test]
    fn test_next_power_of_two_u32() {
        assert_eq!(next_power_of_two(0u32), 1);
        assert_eq!(next_power_of_two(1u32), 1);
        assert_eq!(next_power_of_two(2u32), 2);
        assert_eq!(next_power_of_two(5u32), 8);
        assert_eq!(next_power_of_two(16u32), 16);
        assert_eq!(next_power_of_two(17u32), 32);
    }

    #[test]
    fn test_prev_power_of_two() {
        assert_eq!(prev_power_of_two(0u32), 0);
        assert_eq!(prev_power_of_two(1u32), 1);
        assert_eq!(prev_power_of_two(2u32), 2);
        assert_eq!(prev_power_of_two(5u32), 4);
        assert_eq!(prev_power_of_two(16u32), 16);
        assert_eq!(prev_power_of_two(17u32), 16);
    }

    // Rotate tests

    #[test]
    fn test_rotate_left() {
        // Test with simpler values
        let n: u32 = 1; // 0b0000...0001
        assert_eq!(rotate_left(n, 1), 2); // 0b0000...0010
        assert_eq!(rotate_left(n, 31), 0x8000_0000); // Bit 31 moved to position 0

        let n: u32 = 0x8000_0000;
        assert_eq!(rotate_left(n, 1), 1); // Bit 31 wrapped around

        // Full rotation
        let n: u32 = 12345;
        assert_eq!(rotate_left(n, 32), n);
    }

    #[test]
    fn test_rotate_right() {
        // Test with simpler values
        let n: u32 = 2; // 0b0000...0010
        assert_eq!(rotate_right(n, 1), 1); // 0b0000...0001

        let n: u32 = 1;
        assert_eq!(rotate_right(n, 1), 0x8000_0000); // Bit 0 moved to position 31

        // Full rotation
        let n: u32 = 12345;
        assert_eq!(rotate_right(n, 32), n);
    }

    // Reverse bytes tests

    #[test]
    fn test_reverse_bytes_u32() {
        let n: u32 = 0x12345678;
        assert_eq!(reverse_bytes(n), 0x78563412);
    }

    #[test]
    fn test_reverse_bytes_u64() {
        let n: u64 = 0x0123456789ABCDEF;
        assert_eq!(reverse_bytes(n), 0xEFCDAB8967452301);
    }

    // Reverse bits tests

    #[test]
    fn test_reverse_bits_u8() {
        assert_eq!(reverse_bits(0b1111_0000u8), 0b0000_1111);
        assert_eq!(reverse_bits(0b1010_0101u8), 0b1010_0101);
        assert_eq!(reverse_bits(0u8), 0);
        assert_eq!(reverse_bits(0xFFu8), 0xFF);
    }

    #[test]
    fn test_reverse_bits_u32() {
        assert_eq!(reverse_bits(0x8000_0001u32), 0x8000_0001);
        assert_eq!(reverse_bits(0x0000_0001u32), 0x8000_0000);
    }

    // Signed integer tests

    #[test]
    fn test_signed_operations() {
        // Popcount on signed integers
        assert_eq!(popcount(-1i32), 32);
        assert_eq!(popcount(0i32), 0);

        // Rotate on signed integers - works on the bit representation
        let n: i32 = 1;
        assert_eq!(rotate_left(n, 1), 2);

        // Is power of two on signed integers
        assert!(is_power_of_two(2i32));
        assert!(is_power_of_two(16i32));
        assert!(!is_power_of_two(-2i32));
    }

    // Edge cases

    #[test]
    fn test_edge_cases_u8() {
        assert_eq!(bit_width(0u8), 0);
        assert_eq!(bit_width(0xFFu8), 8);
        assert_eq!(highest_bit(0u8), 0);
        assert_eq!(lowest_bit(0u8), 0);
    }

    #[test]
    fn test_all_ones() {
        assert_eq!(popcount(0xFFFFFFFFu32), 32);
        assert_eq!(bit_width(0xFFFFFFFFu32), 32);
        assert_eq!(highest_bit(0xFFFFFFFFu32), 0x8000_0000);
    }
}
