//! Thread-local random number generator.

use core::cell::Cell;

/// Thread-local random number generator.
///
/// This provides a convenient way to get a random number generator
/// without having to manage seeding manually.
pub struct ThreadLocalRng {
    state: Cell<u64>,
}

impl ThreadLocalRng {
    /// Creates a new thread-local RNG with a fixed seed.
    pub const fn new() -> Self {
        Self {
            state: Cell::new(0x123456789ABCDEF),
        }
    }

    /// Gets the current seed value.
    pub fn seed(&self) -> u64 {
        self.state.get()
    }

    /// Sets a new seed value.
    pub fn set_seed(&self, seed: u64) {
        self.state.set(seed);
    }

    /// Generates a random u64.
    pub fn gen_u64(&self) -> u64 {
        let mut state = self.state.get();
        // SplitMix64 hash function
        state = state.wrapping_add(0x9E3779B97F4A7C15);
        let mut z = state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
        z = z ^ (z >> 31);
        self.state.set(state);
        z
    }

    /// Generates a random u32.
    pub fn gen_u32(&self) -> u32 {
        self.gen_u64() as u32
    }

    /// Generates a random usize.
    pub fn gen_usize(&self) -> usize {
        self.gen_u64() as usize
    }

    /// Generates a random bool.
    pub fn gen_bool(&self) -> bool {
        (self.gen_u64() & 1) == 1
    }

    /// Generates a random i32.
    pub fn gen_i32(&self) -> i32 {
        self.gen_u32() as i32
    }

    /// Generates a random f64 in [0, 1).
    pub fn gen_f64(&self) -> f64 {
        self.gen_u64() as f64 / (u64::MAX as f64)
    }
}

impl Default for ThreadLocalRng {
    fn default() -> Self {
        Self::new()
    }
}

/// Global thread-local random number generator.
///
/// This is a convenience for quick random operations.
pub fn thread_rng() -> ThreadLocalRng {
    ThreadLocalRng::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_thread_local_rng() {
        let rng = ThreadLocalRng::new();
        let _ = rng.gen_u64();
        let _ = rng.gen_u32();
        let _ = rng.gen_bool();
        let _ = rng.gen_f64();
        // Just test it works
    }

    #[test]
    fn test_thread_local_rng_seed() {
        let rng = ThreadLocalRng::new();
        let seed1 = rng.seed();
        rng.set_seed(12345);
        assert_eq!(rng.seed(), 12345);
        let val1 = rng.gen_u64();
        rng.set_seed(12345);
        let val2 = rng.gen_u64();
        assert_eq!(val1, val2); // Same seed, same value
    }
}
