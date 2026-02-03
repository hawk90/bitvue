//! Reservoir sampling utilities - reservoir_sample

use alloc::vec::Vec;

use crate::absl_random::bit_gen::BitGen;

/// Reservoir sampling: samples k elements from a stream of unknown length.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_random::reservoir_sample;
///
/// let mut rng = BitGen::new(42);
/// let stream = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
/// let sampled = reservoir_sample(stream.iter(), 3, &mut rng);
/// assert_eq!(sampled.len(), 3);
/// ```
pub fn reservoir_sample<'a, T, I>(stream: I, k: usize, rng: &mut BitGen) -> Vec<&'a T>
where
    I: IntoIterator<Item = &'a T>,
{
    let mut reservoir: Vec<&'a T> = Vec::with_capacity(k);
    let mut count = 0;

    for item in stream {
        count += 1;
        if reservoir.len() < k {
            reservoir.push(item);
        } else {
            let idx = rng.gen_usize() % count;
            if idx < k {
                reservoir[idx] = item;
            }
        }
    }

    reservoir
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reservoir_sample() {
        let mut rng = BitGen::new(42);
        let stream = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
        let sampled = reservoir_sample(stream.iter(), 3, &mut rng);
        assert_eq!(sampled.len(), 3);
    }

    #[test]
    fn test_reservoir_sample_small_stream() {
        let mut rng = BitGen::new(42);
        let stream = vec![1, 2];
        let sampled = reservoir_sample(stream.iter(), 5, &mut rng);
        assert_eq!(sampled.len(), 2);
    }
}
