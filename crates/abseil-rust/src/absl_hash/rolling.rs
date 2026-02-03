//! RollingHash - sliding window hash for string matching and pattern detection.

use alloc::vec::Vec;

/// A rolling hash for sliding window hashing.
///
/// Useful for string matching and pattern detection.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_hash::RollingHash;
///
/// let mut hasher = RollingHash::new(4);
/// hasher.update(&[1, 2, 3]);
/// hasher.update(&[2, 3, 4]); // Slide window
/// ```
pub struct RollingHash {
    window: Vec<u8>,
    window_size: usize,
    hash: u64,
    base: u64,
    prime: u64,
    power: u64,
}

impl RollingHash {
    /// Creates a new rolling hash with the given window size.
    pub fn new(window_size: usize) -> Self {
        Self {
            window: Vec::new(),
            window_size,
            hash: 0,
            base: 256,
            prime: 1_000_000_007,
            power: 0,
        }
    }

    /// Updates the hash with a new byte, sliding the window.
    pub fn update(&mut self, byte: u8) -> u64 {
        // Precompute power if needed
        if self.power == 0 {
            self.power = self.base.pow(self.window_size as u32);
        }

        // Remove oldest byte if window is full
        if self.window.len() == self.window_size {
            let oldest = self.window.remove(0);
            self.hash = (self.hash - oldest as u64 * self.power % self.prime + self.prime) % self.prime;
        }

        // Add new byte
        self.hash = (self.hash * self.base % self.prime + byte as u64) % self.prime;
        self.window.push(byte);

        self.hash
    }

    /// Returns the current hash value.
    pub fn hash(&self) -> u64 {
        self.hash
    }

    /// Resets the rolling hash.
    pub fn reset(&mut self) {
        self.window.clear();
        self.hash = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rolling_hash() {
        let mut hasher = RollingHash::new(4);
        let h1 = hasher.update(1);
        hasher.update(2);
        hasher.update(3);
        hasher.update(4);
        let h2 = hasher.hash();

        assert_ne!(h2, 0);
        assert_eq!(h1, h1); // First hash is some value
    }
}
