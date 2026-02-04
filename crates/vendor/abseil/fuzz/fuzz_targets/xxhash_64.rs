//! Fuzz target for xxHash 64-bit
//!
//! Tests for:
//! - No panics on any input
//! - Determinism
//! - Correct handling of 32-byte block boundaries
//! - Proper processing of remaining bytes

#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // Test with various seeds
    for &seed in &[0u64, 42, 0xDEADBEEF, u64::MAX] {
        let hash1 = abseil::absl_hash::xxhash_64(data, seed);
        let hash2 = abseil::absl_hash::xxhash_64(data, seed);
        assert_eq!(hash1, hash2, "xxHash64 must be deterministic for seed {}", seed);
    }

    // Test boundary conditions that could cause issues:
    // - Empty input
    let _ = abseil::absl_hash::xxhash_64(&[], 0);

    // - Single byte (1 < 32, triggers remaining bytes path)
    if !data.is_empty() {
        let _ = abseil::absl_hash::xxhash_64(&data[..1.min(data.len())], 42);
    }

    // - Exactly 32 bytes (one full block, no remainder)
    if data.len() >= 32 {
        let _ = abseil::absl_hash::xxhash_64(&data[..32], 0);
        // - Exactly 64 bytes (two full blocks)
        if data.len() >= 64 {
            let _ = abseil::absl_hash::xxhash_64(&data[..64], 0);
        }
    }

    // - One byte past a block boundary (33 bytes = 32 + 1)
    if data.len() >= 33 {
        let _ = abseil::absl_hash::xxhash_64(&data[..33], 0);
    }
});
