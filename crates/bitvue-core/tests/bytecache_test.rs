#![allow(dead_code)]
//! Tests for ByteCache system

use bitvue_core::ByteCache;
use std::path::Path;

#[test]
fn test_bytecache_creation() {
    // Test creating ByteCache from test file
    let test_file = Path::new("test_data/av1_test.ivf");

    if !test_file.exists() {
        println!("Test file not found, skipping");
        return;
    }

    let cache = ByteCache::new(test_file, 64 * 1024, 10 * 1024 * 1024);
    assert!(cache.is_ok(), "Should create ByteCache successfully");
}

#[test]
fn test_bytecache_read_range() {
    let test_file = Path::new("test_data/av1_test.ivf");

    if !test_file.exists() {
        return;
    }

    let cache = ByteCache::new(test_file, 64 * 1024, 10 * 1024 * 1024).unwrap();

    // Read first 100 bytes
    let result = cache.read_range(0, 100);
    assert!(result.is_ok(), "Should read range successfully");

    if let Ok(data) = result {
        assert_eq!(data.len(), 100, "Should read exactly 100 bytes");
    }
}

#[test]
fn test_bytecache_read_boundary() {
    let test_file = Path::new("test_data/av1_test.ivf");

    if !test_file.exists() {
        return;
    }

    let cache = ByteCache::new(test_file, 64 * 1024, 10 * 1024 * 1024).unwrap();

    // Read at page boundary
    let page_size = 64 * 1024;
    let result = cache.read_range(page_size - 50, 100);
    assert!(result.is_ok(), "Should read across page boundary");
}

#[test]
fn test_bytecache_cache_efficiency() {
    let test_file = Path::new("test_data/av1_test.ivf");

    if !test_file.exists() {
        return;
    }

    let cache = ByteCache::new(test_file, 64 * 1024, 1024 * 1024).unwrap();

    // Read same range multiple times - should hit cache
    for _ in 0..10 {
        let result = cache.read_range(0, 1000);
        assert!(result.is_ok());
    }
}

#[test]
fn test_bytecache_large_read() {
    let test_file = Path::new("test_data/av1_test.ivf");

    if !test_file.exists() {
        return;
    }

    let cache = ByteCache::new(test_file, 64 * 1024, 10 * 1024 * 1024).unwrap();

    // Read 1MB
    let result = cache.read_range(0, 1024 * 1024);
    assert!(result.is_ok(), "Should read large chunk");
}

#[test]
fn test_bytecache_sequential_reads() {
    let test_file = Path::new("test_data/av1_test.ivf");

    if !test_file.exists() {
        return;
    }

    let cache = ByteCache::new(test_file, 64 * 1024, 10 * 1024 * 1024).unwrap();

    // Sequential reads
    let chunk_size = 1000usize;
    for i in 0..10 {
        let offset = (i * chunk_size) as u64;
        let result = cache.read_range(offset, chunk_size);
        assert!(result.is_ok(), "Sequential read {} should succeed", i);
    }
}

#[test]
fn test_bytecache_random_access() {
    let test_file = Path::new("test_data/av1_test.ivf");

    if !test_file.exists() {
        return;
    }

    let cache = ByteCache::new(test_file, 64 * 1024, 10 * 1024 * 1024).unwrap();

    // Random access pattern
    let offsets = vec![0, 50000, 1000, 100000, 500];
    for offset in offsets {
        let result = cache.read_range(offset, 100);
        assert!(result.is_ok(), "Random read at {} should succeed", offset);
    }
}

#[test]
fn test_bytecache_memory_limit() {
    let test_file = Path::new("test_data/av1_test.ivf");

    if !test_file.exists() {
        return;
    }

    // Small memory limit
    let cache = ByteCache::new(test_file, 4096, 100 * 1024).unwrap();

    // Read more than memory limit - should evict old pages
    for i in 0..50 {
        let offset = i * 10000;
        let result = cache.read_range(offset, 1000);
        assert!(result.is_ok(), "Should handle memory pressure");
    }
}

#[test]
fn test_bytecache_zero_length_read() {
    let test_file = Path::new("test_data/av1_test.ivf");

    if !test_file.exists() {
        return;
    }

    let cache = ByteCache::new(test_file, 64 * 1024, 10 * 1024 * 1024).unwrap();

    let result = cache.read_range(0, 0);
    assert!(result.is_ok(), "Zero-length read should succeed");

    if let Ok(data) = result {
        assert_eq!(data.len(), 0);
    }
}

#[test]
fn test_bytecache_out_of_bounds() {
    let test_file = Path::new("test_data/av1_test.ivf");

    if !test_file.exists() {
        return;
    }

    let cache = ByteCache::new(test_file, 64 * 1024, 10 * 1024 * 1024).unwrap();

    // Try to read beyond file size
    let _result = cache.read_range(u64::MAX, 100);
    // Should either error or return partial data
}
