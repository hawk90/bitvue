//! Fuzz target for SipHash-2-4
//!
//! SipHash is a keyed hash function. This fuzzer targets:
//! - Key handling (k0, k1)
//! - 8-byte chunk processing
//! - Length masking edge case

#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // Test with various key combinations
    for &(k0, k1) in &[
        (0u64, 0u64),
        (42, 24),
        (0xDEADBEEF, 0xCAFEBABE),
        (u64::MAX, u64::MAX),
        (0xFFFFFFFFFFFFFFFF, 0xAAAAAAAAAAAAAAAA),
    ] {
        let hash1 = abseil::absl_hash::siphash_24(data, k0, k1);
        let hash2 = abseil::absl_hash::siphash_24(data, k0, k1);
        assert_eq!(hash1, hash2, "SipHash-2-4 must be deterministic for keys ({}, {})", k0, k1);
    }

    // SipHash uses chunks(8) which can handle any length
    // Test edge case: length masking (len &= 0xff)
    // This means only the low 8 bits of length are used in finalization

    // Test empty input
    let _ = abseil::absl_hash::siphash_24(&[], 0, 0);

    // Test exact 8-byte chunk boundaries
    if data.len() >= 8 {
        let _ = abseil::absl_hash::siphash_24(&data[..8], 0, 0);
        if data.len() >= 16 {
            let _ = abseil::absl_hash::siphash_24(&data[..16], 0, 0);
        }
    }

    // Test partial chunks (1-7 bytes)
    if !data.is_empty() {
        for &len in [1, 3, 5, 7].iter().filter(|&&l| data.len() >= l) {
            let _ = abseil::absl_hash::siphash_24(&data[..len], 42, 24);
        }
    }

    // Test the b |= (byte as u64) << (i * 8) pattern
    // This builds a u64 from bytes, verify i doesn't overflow
    // chunks(8) ensures max 8 iterations, so i * 8 max is 56, safe for u64
});
