#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unused_mut)]
#![allow(unused_comparisons)]
#![allow(unused_doc_comments)]
//! Unit tests for LEB128 encoding/decoding
//!
//! TDD: Test cases for unsigned LEB128 operations

use bitvue_av1_codec::leb128::{decode_uleb128, encode_uleb128, leb128_size};

mod decode {
    use super::*;

    #[test]
    fn decode_zero() {
        let (value, len) = decode_uleb128(&[0x00]).unwrap();
        assert_eq!(value, 0);
        assert_eq!(len, 1);
    }

    #[test]
    fn decode_one() {
        let (value, len) = decode_uleb128(&[0x01]).unwrap();
        assert_eq!(value, 1);
        assert_eq!(len, 1);
    }

    #[test]
    fn decode_max_single_byte() {
        // 127 = 0x7F (no continuation bit)
        let (value, len) = decode_uleb128(&[0x7F]).unwrap();
        assert_eq!(value, 127);
        assert_eq!(len, 1);
    }

    #[test]
    fn decode_128() {
        // 128 = 0x80 0x01
        // 0x80 = 0b10000000 (continuation + 0)
        // 0x01 = 0b00000001 (no continuation + 1)
        // value = 0 + (1 << 7) = 128
        let (value, len) = decode_uleb128(&[0x80, 0x01]).unwrap();
        assert_eq!(value, 128);
        assert_eq!(len, 2);
    }

    #[test]
    fn decode_129() {
        // 129 = 0x81 0x01
        let (value, len) = decode_uleb128(&[0x81, 0x01]).unwrap();
        assert_eq!(value, 129);
        assert_eq!(len, 2);
    }

    #[test]
    fn decode_255() {
        // 255 = 0xFF 0x01
        // 0xFF = 0b11111111 (continuation + 127)
        // 0x01 = 0b00000001 (no continuation + 1)
        // value = 127 + (1 << 7) = 255
        let (value, len) = decode_uleb128(&[0xFF, 0x01]).unwrap();
        assert_eq!(value, 255);
        assert_eq!(len, 2);
    }

    #[test]
    fn decode_256() {
        // 256 = 0x80 0x02
        let (value, len) = decode_uleb128(&[0x80, 0x02]).unwrap();
        assert_eq!(value, 256);
        assert_eq!(len, 2);
    }

    #[test]
    fn decode_16383() {
        // 16383 = 0xFF 0x7F (max 2-byte value)
        let (value, len) = decode_uleb128(&[0xFF, 0x7F]).unwrap();
        assert_eq!(value, 16383);
        assert_eq!(len, 2);
    }

    #[test]
    fn decode_16384() {
        // 16384 = 0x80 0x80 0x01
        let (value, len) = decode_uleb128(&[0x80, 0x80, 0x01]).unwrap();
        assert_eq!(value, 16384);
        assert_eq!(len, 3);
    }

    #[test]
    fn decode_large_value() {
        // 1000000 in LEB128
        let encoded = encode_uleb128(1000000);
        let (value, _) = decode_uleb128(&encoded).unwrap();
        assert_eq!(value, 1000000);
    }

    #[test]
    fn decode_ignores_trailing_data() {
        // Only reads until termination
        let (value, len) = decode_uleb128(&[0x7F, 0xFF, 0xFF, 0xFF]).unwrap();
        assert_eq!(value, 127);
        assert_eq!(len, 1);
    }

    #[test]
    fn decode_empty_returns_error() {
        assert!(decode_uleb128(&[]).is_err());
    }
}

mod encode {
    use super::*;

    #[test]
    fn encode_zero() {
        assert_eq!(encode_uleb128(0), vec![0x00]);
    }

    #[test]
    fn encode_one() {
        assert_eq!(encode_uleb128(1), vec![0x01]);
    }

    #[test]
    fn encode_127() {
        assert_eq!(encode_uleb128(127), vec![0x7F]);
    }

    #[test]
    fn encode_128() {
        assert_eq!(encode_uleb128(128), vec![0x80, 0x01]);
    }

    #[test]
    fn encode_255() {
        assert_eq!(encode_uleb128(255), vec![0xFF, 0x01]);
    }

    #[test]
    fn encode_256() {
        assert_eq!(encode_uleb128(256), vec![0x80, 0x02]);
    }

    #[test]
    fn encode_16383() {
        assert_eq!(encode_uleb128(16383), vec![0xFF, 0x7F]);
    }

    #[test]
    fn encode_16384() {
        assert_eq!(encode_uleb128(16384), vec![0x80, 0x80, 0x01]);
    }
}

mod roundtrip {
    use super::*;

    #[test]
    fn roundtrip_powers_of_two() {
        for i in 0..56 {
            let value = 1u64 << i;
            let encoded = encode_uleb128(value);
            let (decoded, _) = decode_uleb128(&encoded).unwrap();
            assert_eq!(decoded, value, "Failed for 2^{}", i);
        }
    }

    #[test]
    fn roundtrip_boundary_values() {
        let values = [
            0,
            1,
            126,
            127,
            128,
            129,
            254,
            255,
            256,
            16382,
            16383,
            16384,
            16385,
            u32::MAX as u64,
        ];

        for value in values {
            let encoded = encode_uleb128(value);
            let (decoded, len) = decode_uleb128(&encoded).unwrap();
            assert_eq!(decoded, value);
            assert_eq!(len, encoded.len());
        }
    }
}

mod size {
    use super::*;

    #[test]
    fn size_single_byte_values() {
        assert_eq!(leb128_size(0), 1);
        assert_eq!(leb128_size(1), 1);
        assert_eq!(leb128_size(127), 1);
    }

    #[test]
    fn size_two_byte_values() {
        assert_eq!(leb128_size(128), 2);
        assert_eq!(leb128_size(255), 2);
        assert_eq!(leb128_size(16383), 2);
    }

    #[test]
    fn size_three_byte_values() {
        assert_eq!(leb128_size(16384), 3);
        assert_eq!(leb128_size(2097151), 3); // 2^21 - 1
    }

    #[test]
    fn size_matches_encoded_length() {
        let values = [0, 1, 127, 128, 255, 16383, 16384, 1000000];
        for value in values {
            assert_eq!(
                leb128_size(value),
                encode_uleb128(value).len(),
                "Size mismatch for {}",
                value
            );
        }
    }
}
