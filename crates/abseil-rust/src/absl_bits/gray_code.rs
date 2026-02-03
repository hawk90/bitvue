//! Gray code conversion and related utilities.
//!
//! This module provides functions for converting between binary and Gray code,
//! as well as Hamming distance and parity bit computations.

/// Converts binary to Gray code.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_bits::gray_code::binary_to_gray;
///
/// assert_eq!(binary_to_gray(0b1010u8), 0b1111);
/// ```
pub const fn binary_to_gray(mut value: u8) -> u8 {
    value ^= value >> 1;
    value
}

/// Converts Gray code to binary.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_bits::gray_code::gray_to_binary;
///
/// assert_eq!(gray_to_binary(0b1111u8), 0b1010);
/// ```
pub const fn gray_to_binary(mut value: u8) -> u8 {
    let mut mask = value >> 1;
    while mask != 0 {
        value ^= mask;
        mask >>= 1;
    }
    value
}

/// Converts binary to Gray code (16-bit).
pub const fn binary_to_gray16(mut value: u16) -> u16 {
    value ^= value >> 1;
    value
}

/// Converts Gray code to binary (16-bit).
pub const fn gray_to_binary16(mut value: u16) -> u16 {
    let mut mask = value >> 1;
    while mask != 0 {
        value ^= mask;
        mask >>= 1;
    }
    value
}

/// Converts binary to Gray code (32-bit).
pub const fn binary_to_gray32(mut value: u32) -> u32 {
    value ^= value >> 1;
    value
}

/// Converts Gray code to binary (32-bit).
pub const fn gray_to_binary32(mut value: u32) -> u32 {
    let mut mask = value >> 1;
    while mask != 0 {
        value ^= mask;
        mask >>= 1;
    }
    value
}

/// Computes the Hamming distance between two values (number of differing bits).
///
/// # Examples
///
/// ```rust
/// use abseil::absl_bits::gray_code::hamming_distance;
///
/// assert_eq!(hamming_distance(0b1010u8, 0b1100u8), 2);
/// ```
pub const fn hamming_distance(a: u8, b: u8) -> u8 {
    (a ^ b).count_ones() as u8
}

/// Computes the Hamming distance between two 16-bit values.
pub const fn hamming_distance16(a: u16, b: u16) -> u16 {
    (a ^ b).count_ones() as u16
}

/// Computes the Hamming distance between two 32-bit values.
pub const fn hamming_distance32(a: u32, b: u32) -> u32 {
    (a ^ b).count_ones()
}

/// Computes the Hamming distance between two 64-bit values.
pub const fn hamming_distance64(a: u64, b: u64) -> u64 {
    (a ^ b).count_ones()
}

/// Computes a single parity bit for the given value (XOR of all bits).
///
/// # Examples
///
/// ```rust
/// use abseil::absl_bits::gray_code::parity_bit;
///
/// assert_eq!(parity_bit(0b1010u8), false); // Even number of 1s
/// assert_eq!(parity_bit(0b1011u8), true);  // Odd number of 1s
/// ```
pub const fn parity_bit(value: u8) -> bool {
    let mut v = value;
    v ^= v >> 4;
    v ^= v >> 2;
    v ^= v >> 1;
    (v & 1) == 1
}

/// Computes parity bit for 16-bit value.
pub const fn parity_bit16(value: u16) -> bool {
    let mut v = value;
    v ^= v >> 8;
    v ^= v >> 4;
    v ^= v >> 2;
    v ^= v >> 1;
    (v & 1) == 1
}

/// Computes parity bit for 32-bit value.
pub const fn parity_bit32(value: u32) -> bool {
    let mut v = value;
    v ^= v >> 16;
    v ^= v >> 8;
    v ^= v >> 4;
    v ^= v >> 2;
    v ^= v >> 1;
    (v & 1) == 1
}

/// Finds the first set bit (least significant set bit).
///
/// Returns the bit position (0-indexed) or None if no bits are set.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_bits::gray_code::find_first_set;
///
/// assert_eq!(find_first_set(0b00100000u8), Some(5));
/// assert_eq!(find_first_set(0u8), None);
/// ```
pub const fn find_first_set(value: u8) -> Option<u32> {
    if value == 0 {
        None
    } else {
        Some(value.trailing_zeros())
    }
}

/// Finds the last set bit (most significant set bit).
///
/// Returns the bit position (0-indexed) or None if no bits are set.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_bits::gray_code::find_last_set;
///
/// assert_eq!(find_last_set(0b00101000u8), Some(5));
/// assert_eq!(find_last_set(0u8), None);
/// ```
pub const fn find_last_set(value: u8) -> Option<u32> {
    if value == 0 {
        None
    } else {
        Some(7 - value.leading_zeros())
    }
}

/// Returns the value with only the lowest set bit of the input.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_bits::gray_code::lowest_set_bit_value;
///
/// assert_eq!(lowest_set_bit_value(0b1011000u8), 0b0001000);
/// ```
pub const fn lowest_set_bit_value(value: u8) -> u8 {
    value & value.wrapping_neg()
}

/// Returns the value with only the highest set bit of the input.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_bits::gray_code::highest_set_bit_value;
///
/// assert_eq!(highest_set_bit_value(0b00101100u8), 0b00100000);
/// ```
pub const fn highest_set_bit_value(value: u8) -> u8 {
    if value == 0 {
        0
    } else {
        1u8 << (7 - value.leading_zeros())
    }
}

/// Clears the lowest set bit.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_bits::gray_code::clear_lowest_set_bit;
///
/// assert_eq!(clear_lowest_set_bit(0b1011000u8), 0b1010000);
/// ```
pub const fn clear_lowest_set_bit(value: u8) -> u8 {
    value & (value - 1)
}

/// Sets the lowest clear bit.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_bits::gray_code::set_lowest_clear_bit;
///
/// assert_eq!(set_lowest_clear_bit(0b1010111u8), 0b1011111);
/// ```
pub const fn set_lowest_clear_bit(value: u8) -> u8 {
    value | (value + 1)
}

/// Rounds up to the next power of two (or returns the same value if already a power of two).
///
/// # Examples
///
/// ```rust
/// use abseil::absl_bits::gray_code::round_up_power_of_two;
///
/// assert_eq!(round_up_power_of_two(5u8), 8);
/// assert_eq!(round_up_power_of_two(8u8), 8);
/// ```
pub const fn round_up_power_of_two(value: u8) -> u8 {
    if value == 0 {
        return 1;
    }
    let mut v = value - 1;
    v |= v >> 1;
    v |= v >> 2;
    v |= v >> 4;
    v + 1
}

/// Interleaves bits from two values (Morton code / bit interleaving).
///
/// # Examples
///
/// ```rust
/// use abseil::absl_bits::gray_code::interleave_bits;
///
/// // interleave_bits(0b1010, 0b1100) = 0b01110100
/// ```
pub const fn interleave_bits(a: u16, b: u16) -> u32 {
    let mut result = 0u32;
    let mut x = a as u32;
    let mut y = b as u32;

    let mut i = 0;
    while i < 16 {
        result |= ((x & 1) << (2 * i)) | ((y & 1) << (2 * i + 1));
        x >>= 1;
        y >>= 1;
        i += 1;
    }

    result
}

/// Deinterleaves bits into two values.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_bits::gray_code::deinterleave_bits;
///
/// let (a, b) = deinterleave_bits(0b01110100u32);
/// ```
pub const fn deinterleave_bits(value: u32) -> (u16, u16) {
    let mut a = 0u16;
    let mut b = 0u16;
    let mut v = value;

    let mut i = 0;
    while i < 16 {
        a |= ((v & 1) as u16) << i;
        v >>= 1;
        b |= ((v & 1) as u16) << i;
        v >>= 1;
        i += 1;
    }

    (a, b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_binary_to_gray() {
        assert_eq!(binary_to_gray(0b0000u8), 0b0000);
        assert_eq!(binary_to_gray(0b0001u8), 0b0001);
        assert_eq!(binary_to_gray(0b0010u8), 0b0011);
        assert_eq!(binary_to_gray(0b1010u8), 0b1111);
        assert_eq!(binary_to_gray(0b1111u8), 0b1000);
    }

    #[test]
    fn test_gray_to_binary() {
        assert_eq!(gray_to_binary(0b0000u8), 0b0000);
        assert_eq!(gray_to_binary(0b0001u8), 0b0001);
        assert_eq!(gray_to_binary(0b0011u8), 0b0010);
        assert_eq!(gray_to_binary(0b1111u8), 0b1010);
        assert_eq!(gray_to_binary(0b1000u8), 0b1111);
    }

    #[test]
    fn test_gray_roundtrip() {
        let mut i = 0u8;
        while i < 255 {
            let gray = binary_to_gray(i);
            let binary = gray_to_binary(gray);
            assert_eq!(i, binary);
            i += 1;
        }
    }

    #[test]
    fn test_binary_to_gray16() {
        assert_eq!(binary_to_gray16(0x1234), 0x09DD);
    }

    #[test]
    fn test_gray_to_binary16() {
        assert_eq!(gray_to_binary16(0x09DD), 0x1234);
    }

    #[test]
    fn test_binary_to_gray32() {
        assert_eq!(binary_to_gray32(0x12345678), 0x09A6F05C);
    }

    #[test]
    fn test_gray_to_binary32() {
        assert_eq!(gray_to_binary32(0x09A6F05C), 0x12345678);
    }

    #[test]
    fn test_hamming_distance() {
        assert_eq!(hamming_distance(0b1010u8, 0b1100u8), 2);
        assert_eq!(hamming_distance(0b0000u8, 0b0000u8), 0);
        assert_eq!(hamming_distance(0b0000u8, 0b1111u8), 4);
        assert_eq!(hamming_distance(0xFFu8, 0x00u8), 8);
    }

    #[test]
    fn test_hamming_distance16() {
        assert_eq!(hamming_distance16(0x1234, 0x5678), 8);
        assert_eq!(hamming_distance16(0xFFFF, 0x0000), 16);
    }

    #[test]
    fn test_hamming_distance32() {
        assert_eq!(hamming_distance32(0xFFFFFFFF, 0x00000000), 32);
    }

    #[test]
    fn test_hamming_distance64() {
        assert_eq!(hamming_distance64(0xFFFFFFFFFFFFFFFF, 0), 64);
    }

    #[test]
    fn test_parity_bit() {
        assert_eq!(parity_bit(0b00000000u8), false);
        assert_eq!(parity_bit(0b00000001u8), true);
        assert_eq!(parity_bit(0b10101010u8), false);
        assert_eq!(parity_bit(0b10101011u8), true);
    }

    #[test]
    fn test_parity_bit16() {
        assert_eq!(parity_bit16(0x0000), false);
        assert_eq!(parity_bit16(0x0001), true);
    }

    #[test]
    fn test_parity_bit32() {
        assert_eq!(parity_bit32(0x00000000), false);
        assert_eq!(parity_bit32(0x00000001), true);
    }

    #[test]
    fn test_find_first_set() {
        assert_eq!(find_first_set(0b00000000u8), None);
        assert_eq!(find_first_set(0b00000001u8), Some(0));
        assert_eq!(find_first_set(0b00100000u8), Some(5));
        assert_eq!(find_first_set(0b10000000u8), Some(7));
    }

    #[test]
    fn test_find_last_set() {
        assert_eq!(find_last_set(0b00000000u8), None);
        assert_eq!(find_last_set(0b00000001u8), Some(0));
        assert_eq!(find_last_set(0b00101000u8), Some(5));
        assert_eq!(find_last_set(0b10000000u8), Some(7));
    }

    #[test]
    fn test_lowest_set_bit_value() {
        assert_eq!(lowest_set_bit_value(0b1011000u8), 0b0001000);
        assert_eq!(lowest_set_bit_value(0b00000001u8), 0b00000001);
        assert_eq!(lowest_set_bit_value(0b00000000u8), 0b00000000);
    }

    #[test]
    fn test_highest_set_bit_value() {
        assert_eq!(highest_set_bit_value(0b00101100u8), 0b00100000);
        assert_eq!(highest_set_bit_value(0b00000001u8), 0b00000001);
        assert_eq!(highest_set_bit_value(0b00000000u8), 0b00000000);
    }

    #[test]
    fn test_clear_lowest_set_bit() {
        assert_eq!(clear_lowest_set_bit(0b1011000u8), 0b1010000);
        assert_eq!(clear_lowest_set_bit(0b00000001u8), 0b00000000);
        assert_eq!(clear_lowest_set_bit(0b00000000u8), 0b00000000);
    }

    #[test]
    fn test_set_lowest_clear_bit() {
        assert_eq!(set_lowest_clear_bit(0b1010111u8), 0b1011111);
        assert_eq!(set_lowest_clear_bit(0b11111111u8), 0b11111111);
    }

    #[test]
    fn test_round_up_power_of_two() {
        assert_eq!(round_up_power_of_two(0u8), 1);
        assert_eq!(round_up_power_of_two(1u8), 1);
        assert_eq!(round_up_power_of_two(2u8), 2);
        assert_eq!(round_up_power_of_two(3u8), 4);
        assert_eq!(round_up_power_of_two(5u8), 8);
        assert_eq!(round_up_power_of_two(8u8), 8);
        assert_eq!(round_up_power_of_two(9u8), 16);
        assert_eq!(round_up_power_of_two(128u8), 128);
    }

    #[test]
    fn test_interleave_deinterleave_roundtrip() {
        let a = 0x1234u16;
        let b = 0x5678u16;
        let interleaved = interleave_bits(a, b);
        let (out_a, out_b) = deinterleave_bits(interleaved);
        assert_eq!((out_a, out_b), (a, b));
    }
}
