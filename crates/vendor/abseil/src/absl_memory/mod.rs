//! Memory utilities.
//!
//! This module provides memory utilities similar to Abseil's `absl/memory` directory.
//!
//! # Overview
//!
//! Memory utilities provide safe and efficient memory operations including
//! alignment helpers, memory size calculations, pointer operations, and
//! memory management utilities.
//!
//! # Components
//!
//! - [`Alignment`] - Type-safe memory alignment representation
//! - [`MemoryUnit`] - Human-readable memory size units
//! - [`Bytes`] - Strongly-typed byte slice wrapper
//! - [`MemoryRegion`] - Region of memory with bounds checking
//! - [`MemoryArena`] - Simple bump-pointer arena allocator
//!
//! # Examples
//!
//! ```rust
//! use abseil::absl_memory::{align_up, is_aligned, size_of};
//!
//! // Align a pointer
//! let aligned = align_up(0x1001, 8); // Returns 0x1008
//!
//! // Check alignment
//! assert!(is_aligned(0x1000, 16));
//!
//! // Get type size
//! assert_eq!(size_of::<u32>(), 4);
//! ```


extern crate alloc;

use alloc::vec::Vec;
use core::fmt;

pub mod arena;
pub mod bytes;
pub mod fixed_buffer;
pub mod mem_block;
pub mod mem_diff;
pub mod mem_ops;
pub mod mem_span;
pub mod mem_stats;
pub mod mem_view;
pub mod memory;
pub mod memory_region;
pub mod pool;
pub mod ptr_utils;
pub mod tagged_ptr;
pub mod tracked_arena;

// Keep the memory submodule for existing exports
pub use memory::{
    Alignment, MemoryUnit,
    align_of, size_of, slice_align_of,
    align_up, align_down, is_aligned,
    alignment_offset, alignment_mask,
    next_power_of_two, prev_power_of_two,
    is_valid_allocation_size, is_valid_count,
    checked_allocation_size,
    memory_unit, with_aligned_stack,
};

// Re-exports from bytes
pub use bytes::Bytes;

// Re-exports from memory_region
pub use memory_region::MemoryRegion;

// Re-exports from arena
pub use arena::MemoryArena;

// Re-exports from pool
pub use pool::MemoryPool;

// Re-exports from mem_ops
pub use mem_ops::{MemEq, bzero, memcmp, memcmp_ord, memcpy, memchr, memmove, memset, strlen, memrchr};

// Re-exports from mem_diff
pub use mem_diff::{count_matching_bytes, find_first_diff, memdiff, MemDiff};

// Re-exports from fixed_buffer
pub use fixed_buffer::FixedBuffer;

// Re-exports from tagged_ptr
pub use tagged_ptr::TaggedPtr;

// Re-exports from mem_span
pub use mem_span::MemSpan;

// Re-exports from mem_stats
pub use mem_stats::MemStats;

// Re-exports from tracked_arena
pub use tracked_arena::TrackedArena;

// Re-exports from ptr_utils
pub use ptr_utils::{align_offset_for_ptr, align_ptr_down, align_ptr_up, is_aligned_ptr};

// Re-exports from mem_view
pub use mem_view::MemView;

// Re-exports from mem_block
pub use mem_block::MemBlock;

#[cfg(test)]
mod tests {
    use super::*;

    // Bytes tests
    #[test]
    fn test_bytes_new() {
        let data = vec![1, 2, 3, 4];
        let bytes = Bytes::new(&data);
        assert_eq!(bytes.len(), 4);
        assert!(!bytes.is_empty());
    }

    // MemoryRegion tests
    #[test]
    fn test_memory_region_new() {
        let region = MemoryRegion::new(0x1000, 0x2000);
        assert_eq!(region.start(), 0x1000);
        assert_eq!(region.end(), 0x2000);
        assert_eq!(region.size(), 0x1000);
    }

    // MemoryArena tests
    #[test]
    fn test_memory_arena_allocate() {
        let mut arena = MemoryArena::new(1024);
        let ptr = arena.allocate(100);
        assert!(ptr.is_some());
        assert_eq!(arena.cursor(), 100);
    }

    // MemoryPool tests
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

    // mem_ops tests
    #[test]
    fn test_memcmp() {
        let a = b"hello";
        let b = b"hello";
        assert!(memcmp(a, b));
    }

    // mem_diff tests
    #[test]
    fn test_memdiff_identical() {
        let a = b"hello";
        let b = b"hello";
        let diff = memdiff(a, b);
        assert!(diff.is_identical());
    }

    // FixedBuffer tests
    #[test]
    fn test_fixed_buffer_new() {
        let buffer: FixedBuffer<16> = FixedBuffer::new();
        assert_eq!(buffer.capacity(), 16);
        assert!(buffer.is_empty());
    }

    // MemSpan tests
    #[test]
    fn test_mem_span_new() {
        let span = MemSpan::new(0x1000, 0x100);
        assert_eq!(span.start, 0x1000);
        assert_eq!(span.length, 0x100);
        assert_eq!(span.end(), 0x1100);
    }

    // MemStats tests
    #[test]
    fn test_mem_stats_new() {
        let stats = MemStats::new();
        assert_eq!(stats.alloc_count, 0);
        assert_eq!(stats.current_usage, 0);
    }

    // TrackedArena tests
    #[test]
    fn test_tracked_arena_new() {
        let arena = TrackedArena::new(1024);
        assert_eq!(arena.stats().alloc_count, 0);
    }

    // ptr_utils tests
    #[test]
    fn test_is_aligned_ptr() {
        let aligned = 0x1000 as *const u8;
        assert!(is_aligned_ptr(aligned, 16));

        let unaligned = 0x1001 as *const u8;
        assert!(!is_aligned_ptr(unaligned, 16));
    }

    // MemView tests
    #[test]
    fn test_mem_view_new() {
        let data = vec![1, 2, 3, 4, 5];
        let view = MemView::new(&data);
        assert_eq!(view.len(), 5);
        assert!(!view.is_empty());
    }

    // MemBlock tests
    #[test]
    fn test_mem_block_with_capacity() {
        let block = MemBlock::with_capacity(100);
        assert_eq!(block.capacity(), 100);
        assert!(block.is_empty());
    }
}
