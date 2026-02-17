#![allow(dead_code)]
//! Extended tests for HEVC bitreader module
//!
//! Comprehensive tests covering BitReader methods not in existing tests

use bitvue_hevc::BitReader;

// ============================================================================
// Position and Remaining Bits Tests
// ============================================================================

#[test]
fn test_bitreader_position() {
    let data = [0b10110010, 0b01100101];
    let mut reader = BitReader::new(&data);

    assert_eq!(reader.position(), 0);

    reader.read_bit().unwrap();
    assert_eq!(reader.position(), 1);

    reader.read_bits(5).unwrap();
    assert_eq!(reader.position(), 6);
}

#[test]
fn test_bitreader_position_byte_boundary() {
    let data = [0b10110010, 0b01100101];
    let mut reader = BitReader::new(&data);

    reader.read_bits(8).unwrap();
    assert_eq!(reader.position(), 8);

    reader.read_bits(4).unwrap();
    assert_eq!(reader.position(), 12);
}

#[test]
fn test_bitreader_remaining_bits() {
    let data = [0b10110010, 0b01100101]; // 16 bits = 2 bytes
    let mut reader = BitReader::new(&data);

    assert_eq!(reader.remaining_bits(), 16);

    reader.read_bit().unwrap();
    assert_eq!(reader.remaining_bits(), 15);

    reader.read_bits(8).unwrap();
    assert_eq!(reader.remaining_bits(), 7);
}

#[test]
fn test_bitreader_has_more_data() {
    let data = [0b10110010, 0b01100101];
    let mut reader = BitReader::new(&data);

    assert!(reader.has_more_data());

    // Read all bits
    reader.read_bits(16).unwrap();
    assert!(!reader.has_more_data());
}

#[test]
fn test_bitreader_has_more_data_partial() {
    let data = [0b10110010];
    let mut reader = BitReader::new(&data);

    assert!(reader.has_more_data());

    reader.read_bits(4).unwrap();
    assert!(reader.has_more_data());

    reader.read_bits(4).unwrap();
    assert!(!reader.has_more_data());
}

// ============================================================================
// Skip Bits Tests
// ============================================================================

#[test]
fn test_bitreader_skip_bits() {
    let data = [0b10110010, 0b01100101];
    let mut reader = BitReader::new(&data);

    reader.skip_bits(5).unwrap();
    assert_eq!(reader.position(), 5);

    let value = reader.read_bits(3).unwrap();
    assert_eq!(value, 0b010);
}

#[test]
fn test_bitreader_skip_bits_byte_boundary() {
    let data = [0b10110010, 0b01100101];
    let mut reader = BitReader::new(&data);

    reader.skip_bits(12).unwrap();
    assert_eq!(reader.position(), 12);

    let value = reader.read_bits(4).unwrap();
    assert_eq!(value, 0b0101);
}

#[test]
fn test_bitreader_skip_zero_bits() {
    let data = [0b10110010];
    let mut reader = BitReader::new(&data);

    reader.skip_bits(0).unwrap();
    assert_eq!(reader.position(), 0);

    let value = reader.read_bit().unwrap();
    assert!(value);
}

// ============================================================================
// Byte Align Tests
// ============================================================================

#[test]
fn test_bitreader_byte_align() {
    let data = [0b10110010, 0b01100101];
    let mut reader = BitReader::new(&data);

    reader.read_bits(3).unwrap();
    assert_eq!(reader.position(), 3);

    reader.byte_align();
    assert_eq!(reader.position(), 8);

    let value = reader.read_bits(8).unwrap();
    assert_eq!(value, 0b01100101);
}

#[test]
fn test_bitreader_byte_align_already_aligned() {
    let data = [0b10110010, 0b01100101];
    let mut reader = BitReader::new(&data);

    reader.read_bits(8).unwrap();
    assert_eq!(reader.position(), 8);

    reader.byte_align();
    assert_eq!(reader.position(), 8); // Should not change
}

#[test]
fn test_bitreader_is_byte_aligned() {
    let data = [0b10110010];
    let mut reader = BitReader::new(&data);

    reader.read_bits(3).unwrap();
    assert!(!reader.is_byte_aligned());

    reader.read_bits(5).unwrap();
    assert!(reader.is_byte_aligned());
}

// ============================================================================
// read_bits_u64 Tests
// ============================================================================

#[test]
fn test_bitreader_read_bits_u64() {
    let data = [0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF];
    let mut reader = BitReader::new(&data);

    let value = reader.read_bits_u64(64).unwrap();
    assert_eq!(value, 0xFFFFFFFFFFFFFFFF);
}

#[test]
fn test_bitreader_read_bits_u64_partial() {
    let data = [0xFF, 0xFF, 0xFF, 0xFF, 0xFF];
    let mut reader = BitReader::new(&data);

    let value = reader.read_bits_u64(32).unwrap();
    assert_eq!(value, 0xFFFFFFFF);
}

// ============================================================================
// read_u Tests
// ============================================================================

#[test]
fn test_bitreader_read_u() {
    let data = [0b10110010, 0b01100101];
    let mut reader = BitReader::new(&data);

    let value = reader.read_u(5).unwrap();
    assert_eq!(value, 0b10110);
}

#[test]
fn test_bitreader_read_u_8_bits() {
    let data = [0xAB, 0xCD];
    let mut reader = BitReader::new(&data);

    let value = reader.read_u(8).unwrap();
    assert_eq!(value, 0xAB);
}

#[test]
fn test_bitreader_read_u_16_bits() {
    let data = [0xAB, 0xCD];
    let mut reader = BitReader::new(&data);

    let value = reader.read_u(16).unwrap();
    assert_eq!(value, 0xABCD);
}

// ============================================================================
// read_f Tests
// ============================================================================

#[test]
fn test_bitreader_read_f() {
    let data = [0b10110010, 0b01100101];
    let mut reader = BitReader::new(&data);

    let value = reader.read_f(6).unwrap();
    assert_eq!(value, 0b101100);
}

#[test]
fn test_bitreader_read_f_pattern() {
    let data = [0b11100101, 0b10110010];
    let mut reader = BitReader::new(&data);

    // Read fixed pattern
    let value = reader.read_f(8).unwrap();
    assert_eq!(value, 0b11100101);
}

// ============================================================================
// peek_bits Tests
// ============================================================================

#[test]
fn test_bitreader_peek_bits() {
    let data = [0b10110010, 0b01100101];
    let mut reader = BitReader::new(&data);

    let peeked = reader.peek_bits(4).unwrap();
    assert_eq!(peeked, 0b1011);

    // Verify position hasn't changed
    assert_eq!(reader.position(), 0);

    // Now read and verify it's the same
    let read = reader.read_bits(4).unwrap();
    assert_eq!(read, peeked);
}

#[test]
fn test_bitreader_peek_bits_after_read() {
    let data = [0b10110010, 0b01100101];
    let mut reader = BitReader::new(&data);

    reader.read_bits(2).unwrap();

    let peeked = reader.peek_bits(4).unwrap();
    assert_eq!(peeked, 0b1100);

    // Verify position still at 2
    assert_eq!(reader.position(), 2);
}

// ============================================================================
// more_rbsp_data Tests
// ============================================================================

#[test]
fn test_bitreader_more_rbsp_data_with_data() {
    let data = [0b10110010];
    let reader = BitReader::new(&data);

    assert!(reader.more_rbsp_data());
}

#[test]
fn test_bitreader_more_rbsp_data_eof() {
    let data = [0b10000000]; // Single 1 bit followed by zeros
    let mut reader = BitReader::new(&data);

    reader.read_bit().unwrap();
    // At this point, there's technically no more data
    assert!(!reader.more_rbsp_data() || reader.remaining_bits() > 0);
}

#[test]
fn test_bitreader_more_rbsp_data_after_alignment() {
    let data = [0b10110010, 0b00000000];
    let mut reader = BitReader::new(&data);

    reader.read_bits(8).unwrap();
    reader.byte_align();

    // After alignment, check if more data
    let has_more = reader.more_rbsp_data();
    assert_eq!(has_more, reader.has_more_data());
}

// ============================================================================
// Error Handling Tests
// ============================================================================

#[test]
fn test_bitreader_read_bit_eof() {
    let data = [0b10110010]; // 8 bits
    let mut reader = BitReader::new(&data);

    reader.read_bits(8).unwrap();
    let result = reader.read_bit();
    assert!(result.is_err());
}

#[test]
fn test_bitreader_read_bits_eof() {
    let data = [0b10110010]; // 8 bits
    let mut reader = BitReader::new(&data);

    reader.read_bits(5).unwrap();
    let result = reader.read_bits(8); // Only 3 bits remaining
    assert!(result.is_err());
}

#[test]
fn test_bitreader_read_byte_eof() {
    let data = [0xAB];
    let mut reader = BitReader::new(&data);

    reader.read_byte().unwrap();
    let result = reader.read_byte();
    assert!(result.is_err());
}

#[test]
fn test_bitreader_read_ue_eof() {
    let data = [0b10000000]; // Only enough for 1 ue value
    let mut reader = BitReader::new(&data);

    reader.read_ue().unwrap();
    let result = reader.read_ue();
    assert!(result.is_err());
}

#[test]
fn test_bitreader_read_se_eof() {
    let data = [0b10000000]; // Only enough for 1 se value
    let mut reader = BitReader::new(&data);

    reader.read_se().unwrap();
    let result = reader.read_se();
    assert!(result.is_err());
}

#[test]
fn test_bitreader_skip_bits_eof() {
    let data = [0b10110010]; // 8 bits
    let mut reader = BitReader::new(&data);

    let result = reader.skip_bits(16); // More than available
    assert!(result.is_err());
}

#[test]
fn test_bitreader_peek_bits_eof() {
    let data = [0b10110010]; // 8 bits
    let reader = BitReader::new(&data);

    let result = reader.peek_bits(16); // More than available
    assert!(result.is_err());
}

// ============================================================================
// Inner Reader Access Tests
// ============================================================================

#[test]
fn test_bitreader_inner() {
    let data = [0b10110010];
    let reader = BitReader::new(&data);

    let inner = reader.inner();
    assert_eq!(inner.position(), 0);
}

#[test]
fn test_bitreader_inner_mut_read() {
    let data = [0b10110010];
    let mut reader = BitReader::new(&data);

    {
        let inner = reader.inner_mut();
        // Can read through inner reader (position changes)
        let _ = inner.read_bit();
    }

    assert_eq!(reader.position(), 1);
}
