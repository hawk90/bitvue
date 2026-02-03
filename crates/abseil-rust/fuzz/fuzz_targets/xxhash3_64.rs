//! Fuzz target for xxHash3 64-bit
//!
//! xxHash3 is the improved version of xxHash with more complex
//! remaining byte processing. This fuzzer specifically targets:
//! - Edge cases in remaining byte processing (1-31 bytes)
//! - Block boundary conditions
//! - Potential out-of-bounds access in complex byte handling

#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // Test with various seeds
    for &seed in &[0u64, 42, 0xABCD1234, u64::MAX] {
        let hash1 = abseil::absl_hash::xxhash3_64(data, seed);
        let hash2 = abseil::absl_hash::xxhash3_64(data, seed);
        assert_eq!(hash1, hash2, "xxHash3-64 must be deterministic for seed {}", seed);
    }

    // xxHash3 has complex remaining byte processing with 4 stages (8 bytes each)
    // Test specific lengths that exercise different code paths:

    // Test exact block boundaries
    if data.len() >= 32 {
        let _ = abseil::absl_hash::xxhash3_64(&data[..32], 0);
        if data.len() >= 64 {
            let _ = abseil::absl_hash::xxhash3_64(&data[..64], 0);
        }
    }

    // Test remaining byte edge cases (1-31 bytes after 32-byte blocks)
    for &remaining_len in &[1, 7, 8, 9, 15, 16, 17, 23, 24, 25, 31] {
        if data.len() >= 32 + remaining_len {
            let _ = abseil::absl_hash::xxhash3_64(&data[..32 + remaining_len], 0);
        } else if data.len() >= remaining_len {
            let _ = abseil::absl_hash::xxhash3_64(&data[..remaining_len], 0);
        }
    }

    // Test the 4-stage remaining byte processing (bytes 8-15, 16-23, 24-31)
    // Each stage processes up to 8 bytes
    if data.len() >= 9 {
        let _ = abseil::absl_hash::xxhash3_64(&data[..9], 0); // 8 + 1
    }
    if data.len() >= 17 {
        let _ = abseil::absl_hash::xxhash3_64(&data[..17], 0); // 8 + 8 + 1
    }
    if data.len() >= 25 {
        let _ = abseil::absl_hash::xxhash3_64(&data[..25], 0); // 8 + 8 + 8 + 1
    }
});
