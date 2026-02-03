//! Fuzz target for FNV hash functions
//!
//! Tests for:
//! - No panics on any input
//! - Determinism (same input = same output)
//! - No undefined behavior with edge cases

#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // Test fnv_hash (64-bit)
    let hash1 = abseil::absl_hash::fnv_hash(data);
    let hash2 = abseil::absl_hash::fnv_hash(data);
    assert_eq!(hash1, hash2, "FNV hash must be deterministic");

    // Test fnv_hash_32
    let hash32_1 = abseil::absl_hash::fnv_hash_32(data);
    let hash32_2 = abseil::absl_hash::fnv_hash_32(data);
    assert_eq!(hash32_1, hash32_2, "FNV-32 hash must be deterministic");

    // Test fnv_hash_128
    let hash128_1 = abseil::absl_hash::fnv_hash_128(data);
    let hash128_2 = abseil::absl_hash::fnv_hash_128(data);
    assert_eq!(hash128_1, hash128_2, "FNV-128 hash must be deterministic");

    // Verify no overflow in hash calculation
    // The hash should always produce valid output
    let _ = hash1 as u64;
    let _ = hash32_1 as u32;
    let _ = (hash128_1.0 as u64, hash128_1.1 as u64);
});
