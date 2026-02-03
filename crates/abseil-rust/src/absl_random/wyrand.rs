//! WyRand random number generator.

/// WyRand random number generator.
///
/// A fast PRNG with good statistical properties and small state.
#[derive(Clone, Debug)]
pub struct WyRand {
    state: u64,
}

impl WyRand {
    /// Creates a new WyRand with the given seed.
    pub const fn new(seed: u64) -> Self {
        Self { state: seed.wrapping_add(1) }
    }

    /// Generates a random u64.
    pub fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0xA0761D6478BD642F);
        let t = (self.state as u128).wrapping_mul(self.state as u128);
        let t1 = t as u64;
        let t2 = (t >> 64) as u64;
        let xr = t1.wrapping_add(t2);
        (xr >> 1) | ((xr & 1) << 63)
    }

    /// Generates a random u32.
    pub fn next_u32(&mut self) -> u32 {
        self.next_u64() as u32
    }

    /// Generates a random f64 in [0, 1).
    pub fn next_f64(&mut self) -> f64 {
        self.next_u64() as f64 / (u64::MAX as f64 + 1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wyrand() {
        let mut rng = WyRand::new(42);
        let val1 = rng.next_u64();
        let val2 = rng.next_u64();
        assert_ne!(val1, val2);

        let f = rng.next_f64();
        assert!(f >= 0.0 && f < 1.0);
    }
}
}
