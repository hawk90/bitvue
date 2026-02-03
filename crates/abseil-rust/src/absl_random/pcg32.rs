//! PCG32 random number generator.

/// PCG random number generator (PCG-XSH-RR-64/32).
///
/// A high-quality PRNG with good statistical properties.
#[derive(Clone, Debug)]
pub struct Pcg32 {
    state: u64,
    inc: u64,
}

impl Pcg32 {
    /// Creates a new PCG32 with the given seed.
    pub const fn new(seed: u64) -> Self {
        Self {
            state: seed,
            inc: seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407),
        }
    }

    /// Generates a random u32.
    pub fn next_u32(&mut self) -> u32 {
        let oldstate = self.state;
        self.state = oldstate
            .wrapping_mul(6364136223846793005)
            .wrapping_add(self.inc);
        let xorshifted = (((oldstate >> 18) ^ oldstate) >> 27) as u32;
        let rot = (oldstate >> 59) as u32;
        xorshifted.rotate_right(rot)
    }

    /// Generates a random u64.
    pub fn next_u64(&mut self) -> u64 {
        ((self.next_u32() as u64) << 32) | (self.next_u32() as u64)
    }

    /// Generates a random f64 in [0, 1).
    pub fn next_f64(&mut self) -> f64 {
        self.next_u32() as f64 / (u32::MAX as f64 + 1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pcg32() {
        let mut rng = Pcg32::new(42);
        let val1 = rng.next_u32();
        let val2 = rng.next_u32();
        assert_ne!(val1, val2);

        let f = rng.next_f64();
        assert!(f >= 0.0 && f < 1.0);
    }
}
}
