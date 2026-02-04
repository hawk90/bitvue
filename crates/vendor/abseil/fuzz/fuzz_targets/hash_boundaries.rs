//! Fuzz target for hash function boundary conditions
//!
//! This fuzzer targets specific byte counts that are at or near
//! algorithm block boundaries to catch off-by-one errors and
//! incorrect remainder handling.

#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // Test sizes at and near 8-byte boundaries (SipHash, some stages of xxHash3/WyHash)
    for &offset in &[-2i32, -1, 0, 1, 2] {
        for &base in &[8u32, 16, 24, 32, 40, 48, 56, 64] {
            let len = (base as i32 + offset) as usize;
            if data.len() >= len {
                // Test all hash functions at this boundary
                let slice = &data[..len];

                // Functions using 8-byte chunks
                let _ = abseil::absl_hash::siphash_24(slice, 42, 24);

                // Functions using 16-byte chunks
                let _ = abseil::absl_hash::murmur3_64(slice, 42);
                let _ = abseil::absl_hash::wyhash(slice, 42);

                // Functions using 32-byte chunks
                let _ = abseil::absl_hash::xxhash_64(slice, 42);
                let _ = abseil::absl_hash::xxhash3_64(slice, 42);
                let _ = abseil::absl_hash::highway_hash(slice, 42);

                // FNV (no chunking, but test anyway)
                let _ = abseil::absl_hash::fnv_hash(slice);
            }
        }
    }

    // Specific problematic boundary: 7, 15, 23, 31 bytes (just before chunk boundaries)
    for &len in &[7, 15, 23, 31] {
        if data.len() >= len {
            let slice = &data[..len];

            let murmur1 = abseil::absl_hash::murmur3_64(slice, 42);
            let murmur2 = abseil::absl_hash::murmur3_64(slice, 42);
            assert_eq!(murmur1, murmur2, "MurmurHash3 inconsistent at {} bytes", len);

            let xxh1 = abseil::absl_hash::xxhash_64(slice, 42);
            let xxh2 = abseil::absl_hash::xxhash_64(slice, 42);
            assert_eq!(xxh1, xxh2, "xxHash64 inconsistent at {} bytes", len);

            let xxh3_1 = abseil::absl_hash::xxhash3_64(slice, 42);
            let xxh3_2 = abseil::absl_hash::xxhash3_64(slice, 42);
            assert_eq!(xxh3_1, xxh3_2, "xxHash3 inconsistent at {} bytes", len);

            let hh1 = abseil::absl_hash::highway_hash(slice, 42);
            let hh2 = abseil::absl_hash::highway_hash(slice, 42);
            assert_eq!(hh1, hh2, "HighwayHash inconsistent at {} bytes", len);
        }
    }

    // Specific problematic boundary: 9, 17, 25 bytes (just after chunk boundaries)
    // These trigger the "remaining bytes" path
    for &len in &[9, 17, 25, 33, 41, 49, 57] {
        if data.len() >= len {
            let slice = &data[..len];

            let murmur1 = abseil::absl_hash::murmur3_64(slice, 42);
            let murmur2 = abseil::absl_hash::murmur3_64(slice, 42);
            assert_eq!(murmur1, murmur2, "MurmurHash3 inconsistent at {} bytes", len);

            let xxh1 = abseil::absl_hash::xxhash_64(slice, 42);
            let xxh2 = abseil::absl_hash::xxhash_64(slice, 42);
            assert_eq!(xxh1, xxh2, "xxHash64 inconsistent at {} bytes", len);
        }
    }

    // Test xxHash3 4-stage remaining byte processing
    // Stage 1: bytes 0-7, Stage 2: bytes 8-15, Stage 3: bytes 16-23, Stage 4: bytes 24-31
    for &len in &[1, 7, 8, 9, 15, 16, 17, 23, 24, 25, 31, 33] {
        if data.len() >= len {
            let slice = &data[..len];
            let hash1 = abseil::absl_hash::xxhash3_64(slice, 42);
            let hash2 = abseil::absl_hash::xxhash3_64(slice, 42);
            assert_eq!(hash1, hash2, "xxHash3 inconsistent at {} bytes", len);
        }
    }
});
