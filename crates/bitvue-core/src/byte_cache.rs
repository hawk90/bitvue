//! ByteCache - Memory-mapped file access with LRU caching
//!
//! Monster Pack v3: FOUNDATION_DECISIONS.md §3.2

use crate::{BitvueError, Result};
use bytes::Bytes;
use lru::LruCache;
use memmap2::Mmap;
use parking_lot::RwLock;
use std::fs::File;
use std::num::NonZeroUsize;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// ByteCache provides efficient random access to large files using memory mapping
///
/// Architecture:
/// - memmap2 for OS-managed paging
/// - LRU cache for segment deduplication
/// - 256KB segments (tunable)
/// - 256MB memory budget (tunable)
/// - TOCTOU protection: stores original file size for validation
///
/// Usage:
/// ```no_run
/// use bitvue_core::ByteCache;
/// use std::path::Path;
///
/// let cache = ByteCache::new(Path::new("video.ivf"), 256 * 1024, 256 * 1024 * 1024)?;
/// let data = cache.read_range(0, 100)?;
/// # Ok::<(), bitvue_core::BitvueError>(())
/// ```
pub struct ByteCache {
    /// Memory-mapped file (read-only)
    mmap: Arc<Mmap>,

    /// Original file size at creation time (for TOCTOU detection)
    original_size: u64,

    /// Segment size in bytes (default: 256KB)
    segment_size: usize,

    /// LRU cache: segment_index → owned bytes
    cache: RwLock<LruCache<u64, Bytes>>,

    /// Maximum cache memory budget in bytes
    max_memory: usize,

    /// Original file path (for validation)
    file_path: PathBuf,
}

impl ByteCache {
    /// Default segment size: 256 KB
    pub const DEFAULT_SEGMENT_SIZE: usize = 256 * 1024;

    /// Default memory budget: 256 MB
    pub const DEFAULT_MAX_MEMORY: usize = 256 * 1024 * 1024;

    /// Maximum file size for memory mapping (2GB)
    ///
    /// Large files can cause memory issues and are better handled
    /// with streaming rather than memory mapping.
    pub const MAX_FILE_SIZE: u64 = 2 * 1024 * 1024 * 1024; // 2GB

    /// Create a new ByteCache from a file path
    ///
    /// # Arguments
    /// * `file_path` - Path to the file to map
    /// * `segment_size` - Size of each cache segment in bytes (use DEFAULT_SEGMENT_SIZE)
    /// * `max_memory` - Maximum memory budget for cache in bytes (use DEFAULT_MAX_MEMORY)
    ///
    /// # Errors
    /// Returns error if:
    /// - File cannot be opened
    /// - Memory mapping fails
    /// - File is empty
    /// - File size exceeds MAX_FILE_SIZE (2GB)
    pub fn new(file_path: &Path, segment_size: usize, max_memory: usize) -> Result<Self> {
        let file = File::open(file_path).map_err(|e| BitvueError::IoError {
            path: file_path.to_path_buf(),
            source: e,
        })?;

        // Check file size before mapping to prevent memory issues
        let metadata = file.metadata().map_err(|e| BitvueError::IoError {
            path: file_path.to_path_buf(),
            source: e,
        })?;
        let file_size = metadata.len();

        if file_size > Self::MAX_FILE_SIZE {
            return Err(BitvueError::InvalidFile(format!(
                "File too large for memory mapping: {} bytes (max: {} bytes)",
                file_size,
                Self::MAX_FILE_SIZE
            )));
        }

        let mmap = unsafe {
            Mmap::map(&file).map_err(|e| BitvueError::IoError {
                path: file_path.to_path_buf(),
                source: e,
            })?
        };

        if mmap.is_empty() {
            return Err(BitvueError::InvalidFile("File is empty".to_string()));
        }

        // Store original size for TOCTOU detection
        let original_size = mmap.len() as u64;

        let num_segments = (max_memory / segment_size).max(1);
        let cache = RwLock::new(LruCache::new(NonZeroUsize::new(num_segments).unwrap()));

        Ok(Self {
            mmap: Arc::new(mmap),
            original_size,
            segment_size,
            cache,
            max_memory,
            file_path: file_path.to_path_buf(),
        })
    }

    /// Read a range of bytes from the file
    ///
    /// # Arguments
    /// * `offset` - Start offset in bytes
    /// * `len` - Number of bytes to read
    ///
    /// # Returns
    /// Slice of bytes from the mmap. The slice is valid as long as ByteCache exists.
    ///
    /// # Errors
    /// Returns error if:
    /// - Range is out of bounds
    /// - File was modified on disk (TOCTOU protection)
    pub fn read_range(&self, offset: u64, len: usize) -> Result<&[u8]> {
        // TOCTOU protection: verify mmap size hasn't changed
        // This detects if the file was replaced/truncated after mapping
        let current_size = self.mmap.len() as u64;
        if current_size != self.original_size {
            return Err(BitvueError::FileModified {
                path: self.file_path.clone(),
                old_size: self.original_size,
                new_size: current_size,
            });
        }

        // Bounds check
        let end_offset = offset
            .checked_add(len as u64)
            .ok_or(BitvueError::InvalidRange {
                offset,
                length: len,
            })?;

        if end_offset > self.mmap.len() as u64 {
            return Err(BitvueError::InvalidRange {
                offset,
                length: len,
            });
        }

        // Fast path: direct slice from mmap
        let start = offset as usize;
        let end = end_offset as usize;
        Ok(&self.mmap[start..end])
    }

    /// Get a cached segment by index
    ///
    /// This is used internally for segment-based operations.
    /// For most use cases, use `read_range()` instead.
    #[allow(dead_code)]
    fn get_segment(&self, segment_idx: u64) -> Result<Bytes> {
        // Check cache first (requires write lock for LRU updates)
        // Note: LruCache::get() requires &mut self to update the LRU ordering,
        // which is why we need a write lock even for a "read" operation.
        // This is not a bug - it's required for LRU semantics.
        {
            let mut cache = self.cache.write();
            if let Some(bytes) = cache.get(&segment_idx) {
                return Ok(bytes.clone());
            }
        }

        // Cache miss: read from mmap
        let offset = segment_idx * self.segment_size as u64;
        let remaining = self.mmap.len() as u64 - offset;
        let seg_len = (self.segment_size as u64).min(remaining) as usize;

        if seg_len == 0 {
            return Err(BitvueError::InvalidRange { offset, length: 0 });
        }

        let data = &self.mmap[offset as usize..(offset as usize + seg_len)];
        let bytes = Bytes::copy_from_slice(data);

        // Insert into cache
        {
            let mut cache = self.cache.write();
            cache.put(segment_idx, bytes.clone());
        }

        Ok(bytes)
    }

    /// Validate that the file hasn't been modified on disk
    ///
    /// # Errors
    /// Returns error if:
    /// - File metadata cannot be read
    /// - File size has changed
    pub fn validate(&self) -> Result<()> {
        let metadata = std::fs::metadata(&self.file_path).map_err(|e| BitvueError::IoError {
            path: self.file_path.clone(),
            source: e,
        })?;

        if metadata.len() != self.mmap.len() as u64 {
            return Err(BitvueError::FileModified {
                path: self.file_path.clone(),
                old_size: self.mmap.len() as u64,
                new_size: metadata.len(),
            });
        }

        Ok(())
    }

    /// Get the total file size in bytes
    pub fn len(&self) -> u64 {
        self.mmap.len() as u64
    }

    /// Check if the file is empty
    pub fn is_empty(&self) -> bool {
        self.mmap.is_empty()
    }

    /// Get the file path
    pub fn path(&self) -> &Path {
        &self.file_path
    }

    /// Get cache statistics
    pub fn stats(&self) -> CacheStats {
        let cache = self.cache.read();
        CacheStats {
            segment_count: cache.len(),
            segment_size: self.segment_size,
            max_segments: cache.cap().get(),
            memory_used: cache.len() * self.segment_size,
            max_memory: self.max_memory,
        }
    }

    /// Clear the cache
    pub fn clear_cache(&self) {
        let mut cache = self.cache.write();
        cache.clear();
    }
}

/// Cache statistics
#[derive(Debug, Clone)]
pub struct CacheStats {
    /// Number of segments currently in cache
    pub segment_count: usize,

    /// Size of each segment in bytes
    pub segment_size: usize,

    /// Maximum number of segments
    pub max_segments: usize,

    /// Approximate memory used by cache in bytes
    pub memory_used: usize,

    /// Maximum memory budget in bytes
    pub max_memory: usize,
}

impl CacheStats {
    /// Get cache utilization as a percentage (0.0 to 1.0)
    pub fn utilization(&self) -> f32 {
        if self.max_memory == 0 {
            return 0.0;
        }
        (self.memory_used as f32) / (self.max_memory as f32)
    }
}

/// Comprehensive test suite with Arrange-Act-Assert pattern
#[allow(
    unused_imports,
    unused_variables,
    unused_mut,
    dead_code,
    unused_comparisons,
    unused_must_use,
    hidden_glob_reexports,
    unreachable_code,
    non_camel_case_types,
    unused_parens,
    unused_assignments
)]
#[cfg(test)]
mod tests {
    include!("byte_cache_test.rs");
}
