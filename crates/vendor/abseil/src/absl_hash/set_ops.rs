//! Hash set operations for combining hashes.

use alloc::collections::HashSet;
use core::hash::Hash;

use super::hash::HashState;

/// Hash set operations for combining hashes.
pub struct HashSetOps;

impl HashSetOps {
    /// Computes the union of two hash sets.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use abseil::absl_hash::HashSetOps;
    ///
    /// let set1: HashSet<i32> = [1, 2, 3].iter().cloned().collect();
    /// let set2: HashSet<i32> = [3, 4, 5].iter().cloned().collect();
    /// let hash = HashSetOps::union_hash(&set1, &set2);
    /// ```
    pub fn union_hash<T: Hash>(set1: &HashSet<T>, set2: &HashSet<T>) -> u64 {
        let mut state = HashState::default();
        for item in set1 {
            state = state.update(item);
        }
        for item in set2 {
            state = state.update(item);
        }
        state.finalize()
    }

    /// Computes the intersection hash of two hash sets.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use abseil::absl_hash::HashSetOps;
    ///
    /// let set1: HashSet<i32> = [1, 2, 3].iter().cloned().collect();
    /// let set2: HashSet<i32> = [2, 3, 4].iter().cloned().collect();
    /// let hash = HashSetOps::intersection_hash(&set1, &set2);
    /// ```
    pub fn intersection_hash<T: Hash>(
        set1: &HashSet<T>,
        set2: &HashSet<T>,
    ) -> u64 {
        let mut state = HashState::default();
        for item in set1 {
            if set2.contains(item) {
                state = state.update(item);
            }
        }
        state.finalize()
    }

    /// Computes the difference hash of two hash sets.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use abseil::absl_hash::HashSetOps;
    ///
    /// let set1: HashSet<i32> = [1, 2, 3].iter().cloned().collect();
    /// let set2: HashSet<i32> = [2, 3, 4].iter().cloned().collect();
    /// let hash = HashSetOps::difference_hash(&set1, &set2);
    /// ```
    pub fn difference_hash<T: Hash>(
        set1: &HashSet<T>,
        set2: &HashSet<T>,
    ) -> u64 {
        let mut state = HashState::default();
        for item in set1 {
            if !set2.contains(item) {
                state = state.update(item);
            }
        }
        state.finalize()
    }

    /// Computes the symmetric difference hash of two hash sets.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use abseil::absl_hash::HashSetOps;
    ///
    /// let set1: HashSet<i32> = [1, 2, 3].iter().cloned().collect();
    /// let set2: HashSet<i32> = [2, 3, 4].iter().cloned().collect();
    /// let hash = HashSetOps::symmetric_difference_hash(&set1, &set2);
    /// ```
    pub fn symmetric_difference_hash<T: Hash>(
        set1: &HashSet<T>,
        set2: &HashSet<T>,
    ) -> u64 {
        let mut state = HashState::default();
        for item in set1 {
            if !set2.contains(item) {
                state = state.update(item);
            }
        }
        for item in set2 {
            if !set1.contains(item) {
                state = state.update(item);
            }
        }
        state.finalize()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_set_ops_union() {
        let set1: HashSet<i32> = [1, 2, 3].iter().cloned().collect();
        let set2: HashSet<i32> = [3, 4, 5].iter().cloned().collect();
        let hash = HashSetOps::union_hash(&set1, &set2);
        assert_ne!(hash, 0);
    }

    #[test]
    fn test_hash_set_ops_intersection() {
        let set1: HashSet<i32> = [1, 2, 3].iter().cloned().collect();
        let set2: HashSet<i32> = [2, 3, 4].iter().cloned().collect();
        let hash = HashSetOps::intersection_hash(&set1, &set2);
        assert_ne!(hash, 0);
    }

    #[test]
    fn test_hash_set_ops_difference() {
        let set1: HashSet<i32> = [1, 2, 3].iter().cloned().collect();
        let set2: HashSet<i32> = [2, 3, 4].iter().cloned().collect();
        let hash = HashSetOps::difference_hash(&set1, &set2);
        assert_ne!(hash, 0);
    }

    #[test]
    fn test_hash_set_ops_symmetric_difference() {
        let set1: HashSet<i32> = [1, 2, 3].iter().cloned().collect();
        let set2: HashSet<i32> = [2, 3, 4].iter().cloned().collect();
        let hash = HashSetOps::symmetric_difference_hash(&set1, &set2);
        assert_ne!(hash, 0);
    }
}
