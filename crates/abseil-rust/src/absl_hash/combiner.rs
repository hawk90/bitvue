//! Hash combination utilities - combine, rotate, and random hash operations.

use core::ops::Range;

use super::algorithms::murmur3_64;

/// Combines multiple hashes into one using multiplication.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_hash::combiner::combine_hashes_mult;
///
/// let combined = combine_hashes_mult(&[1, 2, 3, 4]);
/// ```
pub fn combine_hashes_mult(hashes: &[u64]) -> u64 {
    hashes.iter().fold(0u64, |acc, &hash| {
        acc.wrapping_mul(31).wrapping_add(hash)
    })
}

/// Combines multiple hashes using XOR.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_hash::combiner::combine_hashes_xor;
///
/// let combined = combine_hashes_xor(&[1, 2, 3, 4]);
/// ```
pub fn combine_hashes_xor(hashes: &[u64]) -> u64 {
    hashes.iter().fold(0u64, |acc, &hash| acc ^ hash)
}

/// Rotates a hash value left by n bits.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_hash::combiner::rotate_left;
///
/// let rotated = rotate_left(0x1234567890abcdef, 16);
/// ```
#[inline]
pub const fn rotate_left(value: u64, n: u32) -> u64 {
    value.rotate_left(n)
}

/// Rotates a hash value right by n bits.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_hash::combiner::rotate_right;
///
/// let rotated = rotate_right(0x1234567890abcdef, 16);
/// ```
#[inline]
pub const fn rotate_right(value: u64, n: u32) -> u64 {
    value.rotate_right(n)
}

/// Generates a hash-based random value from a seed.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_hash::combiner::hash_random;
///
/// let val = hash_random(42, 0..100);
/// assert!(val >= 0 && val < 100);
/// ```
pub fn hash_random(seed: u64, range: Range<u64>) -> u64 {
    let hash = murmur3_64(&seed.to_le_bytes(), seed);
    let len = range.end.wrapping_sub(range.start);
    range.start.wrapping_add(hash % len)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_combine_hashes_mult() {
        let combined = combine_hashes_mult(&[1, 2, 3, 4]);
        assert_ne!(combined, 0);
    }

    #[test]
    fn test_combine_hashes_xor() {
        let combined = combine_hashes_xor(&[1, 2, 3, 4]);
        assert_ne!(combined, 0);
    }

    #[test]
    fn test_rotate_left() {
        let value = 0x1234567890abcdefu64;
        let rotated = rotate_left(value, 16);
        assert_eq!(rotated, 0x90abcdef12345678);
    }

    #[test]
    fn test_rotate_right() {
        let value = 0x1234567890abcdefu64;
        let rotated = rotate_right(value, 16);
        assert_eq!(rotated, 0xcdef1234567890ab);
    }

    #[test]
    fn test_hash_random() {
        for _ in 0..100 {
            let val = hash_random(42, 10..100);
            assert!(val >= 10 && val < 100);
        }
    }
}
