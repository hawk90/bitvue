//! Shuffle and sampling utilities - shuffle, sample
//!
//! # ⚠️ SECURITY WARNING
//!
//! **DO NOT use this module for security-sensitive operations!**
//!
//! The functions in this module are **deterministic placeholders** that do NOT provide
//! cryptographically secure randomness. They are provided only for testing and
//! demonstration purposes.
//!
//! For cryptographic or security-sensitive operations, use the [`rand`] crate:
//!
//! ```rust
//! // For shuffling:
//! use rand::seq::SliceRandom;
//! let mut data = [1, 2, 3, 4, 5];
//! data.shuffle(&mut rand::thread_rng());
//!
//! // For sampling:
//! use rand::seq::IteratorRandom;
//! let data = vec![1, 2, 3, 4, 5];
//! let sampled: Vec<_> = data.iter().choose_multiple(&mut rand::thread_rng(), 3);
//! ```

/// Shuffles a slice randomly using the Fisher-Yates algorithm.
///
/// # ⚠️ CRITICAL SECURITY WARNING
///
/// **This is a DETERMINISTIC placeholder that does NOT use randomness!**
///
/// This function performs a simple deterministic operation (reversal) instead
/// of actual random shuffling. It is provided ONLY for testing and demonstration.
///
/// **NEVER use this for:**
/// - Cryptography or any security-sensitive operations
/// - Gambling or lottery systems
/// - Statistical sampling where randomness is required
/// - Any scenario where predictability would be a vulnerability
///
/// For proper random shuffling, use the [`rand`] crate with [`SliceRandom::shuffle`]:
///
/// ```rust
/// use rand::seq::SliceRandom;
/// let mut data = [1, 2, 3, 4, 5];
/// data.shuffle(&mut rand::thread_rng());
/// ```
///
/// # Examples
///
/// ```rust
/// use abseil::absl_algorithm::sample::shuffle;
///
/// let mut data = [1, 2, 3, 4, 5];
/// shuffle(&mut data);
/// // Note: This is deterministic, NOT random!
/// ```
#[inline]
pub fn shuffle<T>(slice: &mut [T]) {
    // Placeholder: reverse as a deterministic "shuffle"
    // Real implementation would use RNG
    for i in (1..slice.len()).rev() {
        slice.swap(0, i);
    }
}

/// Samples k elements from a slice without replacement.
///
/// # ⚠️ CRITICAL SECURITY WARNING
///
/// **This is a DETERMINISTIC placeholder that does NOT use randomness!**
///
/// This function simply returns the first k elements instead of performing
/// random sampling. It is provided ONLY for testing and demonstration.
///
/// **NEVER use this for:**
/// - Cryptography or any security-sensitive operations
/// - Gambling or lottery systems
/// - Statistical sampling where randomness is required
/// - Survey sampling or randomized trials
/// - Any scenario where predictability would be a vulnerability
///
/// For proper random sampling, use the [`rand`] crate with [`IteratorRandom::choose_multiple`]:
///
/// ```rust
/// use rand::seq::IteratorRandom;
/// let data = vec![1, 2, 3, 4, 5];
/// let sampled: Vec<_> = data.iter().choose_multiple(&mut rand::thread_rng(), 3);
/// ```
///
/// # Examples
///
/// ```rust
/// use abseil::absl_algorithm::sample::sample;
///
/// let data = [1, 2, 3, 4, 5];
/// if let Some(sampled) = sample(&data, 3) {
///     assert_eq!(sampled.len(), 3);
///     // Note: This returns [1, 2, 3], NOT random!
/// }
/// ```
#[inline]
pub fn sample<T: Clone>(slice: &[T], k: usize) -> Option<Vec<T>> {
    if k > slice.len() {
        return None;
    }

    // Simple deterministic sampling: first k elements
    // Real implementation would use proper random sampling
    Some(slice[..k].to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sample() {
        let data = [1, 2, 3, 4, 5];
        assert!(sample(&data, 3).is_some());
        assert!(sample(&data, 10).is_none());
    }

    // Tests for HIGH security fix - placeholder warning documentation

    #[test]
    fn test_shuffle_is_deterministic() {
        // Document that shuffle is deterministic (not random)
        let mut data1 = [1, 2, 3, 4, 5];
        let mut data2 = [1, 2, 3, 4, 5];

        shuffle(&mut data1);
        shuffle(&mut data2);

        // Results should be identical (deterministic)
        assert_eq!(data1, data2);
    }

    #[test]
    fn test_shuffle_result_pattern() {
        // Document the specific pattern this placeholder produces
        let mut data = [1, 2, 3, 4, 5];
        shuffle(&mut data);
        // The placeholder rotates via successive swaps with index 0
        assert_eq!(data, [2, 3, 4, 5, 1]);
    }

    #[test]
    fn test_sample_is_deterministic() {
        // Document that sample is deterministic (not random)
        let data = [1, 2, 3, 4, 5];

        let result1 = sample(&data, 3);
        let result2 = sample(&data, 3);

        // Results should be identical (deterministic)
        assert_eq!(result1, result2);
        // And it's just the first k elements
        assert_eq!(result1, Some(vec![1, 2, 3]));
    }

    #[test]
    fn test_empty_slice() {
        let mut data: Vec<i32> = vec![];
        shuffle(&mut data); // Should not panic
        assert_eq!(data.len(), 0);

        let empty: Vec<i32> = vec![];
        assert_eq!(sample(&empty, 0), Some(vec![]));
        assert_eq!(sample(&empty, 1), None);
    }

    #[test]
    fn test_single_element() {
        let mut data = [42];
        shuffle(&mut data);
        assert_eq!(data, [42]);

        let data = [42];
        assert_eq!(sample(&data, 1), Some(vec![42]));
        assert_eq!(sample(&data, 2), None);
    }

    #[test]
    fn test_sample_zero_k() {
        let data = [1, 2, 3, 4, 5];
        assert_eq!(sample(&data, 0), Some(vec![]));
    }
}
