//! Memory pool module - fixed-size allocation pool.

use alloc::vec::Vec;
use alloc::boxed::Box;

/// A pool for reusing fixed-size allocations.
///
/// This is useful for reducing allocation overhead for objects of
/// a fixed size.
///
/// # Safety
///
/// Pointers returned by `acquire()` remain valid as long as the `MemoryPool`
/// is not dropped. The memory is owned by the pool and must be returned
/// using `release()` before the pool is dropped.
///
/// # Examples
///
/// ```rust
//! use abseil::absl_memory::MemoryPool;
//!
//! let mut pool = MemoryPool::new(64, 10);
//!
//! // Allocate from pool
//! if let Some(ptr) = pool.acquire() {
//!     // Use the memory...
//!     pool.release(ptr); // Return to pool
//! }
/// ```
pub struct MemoryPool {
    block_size: usize,
    // SAFETY: Use Box<[u8]> instead of Vec<u8> to ensure stable addresses
    // Box has stable address and won't reallocate, so pointers remain valid
    blocks: Vec<Box<[u8]>>,
    available: Vec<usize>,
}

impl MemoryPool {
    /// Creates a new memory pool.
    ///
    /// # Arguments
    ///
    /// * `block_size` - Size of each block in bytes
    /// * `capacity` - Maximum number of blocks
    ///
    /// # Examples
    ///
    /// ```rust
    /// use abseil::absl_memory::MemoryPool;
    ///
    /// let pool = MemoryPool::new(128, 20);
    /// ```
    pub fn new(block_size: usize, capacity: usize) -> Self {
        Self {
            block_size,
            blocks: Vec::with_capacity(capacity),
            available: Vec::new(),
        }
    }

    /// Acquires a block from the pool.
    ///
    /// Returns None if the pool is exhausted.
    ///
    /// # Safety
    ///
    /// The returned pointer remains valid as long as the MemoryPool is alive.
    /// The pointer must be returned to the pool via `release()` before use.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use abseil::absl_memory::MemoryPool;
    ///
    /// let mut pool = MemoryPool::new(64, 10);
    /// let block = pool.acquire();
    /// ```
    pub fn acquire(&mut self) -> Option<*mut u8> {
        if let Some(idx) = self.available.pop() {
            // SAFETY: blocks[idx] is a Box<[u8]> with stable address
            Some(self.blocks[idx].as_mut_ptr())
        } else if self.blocks.len() < self.blocks.capacity() {
            // SAFETY: Allocate a new block using Vec::into_boxed_slice
            // This creates a Box<[u8]> with stable address that won't reallocate
            let block: Box<[u8]> = vec![0u8; self.block_size].into_boxed_slice();
            let ptr = block.as_mut_ptr();
            self.blocks.push(block);
            Some(ptr)
        } else {
            None
        }
    }

    /// Returns a block to the pool.
    ///
    /// # Safety
    ///
    /// The pointer must have been acquired from this pool and must not be
    /// used after calling this function. Releasing the same pointer twice
    /// is undefined behavior.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use abseil::absl_memory::MemoryPool;
    ///
    /// let mut pool = MemoryPool::new(64, 10);
    /// if let Some(ptr) = pool.acquire() {
    ///     pool.release(ptr);
    /// }
    /// ```
    pub unsafe fn release(&mut self, ptr: *mut u8) {
        let addr = ptr as usize;
        for (idx, block) in self.blocks.iter().enumerate() {
            if block.as_ptr() as usize == addr {
                // SAFETY: Check for double-release by verifying the index
                // is not already in the available list
                if self.available.contains(&idx) {
                    panic!("Double-release detected: pointer {} already released", addr);
                }
                self.available.push(idx);
                return;
            }
        }
        panic!("Pointer not from this pool: {}", addr);
    }

    /// Returns the block size.
    pub const fn block_size(&self) -> usize {
        self.block_size
    }

    /// Returns the number of available blocks.
    pub fn available_count(&self) -> usize {
        self.available.len()
    }

    /// Returns the total number of blocks.
    pub const fn total_blocks(&self) -> usize {
        self.blocks.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_pool_acquire_release() {
        let mut pool = MemoryPool::new(64, 10);
        let ptr = pool.acquire();
        assert!(ptr.is_some());

        if let Some(p) = ptr {
            unsafe { pool.release(p); }
        }
        assert_eq!(pool.available_count(), 1);
    }
}
