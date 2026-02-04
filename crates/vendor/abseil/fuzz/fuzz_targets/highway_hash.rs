//! Fuzz target for HighwayHash
//!
//! HighwayHash processes data in 32-byte blocks with 4 u64 values per block.
//! This fuzzer targets:
//! - Block boundary conditions
//! - Remaining byte processing
//! - Safety of array access patterns

#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // Test with various seeds
    for &seed in &[0u64, 42, 0xBEEF1234, u64::MAX] {
        let hash1 = abseil::absl_hash::highway_hash(data, seed);
        let hash2 = abseil::absl_hash::highway_hash(data, seed);
        assert_eq!(hash1, hash2, "HighwayHash must be deterministic for seed {}", seed);
    }

    // HighwayHash uses 32-byte blocks with 4 u64 values
    // Test exact block boundaries
    if data.len() >= 32 {
        let _ = abseil::absl_hash::highway_hash(&data[..32], 0);
        if data.len() >= 64 {
            let _ = abseil::absl_hash::highway_hash(&data[..64], 0);
        }
    }

    // Test remaining byte edge cases
    // The remaining bytes are packed into up to 4 u64 values (32 bytes total)
    for &remaining_len in &[1, 7, 8, 9, 15, 16, 17, 23, 24, 25, 31] {
        if data.len() >= 32 + remaining_len {
            let _ = abseil::absl_hash::highway_hash(&data[..32 + remaining_len], 0);
        } else if data.len() >= remaining_len {
            let _ = abseil::absl_hash::highway_hash(&data[..remaining_len], 0);
        }
    }

    // Test the word_idx calculation: i / 8 and byte_idx: i % 8
    // This could have issues when remaining_len is near boundaries
    if data.len() >= 24 {
        // This exercises word_idx = 3 (bytes 24-31)
        let _ = abseil::absl_hash::highway_hash(&data[..24], 0);
    }
});
