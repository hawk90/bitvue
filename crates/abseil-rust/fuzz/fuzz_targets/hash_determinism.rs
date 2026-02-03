//! Fuzz target for hash function determinism and consistency
//!
//! This fuzzer verifies that all hash functions:
//! - Are deterministic (same input = same output)
//! - Handle all input sizes consistently
//! - Don't have hidden state or race conditions

#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // FNV determinism
    let fnv1 = abseil::absl_hash::fnv_hash(data);
    let fnv2 = abseil::absl_hash::fnv_hash(data);
    assert_eq!(fnv1, fnv2, "FNV-64 not deterministic");

    let fnv32_1 = abseil::absl_hash::fnv_hash_32(data);
    let fnv32_2 = abseil::absl_hash::fnv_hash_32(data);
    assert_eq!(fnv32_1, fnv32_2, "FNV-32 not deterministic");

    // MurmurHash3 determinism
    let murmur1 = abseil::absl_hash::murmur3_64(data, 42);
    let murmur2 = abseil::absl_hash::murmur3_64(data, 42);
    assert_eq!(murmur1, murmur2, "MurmurHash3 not deterministic");

    // xxHash determinism
    let xxh1 = abseil::absl_hash::xxhash_64(data, 42);
    let xxh2 = abseil::absl_hash::xxhash_64(data, 42);
    assert_eq!(xxh1, xxh2, "xxHash64 not deterministic");

    // xxHash3 determinism
    let xxh3_1 = abseil::absl_hash::xxhash3_64(data, 42);
    let xxh3_2 = abseil::absl_hash::xxhash3_64(data, 42);
    assert_eq!(xxh3_1, xxh3_2, "xxHash3 not deterministic");

    // HighwayHash determinism
    let hh1 = abseil::absl_hash::highway_hash(data, 42);
    let hh2 = abseil::absl_hash::highway_hash(data, 42);
    assert_eq!(hh1, hh2, "HighwayHash not deterministic");

    // WyHash determinism
    let wy1 = abseil::absl_hash::wyhash(data, 42);
    let wy2 = abseil::absl_hash::wyhash(data, 42);
    assert_eq!(wy1, wy2, "WyHash not deterministic");

    // DJB2 determinism
    let djb2_1 = abseil::absl_hash::djb2_hash(data);
    let djb2_2 = abseil::absl_hash::djb2_hash(data);
    assert_eq!(djb2_1, djb2_2, "DJB2 not deterministic");

    // SipHash determinism
    let sip1 = abseil::absl_hash::siphash_24(data, 42, 24);
    let sip2 = abseil::absl_hash::siphash_24(data, 42, 24);
    assert_eq!(sip1, sip2, "SipHash not deterministic");

    // Verify that different hash functions produce different outputs
    // (with very high probability for non-empty inputs)
    if !data.is_empty() && data.len() > 4 {
        let hashes = [
            ("FNV", fnv1),
            ("FNV32", fnv32_1 as u64),
            ("Murmur3", murmur1),
            ("xxHash64", xxh1),
            ("xxHash3", xxh3_1),
            ("HighwayHash", hh1),
            ("WyHash", wy1),
            ("DJB2", djb2_1),
            ("SipHash", sip1),
        ];

        // Count unique hashes - should be at least 7 different values
        // (statistically, collision probability is negligible)
        let unique_hashes: std::collections::HashSet<_> = hashes.iter().map(|(_, h)| h).collect();
        assert!(
            unique_hashes.len() >= 7,
            "Too many hash collisions: only {} unique values out of 9 functions",
            unique_hashes.len()
        );
    }
});
