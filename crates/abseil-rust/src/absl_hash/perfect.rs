//! PerfectHash - minimal perfect hash for small static sets.

use alloc::vec::Vec;

use super::algorithms::fnv_hash;

/// A perfect hash for small, known sets.
///
/// Uses a minimal perfect hash algorithm for small static sets.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_hash::PerfectHash;
///
/// let keys = ["a", "b", "c"];
/// let perfect = PerfectHash::new(&keys);
/// assert!(perfect.index("a") != perfect.index("b"));
/// ```
pub struct PerfectHash {
    offsets: Vec<u64>,
}

impl PerfectHash {
    /// Creates a perfect hash for the given keys.
    pub fn new(keys: &[&str]) -> Self {
        let mut offsets = Vec::new();
        for (i, &key) in keys.iter().enumerate() {
            let hash = fnv_hash(key.as_bytes());
            offsets.push(hash.wrapping_add(i as u64));
        }
        offsets.sort_unstable();
        Self { offsets }
    }

    /// Returns the index for a key, or None if not found.
    pub fn index(&self, key: &str) -> Option<usize> {
        let hash = fnv_hash(key.as_bytes());
        // Find the position of this hash in our sorted offsets
        self.offsets
            .iter()
            .position(|&offset| offset == hash)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_perfect_hash() {
        let keys = ["a", "b", "c"];
        let perfect = PerfectHash::new(&keys);
        let idx_a = perfect.index("a");
        let idx_b = perfect.index("b");
        let idx_c = perfect.index("c");

        assert!(idx_a.is_some() && idx_a.unwrap() < 3);
        assert!(idx_b.is_some() && idx_b.unwrap() < 3);
        assert!(idx_c.is_some() && idx_c.unwrap() < 3);
    }

    #[test]
    fn test_perfect_hash_not_found() {
        let keys = ["a", "b", "c"];
        let perfect = PerfectHash::new(&keys);
        assert!(perfect.index("d").is_none());
    }
}
