//! Fuzz target for empty and single-byte edge cases
//!
//! This fuzzer specifically targets edge cases that are often missed:
//! - Empty input (0 bytes)
//! - Single byte input
//! - Two bytes (boundary for some algorithms)
//! - Inputs that are exactly at chunk boundaries

#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // Always test empty input (critical edge case)
    let empty: &[u8] = &[];

    // All hash functions should handle empty input without panicking
    let _ = abseil::absl_hash::fnv_hash(empty);
    let _ = abseil::absl_hash::fnv_hash_32(empty);
    let _ = abseil::absl_hash::murmur3_64(empty, 0);
    let _ = abseil::absl_hash::xxhash_64(empty, 0);
    let _ = abseil::absl_hash::xxhash3_64(empty, 0);
    let _ = abseil::absl_hash::highway_hash(empty, 0);
    let _ = abseil::absl_hash::wyhash(empty, 0);
    let _ = abseil::absl_hash::siphash_24(empty, 0, 0);

    // Test single byte if available
    if !data.is_empty() {
        let single = &data[..1];

        let fnv_single = abseil::absl_hash::fnv_hash(single);
        let fnv32_single = abseil::absl_hash::fnv_hash_32(single);
        let murmur_single = abseil::absl_hash::murmur3_64(single, 0);
        let xxh_single = abseil::absl_hash::xxhash_64(single, 0);
        let xxh3_single = abseil::absl_hash::xxhash3_64(single, 0);
        let hh_single = abseil::absl_hash::highway_hash(single, 0);
        let wy_single = abseil::absl_hash::wyhash(single, 0);
        let sip_single = abseil::absl_hash::siphash_24(single, 0, 0);

        // Verify single byte hashes are different from empty hashes
        let fnv_empty = abseil::absl_hash::fnv_hash(empty);
        let murmur_empty = abseil::absl_hash::murmur3_64(empty, 0);

        assert_ne!(fnv_single, fnv_empty, "Single byte hash should differ from empty hash");
        assert_ne!(murmur_single, murmur_empty, "Single byte hash should differ from empty hash");
    }

    // Test two bytes (boundary case)
    if data.len() >= 2 {
        let two = &data[..2];
        let _ = abseil::absl_hash::fnv_hash(two);
        let _ = abseil::absl_hash::murmur3_64(two, 0);
        let _ = abseil::absl_hash::xxhash_64(two, 0);
    }

    // Test specific boundary sizes for each algorithm
    // 16-byte blocks: FNV-128, WyHash, MurmurHash3, SipHash
    // 32-byte blocks: xxHash, xxHash3, HighwayHash

    if data.len() >= 16 {
        let _ = abseil::absl_hash::murmur3_64(&data[..16], 0);
        let _ = abseil::absl_hash::wyhash(&data[..16], 0);
        let _ = abseil::absl_hash::siphash_24(&data[..16], 0, 0);
    }

    if data.len() >= 32 {
        let _ = abseil::absl_hash::xxhash_64(&data[..32], 0);
        let _ = abseil::absl_hash::xxhash3_64(&data[..32], 0);
        let _ = abseil::absl_hash::highway_hash(&data[..32], 0);
    }
});
