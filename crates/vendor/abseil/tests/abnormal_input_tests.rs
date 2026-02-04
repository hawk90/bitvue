//! Comprehensive abnormal input edge case tests.
//!
//! This module tests edge cases for abnormal inputs including:
//! - Very large inputs (>1MB)
//! - Inputs with special patterns (all zeros, all 0xFF)
//! - Malformed data structures

use abseil::absl_hash::{
    deterministic_hash, djb2_hash, fnv_hash, fnv_hash_128, fnv_hash_32, hash_of, highway_hash,
    murmur3_64, murmur3_mix, siphash_24, wyhash, xxhash3_64, xxhash_64, BloomFilter,
};
use abseil::absl_memory::MemoryRegion;

#[test]
fn test_hash_with_empty_slice() {
    // Test all hash functions with empty input
    let empty: &[u8] = &[];

    // These should not panic and return consistent results
    let h1 = fnv_hash(empty);
    let h2 = fnv_hash_32(empty);
    let h3 = fnv_hash_128(empty);
    let h4 = murmur3_mix(0, empty);
    let h5 = murmur3_64(empty);
    let h6 = xxhash_64(empty);
    let h7 = xxhash3_64(empty);
    let h8 = djb2_hash(empty);
    let h9 = deterministic_hash(empty);

    // Verify they're deterministic
    assert_eq!(fnv_hash(empty), h1);
    assert_eq!(murmur3_64(empty), h5);
}

#[test]
fn test_hash_with_single_byte() {
    // Test hash functions with single byte inputs
    let single = &[42u8];

    let h1 = fnv_hash(single);
    let h2 = murmur3_64(single);
    let h3 = xxhash_64(single);

    // Should not panic
    let _ = (h1, h2, h3);
}

#[test]
fn test_hash_with_all_zeros() {
    // Test hash functions with all-zero inputs
    let zeros_1k = vec![0u8; 1024];
    let zeros_1m = vec![0u8; 1_048_576];

    // Small zero input
    let h1 = fnv_hash(&zeros_1k);
    let h2 = murmur3_64(&zeros_1k);
    let h3 = xxhash_64(&zeros_1k);

    // Large zero input (1MB)
    let h4 = fnv_hash(&zeros_1m);
    let h5 = murmur3_64(&zeros_1m);
    let h6 = xxhash_64(&zeros_1m);

    // Results should be deterministic
    assert_eq!(fnv_hash(&zeros_1k), h1);
    assert_eq!(murmur3_64(&zeros_1m), h5);
}

#[test]
fn test_hash_with_all_ones() {
    // Test hash functions with all-0xFF inputs
    let ones_1k = vec![0xFFu8; 1024];
    let ones_1m = vec![0xFFu8; 1_048_576];

    let h1 = fnv_hash(&ones_1k);
    let h2 = murmur3_64(&ones_1k);
    let h3 = xxhash_64(&ones_1k);

    let h4 = fnv_hash(&ones_1m);
    let h5 = murmur3_64(&ones_1m);
    let h6 = xxhash_64(&ones_1m);

    // Should not panic
    let _ = (h1, h2, h3, h4, h5, h6);
}

#[test]
fn test_hash_with_alternating_pattern() {
    // Test with alternating bit pattern
    let pattern_1k: Vec<u8> = (0..1024)
        .map(|i| if i % 2 == 0 { 0xAA } else { 0x55 })
        .collect();

    let h1 = fnv_hash(&pattern_1k);
    let h2 = murmur3_64(&pattern_1k);
    let h3 = xxhash_64(&pattern_1k);

    // Should not panic
    let _ = (h1, h2, h3);
}

#[test]
fn test_hash_with_incrementing_pattern() {
    // Test with incrementing byte pattern
    let pattern: Vec<u8> = (0..256).cycle().take(1024).collect();

    let h1 = fnv_hash(&pattern);
    let h2 = murmur3_64(&pattern);
    let h3 = xxhash_64(&pattern);

    // Should not panic
    let _ = (h1, h2, h3);
}

#[test]
fn test_hash_with_large_input_1mb() {
    // Test hash functions with 1MB input
    let large = vec![0xABu8; 1_048_576];

    let h1 = fnv_hash(&large);
    let h2 = murmur3_64(&large);
    let h3 = xxhash_64(&large);
    let h4 = xxhash3_64(&large);

    // Should not panic
    let _ = (h1, h2, h3, h4);
}

#[test]
fn test_hash_with_large_input_10mb() {
    // Test hash functions with 10MB input
    let large = vec![0xCDu8; 10_485_760];

    let h1 = fnv_hash(&large);
    let h2 = murmur3_64(&large);
    let h3 = xxhash_64(&large);

    // Should not panic
    let _ = (h1, h2, h3);
}

#[test]
fn test_hash_with_very_large_input() {
    // Test hash functions with larger input (100MB - may be slow, so we skip if needed)
    if std::env::var("CI").is_ok() {
        // Skip in CI to avoid timeouts
        return;
    }

    let large = vec![0xEFu8; 100 * 1_048_576];

    let h1 = fnv_hash(&large);
    let h2 = murmur3_64(&large);
    let h3 = xxhash_64(&large);

    // Should not panic
    let _ = (h1, h2, h3);
}

#[test]
fn test_bloom_filter_empty() {
    // Test BloomFilter with no inserts
    let bloom = BloomFilter::new(1000, 3);

    assert!(!bloom.contains(&"anything"));
    assert!(!bloom.contains(&42u32));
    assert!(!bloom.contains(&0u8));
}

#[test]
fn test_bloom_filter_all_same_value() {
    // Test BloomFilter when inserting the same value many times
    let mut bloom = BloomFilter::new(1000, 3);

    for _ in 0..100 {
        bloom.insert(&42);
    }

    assert!(bloom.contains(&42));
    assert!(!bloom.contains(&43));
}

#[test]
fn test_bloom_filter_zero_values() {
    // Test BloomFilter with various zero representations
    let mut bloom = BloomFilter::new(100, 5);

    bloom.insert(&0u8);
    bloom.insert(&0u16);
    bloom.insert(&0u32);
    bloom.insert(&0u64);
    bloom.insert(&0i8);
    bloom.insert(&0i16);
    bloom.insert(&0i32);
    bloom.insert(&0i64);
    bloom.insert(&0.0f32);
    bloom.insert(&0.0f64);
    bloom.insert(&"");

    // All zeros should be present
    assert!(bloom.contains(&0u32));
    assert!(bloom.contains(&0i32));
    assert!(bloom.contains(&0.0f32));
}

#[test]
fn test_bloom_filter_max_values() {
    // Test BloomFilter with max values for various types
    let mut bloom = BloomFilter::new(100, 5);

    bloom.insert(&u8::MAX);
    bloom.insert(&u16::MAX);
    bloom.insert(&u32::MAX);
    bloom.insert(&u64::MAX);
    bloom.insert(&i8::MAX);
    bloom.insert(&i16::MAX);
    bloom.insert(&i32::MAX);
    bloom.insert(&i64::MAX);

    // All max values should be present
    assert!(bloom.contains(&u8::MAX));
    assert!(bloom.contains(&u32::MAX));
    assert!(bloom.contains(&i32::MAX));
}

#[test]
fn test_bloom_filter_negative_values() {
    // Test BloomFilter with negative values
    let mut bloom = BloomFilter::new(100, 5);

    bloom.insert(&-1i8);
    bloom.insert(&-1i16);
    bloom.insert(&-1i32);
    bloom.insert(&-1i64);
    bloom.insert(&i8::MIN);
    bloom.insert(&i16::MIN);
    bloom.insert(&i32::MIN);
    bloom.insert(&i64::MIN);

    assert!(bloom.contains(&-1i32));
    assert!(bloom.contains(&i32::MIN));
}

#[test]
fn test_bloom_filter_special_floats() {
    // Test BloomFilter with special float values
    let mut bloom = BloomFilter::new(100, 3);

    bloom.insert(&f32::INFINITY);
    bloom.insert(&f32::NEG_INFINITY);
    bloom.insert(&f32::NAN);
    bloom.insert(&f64::INFINITY);
    bloom.insert(&f64::NEG_INFINITY);
    bloom.insert(&f64::NAN);

    // NaN is tricky because NaN != NaN, but we can test infinity
    assert!(bloom.contains(&f32::INFINITY));
    assert!(bloom.contains(&f32::NEG_INFINITY));
}

#[test]
fn test_bloom_filter_unicode_strings() {
    // Test BloomFilter with various Unicode strings
    let mut bloom = BloomFilter::new(100, 3);

    bloom.insert(&"Hello");
    bloom.insert(&"こんにちは");
    bloom.insert(&"안녕하세요");
    bloom.insert(&"🎉🎊");
    bloom.insert(&"مرحبا");
    bloom.insert(&"שלום");

    assert!(bloom.contains(&"Hello"));
    assert!(bloom.contains(&"こんにちは"));
    assert!(bloom.contains(&"🎉🎊"));
}

#[test]
fn test_memory_region_empty() {
    // Test MemoryRegion with zero size
    let region = MemoryRegion::new(0x1000, 0x1000);

    assert!(region.is_empty());
    assert_eq!(region.size(), 0);
    assert!(!region.contains(0x1000)); // Empty region contains nothing
    assert!(!region.contains(0x0FFF));
}

#[test]
fn test_memory_region_single_byte() {
    // Test MemoryRegion with single byte
    let region = MemoryRegion::new(0x1000, 0x1001);

    assert!(!region.is_empty());
    assert_eq!(region.size(), 1);
    assert!(region.contains(0x1000));
    assert!(!region.contains(0x1001));
}

#[test]
fn test_memory_region_at_zero() {
    // Test MemoryRegion starting at address 0
    let region = MemoryRegion::new(0, 0x1000);

    assert!(region.contains(0));
    assert!(region.contains(0x500));
    assert!(!region.contains(0x1000));
}

#[test]
fn test_memory_region_overlap_touching() {
    // Test that touching regions don't overlap (end is exclusive)
    let region1 = MemoryRegion::new(0x1000, 0x2000);
    let region2 = MemoryRegion::new(0x2000, 0x3000);

    assert!(!region1.overlaps(&region2));
    assert!(!region2.overlaps(&region1));
}

#[test]
fn test_hash_collision_detection() {
    // Test that different inputs can have the same hash (hash collision)
    // This is expected behavior for hash functions

    let input1 = &[1u8, 2, 3, 4];
    let input2 = &[5u8, 6, 7, 8];

    let h1 = fnv_hash(input1);
    let h2 = fnv_hash(input2);

    // They *might* collide, but probably won't
    // This is just to verify the function works
    if h1 == h2 {
        // Collision detected - this is acceptable
    } else {
        // No collision - also acceptable
    }
}

#[test]
fn test_hash_of_special_types() {
    // Test hash_of with special types

    // Option
    assert_eq!(hash_of(&Option::<u32>::None), hash_of(&Option::<u32>::None));
    assert_ne!(hash_of(&Some(42u32)), hash_of(&Option::<u32>::None));

    // Result
    assert_eq!(hash_of(&Ok::<u32, &str>(42)), hash_of(&Ok::<u32, &str>(42)));
    assert_ne!(
        hash_of(&Ok::<u32, &str>(42)),
        hash_of(&Err::<&str, u32>("error"))
    );

    // Array
    assert_eq!(hash_of(&[1, 2, 3]), hash_of(&[1, 2, 3]));
    assert_ne!(hash_of(&[1, 2, 3]), hash_of(&[1, 2, 4]));

    // Tuple
    assert_eq!(hash_of(&(1, 2, 3)), hash_of(&(1, 2, 3)));
    assert_ne!(hash_of(&(1, 2, 3)), hash_of(&(1, 2, 4)));
}

#[test]
fn test_bloom_filter_clear_empty() {
    // Test clearing an empty BloomFilter
    let mut bloom = BloomFilter::new(100, 3);

    bloom.clear(); // Should not panic
    assert!(!bloom.contains(&"anything"));
}

#[test]
fn test_bloom_filter_clear_full() {
    // Test clearing a populated BloomFilter
    let mut bloom = BloomFilter::new(100, 3);

    for i in 0..100 {
        bloom.insert(&i);
    }

    for i in 0..100 {
        assert!(bloom.contains(&i));
    }

    bloom.clear();

    for i in 0..100 {
        assert!(!bloom.contains(&i));
    }
}

#[test]
fn test_bloom_filter_false_positive_rate() {
    // Test that false positives exist (as expected for Bloom filter)
    let mut bloom = BloomFilter::new(1000, 7);

    // Insert 100 items
    for i in 0..100 {
        bloom.insert(&i);
    }

    // Check that all inserted items are present
    for i in 0..100 {
        assert!(bloom.contains(&i), "Should contain {}", i);
    }

    // Check for false positives among non-inserted items
    let mut false_positives = 0;
    for i in 100..200 {
        if bloom.contains(&i) {
            false_positives += 1;
        }
    }

    // There should be some false positives (expected behavior)
    // but not too many (depends on capacity and num_hashes)
    // We just verify the test runs without panicking
    println!("False positives: {} out of 100", false_positives);
}
