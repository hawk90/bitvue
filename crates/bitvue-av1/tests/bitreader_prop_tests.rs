//! Property-based tests for BitReader
//!
//! These tests use proptest to verify that BitReader handles all possible
//! input combinations without panicking, returning errors for invalid inputs.

use proptest::prelude::*;
use bitvue_av1::BitReader;

/// Property: BitReader should never panic on any input
///
/// This test generates random byte sequences and verifies that BitReader
/// operations either succeed or return errors, but never panic.
proptest! {
    fn prop_read_bit_never_panics(data in prop::collection::vec(any::<u8>(), 0..1000)) {
        let mut reader = BitReader::new(&data);

        // Try various read operations - should never panic
        let _ = reader.read_bit();
        let _ = reader.read_bits(8);
        let _ = reader.read_bit();
        let _ = reader.read_su(1);

        // Any result is acceptable as long as we didn't panic
        // The test passing means no panic occurred
    }
}

/// Property: Reading zero bits should return 0
///
/// Edge case verification: reading 0 bits should always work.
#[test]
fn prop_read_zero_bits_returns_zero() {
    // Any data will do for this test
    let data = vec![0u8, 1, 2, 3];

    let mut reader = BitReader::new(&data);

    // Reading 0 bits should always succeed and return 0
    let result = reader.read_bits(0);
    assert_eq!(result.unwrap(), 0);
}
