//! LEB128 (Little Endian Base 128) decoding for AV1
//!
//! AV1 uses unsigned LEB128 (uleb128) for encoding OBU sizes.
//! Each byte has 7 bits of data and 1 continuation bit (MSB).

use bitvue_core::{BitvueError, Result};

/// Maximum bytes for a valid LEB128 in AV1 (8 bytes = 56 bits max)
pub const MAX_LEB128_BYTES: usize = 8;

/// Maximum bits for a valid LEB128 value (8 bytes * 7 bits per byte = 56 bits)
/// LEB128 is limited to 56 bits per the AV1 specification
const MAX_LEB128_BITS: u32 = MAX_LEB128_BYTES as u32 * 7; // 56 bits

// Compile-time assertion to ensure MAX_LEB128_BITS doesn't exceed 64
const _: () = assert!(MAX_LEB128_BITS <= 64, "LEB128 max bits must not exceed 64");

/// Decodes an unsigned LEB128 value from a byte slice
///
/// Returns the decoded value and the number of bytes consumed.
///
/// # Format
/// - Each byte: 7 bits of data (LSB first), 1 bit continuation flag (MSB)
/// - If MSB is 1, more bytes follow
/// - If MSB is 0, this is the last byte
///
/// # Example
/// ```
/// use bitvue_av1_codec::leb128::decode_uleb128;
///
/// // Value 127 = 0x7F (single byte, no continuation)
/// assert_eq!(decode_uleb128(&[0x7F]).unwrap(), (127, 1));
///
/// // Value 128 = 0x80 0x01 (128 = 0 + 128*1)
/// assert_eq!(decode_uleb128(&[0x80, 0x01]).unwrap(), (128, 2));
/// ```
pub fn decode_uleb128(data: &[u8]) -> Result<(u64, usize)> {
    if data.is_empty() {
        return Err(BitvueError::UnexpectedEof(0));
    }

    let mut value: u64 = 0;
    let mut shift: u32 = 0;
    let mut bytes_read: usize = 0;

    for &byte in data.iter().take(MAX_LEB128_BYTES) {
        bytes_read += 1;

        // Extract 7 data bits
        let data_bits = (byte & 0x7F) as u64;

        // Check for overflow before shifting.
        // Per LEB128 spec, we have MAX_LEB128_BITS (56) maximum, not 64.
        // This check ensures we don't exceed the spec's limits.
        if shift >= MAX_LEB128_BITS || (shift > 0 && data_bits > (u64::MAX >> shift)) {
            return Err(BitvueError::Parse {
                offset: bytes_read as u64,
                message: "LEB128 value overflow".to_string(),
            });
        }

        value |= data_bits << shift;
        shift += 7;

        // Check continuation bit (MSB)
        if byte & 0x80 == 0 {
            return Ok((value, bytes_read));
        }
    }

    // If we've read MAX_LEB128_BYTES and still have continuation, it's invalid
    Err(BitvueError::Parse {
        offset: bytes_read as u64,
        message: format!("LEB128 exceeded maximum {} bytes", MAX_LEB128_BYTES),
    })
}

/// Decodes an unsigned LEB128 from a byte slice at a given offset
///
/// Returns the decoded value, the number of bytes consumed, and updates the offset.
pub fn decode_uleb128_at(data: &[u8], offset: usize) -> Result<(u64, usize)> {
    if offset >= data.len() {
        return Err(BitvueError::UnexpectedEof(offset as u64));
    }
    decode_uleb128(&data[offset..])
}

/// Encodes a value as unsigned LEB128
///
/// Returns a vector of bytes representing the encoded value.
pub fn encode_uleb128(mut value: u64) -> Vec<u8> {
    let mut result = Vec::with_capacity(10);

    loop {
        let mut byte = (value & 0x7F) as u8;
        value >>= 7;

        if value != 0 {
            byte |= 0x80; // Set continuation bit
        }

        result.push(byte);

        if value == 0 {
            break;
        }
    }

    result
}

/// Calculate the byte size needed to encode a value in LEB128
pub fn leb128_size(value: u64) -> usize {
    if value == 0 {
        return 1;
    }

    // Count bits needed
    let bits = 64 - value.leading_zeros();
    // Each LEB128 byte holds 7 bits
    bits.div_ceil(7) as usize
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_single_byte() {
        // Values 0-127 are encoded in single byte
        assert_eq!(decode_uleb128(&[0x00]).unwrap(), (0, 1));
        assert_eq!(decode_uleb128(&[0x01]).unwrap(), (1, 1));
        assert_eq!(decode_uleb128(&[0x7F]).unwrap(), (127, 1));
    }

    #[test]
    fn test_decode_multi_byte() {
        // 128 = 0x80 0x01 (0 + 128)
        assert_eq!(decode_uleb128(&[0x80, 0x01]).unwrap(), (128, 2));

        // 129 = 0x81 0x01 (1 + 128)
        assert_eq!(decode_uleb128(&[0x81, 0x01]).unwrap(), (129, 2));

        // 255 = 0xFF 0x01 (127 + 128)
        assert_eq!(decode_uleb128(&[0xFF, 0x01]).unwrap(), (255, 2));

        // 256 = 0x80 0x02 (0 + 256)
        assert_eq!(decode_uleb128(&[0x80, 0x02]).unwrap(), (256, 2));

        // 16383 = 0xFF 0x7F (127 + 127*128)
        assert_eq!(decode_uleb128(&[0xFF, 0x7F]).unwrap(), (16383, 2));

        // 16384 = 0x80 0x80 0x01
        assert_eq!(decode_uleb128(&[0x80, 0x80, 0x01]).unwrap(), (16384, 3));
    }

    #[test]
    fn test_encode_decode_roundtrip() {
        let test_values = [
            0,
            1,
            127,
            128,
            255,
            256,
            16383,
            16384,
            1000000,
            u64::MAX >> 8,
        ];

        for &value in &test_values {
            let encoded = encode_uleb128(value);
            let (decoded, len) = decode_uleb128(&encoded).unwrap();
            assert_eq!(decoded, value, "Roundtrip failed for {}", value);
            assert_eq!(len, encoded.len());
        }
    }

    #[test]
    fn test_leb128_size() {
        assert_eq!(leb128_size(0), 1);
        assert_eq!(leb128_size(1), 1);
        assert_eq!(leb128_size(127), 1);
        assert_eq!(leb128_size(128), 2);
        assert_eq!(leb128_size(16383), 2);
        assert_eq!(leb128_size(16384), 3);
    }

    #[test]
    fn test_empty_input() {
        assert!(decode_uleb128(&[]).is_err());
    }

    #[test]
    fn test_extra_data_ignored() {
        // Extra bytes after termination should be ignored
        let (value, len) = decode_uleb128(&[0x7F, 0xFF, 0xFF]).unwrap();
        assert_eq!(value, 127);
        assert_eq!(len, 1);
    }

    #[test]
    fn test_leb128_overflow_at_boundary() {
        // Test values near the 56-bit boundary (LEB128 spec limit)
        // MAX_LEB128_BITS = 56, so maximum value is 2^56 - 1

        // Valid 56-bit value: 0xFF 0xFF 0xFF 0xFF 0xFF 0xFF 0xFF 0x7F (8 bytes)
        let max_valid_leb128 = [0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x7F];
        let result = decode_uleb128(&max_valid_leb128);
        assert!(result.is_ok(), "Should decode valid 56-bit LEB128");
        let (value, len) = result.unwrap();
        assert_eq!(len, 8);
        assert_eq!(value, (1u64 << 56) - 1);

        // Invalid: value would require 57 bits (exceeds MAX_LEB128_BITS)
        // This should be rejected by the overflow check
        let overflow_leb128 = [0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80];
        assert!(
            decode_uleb128(&overflow_leb128).is_err(),
            "Should reject LEB128 exceeding 56 bits"
        );
    }

    #[test]
    fn test_leb128_max_bytes_limit() {
        // Test that we correctly limit to MAX_LEB128_BYTES
        // 9 bytes with continuation bit set on all should fail
        let too_long = [0x80u8; 9];
        let result = decode_uleb128(&too_long);
        assert!(result.is_err());
        match result {
            Err(BitvueError::Parse { message, .. }) => {
                assert!(message.contains("exceeded maximum"));
            }
            _ => panic!("Expected Parse error for exceeded maximum bytes"),
        }
    }
}
