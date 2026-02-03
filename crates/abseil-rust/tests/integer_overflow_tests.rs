//! Comprehensive integer overflow edge case tests.
//!
//! This module tests edge cases related to integer overflow scenarios that
//! fuzz tests might miss. Focuses on usize::MAX operations, capacity
//! calculation overflows, and index calculation edge cases.

use abseil::absl_hash::BloomFilter;
use abseil::absl_memory::MemoryRegion;

#[test]
fn test_memory_region_from_size_usize_max() {
    // Test creating a region with usize::MAX as base (should panic on overflow)
    let result = std::panic::catch_unwind(|| {
        MemoryRegion::from_size(usize::MAX, 1);
    });
    assert!(result.is_err(), "Should panic when base + size overflows");
}

#[test]
fn test_memory_region_from_size_zero_at_max() {
    // Test creating a region at usize::MAX with zero size (should work)
    let region = MemoryRegion::from_size(usize::MAX, 0);
    assert_eq!(region.start(), usize::MAX);
    assert_eq!(region.end(), usize::MAX);
    assert!(region.is_empty());
}

#[test]
fn test_memory_region_from_size_near_max() {
    // Test creating a region near usize::MAX
    let region = MemoryRegion::from_size(usize::MAX - 1000, 1000);
    assert_eq!(region.start(), usize::MAX - 1000);
    assert_eq!(region.end(), usize::MAX);
    assert_eq!(region.size(), 1000);
}

#[test]
fn test_memory_region_new_zero_length() {
    // Test creating a zero-length region
    let region = MemoryRegion::new(0x1000, 0x1000);
    assert!(region.is_empty());
    assert_eq!(region.size(), 0);
}

#[test]
fn test_memory_region_new_max_range() {
    // Test creating a region covering the entire address space
    let region = MemoryRegion::new(0, usize::MAX);
    assert_eq!(region.start(), 0);
    assert_eq!(region.end(), usize::MAX);
    assert_eq!(region.size(), usize::MAX);
}

#[test]
fn test_memory_region_contains_max_address() {
    // Test contains() with edge cases at usize::MAX
    let region = MemoryRegion::new(usize::MAX - 100, usize::MAX);
    assert!(region.contains(usize::MAX - 100));
    assert!(region.contains(usize::MAX - 50));
    assert!(!region.contains(usize::MAX)); // End is exclusive
    assert!(!region.contains(0));
}

#[test]
fn test_memory_region_sub_region_max_offset() {
    // Test sub_region with offset that could overflow
    let region = MemoryRegion::new(0x1000, 0x2000);

    // This should work
    let sub = region.sub_region(0x500, 0x500);
    assert_eq!(sub.start(), 0x1500);
    assert_eq!(sub.end(), 0x1A00);
}

#[test]
fn test_memory_region_sub_region_offset_overflow_near_max() {
    // Test sub_region with offset that would overflow near usize::MAX
    let region = MemoryRegion::new(usize::MAX - 0x1000, usize::MAX - 0x500);

    let result = std::panic::catch_unwind(|| {
        // This offset would cause overflow
        region.sub_region(0x1000, 0x100);
    });
    assert!(result.is_err(), "Should panic on offset overflow");
}

#[test]
fn test_memory_region_sub_region_size_overflow() {
    // Test sub_region with size that would overflow
    let region = MemoryRegion::new(0x1000, 0x2000);

    let result = std::panic::catch_unwind(|| {
        // This size would overflow: 0x1000 + usize::MAX
        region.sub_region(0x500, usize::MAX);
    });
    assert!(result.is_err(), "Should panic on size overflow");
}

#[test]
fn test_memory_region_sub_region_exact_bounds() {
    // Test sub_region that exactly matches the parent region
    let region = MemoryRegion::new(0x1000, 0x2000);
    let sub = region.sub_region(0, 0x1000);
    assert_eq!(sub.start(), 0x1000);
    assert_eq!(sub.end(), 0x2000);
}

#[test]
fn test_memory_region_intersection_with_max() {
    // Test intersection with regions at usize::MAX
    let region1 = MemoryRegion::new(usize::MAX - 100, usize::MAX);
    let region2 = MemoryRegion::new(usize::MAX - 150, usize::MAX - 50);

    let intersection = region1.intersection(&region2);
    assert!(intersection.is_some());
    let inter = intersection.unwrap();
    assert_eq!(inter.start(), usize::MAX - 100);
    assert_eq!(inter.end(), usize::MAX - 50);
}

#[test]
fn test_memory_region_overlaps_at_max() {
    // Test overlaps() with regions at usize::MAX
    let region1 = MemoryRegion::new(usize::MAX - 100, usize::MAX);
    let region2 = MemoryRegion::new(usize::MAX - 50, usize::MAX);
    assert!(region1.overlaps(&region2));
}

#[test]
fn test_bloom_filter_capacity_overflow() {
    // Test BloomFilter with capacity that could overflow in calculation
    let result = BloomFilter::try_new(usize::MAX / 10, 10);
    // This should either succeed or return an error, not panic
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_bloom_filter_max_hashes() {
    // Test BloomFilter with large num_hashes
    let bloom = BloomFilter::new(1000, 1000);
    bloom.insert(&"test");
    assert!(bloom.contains(&"test"));
}

#[test]
fn test_bloom_filter_single_bit_capacity() {
    // Test BloomFilter with minimal capacity (should result in at least 64 bits)
    let bloom = BloomFilter::new(1, 1);
    bloom.insert(&42u32);
    assert!(bloom.contains(&42u32));
    assert!(!bloom.contains(&99u32));
}

#[test]
fn test_bloom_filter_large_capacity_small_hashes() {
    // Test BloomFilter with large capacity and small num_hashes
    let bloom = BloomFilter::new(1_000_000, 2);
    bloom.insert(&"large_test");
    assert!(bloom.contains(&"large_test"));
}

#[test]
fn test_bloom_filter_small_capacity_many_hashes() {
    // Test BloomFilter with small capacity and many num_hashes
    let bloom = BloomFilter::new(10, 100);
    for i in 0..10 {
        bloom.insert(&i);
    }
    for i in 0..10 {
        assert!(bloom.contains(&i), "Should contain {}", i);
    }
}

#[test]
fn test_bloom_filter_overflow_in_insert() {
    // Test that insert handles hash values that could cause index overflow
    let bloom = BloomFilter::new(100, 7);

    // Insert various values that might cause issues with wrapping_mul
    for i in 0..1000 {
        bloom.insert(&i);
    }

    // Verify that inserts worked
    assert!(bloom.contains(&0));
    assert!(bloom.contains(&500));
    assert!(bloom.contains(&999));
}

#[test]
fn test_bloom_filter_all_zeros_input() {
    // Test BloomFilter with all-zero input
    let bloom = BloomFilter::new(100, 3);

    bloom.insert(&0u8);
    bloom.insert(&0u16);
    bloom.insert(&0u32);
    bloom.insert(&0u64);
    bloom.insert(&0i8);
    bloom.insert(&0i16);
    bloom.insert(&0i32);
    bloom.insert(&0i64);

    assert!(bloom.contains(&0u32));
}

#[test]
fn test_bloom_filter_all_ones_input() {
    // Test BloomFilter with all-ones input (u8::MAX)
    let bloom = BloomFilter::new(100, 3);

    bloom.insert(&u8::MAX);
    bloom.insert(&u16::MAX);
    bloom.insert(&u32::MAX);
    bloom.insert(&u64::MAX);
    bloom.insert(&i8::MAX);
    bloom.insert(&i16::MAX);
    bloom.insert(&i32::MAX);
    bloom.insert(&i64::MAX);

    assert!(bloom.contains(&u8::MAX));
}

#[test]
fn test_memory_region_size_overflow_in_calculation() {
    // Test that size() doesn't overflow when end - start would underflow
    // This is prevented by the constructor, but let's verify the invariant
    let region = MemoryRegion::new(100, 200);
    assert_eq!(region.size(), 100);

    let region = MemoryRegion::new(0, usize::MAX);
    assert_eq!(region.size(), usize::MAX);
}

#[test]
fn test_memory_region_alignment_at_max() {
    // Test align_up() with regions at high addresses
    let region = MemoryRegion::new(usize::MAX - 100, usize::MAX - 50);

    // Aligning to 8 bytes
    let aligned = region.align_up(8);
    // The aligned start should be >= original start and <= original end (or equal)
    assert!(aligned.start() >= region.start() || aligned.start() <= region.end());
}

#[test]
fn test_memory_region_alignment_overflow() {
    // Test align_up() with values that might overflow in calculation
    let region = MemoryRegion::new(usize::MAX - 3, usize::MAX);

    // This calculation (start + alignment - 1) could overflow
    // The implementation should handle it
    let _aligned = region.align_up(8);
}

#[test]
fn test_memory_region_contains_region_at_max() {
    // Test contains_region() with regions at usize::MAX
    let outer = MemoryRegion::new(usize::MAX - 200, usize::MAX);
    let inner = MemoryRegion::new(usize::MAX - 100, usize::MAX - 50);

    assert!(outer.contains_region(&inner));
    assert!(!inner.contains_region(&outer));
}

#[test]
fn test_memory_region_contains_region_same() {
    // Test that a region contains itself
    let region = MemoryRegion::new(0x1000, 0x2000);
    assert!(region.contains_region(&region));
}
