//! ChunkedStorage - A container that stores elements in fixed-size chunks

use alloc::vec::Vec;

/// A container that stores elements in fixed-size chunks.
///
/// Useful for managing large amounts of data without reallocation.
#[derive(Clone, Debug)]
pub struct ChunkedStorage<T> {
    chunks: Vec<Vec<T>>,
    chunk_size: usize,
    len: usize,
}

impl<T> ChunkedStorage<T> {
    /// Error type for ChunkedStorage operations.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum Error {
        /// Chunk size must be greater than 0
        InvalidChunkSize,
    }

    /// Creates a new chunked storage with the given chunk size.
    ///
    /// Returns an error if chunk_size is 0.
    #[inline]
    pub fn try_new(chunk_size: usize) -> Result<Self, Error> {
        if chunk_size == 0 {
            return Err(Error::InvalidChunkSize);
        }
        Ok(ChunkedStorage {
            chunks: Vec::new(),
            chunk_size,
            len: 0,
        })
    }

    /// Creates a new chunked storage with the given chunk size.
    ///
    /// # Panics
    ///
    /// Panics if chunk_size is 0.
    #[inline]
    pub fn new(chunk_size: usize) -> Self {
        Self::try_new(chunk_size).expect("Chunk size must be greater than 0")
    }

    /// Returns the number of elements stored.
    #[inline]
    pub const fn len(&self) -> usize {
        self.len
    }

    /// Returns true if the storage is empty.
    #[inline]
    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Returns the number of chunks.
    #[inline]
    pub fn chunk_count(&self) -> usize {
        self.chunks.len()
    }

    /// Pushes a value into the storage.
    #[inline]
    pub fn push(&mut self, value: T) {
        if self.chunks.is_empty() || self.chunks.last().map_or(false, |c| c.len() >= self.chunk_size) {
            self.chunks.push(Vec::with_capacity(self.chunk_size));
        }
        // Safe: we just ensured chunks is non-empty
        if let Some(chunk) = self.chunks.last_mut() {
            chunk.push(value);
        }
        self.len += 1;
    }

    /// Gets a reference to the element at the given index.
    #[inline]
    pub fn get(&self, index: usize) -> Option<&T> {
        if index >= self.len {
            return None;
        }
        let chunk_index = index / self.chunk_size;
        let item_index = index % self.chunk_size;
        self.chunks.get(chunk_index)?.get(item_index)
    }

    /// Gets a mutable reference to the element at the given index.
    #[inline]
    pub fn get_mut(&mut self, index: usize) -> Option<&mut T> {
        if index >= self.len {
            return None;
        }
        let chunk_index = index / self.chunk_size;
        let item_index = index % self.chunk_size;
        self.chunks.get_mut(chunk_index)?.get_mut(item_index)
    }

    /// Clears all elements from the storage.
    #[inline]
    pub fn clear(&mut self) {
        self.chunks.clear();
        self.len = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chunked_storage() {
        let mut storage = ChunkedStorage::new(3);
        assert!(storage.is_empty());

        storage.push(1);
        storage.push(2);
        storage.push(3);
        storage.push(4);
        storage.push(5);

        assert_eq!(storage.len(), 5);
        assert_eq!(storage.chunk_count(), 2);
        assert_eq!(storage.get(0), Some(&1));
        assert_eq!(storage.get(2), Some(&3));
        assert_eq!(storage.get(3), Some(&4));
        assert_eq!(storage.get(5), None);
    }
}
