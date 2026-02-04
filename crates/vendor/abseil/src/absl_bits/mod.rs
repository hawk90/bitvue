//! Bit manipulation utilities.
//!
//! This module provides comprehensive bit-level operations including:
//! - Basic bit operations (popcount, bit rotation, reversal)
//! - Bit masks and bit fields
//! - Gray code conversions
//! - Endianness conversions
//! - Bit buffers for storage
//! - Bit casting and advanced utilities
//!
//! # Examples
//!
//! ```rust
//! use abseil::{popcount, count_trailing_zeros, next_power_of_two};
//!
//! let n: u32 = 0b1010_0100;
//! assert_eq!(popcount(n), 3);
//! assert_eq!(count_trailing_zeros(n), 2);
//! assert_eq!(next_power_of_two(100u32), 128);
//! ```

// Core bit operations module (existing)
mod bits;

// New organized submodules
mod bitmask;
mod bit_ops;
mod bit_buffer;
mod endianness;
mod gray_code;
mod bit_cast;

// Re-export core operations
pub use bits::{
    bit_width, count_leading_zeros, count_trailing_zeros, highest_bit, is_power_of_two,
    lowest_bit, next_power_of_two, popcount, prev_power_of_two, reverse_bits, reverse_bytes,
    rotate_left, rotate_right,
};

// Re-export bitmask types
pub use bitmask::{BitField, BitMask, BitPosition};

// Re-export operation traits
pub use bit_ops::{BitOps, ByteOps, WordOps};

// Re-export buffer type
pub use bit_buffer::BitBuffer;

// Re-export endianness utilities
pub use endianness::{from_be, from_le, swap_u16, swap_u32, swap_u64, to_be, to_le, Endianness};

// Re-export gray code utilities
pub use gray_code::{
    binary_to_gray, binary_to_gray16, binary_to_gray32, clear_lowest_set_bit,
    deinterleave_bits, find_first_set, find_last_set, gray_to_binary,
    gray_to_binary16, gray_to_binary32, hamming_distance, hamming_distance16,
    hamming_distance32, hamming_distance64, highest_set_bit_value, interleave_bits,
    lowest_set_bit_value, parity_bit, parity_bit16, parity_bit32, round_up_power_of_two,
    set_lowest_clear_bit,
};

// Re-export bit casting utilities
pub use bit_cast::{bit_cast, BitHacks, BitMatrix, BitPermutation};

// Module alias for convenience
pub use self::bits as bit_ops_internal;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_bit_operations() {
        assert_eq!(popcount(0b10101010u32), 4);
        assert_eq!(count_trailing_zeros(0b00100000u32), 5);
        assert_eq!(count_leading_zeros(0b00100000u32), 26);
    }

    #[test]
    fn test_bitmask() {
        let mask = BitMask::<u32>::new(0);
        assert!(!mask.test(0));
        let mask = BitMask::new(0).set_bit(5);
        assert!(mask.is_set(5));
    }

    #[test]
    fn test_gray_code() {
        assert_eq!(binary_to_gray(0b1010u8), 0b1111);
        assert_eq!(gray_to_binary(0b1111u8), 0b1010);
    }

    #[test]
    fn test_hamming_distance() {
        assert_eq!(hamming_distance(0b1010u8, 0b1100u8), 2);
    }

    #[test]
    fn test_parity() {
        assert_eq!(parity_bit(0b1010u8), false);
        assert_eq!(parity_bit(0b1011u8), true);
    }

    #[test]
    fn test_endianness_swap() {
        assert_eq!(swap_u16(0x1234), 0x3412);
        assert_eq!(swap_u32(0x12345678), 0x78563412);
    }
}
