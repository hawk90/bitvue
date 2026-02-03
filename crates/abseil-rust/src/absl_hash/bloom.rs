//! BloomFilter - approximate set membership testing.

use alloc::vec::Vec;
use core::fmt;
use core::hash::Hash;

use super::hash::hash_of;

/// Error type for BloomFilter operations.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum BloomFilterError {
    /// Number of hashes must be at least 1.
    InvalidNumHashes,
    /// Capacity must be non-zero.
    InvalidCapacity,
}

impl fmt::Display for BloomFilterError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BloomFilterError::InvalidNumHashes => {
                write!(f, "num_hashes must be at least 1")
            }
            BloomFilterError::InvalidCapacity => {
                write!(f, "capacity must be greater than 0")
            }
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for BloomFilterError {}

/// A Bloom filter for approximate set membership testing.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_hash::BloomFilter;
///
/// let mut bloom = BloomFilter::new(1000, 3);
/// bloom.insert(&"hello");
/// assert!(bloom.contains(&"hello"));
/// assert!(!bloom.contains(&"world"));
/// ```
pub struct BloomFilter {
    bits: Vec<u64>,
    num_bits: usize,
    num_hashes: usize,
}

impl BloomFilter {
    /// Creates a new Bloom filter with the given capacity and hash functions.
    ///
    /// # Panics
    ///
    /// Panics if `num_hashes` is 0 or `capacity` is 0.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use abseil::absl_hash::BloomFilter;
    ///
    /// let mut bloom = BloomFilter::new(1000, 3);
    /// bloom.insert(&"hello");
    /// assert!(bloom.contains(&"hello"));
    /// ```
    pub fn new(capacity: usize, num_hashes: usize) -> Self {
        Self::try_new(capacity, num_hashes).unwrap()
    }

    /// Creates a new Bloom filter with validation.
    ///
    /// Returns `Err` if `num_hashes` is 0 or `capacity` is 0.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use abseil::absl_hash::BloomFilter;
    ///
    /// assert!(BloomFilter::try_new(1000, 3).is_ok());
    /// assert!(BloomFilter::try_new(1000, 0).is_err());
    /// assert!(BloomFilter::try_new(0, 3).is_err());
    /// ```
    pub fn try_new(capacity: usize, num_hashes: usize) -> Result<Self, BloomFilterError> {
        if num_hashes == 0 {
            return Err(BloomFilterError::InvalidNumHashes);
        }
        if capacity == 0 {
            return Err(BloomFilterError::InvalidCapacity);
        }

        let num_bits = (capacity.checked_mul(num_hashes)
            .and_then(|v| v.checked_mul(10))
            .ok_or(BloomFilterError::InvalidCapacity)?)
            .max(64);
        let bits = vec![0u64; (num_bits + 63) / 64];

        Ok(Self {
            bits,
            num_bits,
            num_hashes,
        })
    }

    /// Inserts a value into the Bloom filter.
    pub fn insert<T: Hash>(&mut self, value: &T) {
        let hash = hash_of(value);
        let hash_usize = hash as usize;
        for i in 0..self.num_hashes {
            // Use checked arithmetic to prevent overflow in index calculation
            // The i * hash_usize multiplication could overflow, so we use wrapping_mul
            let offset = i.wrapping_mul(hash_usize);
            let index = hash_usize.wrapping_add(offset) % self.num_bits;
            self.bits[index / 64] |= 1 << (index % 64);
        }
    }

    /// Returns true if the value might be in the filter (false positives possible).
    pub fn contains<T: Hash>(&self, value: &T) -> bool {
        let hash = hash_of(value);
        let hash_usize = hash as usize;
        (0..self.num_hashes).all(|i| {
            // Use checked arithmetic to prevent overflow in index calculation
            let offset = i.wrapping_mul(hash_usize);
            let index = hash_usize.wrapping_add(offset) % self.num_bits;
            (self.bits[index / 64] & (1 << (index % 64))) != 0
        })
    }

    /// Clears the Bloom filter.
    pub fn clear(&mut self) {
        self.bits.fill(0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bloom_filter() {
        let mut bloom = BloomFilter::new(1000, 3);
        bloom.insert(&"hello");
        assert!(bloom.contains(&"hello"));
        assert!(!bloom.contains(&"world"));

        bloom.clear();
        assert!(!bloom.contains(&"hello"));
    }

    // Tests for HIGH security fix - division by zero prevention

    #[test]
    fn test_try_new_valid_inputs() {
        assert!(BloomFilter::try_new(1000, 3).is_ok());
        assert!(BloomFilter::try_new(1, 1).is_ok());
        assert!(BloomFilter::try_new(1000000, 10).is_ok());
    }

    #[test]
    fn test_try_new_zero_num_hashes() {
        let result = BloomFilter::try_new(1000, 0);
        assert!(result.is_err());
        assert_eq!(result, Err(BloomFilterError::InvalidNumHashes));
    }

    #[test]
    fn test_try_new_zero_capacity() {
        let result = BloomFilter::try_new(0, 3);
        assert!(result.is_err());
        assert_eq!(result, Err(BloomFilterError::InvalidCapacity));
    }

    #[test]
    fn test_new_panics_on_invalid_inputs() {
        // Note: These tests would panic, so we test try_new instead
        assert!(BloomFilter::try_new(1000, 0).is_err());
        assert!(BloomFilter::try_new(0, 3).is_err());
    }

    #[test]
    fn test_bloom_filter_error_display() {
        assert_eq!(
            format!("{}", BloomFilterError::InvalidNumHashes),
            "num_hashes must be at least 1"
        );
        assert_eq!(
            format!("{}", BloomFilterError::InvalidCapacity),
            "capacity must be greater than 0"
        );
    }

    #[test]
    fn test_bloom_filter_with_min_values() {
        // Test with minimum valid values
        let mut bloom = BloomFilter::new(1, 1);
        bloom.insert(&42u32);
        assert!(bloom.contains(&42u32));
        assert!(!bloom.contains(&99u32));
    }

    #[test]
    fn test_bloom_filter_multiple_values() {
        let mut bloom = BloomFilter::new(100, 5);

        // Insert multiple values
        for i in 0..10 {
            bloom.insert(&i);
        }

        // Check they all exist
        for i in 0..10 {
            assert!(bloom.contains(&i));
        }

        // Check non-existent values
        for i in 100..110 {
            assert!(!bloom.contains(&i));
        }
    }

    #[test]
    fn test_bloom_filter_clear_and_reuse() {
        let mut bloom = BloomFilter::new(100, 3);

        bloom.insert(&"hello");
        assert!(bloom.contains(&"hello"));

        bloom.clear();
        assert!(!bloom.contains(&"hello"));

        // Reuse after clear
        bloom.insert(&"world");
        assert!(bloom.contains(&"world"));
        assert!(!bloom.contains(&"hello"));
    }
}
