#![allow(hidden_glob_reexports)]
#![allow(unreachable_code)]
#![allow(non_camel_case_types)]
#![allow(unused_assignments)]
#![allow(unused_parens)]
#![allow(unused_imports)]
#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unused_mut)]
#![allow(unused_comparisons)]
#![allow(unused_doc_comments)]
//! Tests for byte_cache module

use crate::byte_cache::ByteCache;
use crate::error::Result;
use std::io::Write;
use tempfile::NamedTempFile;

#[test]
fn test_byte_cache_read_range() -> Result<()> {
    // Create temp file with test data
    let mut file = NamedTempFile::new().unwrap();
    let data = b"Hello, World! This is a test file.";
    file.write_all(data).unwrap();
    file.flush().unwrap();

    let cache = ByteCache::new(
        file.path(),
        ByteCache::DEFAULT_SEGMENT_SIZE,
        ByteCache::DEFAULT_MAX_MEMORY,
    )?;

    // Read full range
    let read_data = cache.read_range(0, data.len())?;
    assert_eq!(read_data, data);

    // Read partial range
    let partial = cache.read_range(7, 5)?;
    assert_eq!(partial, b"World");

    Ok(())
}

#[test]
fn test_byte_cache_bounds_check() -> Result<()> {
    let mut file = NamedTempFile::new().unwrap();
    file.write_all(b"test").unwrap();
    file.flush().unwrap();

    let cache = ByteCache::new(
        file.path(),
        ByteCache::DEFAULT_SEGMENT_SIZE,
        ByteCache::DEFAULT_MAX_MEMORY,
    )?;

    // Valid range
    assert!(cache.read_range(0, 4).is_ok());

    // Out of bounds
    assert!(cache.read_range(0, 5).is_err());
    assert!(cache.read_range(5, 1).is_err());

    Ok(())
}

#[test]
fn test_byte_cache_validate() -> Result<()> {
    let mut file = NamedTempFile::new().unwrap();
    file.write_all(b"test").unwrap();
    file.flush().unwrap();

    let cache = ByteCache::new(
        file.path(),
        ByteCache::DEFAULT_SEGMENT_SIZE,
        ByteCache::DEFAULT_MAX_MEMORY,
    )?;

    // Initial validation should pass
    assert!(cache.validate().is_ok());

    // Modify file (in real scenario, external process would do this)
    // For test, we just check the validation logic exists
    assert_eq!(cache.len(), 4);

    Ok(())
}

#[test]
fn test_cache_stats() -> Result<()> {
    let mut file = NamedTempFile::new().unwrap();
    file.write_all(&vec![0u8; 1024 * 1024]).unwrap(); // 1MB file
    file.flush().unwrap();

    let cache = ByteCache::new(
        file.path(),
        256 * 1024,  // 256KB segments
        1024 * 1024, // 1MB budget
    )?;

    let stats = cache.stats();
    assert_eq!(stats.segment_size, 256 * 1024);
    assert_eq!(stats.max_memory, 1024 * 1024);
    assert_eq!(stats.max_segments, 4); // 1MB / 256KB

    Ok(())
}
