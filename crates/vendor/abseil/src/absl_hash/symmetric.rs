//! Symmetric hash - order-independent hash for unordered collections.

use alloc::vec::Vec;
use core::hash::Hash;

use super::hash::hash_of;
use super::hash::HashState;

/// A symmetric hash that is independent of element order.
///
/// This is useful for hashing unordered collections where the same elements
/// should produce the same hash regardless of their order.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_hash::SymmetricHash;
///
/// let items1 = vec![1, 2, 3, 4];
/// let items2 = vec![4, 3, 2, 1];
///
/// let hash1 = SymmetricHash::of(&items1);
/// let hash2 = SymmetricHash::of(&items2);
///
/// assert_eq!(hash1, hash2); // Same elements, different order
/// ```
pub struct SymmetricHash;

impl SymmetricHash {
    /// Computes a symmetric hash of a slice.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use abseil::absl_hash::SymmetricHash;
    ///
    /// let hash = SymmetricHash::of(&[1, 2, 3]);
    /// ```
    pub fn of<T: Hash>(slice: &[T]) -> u64 {
        // Compute individual hashes and combine them in order-independent way
        let mut hashes: Vec<u64> = slice.iter().map(hash_of).collect();
        hashes.sort_unstable(); // Sort for order independence

        // Combine sorted hashes
        let mut state = HashState::default();
        for hash in hashes {
            state = state.update_raw(hash);
        }
        state.finalize()
    }

    /// Computes a symmetric hash of an iterator.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use abseil::absl_hash::SymmetricHash;
    ///
    /// let hash = SymmetricHash::of_iter([1, 2, 3].iter().copied());
    /// ```
    pub fn of_iter<T: Hash, I: IntoIterator<Item = T>>(iter: I) -> u64 {
        let mut hashes: Vec<u64> = iter.into_iter().map(hash_of).collect();
        hashes.sort_unstable();

        let mut state = HashState::default();
        for hash in hashes {
            state = state.update_raw(hash);
        }
        state.finalize()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_symmetric_hash() {
        let items1 = vec![1, 2, 3, 4];
        let items2 = vec![4, 3, 2, 1];

        let hash1 = SymmetricHash::of(&items1);
        let hash2 = SymmetricHash::of(&items2);

        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_symmetric_hash_iter() {
        let hash1 = SymmetricHash::of_iter([1, 2, 3, 4].iter().copied());
        let hash2 = SymmetricHash::of_iter([4, 3, 2, 1].iter().copied());

        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_symmetric_hash_empty() {
        let hash = SymmetricHash::of::<i32>(&[]);
        assert_eq!(hash, 0);
    }

    #[test]
    fn test_symmetric_hash_single() {
        let hash = SymmetricHash::of(&[42]);
        assert_ne!(hash, 0);
    }
}
