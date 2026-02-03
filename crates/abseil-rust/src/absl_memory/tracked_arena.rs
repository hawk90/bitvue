//! Tracked arena module - arena with allocation statistics.

use crate::absl_memory::{arena::MemoryArena, mem_stats::MemStats};

/// A memory arena that tracks allocation statistics.
pub struct TrackedArena {
    arena: MemoryArena,
    stats: MemStats,
}

impl TrackedArena {
    /// Creates a new tracked arena.
    pub fn new(capacity: usize) -> Self {
        Self {
            arena: MemoryArena::new(capacity),
            stats: MemStats::new(),
        }
    }

    /// Allocates from the arena, tracking statistics.
    pub fn allocate(&mut self, size: usize) -> Option<*mut u8> {
        match self.arena.allocate(size) {
            Some(ptr) => {
                self.stats.record_alloc(size);
                Some(ptr)
            }
            None => {
                self.stats.record_failed_alloc();
                None
            }
        }
    }

    /// Resets the arena, tracking deallocation of all memory.
    pub fn reset(&mut self) {
        let usage = self.arena.cursor();
        self.stats.record_dealloc(usage);
        self.arena.reset();
    }

    /// Returns a reference to the statistics.
    pub fn stats(&self) -> &MemStats {
        &self.stats
    }

    /// Returns the underlying arena.
    pub fn arena(&self) -> &MemoryArena {
        &self.arena
    }

    /// Returns the underlying arena mutably.
    pub fn arena_mut(&mut self) -> &mut MemoryArena {
        &mut self.arena
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tracked_arena_new() {
        let arena = TrackedArena::new(1024);
        assert_eq!(arena.stats().alloc_count, 0);
    }

    #[test]
    fn test_tracked_arena_allocate() {
        let mut arena = TrackedArena::new(1024);
        let ptr = arena.allocate(100);
        assert!(ptr.is_some());
        assert_eq!(arena.stats().alloc_count, 1);
        assert_eq!(arena.stats().current_usage, 100);
    }

    #[test]
    fn test_tracked_arena_reset() {
        let mut arena = TrackedArena::new(1024);
        arena.allocate(100);
        arena.allocate(50);
        arena.reset();
        assert_eq!(arena.stats().current_usage, 0);
        assert_eq!(arena.stats().dealloc_count, 1);
    }
}
