//! Arena module - bump-pointer arena allocator.

use alloc::vec::Vec;
use core::ptr;

/// A simple bump-pointer arena allocator.
///
/// This allocator allocates memory linearly from a pre-allocated buffer.
/// Allocations can never be freed individually; the entire arena must
/// be reset at once.
///
/// # Thread Safety
///
/// `MemoryArena` is `Send` but NOT `Sync`. This means:
/// - You can move an arena between threads
/// - You CANNOT share an arena across threads via `&MemoryArena`
/// - For thread-safe sharing, wrap in `Mutex<MemoryArena>` or `RwLock<MemoryArena>`
///
/// # Examples
///
/// ```rust
//! use abseil::absl_memory::MemoryArena;
//!
//! let mut arena = MemoryArena::new(1024);
//! let ptr1 = arena.allocate(32);
//! let ptr2 = arena.allocate(64);
//!
//! arena.reset(); // Free everything
//! let ptr3 = arena.allocate(128); // Reuses the space
/// ```
pub struct MemoryArena {
    buffer: Vec<u8>,
    cursor: usize,
}

impl MemoryArena {
    /// Creates a new arena with the given capacity.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use abseil::absl_memory::MemoryArena;
    ///
    /// let arena = MemoryArena::new(4096);
    /// ```
    pub fn new(capacity: usize) -> Self {
        Self {
            buffer: Vec::with_capacity(capacity),
            cursor: 0,
        }
    }

    /// Allocates `size` bytes from the arena.
    ///
    /// Returns a pointer to the allocated memory, or None if out of space.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use abseil::absl_memory::MemoryArena;
    ///
    /// let mut arena = MemoryArena::new(1024);
    /// let ptr = arena.allocate(100);
    /// assert!(ptr.is_some());
    /// ```
    pub fn allocate(&mut self, size: usize) -> Option<*mut u8> {
        let new_cursor = self.cursor.checked_add(size)?;
        if new_cursor > self.buffer.capacity() {
            return None;
        }

        // Ensure buffer has enough space
        while self.buffer.len() < new_cursor {
            self.buffer.push(0);
        }

        let ptr = unsafe { self.buffer.as_mut_ptr().add(self.cursor) };

        self.cursor = new_cursor;
        Some(ptr)
    }

    /// Allocates and initializes memory.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use abseil::absl_memory::MemoryArena;
    ///
    /// let mut arena = MemoryArena::new(1024);
    /// let ptr = arena.allocate_initialized(4, &[1, 2, 3, 4]);
    /// ```
    pub fn allocate_initialized(&mut self, size: usize, data: &[u8]) -> Option<*mut u8> {
        let ptr = self.allocate(size)?;
        unsafe {
            ptr::copy_nonoverlapping(data.as_ptr(), ptr, data.len());
        }
        Some(ptr)
    }

    /// Resets the arena, freeing all allocations.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use abseil::absl_memory::MemoryArena;
    ///
    /// let mut arena = MemoryArena::new(1024);
    /// arena.allocate(100);
    /// arena.reset(); // All memory is freed
    /// ```
    pub fn reset(&mut self) {
        self.cursor = 0;
    }

    /// Returns the current cursor position (bytes allocated).
    pub const fn cursor(&self) -> usize {
        self.cursor
    }

    /// Returns the remaining capacity in bytes.
    pub fn remaining(&self) -> usize {
        self.buffer.capacity() - self.cursor
    }

    /// Returns the total capacity of the arena.
    pub const fn capacity(&self) -> usize {
        self.buffer.capacity()
    }
}

// SAFETY: MemoryArena is Send because Vec<u8> and usize are Send.
// You can safely move the entire arena to another thread.
// MemoryArena is NOT Sync because sharing &MemoryArena across threads
// would allow data races through the mutable cursor and buffer.
// For thread-safe sharing, use Mutex<MemoryArena> or similar.
unsafe impl Send for MemoryArena {}

// Explicitly mark MemoryArena as !Sync to prevent unsafe concurrent access
// If someone tries to share &MemoryArena across threads, they'll get a compile error
impl !Sync for MemoryArena {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_arena_allocate() {
        let mut arena = MemoryArena::new(1024);
        let ptr = arena.allocate(100);
        assert!(ptr.is_some());
        assert_eq!(arena.cursor(), 100);
    }

    #[test]
    fn test_memory_arena_reset() {
        let mut arena = MemoryArena::new(1024);
        arena.allocate(100);
        arena.reset();
        assert_eq!(arena.cursor(), 0);
    }
}
