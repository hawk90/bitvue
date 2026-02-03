//! Permutation utilities - random_permutation

use alloc::vec::Vec;

use crate::absl_random::bit_gen::BitGen;

/// Generates a random permutation of indices 0..n.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_random::random_permutation;
///
/// let mut rng = BitGen::new(42);
/// let perm = random_permutation(5, &mut rng);
/// assert_eq!(perm.len(), 5);
/// assert!(perm.iter().all(|&i| i < 5));
/// ```
pub fn random_permutation(n: usize, rng: &mut BitGen) -> Vec<usize> {
    let mut perm: Vec<usize> = (0..n).collect();
    super::shuffle(&mut perm, rng);
    perm
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_random_permutation() {
        let mut rng = BitGen::new(42);
        let perm = random_permutation(5, &mut rng);
        assert_eq!(perm.len(), 5);
        assert!(perm.iter().all(|&i| i < 5));

        // Check it's a valid permutation
        let mut sorted = perm.clone();
        sorted.sort();
        assert_eq!(sorted, vec![0, 1, 2, 3, 4]);
    }
}
