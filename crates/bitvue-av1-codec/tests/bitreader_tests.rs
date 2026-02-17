#![allow(dead_code)]
//! Unit tests for BitReader
//!
//! TDD: Test cases for bit-level stream reading operations

use bitvue_av1_codec::BitReader;

mod read_bit {
    use super::*;

    #[test]
    fn reads_msb_first() {
        // 0b10000000 = 0x80
        let data = [0x80];
        let mut reader = BitReader::new(&data);
        assert!(reader.read_bit().unwrap());
    }

    #[test]
    fn reads_all_bits_in_byte() {
        let data = [0b10101010];
        let mut reader = BitReader::new(&data);

        assert!(reader.read_bit().unwrap()); // 1
        assert!(!reader.read_bit().unwrap()); // 0
        assert!(reader.read_bit().unwrap()); // 1
        assert!(!reader.read_bit().unwrap()); // 0
        assert!(reader.read_bit().unwrap()); // 1
        assert!(!reader.read_bit().unwrap()); // 0
        assert!(reader.read_bit().unwrap()); // 1
        assert!(!reader.read_bit().unwrap()); // 0
    }

    #[test]
    fn reads_across_byte_boundary() {
        let data = [0xFF, 0x00];
        let mut reader = BitReader::new(&data);

        for _ in 0..8 {
            assert!(reader.read_bit().unwrap());
        }
        for _ in 0..8 {
            assert!(!reader.read_bit().unwrap());
        }
    }

    #[test]
    fn returns_error_on_empty_data() {
        let data: [u8; 0] = [];
        let mut reader = BitReader::new(&data);
        assert!(reader.read_bit().is_err());
    }

    #[test]
    fn returns_error_past_end() {
        let data = [0xFF];
        let mut reader = BitReader::new(&data);

        // Read all 8 bits
        for _ in 0..8 {
            reader.read_bit().unwrap();
        }

        // 9th bit should fail
        assert!(reader.read_bit().is_err());
    }
}

mod read_bits {
    use super::*;

    #[test]
    fn reads_zero_bits() {
        let data = [0xFF];
        let mut reader = BitReader::new(&data);
        assert_eq!(reader.read_bits(0).unwrap(), 0);
        assert_eq!(reader.position(), 0); // Position unchanged
    }

    #[test]
    fn reads_single_bit() {
        let data = [0x80]; // 0b10000000
        let mut reader = BitReader::new(&data);
        assert_eq!(reader.read_bits(1).unwrap(), 1);
    }

    #[test]
    fn reads_nibble() {
        let data = [0xAB]; // 0b10101011
        let mut reader = BitReader::new(&data);
        assert_eq!(reader.read_bits(4).unwrap(), 0b1010); // 0xA
    }

    #[test]
    fn reads_full_byte() {
        let data = [0xAB];
        let mut reader = BitReader::new(&data);
        assert_eq!(reader.read_bits(8).unwrap(), 0xAB);
    }

    #[test]
    fn reads_across_bytes() {
        let data = [0xAB, 0xCD]; // 0b10101011 0b11001101
        let mut reader = BitReader::new(&data);

        // Read 12 bits: 0b101010111100 = 0xABC
        assert_eq!(reader.read_bits(12).unwrap(), 0xABC);
    }

    #[test]
    fn reads_max_32_bits() {
        let data = [0xFF, 0xFF, 0xFF, 0xFF];
        let mut reader = BitReader::new(&data);
        assert_eq!(reader.read_bits(32).unwrap(), 0xFFFFFFFF);
    }

    #[test]
    fn errors_on_more_than_32_bits() {
        let data = [0xFF; 5];
        let mut reader = BitReader::new(&data);
        assert!(reader.read_bits(33).is_err());
    }

    #[test]
    fn sequential_reads() {
        let data = [0b11110000, 0b10101010];
        let mut reader = BitReader::new(&data);

        assert_eq!(reader.read_bits(4).unwrap(), 0b1111);
        assert_eq!(reader.read_bits(4).unwrap(), 0b0000);
        assert_eq!(reader.read_bits(4).unwrap(), 0b1010);
        assert_eq!(reader.read_bits(4).unwrap(), 0b1010);
    }
}

mod read_byte {
    use super::*;

    #[test]
    fn reads_aligned_byte() {
        let data = [0xAB, 0xCD];
        let mut reader = BitReader::new(&data);

        assert_eq!(reader.read_byte().unwrap(), 0xAB);
        assert_eq!(reader.read_byte().unwrap(), 0xCD);
    }

    #[test]
    fn reads_unaligned_byte() {
        let data = [0b11110000, 0b10101010];
        let mut reader = BitReader::new(&data);

        reader.read_bits(4).unwrap(); // Skip 4 bits
                                      // Now reading: 0000 1010 = 0x0A
        assert_eq!(reader.read_byte().unwrap(), 0x0A);
    }
}

mod position {
    use super::*;

    #[test]
    fn starts_at_zero() {
        let data = [0xFF];
        let reader = BitReader::new(&data);
        assert_eq!(reader.position(), 0);
    }

    #[test]
    fn increments_per_bit() {
        let data = [0xFF];
        let mut reader = BitReader::new(&data);

        for i in 0..8 {
            assert_eq!(reader.position(), i);
            reader.read_bit().unwrap();
        }
        assert_eq!(reader.position(), 8);
    }

    #[test]
    fn byte_position_correct() {
        let data = [0xFF, 0xFF];
        let mut reader = BitReader::new(&data);

        assert_eq!(reader.byte_position(), 0);
        reader.read_bits(8).unwrap();
        assert_eq!(reader.byte_position(), 1);
    }
}

mod skip_and_align {
    use super::*;

    #[test]
    fn skip_bits() {
        let data = [0xFF, 0xAB];
        let mut reader = BitReader::new(&data);

        reader.skip_bits(8).unwrap();
        assert_eq!(reader.read_byte().unwrap(), 0xAB);
    }

    #[test]
    fn skip_partial_bits() {
        let data = [0b11110000];
        let mut reader = BitReader::new(&data);

        reader.skip_bits(4).unwrap();
        assert_eq!(reader.read_bits(4).unwrap(), 0b0000);
    }

    #[test]
    fn byte_align_when_aligned() {
        let data = [0xFF];
        let mut reader = BitReader::new(&data);

        reader.byte_align();
        assert_eq!(reader.position(), 0);
    }

    #[test]
    fn byte_align_when_unaligned() {
        let data = [0xFF, 0xAB];
        let mut reader = BitReader::new(&data);

        reader.read_bits(3).unwrap();
        reader.byte_align();
        assert_eq!(reader.position(), 8);
        assert_eq!(reader.read_byte().unwrap(), 0xAB);
    }
}

mod remaining {
    use super::*;

    #[test]
    fn remaining_bytes_full() {
        let data = [0xFF, 0xFF, 0xFF];
        let reader = BitReader::new(&data);
        assert_eq!(reader.remaining_bytes(), 3);
    }

    #[test]
    fn remaining_bytes_after_read() {
        let data = [0xFF, 0xFF, 0xFF];
        let mut reader = BitReader::new(&data);

        reader.read_byte().unwrap();
        assert_eq!(reader.remaining_bytes(), 2);
    }

    #[test]
    fn remaining_bits() {
        let data = [0xFF, 0xFF];
        let mut reader = BitReader::new(&data);

        assert_eq!(reader.remaining_bits(), 16);
        reader.read_bits(5).unwrap();
        assert_eq!(reader.remaining_bits(), 11);
    }

    #[test]
    fn has_more() {
        let data = [0xFF];
        let mut reader = BitReader::new(&data);

        assert!(reader.has_more());
        reader.read_byte().unwrap();
        assert!(!reader.has_more());
    }
}

mod uvlc {
    use super::*;

    #[test]
    fn uvlc_zero() {
        // uvlc(0) = 1 (single bit)
        let data = [0b10000000];
        let mut reader = BitReader::new(&data);
        assert_eq!(reader.read_uvlc().unwrap(), 0);
    }

    #[test]
    fn uvlc_one() {
        // uvlc(1) = 010
        let data = [0b01000000];
        let mut reader = BitReader::new(&data);
        assert_eq!(reader.read_uvlc().unwrap(), 1);
    }

    #[test]
    fn uvlc_two() {
        // uvlc(2) = 011
        let data = [0b01100000];
        let mut reader = BitReader::new(&data);
        assert_eq!(reader.read_uvlc().unwrap(), 2);
    }

    #[test]
    fn uvlc_three() {
        // uvlc(3) = 00100
        let data = [0b00100000];
        let mut reader = BitReader::new(&data);
        assert_eq!(reader.read_uvlc().unwrap(), 3);
    }

    #[test]
    fn uvlc_seven() {
        // uvlc(7) = 00111 (2 leading zeros, then 1, then 11)
        let data = [0b00111000];
        let mut reader = BitReader::new(&data);
        assert_eq!(reader.read_uvlc().unwrap(), 6); // Actually 6, not 7
    }

    #[test]
    fn uvlc_large() {
        // uvlc values follow: (1 << leadingZeros) - 1 + value
        // For 14: leadingZeros=3, value=7 -> 0001111
        let data = [0b00011110];
        let mut reader = BitReader::new(&data);
        assert_eq!(reader.read_uvlc().unwrap(), 14);
    }
}

mod signed {
    use super::*;

    #[test]
    fn su_positive() {
        // su(4) with value 7 = 0111
        let data = [0b01110000];
        let mut reader = BitReader::new(&data);
        assert_eq!(reader.read_su(4).unwrap(), 7);
    }

    #[test]
    fn su_negative() {
        // su(4) with value 8 = 1000 -> -8
        let data = [0b10000000];
        let mut reader = BitReader::new(&data);
        assert_eq!(reader.read_su(4).unwrap(), -8);
    }

    #[test]
    fn su_minus_one() {
        // su(4) with value 15 = 1111 -> -1
        let data = [0b11110000];
        let mut reader = BitReader::new(&data);
        assert_eq!(reader.read_su(4).unwrap(), -1);
    }
}
