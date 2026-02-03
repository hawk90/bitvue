//! Sampling utilities - shuffle, sample, sample_one, sample_with_replacement, random_index.

use alloc::vec::Vec;

use crate::absl_random::bit_gen::BitGen;

/// Shuffles a slice using Fisher-Yates algorithm.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_random::shuffle;
///
/// let mut rng = BitGen::new(42);
/// let mut data = vec![1, 2, 3, 4, 5];
/// let original = data.clone();
/// shuffle(&mut data, &mut rng);
/// // Order is now randomized
/// ```
pub fn shuffle<T>(slice: &mut [T], rng: &mut BitGen) {
    let len = slice.len();
    if len <= 1 {
        return;
    }

    for i in (1..len).rev() {
        let j = rng.gen_usize() % (i + 1);
        slice.swap(i, j);
    }
}

/// Partially shuffles a slice, shuffling only the first `n` elements.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_random::partial_shuffle;
///
/// let mut rng = BitGen::new(42);
/// let mut data = vec![1, 2, 3, 4, 5];
/// // Shuffle only first 3 elements
/// partial_shuffle(&mut data, 3, &mut rng);
/// ```
pub fn partial_shuffle<T>(slice: &mut [T], n: usize, rng: &mut BitGen) {
    let len = slice.len().min(n);
    if len <= 1 {
        return;
    }

    for i in (1..len).rev() {
        let j = rng.gen_usize() % (i + 1);
        slice.swap(i, j);
    }
}

/// Samples `n` random elements from a slice without replacement.
///
/// Returns a new Vec containing the sampled elements.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_random::sample;
///
/// let mut rng = BitGen::new(42);
/// let items = vec!['a', 'b', 'c', 'd', 'e'];
/// let chosen = sample(&items, 3, &mut rng);
/// assert_eq!(chosen.len(), 3);
/// ```
pub fn sample<'a, T: Clone>(slice: &'a [T], n: usize, rng: &mut BitGen) -> Vec<&'a T> {
    let len = slice.len();
    if len == 0 || n == 0 {
        return Vec::new();
    }

    let n = n.min(len);
    let mut indices: Vec<usize> = (0..len).collect();
    partial_shuffle(&mut indices, n, rng);

    indices[..n].iter().map(|&i| &slice[i]).collect()
}

/// Samples a single random element from a slice.
///
/// Returns None if the slice is empty.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_random::sample_one;
///
/// let mut rng = BitGen::new(42);
/// let items = vec![1, 2, 3, 4, 5];
/// if let Some(&item) = sample_one(&items, &mut rng) {
///     println!("Got: {}", item);
/// }
/// ```
pub fn sample_one<'a, T>(slice: &'a [T], rng: &mut BitGen) -> Option<&'a T> {
    if slice.is_empty() {
        return None;
    }
    let index = rng.gen_usize() % slice.len();
    Some(&slice[index])
}

/// Samples `n` random elements from a slice with replacement.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_random::sample_with_replacement;
///
/// let mut rng = BitGen::new(42);
/// let items = vec!['a', 'b', 'c'];
/// let chosen = sample_with_replacement(&items, 5, &mut rng);
/// assert_eq!(chosen.len(), 5);
/// ```
pub fn sample_with_replacement<'a, T: Clone>(slice: &'a [T], n: usize, rng: &mut BitGen) -> Vec<&'a T> {
    if slice.is_empty() || n == 0 {
        return Vec::new();
    }

    (0..n)
        .map(|_| {
            let index = rng.gen_usize() % slice.len();
            &slice[index]
        })
        .collect()
}

/// Returns a random index for the given slice length.
///
/// Returns None if the length is 0.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_random::random_index;
///
/// let mut rng = BitGen::new(42);
/// if let Some(index) = random_index(10, &mut rng) {
///     assert!(index < 10);
/// }
/// ```
#[inline]
pub fn random_index(len: usize, rng: &mut BitGen) -> Option<usize> {
    if len == 0 {
        return None;
    }
    Some(rng.gen_usize() % len)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shuffle() {
        let mut rng = BitGen::new(42);
        let mut data = vec![1, 2, 3, 4, 5];
        let original = data.clone();
        shuffle(&mut data, &mut rng);
        // Same elements, different order (most likely)
        assert_eq!(data.len(), original.len());
        for &item in &original {
            assert!(data.contains(&item));
        }
    }

    #[test]
    fn test_partial_shuffle() {
        let mut rng = BitGen::new(42);
        let mut data = vec![1, 2, 3, 4, 5];
        let original = data.clone();
        partial_shuffle(&mut data, 3, &mut rng);
        // First 3 may be shuffled, rest unchanged
        assert_eq!(data.len(), original.len());
    }

    #[test]
    fn test_sample() {
        let mut rng = BitGen::new(42);
        let items = vec![1, 2, 3, 4, 5];
        let chosen = sample(&items, 3, &mut rng);
        assert_eq!(chosen.len(), 3);
        // All unique
        let unique: std::collections::HashSet<_> = chosen.iter().collect();
        assert_eq!(chosen.len(), unique.len());
    }

    #[test]
    fn test_sample_one() {
        let mut rng = BitGen::new(42);
        let items = vec![1, 2, 3, 4, 5];
        if let Some(&item) = sample_one(&items, &mut rng) {
            assert!(items.contains(&item));
        } else {
            panic!("Should return Some");
        }
    }

    #[test]
    fn test_sample_one_empty() {
        let mut rng = BitGen::new(42);
        let items: Vec<i32> = vec![];
        assert!(sample_one(&items, &mut rng).is_none());
    }

    #[test]
    fn test_sample_with_replacement() {
        let mut rng = BitGen::new(42);
        let items = vec![1, 2, 3];
        let chosen = sample_with_replacement(&items, 10, &mut rng);
        assert_eq!(chosen.len(), 10);
    }

    #[test]
    fn test_random_index() {
        let mut rng = BitGen::new(42);
        if let Some(index) = random_index(10, &mut rng) {
            assert!(index < 10);
        } else {
            panic!("Should return Some");
        }
    }

    #[test]
    fn test_random_index_zero() {
        let mut rng = BitGen::new(42);
        assert!(random_index(0, &mut rng).is_none());
    }
}
