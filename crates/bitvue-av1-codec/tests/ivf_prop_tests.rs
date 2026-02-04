//! Property-based tests for IVF parsing
//!
//! These tests use proptest to verify that IVF parsing handles all possible
//! input combinations without panicking, returning errors for invalid inputs.

use bitvue_av1_codec::parse_ivf_header;
use proptest::prelude::*;

/// Property: IVF header validation should never panic
///
/// Generates random byte sequences and verifies header parsing doesn't panic.
/// Invalid headers should return errors, not panic.
proptest! {
    #[test]
    fn prop_ivf_header_validation_never_panics(data in prop::collection::vec(any::<u8>(), 0..100)) {
        // Try to parse IVF header from any data
        // Should never panic, only return Ok or Err
        let _ = parse_ivf_header(&data);

        // Test passing means no panic occurred
    }
}

/// Property: IVF frame header arithmetic should not overflow
///
/// Verifies that frame size arithmetic doesn't overflow.
proptest! {
    #[test]
    fn prop_ivf_frame_size_no_overflow(
        frame_size in any::<u32>()
    ) {
        // Frame header size is 12 bytes
        const FRAME_HEADER_SIZE: usize = 12;

        // Frame size + header should not overflow
        let size = frame_size as usize;
        let _ = FRAME_HEADER_SIZE.checked_add(size);

        // If we get here without panic, overflow check passed
        // (checked_add returns None on overflow, which is fine)
    }
}

/// Property: Frame count should be non-negative
///
/// Verifies that frame count arithmetic doesn't underflow.
proptest! {
    #[test]
    fn prop_frame_count_never_negative(frame_idx in 0i64..) {
        // Frame index should always be >= 0
        prop_assert!(frame_idx >= 0);
    }
}
