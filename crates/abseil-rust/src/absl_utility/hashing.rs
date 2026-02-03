//! Hashing utilities.

/// A simple hash combiner for combining hash values.
///
/// This implements the Boost hash combine approach.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_utility::hashing::HashCombiner;
///
/// let mut combiner = HashCombiner::new(0);
/// combiner.combine(1);
/// combiner.combine(2);
/// let hash = combiner.finish();
/// ```
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct HashCombiner {
    state: u64,
}

impl HashCombiner {
    /// Creates a new hash combiner with the given initial state.
    #[inline]
    pub const fn new(state: u64) -> Self {
        HashCombiner { state }
    }

    /// Combines another hash value into this one.
    #[inline]
    pub fn combine(&mut self, hash: u64) {
        self.state = self.state.wrapping_add(hash);
        self.state = self.state.wrapping_mul(0x9e3779b97f4a7c15);
        self.state = self.state.rotate_left(31);
        self.state = self.state.wrapping_mul(0x85ebca6b);
    }

    /// Returns the final hash value.
    #[inline]
    pub const fn finish(&self) -> u64 {
        self.state
    }
}

/// Combines two hash values into one.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_utility::hashing::combine_hash;
///
/// let hash = combine_hash(1, 2);
/// assert_ne!(hash, 0);
/// ```
#[inline]
pub fn combine_hash(a: u64, b: u64) -> u64 {
    let mut combiner = HashCombiner::new(a);
    combiner.combine(b);
    combiner.finish()
}

/// Computes a hash of the given bytes using FNV-1a algorithm.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_utility::hashing::fnv1a_hash;
///
/// let hash = fnv1a_hash(b"hello");
/// assert_ne!(hash, 0);
/// ```
#[inline]
pub fn fnv1a_hash(bytes: &[u8]) -> u64 {
    const FNV_PRIME: u64 = 1099511628211;
    const FNV_OFFSET_BASIS: u64 = 14695981039346656037;

    let mut hash = FNV_OFFSET_BASIS;
    for &byte in bytes {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}

/// Computes a simple hash of the given bytes.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_utility::hashing::hash_bytes;
///
/// let hash = hash_bytes(b"hello");
/// assert_ne!(hash, 0);
/// ```
#[inline]
pub fn hash_bytes(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf29ce484222325;
    for &byte in bytes {
        hash = hash.wrapping_mul(0x100000001).wrapping_add(byte as u64);
        hash ^= hash >> 6;
    }
    hash
}

/// Computes a 32-bit hash of the given bytes.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_utility::hashing::hash_bytes_32;
///
/// let hash = hash_bytes_32(b"hello");
/// assert_ne!(hash, 0);
/// ```
#[inline]
pub fn hash_bytes_32(bytes: &[u8]) -> u32 {
    let mut hash = 0x811c9dc5u32;
    for &byte in bytes {
        hash = hash.wrapping_mul(0x1000193).wrapping_add(byte as u32);
        hash ^= hash >> 6;
        hash = hash.wrapping_mul(0x1000193);
    }
    hash
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_combiner() {
        let mut combiner = HashCombiner::new(0);
        combiner.combine(1);
        combiner.combine(2);
        let hash = combiner.finish();
        assert_ne!(hash, 0);
    }

    #[test]
    fn test_combine_hash() {
        let hash = combine_hash(1, 2);
        assert_ne!(hash, 0);
    }

    #[test]
    fn test_fnv1a_hash() {
        let hash = fnv1a_hash(b"hello");
        assert_ne!(hash, 0);
        // FNV-1a is deterministic
        assert_eq!(fnv1a_hash(b"hello"), fnv1a_hash(b"hello"));
        assert_ne!(fnv1a_hash(b"hello"), fnv1a_hash(b"world"));
    }

    #[test]
    fn test_hash_bytes() {
        let hash = hash_bytes(b"hello");
        assert_ne!(hash, 0);
    }

    #[test]
    fn test_hash_bytes_32() {
        let hash = hash_bytes_32(b"hello");
        assert_ne!(hash, 0);
    }
}
