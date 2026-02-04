//! Fuzz target for DJB2 hash
//!
//! DJB2 is a simple hash function using:
//!   hash = hash * 33 + byte
//!
//! Tests for:
//! - No overflow/underflow issues
//! - Determinism
//! - Handling of empty input

#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // Test determinism
    let hash1 = abseil::absl_hash::djb2_hash(data);
    let hash2 = abseil::absl_hash::djb2_hash(data);
    assert_eq!(hash1, hash2, "DJB2 hash must be deterministic");

    // DJB2 uses wrapping_mul(33) which should never panic
    // Verify it produces valid output for all inputs

    // Test empty input (initial value is 5381)
    let empty_hash = abseil::absl_hash::djb2_hash(&[]);
    assert_eq!(empty_hash, 5381, "DJB2 of empty input should be initial value");

    // Test single byte
    if !data.is_empty() {
        let _ = abseil::absl_hash::djb2_hash(&data[..1]);
    }
});
