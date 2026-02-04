//! CRC utility functions.
//!
//! This module provides utility functions for CRC computation,
//! including bit manipulation, hex conversion, and helper functions.

/// Reflects the lower `bits` bits of a value.
///
/// # Examples
///
/// ```
/// use abseil::absl_crc::utils::reflect_bits;
///
/// assert_eq!(reflect_bits(0b1100_0000u8, 8), 0b0000_0011u8);
/// assert_eq!(reflect_bits(0x12345678u32, 32), 0x1e6a2c48u32);
/// ```
pub const fn reflect_bits(mut v: u64, bits: u8) -> u64 {
    let mut result = 0u64;
    let mut i = 0;
    while i < bits {
        result = (result << 1) | (v & 1);
        v >>= 1;
        i += 1;
    }
    result
}

/// Reflects all 64 bits of a value.
///
/// # Examples
///
/// ```
/// use abseil::absl_crc::utils::reflect_u64;
///
/// assert_eq!(reflect_u64(0x8000000000000000), 0x0000000000000001);
/// ```
pub const fn reflect_u64(v: u64) -> u64 {
    reflect_bits(v, 64)
}

/// Reflects all 32 bits of a value.
///
/// # Examples
///
/// ```
/// use abseil::absl_crc::utils::reflect_u32;
///
/// assert_eq!(reflect_u32(0x80000000), 0x00000001);
/// ```
pub const fn reflect_u32(v: u32) -> u32 {
    reflect_bits(v as u64, 32) as u32
}

/// Reflects all 16 bits of a value.
///
/// # Examples
///
/// ```
/// use abseil::absl_crc::utils::reflect_u16;
///
/// assert_eq!(reflect_u16(0x8000), 0x0001);
/// ```
pub const fn reflect_u16(v: u16) -> u16 {
    reflect_bits(v as u64, 16) as u16
}

/// Reflects all 8 bits of a value.
///
/// # Examples
///
/// ```
/// use abseil::absl_crc::utils::reflect_u8;
///
/// assert_eq!(reflect_u8(0x80), 0x01);
/// ```
pub const fn reflect_u8(v: u8) -> u8 {
    reflect_bits(v as u64, 8) as u8
}

/// Reverses the byte order of a 32-bit value.
///
/// # Examples
///
/// ```
/// use abseil::absl_crc::utils::swap_bytes_u32;
///
/// assert_eq!(swap_bytes_u32(0x12345678), 0x78563412);
/// ```
pub const fn swap_bytes_u32(v: u32) -> u32 {
    v.swap_bytes()
}

/// Reverses the byte order of a 64-bit value.
///
/// # Examples
///
/// ```
/// use abseil::absl_crc::utils::swap_bytes_u64;
///
/// assert_eq!(swap_bytes_u64(0x0123456789abcdef0), 0xf0cdab8967452301);
/// ```
pub const fn swap_bytes_u64(v: u64) -> u64 {
    v.swap_bytes()
}

/// Reverses the byte order of a 16-bit value.
///
/// # Examples
///
/// ```
/// use abseil::absl_crc::utils::swap_bytes_u16;
///
/// assert_eq!(swap_bytes_u16(0x1234), 0x3412);
/// ```
pub const fn swap_bytes_u16(v: u16) -> u16 {
    v.swap_bytes()
}

/// Converts a CRC value to hexadecimal string representation.
///
/// # Examples
///
/// ```
/// use abseil::absl_crc::utils::crc_to_hex;
///
/// assert_eq!(crc_to_hex(0x12345678u32), "12345678");
/// assert_eq!(crc_to_hex(0x0u32), "00000000");
/// ```
pub fn crc_to_hex(crc: u32) -> alloc::string::String {
    format!("{:08x}", crc)
}

/// Converts a CRC-64 value to hexadecimal string representation.
///
/// # Examples
///
/// ```
/// use abseil::absl_crc::utils::crc64_to_hex;
///
/// assert_eq!(crc64_to_hex(0x0123456789abcdef0), "0123456789abcdef0");
/// ```
pub fn crc64_to_hex(crc: u64) -> alloc::string::String {
    format!("{:016x}", crc)
}

/// Parses a hexadecimal string to a CRC value.
///
/// # Examples
///
/// ```
/// use abseil::absl_crc::utils::hex_to_crc;
///
/// assert_eq!(hex_to_crc("12345678"), Some(0x12345678u32));
/// assert!(hex_to_crc("invalid").is_none());
/// ```
pub fn hex_to_crc(hex: &str) -> Option<u32> {
    u32::from_str_radix(hex, 16).ok()
}

/// Parses a hexadecimal string to a CRC-64 value.
///
/// # Examples
///
/// ```
/// use abseil::absl_crc::utils::hex_to_crc64;
///
/// assert_eq!(hex_to_crc64("0123456789abcdef0"), Some(0x0123456789abcdef0u64));
/// assert!(hex_to_crc64("invalid").is_none());
/// ```
pub fn hex_to_crc64(hex: &str) -> Option<u64> {
    u64::from_str_radix(hex, 16).ok()
}

/// Returns the Hamming distance between two CRC values.
///
/// This is the number of bit positions where the values differ.
///
/// # Examples
///
/// ```
/// use abseil::absl_crc::utils::crc_hamming_distance;
///
/// assert_eq!(crc_hamming_distance(0xFFFFFFFF, 0x00000000), 32);
/// assert_eq!(crc_hamming_distance(0x12345678, 0x12345678), 0);
/// assert_eq!(crc_hamming_distance(0x12345678, 0x72345678), 3);
/// ```
pub fn crc_hamming_distance(a: u32, b: u32) -> u32 {
    (a ^ b).count_ones()
}

/// Returns the Hamming distance between two CRC-64 values.
///
/// # Examples
///
/// ```
/// use abseil::absl_crc::utils::crc64_hamming_distance;
///
/// assert_eq!(crc64_hamming_distance(0xFFFFFFFFFFFFFFFF, 0x0000000000000000), 64);
/// assert_eq!(crc64_hamming_distance(0x0123456789abcdef0, 0x0123456789abcdef0), 0);
/// ```
pub fn crc64_hamming_distance(a: u64, b: u64) -> u32 {
    (a ^ b).count_ones()
}

/// XORs two CRC values.
///
/// # Examples
///
/// ```
/// use abseil::absl_crc::utils::crc_xor;
///
/// let a = 0x12345678u32;
/// let b = 0x9abcdef0u32;
/// let xor = crc_xor(a, b);
/// assert_eq!(xor, 0x89e8d988);
/// ```
pub const fn crc_xor(a: u32, b: u32) -> u32 {
    a ^ b
}

/// XORs two CRC-64 values.
///
/// # Examples
///
/// ```
/// use abseil::absl_crc::utils::crc64_xor;
///
/// let a = 0x0123456789abcdef0u64;
/// let b = 0xfedcba9876543210u64;
/// let xor = crc64_xor(a, b);
/// assert_eq!(xor, 0xffebee1fff9edfe0);
/// ```
pub const fn crc64_xor(a: u64, b: u64) -> u64 {
    a ^ b
}

/// Negates a CRC value (for error injection testing).
///
/// # Examples
///
/// ```
/// use abseil::absl_crc::utils::crc_negate;
///
/// let crc = 0x12345678u32;
/// let negated = crc_negate(crc);
/// assert_eq!(crc ^ negated, 0xFFFFFFFF);
/// ```
pub const fn crc_negate(crc: u32) -> u32 {
    !crc
}

/// Negates a CRC-64 value.
///
/// # Examples
///
/// ```
/// use abseil::absl_crc::utils::crc64_negate;
///
/// let crc = 0x0123456789abcdef0u64;
/// let negated = crc64_negate(crc);
/// assert_eq!(crc ^ negated, 0xFFFFFFFFFFFFFFFF);
/// ```
pub const fn crc64_negate(crc: u64) -> u64 {
    !crc
}

/// Checks if a CRC value is zero.
///
/// # Examples
///
/// ```
/// use abseil::absl_crc::utils::crc_is_zero;
///
/// assert!(crc_is_zero(0));
/// assert!(!crc_is_zero(0x12345678));
/// ```
pub const fn crc_is_zero(crc: u32) -> bool {
    crc == 0
}

/// Checks if a CRC-64 value is zero.
///
/// # Examples
///
/// ```
/// use abseil::absl_crc::utils::crc64_is_zero;
///
/// assert!(crc64_is_zero(0));
/// assert!(!crc64_is_zero(0x0123456789abcdef0));
/// ```
pub const fn crc64_is_zero(crc: u64) -> bool {
    crc == 0
}

/// Returns true if two CRC values match.
///
/// # Examples
///
/// ```
/// use abseil::absl_crc::utils::crc_match;
///
/// assert!(crc_match(0x12345678, 0x12345678));
/// assert!(!crc_match(0x12345678, 0x87654321));
/// ```
pub const fn crc_match(a: u64, b: u64) -> bool {
    a == b
}

/// Returns the bit width of a CRC algorithm.
///
/// # Examples
///
/// ```
/// use abseil::absl_crc::utils::crc_algorithm_width;
/// use abseil::absl_crc::variants::CrcAlgorithm;
///
/// assert_eq!(crc_algorithm_width(CrcAlgorithm::Crc32), 32);
/// assert_eq!(crc_algorithm_width(CrcAlgorithm::Crc64Ecma), 64);
/// assert_eq!(crc_algorithm_width(CrcAlgorithm::Crc8), 8);
/// ```
pub const fn crc_algorithm_width(algorithm: crate::variants::CrcAlgorithm) -> usize {
    algorithm.width()
}

/// Returns true if the algorithm is a 32-bit CRC.
///
/// # Examples
///
/// ```
/// use abseil::absl_crc::utils::is_crc32;
/// use abseil::absl_crc::variants::CrcAlgorithm;
///
/// assert!(is_crc32(CrcAlgorithm::Crc32));
/// assert!(!is_crc32(CrcAlgorithm::Crc64Ecma));
/// assert!(is_crc32(CrcAlgorithm::Crc32C));
/// ```
pub const fn is_crc32(algorithm: crate::variants::CrcAlgorithm) -> bool {
    algorithm.width() == 32
}

/// Returns true if the algorithm is a 64-bit CRC.
///
/// # Examples
///
/// ```
/// use abseil::absl_crc::utils::is_crc64;
/// use abseil::absl_crc::variants::CrcAlgorithm;
///
/// assert!(is_crc64(CrcAlgorithm::Crc64Ecma));
/// assert!(is_crc64(CrcAlgorithm::Crc64Iso));
/// assert!(!is_crc64(CrcAlgorithm::Crc32));
/// ```
pub const fn is_crc64(algorithm: crate::variants::CrcAlgorithm) -> bool {
    algorithm.width() == 64
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::variants::CrcAlgorithm;

    #[test]
    fn test_reflect_bits_u32() {
        let v = 0x12345678u32;
        let reflected = reflect_u32(v);
        assert_ne!(v, reflected);
        // Reflecting twice should give original
        assert_eq!(reflect_u32(reflected), v);
    }

    #[test]
    fn test_reflect_bits_u64() {
        let v = 0x1234567890ABCDEFu64;
        let reflected = reflect_u64(v);
        assert_ne!(v, reflected);
        // Reflecting twice should give original
        assert_eq!(reflect_u64(reflected), v);
    }

    #[test]
    fn test_reflect_bits_u16() {
        let v = 0x1234u16;
        let reflected = reflect_u16(v);
        assert_ne!(v, reflected);
        // Reflecting twice should give original
        assert_eq!(reflect_u16(reflected), v);
    }

    #[test]
    fn test_reflect_bits_u8() {
        let v = 0xABu8;
        let reflected = reflect_u8(v);
        assert_eq!(reflected, 0xD5);
        // Reflecting twice should give original
        assert_eq!(reflect_u8(reflected), v);
    }

    #[test]
    fn test_reflect_bits_const() {
        const V: u64 = 0x8000000000000001;
        const REFLECTED: u64 = reflect_bits(V, 64);
        assert_eq!(REFLECTED, 0x1000000000000080);
    }

    #[test]
    fn test_reflect_bits_partial() {
        // Reflect only 8 bits of a 32-bit value
        let v = 0xABu32;
        let reflected = reflect_bits(v as u64, 8) as u32;
        assert_eq!(reflected, 0xD5);
    }

    #[test]
    fn test_swap_bytes_u32() {
        let v = 0x12345678u32;
        let swapped = swap_bytes_u32(v);
        assert_eq!(swapped, 0x78563412);
    }

    #[test]
    fn test_swap_bytes_u16() {
        let v = 0x1234u16;
        let swapped = swap_bytes_u16(v);
        assert_eq!(swapped, 0x3412);
    }

    #[test]
    fn test_crc_to_hex() {
        let crc = 0x12345678;
        let hex = crc_to_hex(crc);
        assert_eq!(hex, "12345678");
    }

    #[test]
    fn test_crc_to_hex_zero() {
        let crc = 0x00000000;
        let hex = crc_to_hex(crc);
        assert_eq!(hex, "00000000");
    }

    #[test]
    fn test_crc_to_hex_max() {
        let crc = 0xFFFFFFFFu32;
        let hex = crc_to_hex(crc);
        assert_eq!(hex, "ffffffff");
    }

    #[test]
    fn test_hex_to_crc_valid() {
        let hex = "1a2b3c4d";
        let crc = hex_to_crc(hex);
        assert_eq!(crc, Some(0x1a2b3c4d));
    }

    #[test]
    fn test_hex_to_crc_invalid() {
        let hex = "xyz123";
        let crc = hex_to_crc(hex);
        assert_eq!(crc, None);
    }

    #[test]
    fn test_hex_to_crc_empty() {
        let hex = "";
        let crc = hex_to_crc(hex);
        assert_eq!(crc, None);
    }

    #[test]
    fn test_hex_to_crc_roundtrip() {
        let original = 0xABCD1234;
        let hex = crc_to_hex(original);
        let decoded = hex_to_crc(&hex);
        assert_eq!(decoded, Some(original));
    }

    #[test]
    fn test_crc_hamming_distance() {
        let a = 0xFF000000u32;
        let b = 0x00FF0000u32;
        let distance = crc_hamming_distance(a, b);
        // Should have 16 bits different
        assert_eq!(distance, 16);
    }

    #[test]
    fn test_crc_hamming_distance_same() {
        let a = 0x12345678u32;
        let distance = crc_hamming_distance(a, a);
        assert_eq!(distance, 0);
    }

    #[test]
    fn test_crc_hamming_distance_completely_different() {
        let a = 0xFFFFFFFFu32;
        let b = 0x00000000u32;
        let distance = crc_hamming_distance(a, b);
        assert_eq!(distance, 32);
    }

    #[test]
    fn test_crc_xor() {
        let a = 0x12345678u32;
        let b = 0x9ABCDEF0u32;
        let result = crc_xor(a, b);
        assert_eq!(result, 0x89E8D988);
    }

    #[test]
    fn test_crc_xor_self() {
        let a = 0x12345678u32;
        let result = crc_xor(a, a);
        assert_eq!(result, 0);
    }

    #[test]
    fn test_crc_negate() {
        let crc = 0x12345678u32;
        let negated = crc_negate(crc);
        assert_eq!(negated, 0xEDCBA987);
    }

    #[test]
    fn test_crc_negate_zero() {
        let crc = 0x00000000u32;
        let negated = crc_negate(crc);
        assert_eq!(negated, 0xFFFFFFFF);
    }

    #[test]
    fn test_crc_negate_double() {
        let crc = 0x12345678u32;
        let double_negated = crc_negate(crc_negate(crc));
        assert_eq!(double_negated, crc);
    }

    #[test]
    fn test_crc_is_zero_true() {
        assert!(crc_is_zero(0));
    }

    #[test]
    fn test_crc_is_zero_false() {
        assert!(!crc_is_zero(0x12345678));
    }

    #[test]
    fn test_crc_is_zero_false_max() {
        assert!(!crc_is_zero(0xFFFFFFFF));
    }

    #[test]
    fn test_crc_algorithm_width() {
        assert_eq!(crc_algorithm_width(CrcAlgorithm::Crc32), 32);
        assert_eq!(crc_algorithm_width(CrcAlgorithm::Crc64Ecma), 64);
        assert_eq!(crc_algorithm_width(CrcAlgorithm::Crc8), 8);
    }

    #[test]
    fn test_is_crc32() {
        assert!(is_crc32(CrcAlgorithm::Crc32));
        assert!(!is_crc32(CrcAlgorithm::Crc64Ecma));
        assert!(is_crc32(CrcAlgorithm::Crc32C));
    }

    #[test]
    fn test_is_crc64() {
        assert!(is_crc64(CrcAlgorithm::Crc64Ecma));
        assert!(is_crc64(CrcAlgorithm::Crc64Iso));
        assert!(!is_crc64(CrcAlgorithm::Crc32));
    }
}
