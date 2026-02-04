//! Seed sequence module - generating multiple seeds from one seed.

use alloc::vec::Vec;

/// Seed sequence for generating multiple seeds from a single seed.
///
/// Useful for seeding multiple independent RNGs.
#[derive(Clone, Debug)]
pub struct SeedSeq {
    state: u64,
}

impl SeedSeq {
    /// Creates a new seed sequence from a seed.
    pub const fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    /// Generates the next seed in the sequence.
    pub fn next(&mut self) -> super::Seed {
        // SplitMix64 for seed generation
        self.state = self.state.wrapping_add(0x9E3779B97F4A7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
        z = z ^ (z >> 31);
        z
    }

    /// Generates `n` seeds.
    pub fn generate(&mut self, n: usize) -> Vec<super::Seed> {
        (0..n).map(|_| self.next()).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_seed_seq() {
        let mut seq = SeedSeq::new(42);
        let seeds = seq.generate(5);
        assert_eq!(seeds.len(), 5);
        // All seeds should be different (very likely)
        let unique: std::collections::HashSet<_> = seeds.iter().collect();
        assert_eq!(unique.len(), 5);
    }
}
