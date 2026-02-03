//! Fuzz target for WyHash
//!
//! WyHash processes data in 16-byte blocks. This fuzzer targets:
//! - 16-byte block boundaries
//! - Remaining byte processing
//! - Tail processing edge cases

#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // Test with various seeds
    for &seed in &[0u64, 42, 0xDEAD1234, u64::MAX] {
        let hash1 = abseil::absl_hash::wyhash(data, seed);
        let hash2 = abseil::absl_hash::wyhash(data, seed);
        assert_eq!(hash1, hash2, "WyHash must be deterministic for seed {}", seed);
    }

    // WyHash uses 16-byte blocks
    // Test exact block boundaries
    if data.len() >= 16 {
        let _ = abseil::absl_hash::wyhash(&data[..16], 0);
        if data.len() >= 32 {
            let _ = abseil::absl_hash::wyhash(&data[..32], 0);
        }
    }

    // Test remaining byte edge cases
    // WyHash has special handling for > 8 bytes remaining
    for &remaining_len in &[1, 7, 8, 9, 15] {
        if data.len() >= 16 + remaining_len {
            let _ = abseil::absl_hash::wyhash(&data[..16 + remaining_len], 0);
        } else if data.len() >= remaining_len {
            let _ = abseil::absl_hash::wyhash(&data[..remaining_len], 0);
        }
    }

    // Test the special tail processing (> 8 bytes vs <= 8 bytes)
    if data.len() >= 9 {
        // This triggers the > 8 bytes path
        let _ = abseil::absl_hash::wyhash(&data[..9], 0);
    }

    // Test edge case in tail processing: remaining_len > 8 triggers t2 = tail >> 8
    // Need to verify this doesn't cause issues
    if data.len() >= 17 {
        // 16 bytes full block + 1 byte remaining (<= 8 bytes path)
        let _ = abseil::absl_hash::wyhash(&data[..17], 0);
    }
    if data.len() >= 25 {
        // 16 bytes full block + 9 bytes remaining (> 8 bytes path)
        let _ = abseil::absl_hash::wyhash(&data[..25], 0);
    }
});
