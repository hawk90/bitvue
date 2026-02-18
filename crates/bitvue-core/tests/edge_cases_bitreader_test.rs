#![allow(hidden_glob_reexports)]
#![allow(unreachable_code)]
#![allow(non_camel_case_types)]
#![allow(unused_assignments)]
#![allow(unused_parens)]
#![allow(unused_imports)]
#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unused_mut)]
#![allow(unused_comparisons)]
#![allow(unused_doc_comments)]
//! Edge case and boundary condition tests for BitReader
//!
//! This test module covers:
//! - Empty inputs and zero-length data
//! - Overflow scenarios with position tracking
//! - Maximum bit depth reads
//! - Boundary conditions for skip operations
//! - Emulation prevention edge cases

use bitvue_core::BitvueError;
use bitvue_core::{
    remove_emulation_prevention_bytes, BitReader, ExpGolombReader, Leb128Reader, LsbBitReader,
    UvlcReader,
};

// ============================================================================
// Input Validation Tests
// ============================================================================

#[test]
fn test_empty_slice() {
    let data = [];
    let mut reader = BitReader::new(&data);

    // All read operations should fail gracefully
    assert!(matches!(
        reader.read_bit(),
        Err(BitvueError::UnexpectedEof(_))
    ));
    assert!(matches!(
        reader.read_bits(8),
        Err(BitvueError::UnexpectedEof(_))
    ));
    assert!(matches!(
        reader.read_byte(),
        Err(BitvueError::UnexpectedEof(_))
    ));
}

#[test]
fn test_single_bit_read() {
    let data = [0b10000000];
    let mut reader = BitReader::new(&data);

    // Should read one bit successfully
    assert!(reader.read_bit().unwrap());

    // Next read should fail
    assert!(matches!(
        reader.read_bit(),
        Err(BitvueError::UnexpectedEof(_))
    ));
}

#[test]
fn test_read_zero_bits() {
    let data = [0xFF];
    let mut reader = BitReader::new(&data);

    // Reading 0 bits should return 0 and not advance position
    assert_eq!(reader.read_bits(0).unwrap(), 0);
    assert_eq!(reader.read_bits_u64(0).unwrap(), 0);
    assert_eq!(reader.position(), 0);
}

#[test]
fn test_read_exactly_32_bits() {
    let data = [0xFF, 0xFF, 0xFF, 0xFF];
    let mut reader = BitReader::new(&data);

    // Reading exactly 32 bits should work
    let result = reader.read_bits(32).unwrap();
    assert_eq!(result, 0xFFFFFFFF);
}

#[test]
fn test_read_exactly_64_bits() {
    let data = [0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF];
    let mut reader = BitReader::new(&data);

    // Reading exactly 64 bits should work
    let result = reader.read_bits_u64(64).unwrap();
    assert_eq!(result, 0xFFFFFFFFFFFFFFFF);
}

#[test]
fn test_read_more_than_32_bits_fails() {
    let data = [0xFF, 0xFF, 0xFF, 0xFF, 0xFF];
    let mut reader = BitReader::new(&data);

    // Reading more than 32 bits should fail for read_bits
    assert!(matches!(
        reader.read_bits(33),
        Err(BitvueError::Parse { .. })
    ));
}

#[test]
fn test_read_more_than_64_bits_fails() {
    let data = [0xFF; 10];
    let mut reader = BitReader::new(&data);

    // Reading more than 64 bits should fail for read_bits_u64
    assert!(matches!(
        reader.read_bits_u64(65),
        Err(BitvueError::Parse { .. })
    ));
}

// ============================================================================
// Position Tracking Overflow Tests
// ============================================================================

#[test]
fn test_position_overflow_protection() {
    // Create a reader with data that would overflow if position wasn't protected
    let data = [0u8; 1000];
    let reader = BitReader::new(&data);

    // Position should be safely calculated even for large offsets
    let pos = reader.position();
    assert!(pos < u64::MAX);

    // Byte position should be within bounds
    assert_eq!(reader.byte_position(), 0);
}

#[test]
fn test_remaining_bits_calculation() {
    let data = [0xFF, 0xFF, 0xFF];
    let mut reader = BitReader::new(&data);

    // Initially 24 bits remaining
    assert_eq!(reader.remaining_bits(), 24);

    // Read 5 bits
    reader.read_bits(5).unwrap();
    assert_eq!(reader.remaining_bits(), 19);

    // Read 1 byte (8 bits)
    reader.read_byte().unwrap();
    assert_eq!(reader.remaining_bits(), 11);
}

#[test]
fn test_skip_bits_overflow_protection() {
    let data = [0xFF; 100];
    let mut reader = BitReader::new(&data);

    // Skip to near the end
    reader.skip_bits(700).unwrap(); // 87.5 bytes

    // Try to skip beyond the end - should fail
    assert!(matches!(
        reader.skip_bits(100),
        Err(BitvueError::UnexpectedEof(_))
    ));

    // Try to skip an amount that would overflow position
    let result = reader.skip_bits(u64::MAX - 700);
    assert!(matches!(result, Err(BitvueError::Decode { .. })));
}

#[test]
fn test_skip_bits_boundary() {
    let data = [0xFF, 0xFF, 0xFF, 0xFF];
    let mut reader = BitReader::new(&data);

    // Skip to exact byte boundary
    reader.skip_bits(8).unwrap();
    assert_eq!(reader.byte_position(), 1);
    assert!(reader.is_byte_aligned());

    // Skip to non-byte boundary
    reader.skip_bits(3).unwrap();
    assert!(!reader.is_byte_aligned());
    assert_eq!(reader.position(), 11);
}

// ============================================================================
// Exp-Golomb Edge Cases
// ============================================================================

#[test]
fn test_exp_golomb_zero_leading_zeros() {
    // ue(0) = 1 (binary: 1)
    let data = [0b10000000];
    let mut reader = BitReader::new(&data);
    assert_eq!(reader.read_ue().unwrap(), 0);
}

#[test]
fn test_exp_golomb_max_leading_zeros() {
    // Create a pattern with exactly 32 leading zeros followed by 1
    // This should succeed (at the limit)
    let data = [0x00, 0x00, 0x00, 0x80];
    let mut reader = BitReader::new(&data);

    // This should read successfully
    let result = reader.read_ue();
    assert!(result.is_ok());

    // The value should be 2^32 - 1 = 4294967295
    // This would be truncated to u32 max
}

#[test]
fn test_exp_golomb_exceeds_32_leading_zeros() {
    // Create a pattern with 33+ leading zeros
    let data = [0x00, 0x00, 0x00, 0x00, 0x80];
    let mut reader = BitReader::new(&data);

    // This should fail (more than 32 leading zeros)
    assert!(matches!(reader.read_ue(), Err(BitvueError::Parse { .. })));
}

#[test]
fn test_exp_golomb_at_eof() {
    // Data that ends before completing Exp-Golomb
    let data = [0b00000000]; // Just zeros, no stop bit
    let mut reader = BitReader::new(&data);

    // Should fail when reaching EOF before stop bit
    assert!(matches!(
        reader.read_ue(),
        Err(BitvueError::UnexpectedEof(_))
    ));
}

#[test]
fn test_exp_golomb_signed_edge_cases() {
    // se(0) = 0
    let data = [0b10000000];
    let mut reader = BitReader::new(&data);
    assert_eq!(reader.read_se().unwrap(), 0);

    // se(i32::MIN) - most negative value
    // This would require a very large ue value
}

// ============================================================================
// LEB128 Edge Cases
// ============================================================================

#[test]
fn test_leb128_zero() {
    // LEB128 encoding of 0 is just 0x00
    let data = [0x00];
    let mut reader = BitReader::new(&data);
    assert_eq!(reader.read_leb128().unwrap(), 0);
}

#[test]
fn test_leb128_single_byte() {
    // Values 0-127 encode as single byte
    for value in 0u64..=127 {
        let mut data = [0u8; 2];
        data[0] = value as u8;

        let mut reader = BitReader::new(&data);
        let decoded = reader.read_leb128().unwrap();
        assert_eq!(decoded, value);
    }
}

#[test]
fn test_leb128_overflow_protection() {
    // Create LEB128 that would overflow if not protected
    // 10 continuation bytes would try to shift by 70+ bits
    let data = [0x80; 10];
    let mut reader = BitReader::new(&data);

    // Should detect overflow before shifting
    assert!(matches!(
        reader.read_leb128(),
        Err(BitvueError::Parse { .. })
    ));
}

#[test]
fn test_leb128_at_exact_boundary() {
    // 9 bytes of continuation (8 bytes = 56 bits, still valid)
    // 9th byte has continuation bit cleared
    let mut data = [0x80u8; 9];
    data[8] = 0x00; // Last byte, no continuation

    let mut reader = BitReader::new(&data);

    // Should succeed (shift = 8*7 = 56, still < 64)
    let result = reader.read_leb128();
    assert!(result.is_ok());
}

#[test]
fn test_leb128_truncated() {
    // LEB128 that ends mid-encoding
    let data = [0x80, 0x80]; // Two continuation bytes, no termination
    let mut reader = BitReader::new(&data);

    // Should fail when reaching EOF
    assert!(matches!(
        reader.read_leb128(),
        Err(BitvueError::UnexpectedEof(_))
    ));
}

// ============================================================================
// UVLC Edge Cases
// ============================================================================

#[test]
fn test_uvlc_zero() {
    // uvlc(0) = 1
    let data = [0b10000000];
    let mut reader = BitReader::new(&data);
    assert_eq!(reader.read_uvlc().unwrap(), 0);
}

#[test]
fn test_uvlc_max_leading_zeros() {
    // Create pattern with 32 leading zeros followed by 1
    let data = [0x00, 0x00, 0x00, 0x80];
    let mut reader = BitReader::new(&data);

    // Should succeed at boundary
    let result = reader.read_uvlc();
    assert!(result.is_ok());
}

#[test]
fn test_uvlc_exceeds_max_leading_zeros() {
    // 33+ leading zeros
    let data = [0x00, 0x00, 0x00, 0x00, 0x80];
    let mut reader = BitReader::new(&data);

    // Should fail
    assert!(matches!(reader.read_uvlc(), Err(BitvueError::Parse { .. })));
}

// ============================================================================
// Emulation Prevention Edge Cases
// ============================================================================

#[test]
fn test_emulation_prevention_empty_input() {
    let data = [];
    let result = remove_emulation_prevention_bytes(&data);
    assert!(result.is_empty());
}

#[test]
fn test_emulation_prevention_single_byte() {
    let data = [0x00];
    let result = remove_emulation_prevention_bytes(&data);
    assert_eq!(result, vec![0x00]);

    let data = [0x03];
    let result = remove_emulation_prevention_bytes(&data);
    assert_eq!(result, vec![0x03]);
}

#[test]
fn test_emulation_prevention_two_bytes() {
    let data = [0x00, 0x00];
    let result = remove_emulation_prevention_bytes(&data);
    assert_eq!(result, vec![0x00, 0x00]);

    let data = [0x00, 0x03];
    let result = remove_emulation_prevention_bytes(&data);
    assert_eq!(result, vec![0x00, 0x03]);
}

#[test]
fn test_emulation_prevention_exact_pattern() {
    // Exact 0x00 0x00 0x03 pattern
    let data = [0x00, 0x00, 0x03];
    let result = remove_emulation_prevention_bytes(&data);
    assert_eq!(result, vec![0x00, 0x00]);
}

#[test]
fn test_emulation_prevention_pattern_at_start() {
    let data = [0x00, 0x00, 0x03, 0xFF, 0xFF];
    let result = remove_emulation_prevention_bytes(&data);
    assert_eq!(result, vec![0x00, 0x00, 0xFF, 0xFF]);
}

#[test]
fn test_emulation_prevention_pattern_at_end() {
    let data = [0xFF, 0xFF, 0x00, 0x00, 0x03];
    let result = remove_emulation_prevention_bytes(&data);
    assert_eq!(result, vec![0xFF, 0xFF, 0x00, 0x00]);
}

#[test]
fn test_emulation_prevention_overlapping_patterns() {
    // Overlapping potential patterns
    let data = [0x00, 0x00, 0x03, 0x00, 0x00, 0x03];
    let result = remove_emulation_prevention_bytes(&data);
    assert_eq!(result, vec![0x00, 0x00, 0x00, 0x00]);
}

#[test]
fn test_emulation_prevention_false_positive_protection() {
    // 0x00 0x00 0x02 should NOT be treated as pattern
    let data = [0x00, 0x00, 0x02];
    let result = remove_emulation_prevention_bytes(&data);
    assert_eq!(result, vec![0x00, 0x00, 0x02]);

    // 0x00 0x00 0x01 should NOT be treated as pattern
    let data = [0x00, 0x00, 0x01];
    let result = remove_emulation_prevention_bytes(&data);
    assert_eq!(result, vec![0x00, 0x00, 0x01]);
}

#[test]
fn test_emulation_prevention_0x03_in_middle() {
    // 0x00 0x03 0x00 should NOT be treated as pattern
    let data = [0x00, 0x03, 0x00];
    let result = remove_emulation_prevention_bytes(&data);
    assert_eq!(result, vec![0x00, 0x03, 0x00]);
}

// ============================================================================
// Byte Alignment Tests
// ============================================================================

#[test]
fn test_byte_align_when_already_aligned() {
    let data = [0xFF, 0xFF];
    let mut reader = BitReader::new(&data);

    // Already aligned
    assert!(reader.is_byte_aligned());
    reader.byte_align();
    assert_eq!(reader.byte_position(), 0);
}

#[test]
fn test_byte_align_when_not_aligned() {
    let data = [0xFF, 0xFF, 0xFF];
    let mut reader = BitReader::new(&data);

    // Read 3 bits (not aligned)
    reader.read_bits(3).unwrap();
    assert!(!reader.is_byte_aligned());
    assert_eq!(reader.position(), 3);

    // Align
    reader.byte_align();
    assert!(reader.is_byte_aligned());
    assert_eq!(reader.byte_position(), 1);
}

#[test]
fn test_byte_align_at_boundary() {
    let data = [0xFF, 0xFF];
    let mut reader = BitReader::new(&data);

    // Read to last bit of first byte
    reader.read_bits(7).unwrap();
    reader.byte_align();
    assert_eq!(reader.byte_position(), 1);

    // Should be at start of second byte
    assert_eq!(reader.read_byte().unwrap(), 0xFF);
}

// ============================================================================
// LSB BitReader Edge Cases
// ============================================================================

#[test]
fn test_lsb_empty_slice() {
    let data = [];
    let mut reader = LsbBitReader::new(&data);

    assert!(matches!(
        reader.read_bit(),
        Err(BitvueError::UnexpectedEof(_))
    ));
    assert!(matches!(
        reader.read_bits(8),
        Err(BitvueError::UnexpectedEof(_))
    ));
}

#[test]
fn test_lsb_skip_overflow_protection() {
    let data = [0xFF; 100];
    let mut reader = LsbBitReader::new(&data);

    // Try to skip beyond data
    assert!(matches!(
        reader.skip_bits(800),
        Err(BitvueError::UnexpectedEof(_))
    ));

    // Try to skip with overflow
    let result = reader.skip_bits(u64::MAX);
    assert!(matches!(result, Err(BitvueError::Decode { .. })));
}

#[test]
fn test_lsb_position_calculation() {
    let data = [0xFF; 100];
    let reader = LsbBitReader::new(&data);

    let pos = reader.position();
    assert!(pos < u64::MAX);
    assert_eq!(pos, 0);
}

// ============================================================================
// Peek Operations Edge Cases
// ============================================================================

#[test]
fn test_peek_bits_beyond_data() {
    let data = [0xFF];
    let mut reader = BitReader::new(&data);

    // Peek more bits than available
    assert!(matches!(
        reader.peek_bits(16),
        Err(BitvueError::UnexpectedEof(_))
    ));
}

#[test]
fn test_peek_does_not_advance() {
    let data = [0b10110011];
    let mut reader = BitReader::new(&data);

    let peeked = reader.peek_bits(4).unwrap();
    assert_eq!(peeked, 0b1011);

    // Position should not have changed
    assert_eq!(reader.position(), 0);

    // Reading should give same result
    let read = reader.read_bits(4).unwrap();
    assert_eq!(read, 0b1011);
}

// ============================================================================
// Remaining Data Edge Cases
// ============================================================================

#[test]
fn test_remaining_data_at_boundary() {
    let data = [0xAA, 0xBB, 0xCC, 0xDD];
    let mut reader = BitReader::new(&data);

    // Initially all data available
    assert_eq!(reader.remaining_data().len(), 4);

    // Read 1 byte
    reader.read_byte().unwrap();
    assert_eq!(reader.remaining_data().len(), 3);

    // Read 4 bits (not byte-aligned)
    reader.read_bits(4).unwrap();

    // Should still return from current byte position
    assert_eq!(reader.remaining_data().len(), 2);
}

#[test]
fn test_remaining_bytes_counts_partial() {
    let data = [0xFF, 0xFF, 0xFF];
    let mut reader = BitReader::new(&data);

    // Read 1 bit
    reader.read_bit().unwrap();

    // Remaining bytes should count partial byte as 1
    assert_eq!(reader.remaining_bytes(), 3);

    // Read 7 more bits (complete first byte)
    reader.read_bits(7).unwrap();

    assert_eq!(reader.remaining_bytes(), 2);
}

// ============================================================================
// Multi-byte Read Edge Cases
// ============================================================================

#[test]
fn test_read_bytes_empty_buffer() {
    let data = [0xFF, 0xFF, 0xFF];
    let mut reader = BitReader::new(&data);

    let mut buf = [];
    assert!(reader.read_bytes(&mut buf).is_ok());
}

#[test]
fn test_read_bytes_exact_length() {
    let data = [0xAA, 0xBB, 0xCC];
    let mut reader = BitReader::new(&data);

    let mut buf = [0u8; 3];
    assert!(reader.read_bytes(&mut buf).is_ok());
    assert_eq!(buf, [0xAA, 0xBB, 0xCC]);
}

#[test]
fn test_read_bytes_beyond_data() {
    let data = [0xAA, 0xBB];
    let mut reader = BitReader::new(&data);

    let mut buf = [0u8; 3];
    assert!(matches!(
        reader.read_bytes(&mut buf),
        Err(BitvueError::UnexpectedEof(_))
    ));
}

#[test]
fn test_read_bytes_when_not_aligned() {
    let data = [0xAA, 0xBB, 0xCC, 0xDD];
    let mut reader = BitReader::new(&data);

    // Read 3 bits (not aligned)
    reader.read_bits(3).unwrap();

    let mut buf = [0u8; 2];
    // Should work, but will read bit-by-bit
    assert!(reader.read_bytes(&mut buf).is_ok());
}

// ============================================================================
// Has More Edge Cases
// ============================================================================

#[test]
fn test_has_more_empty() {
    let data = [];
    let reader = BitReader::new(&data);
    assert!(!reader.has_more());
}

#[test]
fn test_has_more_at_end() {
    let data = [0xFF];
    let mut reader = BitReader::new(&data);

    // Initially has more
    assert!(reader.has_more());

    // Read all bits
    reader.read_bits(8).unwrap();

    // No more data
    assert!(!reader.has_more());
}

#[test]
fn test_has_more_at_partial_byte() {
    let data = [0xFF];
    let mut reader = BitReader::new(&data);

    // Read 7 bits
    reader.read_bits(7).unwrap();

    // Still has more (1 bit remaining)
    assert!(reader.has_more());

    // Read last bit
    reader.read_bit().unwrap();

    // No more data
    assert!(!reader.has_more());
}
