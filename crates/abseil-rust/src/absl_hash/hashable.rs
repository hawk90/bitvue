//! Hashable trait and hash cache for memoized hash computation.

use alloc::vec::Vec;
use core::hash::Hash;

use super::hash::hash_of;

/// Trait for types that can compute their own hash value.
///
/// This is useful for custom types that want to provide optimized
/// hash computation.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_hash::Hashable;
///
/// struct MyType {
///     data: Vec<u8>,
/// }
///
/// impl Hashable for MyType {
///     fn hash_value(&self) -> u64 {
///         // Custom hash computation
///         self.data.iter().fold(0u64, |acc, &b| {
///             acc.wrapping_mul(31).wrapping_add(b as u64)
///         })
///     }
/// }
/// ```
pub trait Hashable {
    /// Computes the hash value for this type.
    fn hash_value(&self) -> u64;
}

/// A hash cache that memoizes hash values for types.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_hash::HashCache;
///
/// let mut cache = HashCache::new();
/// let hash1 = cache.get_or_compute(&vec![1, 2, 3]);
/// let hash2 = cache.get_or_compute(&vec![1, 2, 3]);
/// assert_eq!(hash1, hash2);
/// ```
#[derive(Default)]
pub struct HashCache {
    cache: Vec<(usize, u64)>,
}

impl HashCache {
    /// Creates a new hash cache.
    pub fn new() -> Self {
        Self::default()
    }

    /// Gets or computes the hash value for a type.
    pub fn get_or_compute<T: Hash>(&mut self, value: &T) -> u64 {
        let addr = value as *const T as usize;

        // Check cache
        for &(cached_addr, cached_hash) in &self.cache {
            if cached_addr == addr {
                return cached_hash;
            }
        }

        // Compute and cache
        let hash = hash_of(value);
        self.cache.push((addr, hash));
        hash
    }

    /// Clears the cache.
    pub fn clear(&mut self) {
        self.cache.clear();
    }

    /// Returns the number of cached entries.
    pub fn len(&self) -> usize {
        self.cache.len()
    }

    /// Returns true if the cache is empty.
    pub fn is_empty(&self) -> bool {
        self.cache.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_cache() {
        let mut cache = HashCache::new();
        let hash1 = cache.get_or_compute(&vec![1, 2, 3]);
        let hash2 = cache.get_or_compute(&vec![1, 2, 3]);
        assert_eq!(hash1, hash2);
        assert_eq!(cache.len(), 1);
    }

    #[test]
    fn test_hash_cache_clear() {
        let mut cache = HashCache::new();
        cache.get_or_compute(&42);
        cache.clear();
        assert_eq!(cache.len(), 0);
    }
}
