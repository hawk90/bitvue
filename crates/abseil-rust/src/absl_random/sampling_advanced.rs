//! Advanced sampling utilities - weighted_sample, uniform_sample, coin_flip, roll_die, etc.

use crate::absl_random::bit_gen::BitGen;

/// Randomly selects an element from a slice based on weights.
///
/// Each element has a corresponding weight. Higher weight means
/// higher probability of being selected.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_random::weighted_sample;
///
/// let mut rng = BitGen::new(42);
/// let items = vec!['a', 'b', 'c'];
/// let weights = vec![1.0, 2.0, 1.0]; // 'b' has 50% chance
/// if let Some(&item) = weighted_sample(&items, &weights, &mut rng) {
///     println!("Selected: {}", item);
/// }
/// ```
pub fn weighted_sample<'a, T>(
    items: &'a [T],
    weights: &[f64],
    rng: &mut BitGen,
) -> Option<&'a T> {
    if items.is_empty() || weights.is_empty() || items.len() != weights.len() {
        return None;
    }

    let total_weight: f64 = weights.iter().sum();
    if total_weight <= 0.0 {
        return Some(&items[rng.gen_usize() % items.len()]);
    }

    let mut random_weight = super::random_f64(rng) * total_weight;
    for (item, &weight) in items.iter().zip(weights.iter()) {
        random_weight -= weight;
        if random_weight <= 0.0 {
            return Some(item);
        }
    }

    Some(&items[items.len() - 1])
}

/// Randomly selects an element from a slice using uniform distribution.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_random::uniform_sample;
///
/// let mut rng = BitGen::new(42);
/// let items = vec![1, 2, 3, 4, 5];
/// if let Some(&item) = uniform_sample(&items, &mut rng) {
///     println!("Selected: {}", item);
/// }
/// ```
pub fn uniform_sample<'a, T>(items: &'a [T], rng: &mut BitGen) -> Option<&'a T> {
    super::sample_one(items, rng)
}

/// Generates a random duration in the given range (in seconds).
///
/// # Examples
///
/// ```rust
/// use abseil::absl_random::random_duration;
///
/// let mut rng = BitGen::new(42);
/// let duration = random_duration(1.0..60.0, &mut rng);
/// assert!(duration >= 1.0 && duration < 60.0);
/// ```
#[inline]
pub fn random_duration(range: core::ops::Range<f64>, rng: &mut BitGen) -> f64 {
    super::random_range_f64(range, rng)
}

/// Coin flip with 50/50 probability.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_random::coin_flip;
///
/// let mut rng = BitGen::new(42);
/// let heads = coin_flip(&mut rng);
/// ```
#[inline]
pub fn coin_flip(rng: &mut BitGen) -> bool {
    super::random_bool(0.5, rng)
}

/// Roll a die with the given number of sides (1 to sides).
///
/// # Examples
///
/// ```rust
/// use abseil::absl_random::roll_die;
///
/// let mut rng = BitGen::new(42);
/// let result = roll_die(6, &mut rng);
/// assert!(result >= 1 && result <= 6);
/// ```
#[inline]
pub fn roll_die(sides: u32, rng: &mut BitGen) -> u32 {
    if sides == 0 {
        return 0;
    }
    (rng.gen_u32() % sides) + 1
}

/// Roll multiple dice and return the sum.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_random::roll_dice;
///
/// let mut rng = BitGen::new(42);
/// let sum = roll_dice(3, 6, &mut rng); // 3d6
/// assert!(sum >= 3 && sum <= 18);
/// ```
pub fn roll_dice(count: u32, sides: u32, rng: &mut BitGen) -> u32 {
    (0..count).map(|_| roll_die(sides, rng)).sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_weighted_sample() {
        let mut rng = BitGen::new(42);
        let items = vec![1, 2, 3];
        let weights = vec![1.0, 2.0, 1.0];

        let mut counts = [0; 3];
        for _ in 0..100 {
            if let Some(&item) = weighted_sample(&items, &weights, &mut rng) {
                counts[item - 1] += 1;
            }
        }
        // Item 2 (index 1) should have highest count due to weight
        assert!(counts[1] >= counts[0]);
        assert!(counts[1] >= counts[2]);
    }

    #[test]
    fn test_weighted_sample_mismatch() {
        let mut rng = BitGen::new(42);
        let items = vec![1, 2, 3];
        let weights = vec![1.0, 2.0]; // Mismatched lengths

        assert!(weighted_sample(&items, &weights, &mut rng).is_none());
    }

    #[test]
    fn test_coin_flip() {
        let mut rng = BitGen::new(42);
        let _ = coin_flip(&mut rng);
        // Just test it doesn't panic
    }

    #[test]
    fn test_roll_die() {
        let mut rng = BitGen::new(42);
        for _ in 0..100 {
            let result = roll_die(6, &mut rng);
            assert!(result >= 1 && result <= 6);
        }
    }

    #[test]
    fn test_roll_dice() {
        let mut rng = BitGen::new(42);
        for _ in 0..100 {
            let sum = roll_dice(3, 6, &mut rng);
            assert!(sum >= 3 && sum <= 18);
        }
    }
}
