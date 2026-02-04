//! Comprehensive stress tests for edge cases.
//!
//! This module performs stress testing to find edge cases that might be missed:
//! - Repeated operations to test for memory leaks
//! - Large data structure operations
//! - Performance boundary tests

use abseil::absl_base::call_once::{call_once, is_done, OnceFlag};
use abseil::absl_hash::BloomFilter;
use abseil::absl_hash::{
    deterministic_hash, djb2_hash, fnv_hash, fnv_hash_128, fnv_hash_32, murmur3_64, murmur3_mix,
    xxhash3_64, xxhash_64,
};
use abseil::absl_memory::MemoryRegion;

#[test]
fn test_once_flag_repeated_creation() {
    // Test creating and dropping many OnceFlags
    for _ in 0..10_000 {
        let flag = OnceFlag::new();
        assert!(!is_done(&flag));
    }
}

#[test]
fn test_once_flag_repeated_calls() {
    // Test many repeated calls to the same OnceFlag
    let flag = OnceFlag::new();
    let mut counter = 0;

    for _ in 0..10_000 {
        call_once(&flag, || {
            counter += 1;
        });
    }

    assert_eq!(counter, 1);
}

#[test]
fn test_once_flag_many_flags() {
    // Test many different OnceFlags
    let flags: Vec<OnceFlag> = (0..1000).map(|_| OnceFlag::new()).collect();

    for flag in &flags {
        assert!(!is_done(flag));
    }

    // Initialize all flags
    for (i, flag) in flags.iter().enumerate() {
        call_once(flag, move || {
            // Each flag should be initialized exactly once
        });
    }

    // All should be done
    for flag in &flags {
        assert!(is_done(flag));
    }
}

#[test]
fn test_memory_region_many_regions() {
    // Test creating many MemoryRegions
    let regions: Vec<MemoryRegion> = (0..10_000)
        .map(|i| MemoryRegion::new(i * 1000, (i + 1) * 1000))
        .collect();

    for (i, region) in regions.iter().enumerate() {
        assert_eq!(region.start(), i * 1000);
        assert_eq!(region.end(), (i + 1) * 1000);
    }
}

#[test]
fn test_memory_region_many_sub_regions() {
    // Test creating many sub-regions
    let region = MemoryRegion::new(0, 1_000_000);

    for i in 0..10_000 {
        let offset = i * 50;
        let size = 50;
        if offset + size <= region.size() {
            let sub = region.sub_region(offset, size);
            assert_eq!(sub.start(), region.start() + offset);
            assert_eq!(sub.size(), size);
        }
    }
}

#[test]
fn test_bloom_filter_many_filters() {
    // Test creating many BloomFilters
    let filters: Vec<BloomFilter> = (0..100).map(|_| BloomFilter::new(1000, 7)).collect();

    for (i, bloom) in filters.iter().enumerate() {
        bloom.insert(&i);
        assert!(bloom.contains(&i));
    }
}

#[test]
fn test_bloom_filter_many_inserts() {
    // Test many inserts to a single BloomFilter
    let mut bloom = BloomFilter::new(100_000, 7);

    for i in 0..10_000 {
        bloom.insert(&i);
    }

    // Verify all inserts
    for i in 0..10_000 {
        assert!(bloom.contains(&i), "Should contain {}", i);
    }
}

#[test]
fn test_bloom_filter_repeated_clear() {
    // Test repeated clear operations
    let mut bloom = BloomFilter::new(1000, 7);

    for round in 0..100 {
        // Insert some values
        for i in 0..100 {
            bloom.insert(&(round * 100 + i));
        }

        // Verify they're present
        for i in 0..100 {
            assert!(bloom.contains(&(round * 100 + i)));
        }

        // Clear
        bloom.clear();

        // Verify they're gone
        for i in 0..100 {
            assert!(!bloom.contains(&(round * 100 + i)));
        }
    }
}

#[test]
fn test_hash_repeated_operations() {
    // Test repeated hash operations
    let data = b"Hello, World!";

    for _ in 0..10_000 {
        let h1 = fnv_hash(data);
        let h2 = murmur3_64(data);
        let h3 = xxhash_64(data);
        let h4 = djb2_hash(data);
        let h5 = deterministic_hash(data);

        // Just verify they produce consistent results
        assert_eq!(fnv_hash(data), h1);
        assert_eq!(murmur3_64(data), h2);
        assert_eq!(xxhash_64(data), h3);
    }
}

#[test]
fn test_hash_many_different_inputs() {
    // Test hashing many different inputs
    let inputs: Vec<Vec<u8>> = (0..1_000)
        .map(|i| format!("test_data_{}", i).into_bytes())
        .collect();

    for input in &inputs {
        let h1 = fnv_hash(input);
        let h2 = murmur3_64(input);
        let h3 = xxhash_64(input);

        // Should not panic
        let _ = (h1, h2, h3);
    }
}

#[test]
fn test_bloom_filter_collision_stress() {
    // Test for false positive rate under stress
    let mut bloom = BloomFilter::new(10_000, 7);

    // Insert 5000 items
    for i in 0..5000 {
        bloom.insert(&i);
    }

    // Check all inserted items are present
    for i in 0..5000 {
        assert!(bloom.contains(&i), "Should contain {}", i);
    }

    // Count false positives
    let mut false_positives = 0;
    for i in 5000..10_000 {
        if bloom.contains(&i) {
            false_positives += 1;
        }
    }

    // False positive rate should be reasonable (< 10%)
    let fp_rate = false_positives as f64 / 5000.0;
    assert!(fp_rate < 0.1, "False positive rate too high: {}", fp_rate);
    println!("False positive rate: {:.2}%", fp_rate * 100.0);
}

#[test]
fn test_memory_region_intersection_stress() {
    // Test many intersection operations
    let region1 = MemoryRegion::new(0, 10_000);

    for i in 0..1000 {
        let region2 = MemoryRegion::new(i * 10, (i + 1) * 10);
        let intersection = region1.intersection(&region2);

        if region2.end() <= region1.end() {
            assert!(intersection.is_some());
        }
    }
}

#[test]
fn test_memory_region_overlaps_stress() {
    // Test many overlap checks
    let region1 = MemoryRegion::new(1000, 2000);

    for i in 0..2000 {
        let region2 = MemoryRegion::new(i, i + 500);

        // Just verify it doesn't panic
        let _ = region1.overlaps(&region2);
    }
}

#[test]
fn test_bloom_filter_unicode_stress() {
    // Test with many Unicode strings
    let mut bloom = BloomFilter::new(10_000, 7);

    let strings: Vec<String> = (0..1000)
        .map(|i| format!("测试_테스트_試験_{}", ["🎉", "🚀", "💻", "🌟", "🔥"][i % 5]))
        .collect();

    for s in &strings {
        bloom.insert(s);
    }

    // Verify all strings are present
    for s in &strings {
        assert!(bloom.contains(s));
    }
}

#[test]
fn test_hash_sized_inputs() {
    // Test hashing with various input sizes
    let sizes = vec![
        1, 2, 3, 4, 5, 7, 8, 15, 16, 17, 31, 32, 33, 63, 64, 65, 127, 128, 129, 255, 256, 257, 511,
        512, 513, 1023, 1024, 2048, 4096, 8192,
    ];

    for size in sizes {
        let data: Vec<u8> = (0..size).map(|i| i as u8).collect();

        let h1 = fnv_hash(&data);
        let h2 = murmur3_64(&data);
        let h3 = xxhash_64(&data);

        // Just verify they don't panic
        let _ = (h1, h2, h3);
    }
}

#[test]
fn test_memory_region_edge_combinations() {
    // Test various edge combinations
    let test_cases = vec![
        (0, 1),
        (0, 100),
        (1, 1),
        (1, 100),
        (100, 100),
        (100, 200),
        (usize::MAX - 100, usize::MAX),
    ];

    for (start, end) in test_cases {
        let region = MemoryRegion::new(start, end);
        assert_eq!(region.start(), start);
        assert_eq!(region.end(), end);
        assert_eq!(region.size(), end - start);
    }
}

#[test]
fn test_bloom_filter_pattern_stress() {
    // Test BloomFilter with various bit patterns
    let patterns: Vec<Vec<u8>> = vec![
        vec![0; 100],             // All zeros
        vec![0xFF; 100],          // All ones
        vec![0xAA; 100],          // Alternating bits (1)
        vec![0x55; 100],          // Alternating bits (2)
        (0..100).collect(),       // Incrementing
        (0..100).rev().collect(), // Decrementing
        vec![0x00, 0xFF, 0xAA, 0x55]
            .iter()
            .cycle()
            .take(100)
            .cloned()
            .collect(), // Pattern
    ];

    for pattern in &patterns {
        let mut bloom = BloomFilter::new(1000, 7);

        // Insert the pattern
        for i in 0..100 {
            bloom.insert(&pattern[i]);
        }

        // Verify all are present
        for i in 0..100 {
            assert!(bloom.contains(&pattern[i]));
        }
    }
}

#[test]
fn test_hash_consistency_stress() {
    // Test hash consistency across many calls
    let data = b"test_data_for_consistency_check";

    let mut fnv_results = vec![];
    let mut murmur_results = vec![];
    let mut xxhash_results = vec![];

    for _ in 0..1000 {
        fnv_results.push(fnv_hash(data));
        murmur_results.push(murmur3_64(data));
        xxhash_results.push(xxhash_64(data));
    }

    // All results should be identical
    assert!(fnv_results.iter().all(|&h| h == fnv_results[0]));
    assert!(murmur_results.iter().all(|&h| h == murmur_results[0]));
    assert!(xxhash_results.iter().all(|&h| h == xxhash_results[0]));
}

#[test]
fn test_bloom_filter_capacity_stress() {
    // Test BloomFilter with various capacities
    let capacities = vec![1, 2, 3, 5, 10, 100, 1000, 10_000];

    for capacity in capacities {
        let mut bloom = BloomFilter::new(capacity, 7);

        // Insert up to capacity
        for i in 0..capacity {
            bloom.insert(&i);
        }

        // Verify all are present
        for i in 0..capacity {
            assert!(bloom.contains(&i));
        }
    }
}

#[test]
fn test_bloom_filter_num_hashes_stress() {
    // Test BloomFilter with various num_hashes
    let hash_counts = vec![1, 2, 3, 5, 7, 10, 15, 20];

    for num_hashes in hash_counts {
        let mut bloom = BloomFilter::new(1000, num_hashes);

        for i in 0..100 {
            bloom.insert(&i);
        }

        // Verify all are present
        for i in 0..100 {
            assert!(bloom.contains(&i));
        }
    }
}

#[test]
fn test_once_flag_memory_stress() {
    // Test that OnceFlag doesn't leak memory
    for _ in 0..1000 {
        let flag = OnceFlag::new();
        let mut counter = 0;

        call_once(&flag, || {
            counter += 1;
        });

        assert_eq!(counter, 1);
        // Flag is dropped here
    }
    // If we got here without running out of memory, we're good
}
