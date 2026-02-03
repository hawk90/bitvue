//! Memory block module - owned block of memory.

use alloc::vec::Vec;

use crate::absl_memory::mem_view::MemView;

/// A block of memory with ownership and size tracking.
pub struct MemBlock {
    data: Vec<u8>,
}

impl MemBlock {
    /// Creates a new memory block with the given capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            data: Vec::with_capacity(capacity),
        }
    }

    /// Creates a new memory block initialized with zeros.
    pub fn new_zeroed(size: usize) -> Self {
        Self {
            data: vec![0; size],
        }
    }

    /// Returns the size of the block.
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// Returns true if the block is empty.
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// Returns the capacity of the block.
    pub fn capacity(&self) -> usize {
        self.data.capacity()
    }

    /// Reserves additional capacity.
    pub fn reserve(&mut self, additional: usize) {
        self.data.reserve(additional);
    }

    /// Clears the block.
    pub fn clear(&mut self) {
        self.data.clear();
    }

    /// Appends bytes to the block.
    pub fn extend(&mut self, bytes: &[u8]) {
        self.data.extend_from_slice(bytes);
    }

    /// Returns a view of the entire block.
    pub fn as_view(&self) -> MemView<'_> {
        MemView::new(&self.data)
    }

    /// Returns a mutable slice of the data.
    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        &mut self.data
    }

    /// Returns a slice of the data.
    pub fn as_slice(&self) -> &[u8] {
        &self.data
    }
}

impl Default for MemBlock {
    fn default() -> Self {
        Self {
            data: Vec::new(),
        }
    }
}

impl core::fmt::Debug for MemBlock {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "MemBlock({} bytes, capacity {})",
            self.len(),
            self.capacity()
        )
    }
}

impl Clone for MemBlock {
    fn clone(&self) -> Self {
        Self {
            data: self.data.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mem_block_with_capacity() {
        let block = MemBlock::with_capacity(100);
        assert_eq!(block.capacity(), 100);
        assert!(block.is_empty());
    }

    #[test]
    fn test_mem_block_new_zeroed() {
        let block = MemBlock::new_zeroed(10);
        assert_eq!(block.len(), 10);
        assert!(block.as_slice().iter().all(|&b| b == 0));
    }

    #[test]
    fn test_mem_block_extend() {
        let mut block = MemBlock::with_capacity(10);
        block.extend(&[1, 2, 3, 4]);
        assert_eq!(block.len(), 4);
        assert_eq!(block.as_slice(), &[1, 2, 3, 4]);
    }

    #[test]
    fn test_mem_block_clear() {
        let mut block = MemBlock::new_zeroed(10);
        block.clear();
        assert!(block.is_empty());
        assert_eq!(block.capacity(), 10);
    }

    #[test]
    fn test_mem_block_reserve() {
        let mut block = MemBlock::with_capacity(10);
        block.reserve(20);
        assert!(block.capacity() >= 30);
    }

    #[test]
    fn test_mem_block_as_view() {
        let block = MemBlock::new_zeroed(10);
        let view = block.as_view();
        assert_eq!(view.len(), 10);
    }

    #[test]
    fn test_mem_block_clone() {
        let mut block = MemBlock::with_capacity(10);
        block.extend(&[1, 2, 3, 4]);
        let cloned = block.clone();
        assert_eq!(cloned.as_slice(), &[1, 2, 3, 4]);
    }
}
