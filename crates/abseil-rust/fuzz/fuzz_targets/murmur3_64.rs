//! Fuzz target for MurmurHash3 64-bit
//!
//! Tests for:
//! - No panics on any input
//! - Determinism
//! - Correct handling of chunk boundaries
//! - No out-of-bounds memory access

#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // Test with various seeds to catch seed-related bugs
    for &seed in &[0u64, 42, 0xDEADBEEF, u64::MAX, 1, 0xFFFFFFFFFFFFFFFF] {
        let hash1 = abseil::absl_hash::murmur3_64(data, seed);
        let hash2 = abseil::absl_hash::murmur3_64(data, seed);
        assert_eq!(hash1, hash2, "MurmurHash3 must be deterministic for seed {}", seed);

        // Test murmur3_mix (the finalization function)
        let mixed = abseil::absl_hash::murmur3_mix(hash1);
        // Should not panic and should produce a valid u64
        let _ = mixed as u64;
    }

    // Test edge case: exact 16-byte chunks (no remainder)
    if data.len() >= 32 {
        let _ = abseil::absl_hash::murmur3_64(&data[..32], 0);
    }

    // Test edge case: single byte
    if !data.is_empty() {
        let _ = abseil::absl_hash::murmur3_64(&data[..1], 0);
    }
});
