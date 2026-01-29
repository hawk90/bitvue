//! Resource limits for security and DoS prevention
//!
//! This module defines constants for resource limits throughout Bitvue.
//! These limits prevent resource exhaustion attacks and ensure the
//! application remains responsive when processing untrusted input.

/// Maximum number of worker threads for parallel processing
///
/// Prevents excessive thread creation which could exhaust system resources.
/// Most video processing operations benefit from 4-16 threads; beyond 32
/// threads typically see diminishing returns due to contention.
pub const MAX_WORKER_THREADS: usize = 32;

/// Minimum number of worker threads (must be at least 1)
pub const MIN_WORKER_THREADS: usize = 1;

/// Maximum size of in-memory caches (in entries)
///
/// Prevents memory exhaustion from unbounded cache growth.
/// Each cache entry can be 1-10 MB depending on frame size.
pub const MAX_CACHE_ENTRIES: usize = 1000;

/// Maximum buffer size for file I/O operations (100 MB)
///
/// Prevents memory exhaustion when reading large files.
/// IVF frames are typically 10-500 KB, so 100 MB is generous.
pub const MAX_BUFFER_SIZE: usize = 100 * 1024 * 1024;

/// Maximum number of frames to process from a single file
///
/// Prevents DoS via files with millions of tiny frames.
/// 100,000 frames at 60fps ≈ 27 minutes of video.
pub const MAX_FRAMES_PER_FILE: usize = 100_000;

/// Maximum frame size in bytes (100 MB)
///
/// Prevents memory exhaustion from malformed frame headers.
/// Real video frames are rarely larger than 10 MB even for 4K.
pub const MAX_FRAME_SIZE: usize = 100 * 1024 * 1024;

/// Maximum recursion depth for parsing nested structures
///
/// Prevents stack overflow from deeply nested structures.
/// AV1/OBU nesting is typically <10 levels deep.
pub const MAX_RECURSION_DEPTH: usize = 100;

/// Validate thread count is within acceptable range
///
/// # Arguments
/// * `thread_count` - Requested number of threads
///
/// # Returns
/// `Ok(())` if valid, `Err(BitvueError)` if out of range
pub fn validate_thread_count(thread_count: usize) -> Result<(), crate::BitvueError> {
    if thread_count < MIN_WORKER_THREADS {
        return Err(crate::BitvueError::InvalidData(format!(
            "Thread count {} is below minimum {}",
            thread_count, MIN_WORKER_THREADS
        )));
    }
    if thread_count > MAX_WORKER_THREADS {
        return Err(crate::BitvueError::InvalidData(format!(
            "Thread count {} exceeds maximum {}",
            thread_count, MAX_WORKER_THREADS
        )));
    }
    Ok(())
}

/// Validate buffer size is within acceptable range
///
/// # Arguments
/// * `buffer_size` - Requested buffer size in bytes
///
/// # Returns
/// `Ok(())` if valid, `Err(BitvueError)` if too large
pub fn validate_buffer_size(buffer_size: usize) -> Result<(), crate::BitvueError> {
    if buffer_size > MAX_BUFFER_SIZE {
        return Err(crate::BitvueError::InvalidData(format!(
            "Buffer size {} bytes exceeds maximum {} bytes",
            buffer_size, MAX_BUFFER_SIZE
        )));
    }
    Ok(())
}

/// Validate cache size is within acceptable range
///
/// # Arguments
/// * `cache_entries` - Requested number of cache entries
///
/// # Returns
/// `Ok(())` if valid, `Err(BitvueError)` if too large
pub fn validate_cache_size(cache_entries: usize) -> Result<(), crate::BitvueError> {
    if cache_entries > MAX_CACHE_ENTRIES {
        return Err(crate::BitvueError::InvalidData(format!(
            "Cache size {} entries exceeds maximum {} entries",
            cache_entries, MAX_CACHE_ENTRIES
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_thread_count_valid() {
        assert!(validate_thread_count(1).is_ok());
        assert!(validate_thread_count(4).is_ok());
        assert!(validate_thread_count(16).is_ok());
        assert!(validate_thread_count(32).is_ok());
    }

    #[test]
    fn test_validate_thread_count_invalid() {
        assert!(validate_thread_count(0).is_err());
        assert!(validate_thread_count(33).is_err());
        assert!(validate_thread_count(100).is_err());
    }

    #[test]
    fn test_validate_buffer_size_valid() {
        assert!(validate_buffer_size(1024).is_ok());
        assert!(validate_buffer_size(10_000_000).is_ok());
        assert!(validate_buffer_size(MAX_BUFFER_SIZE).is_ok());
    }

    #[test]
    fn test_validate_buffer_size_invalid() {
        assert!(validate_buffer_size(MAX_BUFFER_SIZE + 1).is_err());
        assert!(validate_buffer_size(200 * 1024 * 1024).is_err());
    }

    #[test]
    fn test_validate_cache_size_valid() {
        assert!(validate_cache_size(10).is_ok());
        assert!(validate_cache_size(100).is_ok());
        assert!(validate_cache_size(MAX_CACHE_ENTRIES).is_ok());
    }

    #[test]
    fn test_validate_cache_size_invalid() {
        assert!(validate_cache_size(MAX_CACHE_ENTRIES + 1).is_err());
        assert!(validate_cache_size(2000).is_err());
    }
}
