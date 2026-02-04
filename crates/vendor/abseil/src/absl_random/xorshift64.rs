//! XorShift64* random number generator.

/// XorShift64* random number generator.
///
/// A fast, high-quality PRNG suitable for non-cryptographic use.
#[derive(Clone, Debug)]
pub struct XorShift64 {
    state: u64,
}

impl XorShift64 {
    /// Creates a new XorShift64* with the given seed.
    pub const fn new(seed: u64) -> Self {
        Self { state: seed.wrapping_add(1) }
    }

    /// Generates a random u64.
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.state = x;
        x.wrapping_mul(0x2545F4914F6CDD1D)
    }

    /// Generates a random u32.
    pub fn next_u32(&mut self) -> u32 {
        self.next_u64() as u32
    }

    /// Generates a random f64 in [0, 1).
    pub fn next_f64(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_xorshift64() {
        let mut rng = XorShift64::new(42);
        let val1 = rng.next_u64();
        let val2 = rng.next_u64();
        assert_ne!(val1, val2);

        let f = rng.next_f64();
        assert!(f >= 0.0 && f < 1.0);
    }
}
