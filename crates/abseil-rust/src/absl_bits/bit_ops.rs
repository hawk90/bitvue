//! Bit operation traits and extensions.
//!
//! This module provides traits for common bit operations on integer types.

use crate::bits;

/// Compile-time bit manipulation trait.
pub trait BitOps
where
    Self: Sized
        + Copy
        + BitAnd<Output = Self>
        + BitOr<Output = Self>
        + BitXor<Output = Self>
        + Not<Output = Self>
        + PartialEq
        + Eq
        + From<u8>,
{
    /// Returns true if this value is a power of two.
    fn is_power_of_two(self) -> bool {
        bits::is_power_of_two(self)
    }

    /// Returns the next power of two greater than or equal to this value.
    fn next_power_of_two(self) -> Self {
        bits::next_power_of_two(self)
    }

    /// Returns the previous power of two less than or equal to this value.
    fn prev_power_of_two(self) -> Self {
        bits::prev_power_of_two(self)
    }

    /// Returns the number of leading zeros.
    fn count_leading_zeros(self) -> u32 {
        bits::count_leading_zeros(self)
    }

    /// Returns the number of trailing zeros.
    fn count_trailing_zeros(self) -> u32 {
        bits::count_trailing_zeros(self)
    }

    /// Returns the number of set bits.
    fn count_ones(self) -> u32 {
        bits::popcount(self)
    }

    /// Returns the position of the highest set bit.
    fn highest_bit(self) -> Option<u32> {
        bits::highest_bit(self)
    }

    /// Returns the position of the lowest set bit.
    fn lowest_bit(self) -> Option<u32> {
        bits::lowest_bit(self)
    }

    /// Reverses the bits.
    fn reverse_bits(self) -> Self {
        bits::reverse_bits(self)
    }

    /// Rotates bits left.
    fn rotate_left(self, n: u32) -> Self {
        bits::rotate_left(self, n)
    }

    /// Rotates bits right.
    fn rotate_right(self, n: u32) -> Self {
        bits::rotate_right(self, n)
    }
}

impl<T> BitOps for T
where
    T: Sized
        + Copy
        + BitAnd<Output = T>
        + BitOr<Output = T>
        + BitXor<Output = T>
        + Not<Output = T>
        + PartialEq
        + Eq
        + From<u8>,
{
}

/// Extension trait for bytes.
pub trait ByteOps {
    /// Returns true if this byte has an odd number of bits set.
    fn has_odd_parity(self) -> bool;

    /// Returns true if this byte has an even number of bits set.
    fn has_even_parity(self) -> bool;

    /// Returns the nibble (lower 4 bits).
    fn low_nibble(self) -> u8;

    /// Returns the high nibble (upper 4 bits).
    fn high_nibble(self) -> u8;

    /// Swaps the nibbles in this byte.
    fn swap_nibbles(self) -> u8;
}

impl ByteOps for u8 {
    fn has_odd_parity(self) -> bool {
        self.count_ones() % 2 == 1
    }

    fn has_even_parity(self) -> bool {
        !self.has_odd_parity()
    }

    fn low_nibble(self) -> u8 {
        self & 0x0F
    }

    fn high_nibble(self) -> u8 {
        (self >> 4) & 0x0F
    }

    fn swap_nibbles(self) -> u8 {
        (self << 4) | (self >> 4)
    }
}

/// Extension trait for word-sized integers.
pub trait WordOps: Sized {
    /// Returns true if this word has an odd number of bits set.
    fn has_odd_parity(&self) -> bool;

    /// Returns true if this word has an even number of bits set.
    fn has_even_parity(&self) -> bool;

    /// Returns true if this word is a valid UTF-8 character.
    fn is_utf8_char(&self) -> bool;

    /// Returns the number of bytes in this UTF-8 character, or 0 if invalid.
    fn utf8_char_len(&self) -> usize;

    /// Writes this word as a UTF-8 character to a slice.
    fn write_utf8_char(&self, slice: &mut [u8]) -> usize;
}

impl WordOps for u8 {
    fn has_odd_parity(&self) -> bool {
        self.count_ones() % 2 == 1
    }

    fn has_even_parity(&self) -> bool {
        !self.has_odd_parity()
    }

    fn is_utf8_char(&self) -> bool {
        match *self {
            0x00..=0x7F => true,
            0xC2..=0xF4 => true,
            _ => false,
        }
    }

    fn utf8_char_len(&self) -> usize {
        match *self {
            0x00..=0x7F => 1,
            0xC2..=0xDF => 2,
            0xE0..=0xEF => 3,
            0xF0..=0xF4 => 4,
            _ => 0,
        }
    }

    fn write_utf8_char(&self, slice: &mut [u8]) -> usize {
        let len = self.utf8_char_len();
        if len == 0 || len > slice.len() {
            return 0;
        }
        slice[0] = *self;
        1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_byte_ops_parity() {
        assert!(0u8.has_even_parity());
        assert!(!0x01u8.has_even_parity());
    }

    #[test]
    fn test_byte_ops_nibbles() {
        assert_eq!(0xABu8.low_nibble(), 0x0B);
        assert_eq!(0xABu8.high_nibble(), 0x0A);
    }

    #[test]
    fn test_byte_ops_swap_nibbles() {
        assert_eq!(0xABu8.swap_nibbles(), 0xBA);
    }

    #[test]
    fn test_word_ops_utf8() {
        assert!(0x41u8.is_utf8_char());
        assert!(0xC2u8.is_utf8_char());
    }
}
