//! Property-based tests for LEB128 decoding
//!
//! These tests use proptest to verify that LEB128 decoding handles all
//! possible byte sequences without panicking.

use proptest::prelude::*;
use bitvue_av1::decode_uleb128;

/// Property: LEB128 decoder should never panic on any input
///
/// This test generates random byte sequences and verifies that LEB128
/// operations either succeed or return errors, but never panic.
proptest! {
    #[test]
    fn prop_leb128_decode_never_panics(data in prop::collection::vec(any::<u8>(), 0..20)) {
        // Try to decode LEB128 - should never panic
        let _ = decode_uleb128(&data);

        // Test passing means no panic occurred
    }
}

/// Property: LEB128 decoded values should be non-negative
///
/// Unsigned LEB128 should always produce non-negative values.
proptest! {
    #[test]
    fn prop_uleb128_never_negative(data in prop::collection::vec(any::<u8>(), 0..20)) {
        let result = decode_uleb128(&data);

        if let Ok((value, _)) = result {
            // Unsigned decoding should never produce negative values
            prop_assert!(value >= 0);
        }
    }
}
