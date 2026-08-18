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
//! Property-based tests for LEB128 decoding
//!
//! These tests use proptest to verify that LEB128 decoding handles all
//! possible byte sequences without panicking.

use bitvue_av1_codec::decode_uleb128;
use bitvue_av1_codec::leb128::MAX_LEB128_BYTES;
use proptest::prelude::*;

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

/// Property: successfully decoded LEB128 values must fit within the
/// spec-mandated bit width (MAX_LEB128_BYTES * 7 = 56 bits).
///
/// `decode_uleb128` returns `u64`, so `value >= 0` is a tautology (a `u64`
/// can never be negative) and doesn't actually exercise the decoder's
/// overflow handling. The real invariant is that the parser reads at most
/// `MAX_LEB128_BYTES` continuation bytes, so any successfully decoded value
/// must be representable in `MAX_LEB128_BYTES * 7` bits; anything wider
/// should have been rejected as an overflow error instead.
proptest! {
    #[test]
    fn prop_uleb128_within_max_bit_width(data in prop::collection::vec(any::<u8>(), 0..20)) {
        let result = decode_uleb128(&data);

        if let Ok((value, bytes_read)) = result {
            prop_assert!(
                bytes_read <= MAX_LEB128_BYTES,
                "decoder consumed {} bytes, exceeding MAX_LEB128_BYTES ({})",
                bytes_read,
                MAX_LEB128_BYTES
            );

            let max_bits = (MAX_LEB128_BYTES as u32) * 7;
            // max_bits is 56 here, so this shift is well within u64 range.
            let upper_bound = 1u64 << max_bits;
            prop_assert!(
                value < upper_bound,
                "decoded value {} exceeds the {}-bit LEB128 limit",
                value,
                max_bits
            );
        }
    }
}
