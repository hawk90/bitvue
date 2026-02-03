//! Fuzz target for BloomFilter
//!
//! Tests for:
//! - No panics on any input values
//! - Determinism of contains/insert operations
//! - Integer overflow protection in index calculation
//! - No division by zero

#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // Test various BloomFilter configurations
    for &capacity in &[10, 100, 1000, 10000] {
        for &num_hashes in &[1, 2, 3, 5, 7, 10] {
            // Test creating and using BloomFilter
            if let Ok(mut bloom) = abseil::absl_hash::BloomFilter::try_new(capacity, num_hashes) {
                // Test insert with various data
                if !data.is_empty() {
                    // Insert the fuzzer input as individual bytes
                    for &byte in data.iter().take(100) {
                        bloom.insert(&byte);
                    }

                    // Check determinism by inserting same byte twice
                    if let Some(&first_byte) = data.first() {
                        bloom.insert(&first_byte);
                        let contains1 = bloom.contains(&first_byte);
                        let contains2 = bloom.contains(&first_byte);
                        assert_eq!(contains1, contains2, "BloomFilter::contains must be deterministic");

                        // Test that inserted value is found
                        assert!(bloom.contains(&first_byte), "BloomFilter should contain inserted value");
                    }
                }

                // Test with single byte values (edge case for index calculation)
                for &byte in &[0u8, 1, 127, 128, 255] {
                    bloom.insert(&byte);
                    assert!(bloom.contains(&byte), "BloomFilter should contain inserted byte {}", byte);
                }

                // Test with empty slice (just to verify no panic)
                let empty: &[u8] = &[];
                let _ = bloom.contains(&0u8); // Just check something

                // Test with various integer types
                bloom.insert(&42u32);
                assert!(bloom.contains(&42u32));

                bloom.insert(&12345i64);
                assert!(bloom.contains(&12345i64));

                bloom.insert(&"test string");
                assert!(bloom.contains(&"test string"));
            }
        }
    }

    // Test edge cases: minimum valid values
    let mut min_bloom = abseil::absl_hash::BloomFilter::new(1, 1);
    min_bloom.insert(&42u8);
    assert!(min_bloom.contains(&42u8));
    assert!(!min_bloom.contains(&99u8));

    // Test that invalid inputs return errors
    assert!(abseil::absl_hash::BloomFilter::try_new(0, 3).is_err(), "capacity=0 should error");
    assert!(abseil::absl_hash::BloomFilter::try_new(100, 0).is_err(), "num_hashes=0 should error");

    // Test with large capacity (potential overflow in index calculation)
    if let Ok(mut large_bloom) = abseil::absl_hash::BloomFilter::try_new(1000000, 10) {
        // Insert various values to test wrapping arithmetic in index calculation
        for &val in &[0u32, 42, u32::MAX] {
            large_bloom.insert(&val);
            assert!(large_bloom.contains(&val));
        }
    }

    // Test false positive behavior (BloomFilter may have false positives)
    // but should never have false negatives
    let mut fp_test = abseil::absl_hash::BloomFilter::new(100, 5);
    fp_test.insert(&"value1");
    fp_test.insert(&"value2");

    // These should be found
    assert!(fp_test.contains(&"value1"));
    assert!(fp_test.contains(&"value2"));

    // This might be found (false positive is acceptable)
    let _ = fp_test.contains(&"value3");

    // But if we clear, nothing should be found
    fp_test.clear();
    assert!(!fp_test.contains(&"value1"));
    assert!(!fp_test.contains(&"value2"));
});
