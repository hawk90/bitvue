// ByteCache module tests
//
// Per generate-tests skill: Arrange-Act-Assert pattern with fixtures
// and edge case coverage.

use super::*;
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use tempfile::TempDir;

// ============================================================================
// Fixtures
// ============================================================================

/// Create a temporary test file with content
fn create_test_file(size: usize) -> (TempDir, PathBuf) {
    let dir = TempDir::new().unwrap();
    let file_path = dir.path().join("test.ivf");
    let mut file = fs::File::create(&file_path).unwrap();

    // Write some test data
    let data = vec![0u8; size];
    file.write_all(&data).unwrap();

    (dir, file_path)
}

/// Create a temporary test file with pattern
fn create_test_file_with_pattern(pattern: &[u8]) -> (TempDir, PathBuf) {
    let dir = TempDir::new().unwrap();
    let file_path = dir.path().join("test.ivf");
    let mut file = fs::File::create(&file_path).unwrap();

    // Write pattern multiple times
    for _ in 0..100 {
        file.write_all(pattern).unwrap();
    }

    (dir, file_path)
}

// ============================================================================
// CacheStats Tests
// ============================================================================

#[cfg(test)]
mod cache_stats_tests {
    use super::*;

    #[test]
    fn test_cache_stats_utilization_zero() {
        // Arrange
        let stats = CacheStats {
            segment_count: 0,
            segment_size: 1024,
            max_segments: 10,
            memory_used: 0,
            max_memory: 10240,
        };

        // Act
        let utilization = stats.utilization();

        // Assert
        assert_eq!(utilization, 0.0);
    }

    #[test]
    fn test_cache_stats_utilization_half() {
        // Arrange
        let stats = CacheStats {
            segment_count: 5,
            segment_size: 1024,
            max_segments: 10,
            memory_used: 5120,
            max_memory: 10240,
        };

        // Act
        let utilization = stats.utilization();

        // Assert
        assert_eq!(utilization, 0.5);
    }

    #[test]
    fn test_cache_stats_utilization_full() {
        // Arrange
        let stats = CacheStats {
            segment_count: 10,
            segment_size: 1024,
            max_segments: 10,
            memory_used: 10240,
            max_memory: 10240,
        };

        // Act
        let utilization = stats.utilization();

        // Assert
        assert_eq!(utilization, 1.0);
    }

    #[test]
    fn test_cache_stats_utilization_zero_max_memory() {
        // Arrange
        let stats = CacheStats {
            segment_count: 5,
            segment_size: 1024,
            max_segments: 10,
            memory_used: 5120,
            max_memory: 0,
        };

        // Act
        let utilization = stats.utilization();

        // Assert - Should return 0 when max_memory is 0
        assert_eq!(utilization, 0.0);
    }
}

// ============================================================================
// ByteCache::new Tests
// ============================================================================

#[cfg(test)]
mod byte_cache_new_tests {
    use super::*;

    #[test]
    fn test_byte_cache_new_with_defaults() {
        // Arrange
        let (_dir, file_path) = create_test_file(1024);

        // Act
        let cache = ByteCache::new(
            &file_path,
            ByteCache::DEFAULT_SEGMENT_SIZE,
            ByteCache::DEFAULT_MAX_MEMORY,
        );

        // Assert
        assert!(cache.is_ok());
        let cache = cache.unwrap();
        assert_eq!(cache.len(), 1024);
        assert!(!cache.is_empty());
        assert_eq!(cache.path(), file_path);
    }

    #[test]
    fn test_byte_cache_new_custom_segment_size() {
        // Arrange
        let (_dir, file_path) = create_test_file(2048);

        // Act
        let cache = ByteCache::new(&file_path, 512, 1024 * 1024);

        // Assert
        assert!(cache.is_ok());
        let cache = cache.unwrap();
        assert_eq!(cache.segment_size, 512);
    }

    #[test]
    fn test_byte_cache_new_empty_file_error() {
        // Arrange
        let dir = TempDir::new().unwrap();
        let file_path = dir.path().join("empty.ivf");
        fs::File::create(&file_path).unwrap(); // Empty file

        // Act
        let cache = ByteCache::new(&file_path, 256, 1024);

        // Assert
        assert!(cache.is_err());
        if let Err(BitvueError::InvalidFile(_)) = cache {
            // Expected
        } else {
            panic!("Expected InvalidFile error");
        }
    }

    #[test]
    fn test_byte_cache_new_nonexistent_file() {
        // Arrange
        let file_path = PathBuf::from("/nonexistent/path/test.ivf");

        // Act
        let cache = ByteCache::new(&file_path, 256, 1024);

        // Assert
        assert!(cache.is_err());
    }
}

// ============================================================================
// ByteCache::read_range Tests
// ============================================================================

#[cfg(test)]
mod byte_cache_read_range_tests {
    use super::*;

    #[test]
    fn test_read_range_from_start() {
        // Arrange
        let (_dir, file_path) = create_test_file_with_pattern(&[0x00, 0x01, 0x02, 0x03]);
        let cache = ByteCache::new(&file_path, 256, 1024).unwrap();

        // Act
        let data = cache.read_range(0, 4);

        // Assert
        assert!(data.is_ok());
        let data = data.unwrap();
        assert_eq!(data.len(), 4);
        assert_eq!(data[0], 0x00);
        assert_eq!(data[1], 0x01);
        assert_eq!(data[2], 0x02);
        assert_eq!(data[3], 0x03);
    }

    #[test]
    fn test_read_range_middle() {
        // Arrange
        let (_dir, file_path) = create_test_file_with_pattern(&[0x00, 0x01, 0x02, 0x03]);
        let cache = ByteCache::new(&file_path, 256, 1024).unwrap();

        // Act
        let data = cache.read_range(10, 4);

        // Assert
        assert!(data.is_ok());
        let data = data.unwrap();
        assert_eq!(data.len(), 4);
    }

    #[test]
    fn test_read_range_end() {
        // Arrange
        let (_dir, file_path) = create_test_file(400);
        let cache = ByteCache::new(&file_path, 256, 1024).unwrap();

        // Act
        let data = cache.read_range(396, 4);

        // Assert
        assert!(data.is_ok());
        let data = data.unwrap();
        assert_eq!(data.len(), 4);
    }

    #[test]
    fn test_read_range_zero_length() {
        // Arrange
        let (_dir, file_path) = create_test_file(100);
        let cache = ByteCache::new(&file_path, 256, 1024).unwrap();

        // Act
        let data = cache.read_range(50, 0);

        // Assert
        assert!(data.is_ok());
        let data = data.unwrap();
        assert_eq!(data.len(), 0);
    }

    #[test]
    fn test_read_range_out_of_bounds() {
        // Arrange
        let (_dir, file_path) = create_test_file(100);
        let cache = ByteCache::new(&file_path, 256, 1024).unwrap();

        // Act
        let data = cache.read_range(90, 20);

        // Assert
        assert!(data.is_err());
        assert!(matches!(data.unwrap_err(), BitvueError::InvalidRange { .. }));
    }

    #[test]
    fn test_read_range_beyond_end() {
        // Arrange
        let (_dir, file_path) = create_test_file(100);
        let cache = ByteCache::new(&file_path, 256, 1024).unwrap();

        // Act
        let data = cache.read_range(100, 1);

        // Assert
        assert!(data.is_err());
    }

    #[test]
    fn test_read_range_overflow() {
        // Arrange
        let (_dir, file_path) = create_test_file(100);
        let cache = ByteCache::new(&file_path, 256, 1024).unwrap();

        // Act - Read length that would overflow when added to offset
        let data = cache.read_range(u64::MAX - 10, 100);

        // Assert
        assert!(data.is_err());
        assert!(matches!(data.unwrap_err(), BitvueError::InvalidRange { .. }));
    }
}

// ============================================================================
// ByteCache::len Tests
// ============================================================================

#[cfg(test)]
mod byte_cache_len_tests {
    use super::*;

    #[test]
    fn test_len_small_file() {
        // Arrange
        let (_dir, file_path) = create_test_file(100);
        let cache = ByteCache::new(&file_path, 256, 1024).unwrap();

        // Act
        let len = cache.len();

        // Assert
        assert_eq!(len, 100);
    }

    #[test]
    fn test_len_large_file() {
        // Arrange
        let (_dir, file_path) = create_test_file(10000);
        let cache = ByteCache::new(&file_path, 256, 1024).unwrap();

        // Act
        let len = cache.len();

        // Assert
        assert_eq!(len, 10000);
    }
}

// ============================================================================
// ByteCache::is_empty Tests
// ============================================================================

#[cfg(test)]
mod byte_cache_is_empty_tests {
    use super::*;

    #[test]
    fn test_is_empty_false() {
        // Arrange
        let (_dir, file_path) = create_test_file(100);
        let cache = ByteCache::new(&file_path, 256, 1024).unwrap();

        // Act
        let empty = cache.is_empty();

        // Assert
        assert!(!empty);
    }

    #[test]
    fn test_empty_file_returns_error() {
        // Note: Empty files return error from new(), so we can't test is_empty
        // This is documented behavior
    }
}

// ============================================================================
// ByteCache::validate Tests
// ============================================================================

#[cfg(test)]
mod byte_cache_validate_tests {
    use super::*;

    #[test]
    fn test_validate_unchanged() {
        // Arrange
        let (_dir, file_path) = create_test_file(100);
        let cache = ByteCache::new(&file_path, 256, 1024).unwrap();

        // Act
        let result = cache.validate();

        // Assert
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_file_modified() {
        // Arrange
        let (dir, file_path) = create_test_file(100);
        let cache = ByteCache::new(&file_path, 256, 1024).unwrap();

        // Modify file - append data to change size
        {
            let mut file = fs::OpenOptions::new()
                .append(true)
                .open(&file_path)
                .unwrap();
            file.write_all(&[0x01]).unwrap();
            file.flush().unwrap();
        }

        // Act
        let result = cache.validate();

        // Assert
        assert!(result.is_err());
        if let Err(BitvueError::FileModified { .. }) = result {
            // Expected
        } else {
            panic!("Expected FileModified error");
        }

        // Drop tempdir explicitly
        drop(dir);
    }
}

// ============================================================================
// ByteCache::stats Tests
// ============================================================================

#[cfg(test)]
mod byte_cache_stats_tests {
    use super::*;

    #[test]
    fn test_stats_initial() {
        // Arrange
        let (_dir, file_path) = create_test_file(1000);
        let cache = ByteCache::new(&file_path, 256, 1024).unwrap();

        // Act
        let stats = cache.stats();

        // Assert
        assert_eq!(stats.segment_count, 0);
        assert_eq!(stats.segment_size, 256);
        assert!(stats.max_segments > 0);
        assert_eq!(stats.memory_used, 0);
        assert_eq!(stats.max_memory, 1024);
    }

    #[test]
    fn test_stats_after_clear() {
        // Arrange
        let (_dir, file_path) = create_test_file(1000);
        let cache = ByteCache::new(&file_path, 256, 1024).unwrap();

        // Act
        cache.clear_cache();
        let stats = cache.stats();

        // Assert
        assert_eq!(stats.segment_count, 0);
    }
}

// ============================================================================
// ByteCache::clear_cache Tests
// ============================================================================

#[cfg(test)]
mod byte_cache_clear_tests {
    use super::*;

    #[test]
    fn test_clear_cache() {
        // Arrange
        let (_dir, file_path) = create_test_file(1000);
        let cache = ByteCache::new(&file_path, 256, 1024).unwrap();

        // Act
        cache.clear_cache();
        let stats = cache.stats();

        // Assert
        assert_eq!(stats.segment_count, 0);
    }
}

// ============================================================================
// ByteCache::path Tests
// ============================================================================

#[cfg(test)]
mod byte_cache_path_tests {
    use super::*;

    #[test]
    fn test_path() {
        // Arrange
        let (_dir, file_path) = create_test_file(100);
        let cache = ByteCache::new(&file_path, 256, 1024).unwrap();

        // Act
        let path = cache.path();

        // Assert
        assert_eq!(path, file_path);
    }
}

// ============================================================================
// Constants Tests
// ============================================================================

#[cfg(test)]
mod constants_tests {
    use super::*;

    #[test]
    fn test_default_segment_size() {
        // Act & Assert
        assert_eq!(ByteCache::DEFAULT_SEGMENT_SIZE, 256 * 1024);
    }

    #[test]
    fn test_default_max_memory() {
        // Act & Assert
        assert_eq!(ByteCache::DEFAULT_MAX_MEMORY, 256 * 1024 * 1024);
    }
}

// ============================================================================
// Edge Cases
// ============================================================================

#[cfg(test)]
mod edge_case_tests {
    use super::*;

    #[test]
    fn test_read_entire_file() {
        // Arrange
        let (_dir, file_path) = create_test_file(400);
        let cache = ByteCache::new(&file_path, 256, 1024).unwrap();

        // Act
        let data = cache.read_range(0, 400);

        // Assert
        assert!(data.is_ok());
        let data = data.unwrap();
        assert_eq!(data.len(), 400);
    }

    #[test]
    fn test_multiple_sequential_reads() {
        // Arrange
        let (_dir, file_path) = create_test_file_with_pattern(&[0xAA, 0xBB, 0xCC, 0xDD]);
        let cache = ByteCache::new(&file_path, 256, 1024).unwrap();

        // Act - Multiple reads
        let data1 = cache.read_range(0, 4);
        let data2 = cache.read_range(100, 4);
        let data3 = cache.read_range(200, 4);

        // Assert - All should succeed
        assert!(data1.is_ok());
        assert!(data2.is_ok());
        assert!(data3.is_ok());
    }

    #[test]
    fn test_large_segment_size() {
        // Arrange
        let (_dir, file_path) = create_test_file(10000);
        let cache = ByteCache::new(&file_path, 10 * 1024 * 1024, 100 * 1024 * 1024);

        // Assert
        assert!(cache.is_ok());
    }

    #[test]
    fn test_small_max_memory() {
        // Arrange
        let (_dir, file_path) = create_test_file(10000);
        let cache = ByteCache::new(&file_path, 256, 512);

        // Assert - Should work even with small cache
        assert!(cache.is_ok());
        let cache = cache.unwrap();
        assert_eq!(cache.max_memory, 512);
    }
}
