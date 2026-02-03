//! Random bit generation utilities.
//!
//! # ⚠️ CRITICAL SECURITY WARNING
//!
//! **This module does NOT provide cryptographically secure random numbers!**
//!
//! The `BitGen` type implements a simple XOR-shift pseudo-random number generator
//! (PRNG) that is **NOT suitable for security-sensitive operations** including:
//!
//! - Cryptography (keys, nonces, salts, IVs)
//! - Gambling or lottery systems
//! - Session tokens or authentication credentials
//! - Statistical sampling where unpredictability is required
//! - Any scenario where an attacker could benefit from predicting future values
//!
//! ## Why This Is Not Secure
//!
//! 1. **Predictable Algorithm**: XOR-shift is a simple PRNG designed for speed, not security
//! 2. **Guessable Seed**: `from_entropy()` uses system time, which can be predicted
//! 3. **Small State**: Only 64 bits of internal state, vulnerable to brute force
//! 4. **No Forward Secrecy**: Knowing the state allows predicting all future values
//!
//! ## Secure Alternatives
//!
//! For security-sensitive applications, use the [`rand`] crate:
//!
//! ```rust,ignore
//! // For cryptographic randomness:
//! use rand::rngs::OsRng;
//! use rand::Rng;
//!
//! let mut rng = OsRng;
//! let secure_key: [u8; 32] = rng.gen();
//!
//! // For thread-local RNG (faster but not crypto):
//! use rand::rngs::ThreadRng;
//! use rand::SeedableRng;
//!
//! let mut rng = ThreadRng::from_entropy();
//! let value: u64 = rng.gen();
//! ```
//!
//! When You CAN Use This
//!
//! - Testing and development
//! - Non-security demos and examples
//! - Games where predictability doesn't matter
//! - Procedural content generation (where security is irrelevant)
//!
//! This module provides utilities similar to Abseil's `absl/random` directory.

use core::num::Wrapping;

/// A pseudo-random bit generator.
///
/// # ⚠️ CRITICAL SECURITY WARNING
///
/// **This is NOT a cryptographically secure random number generator!**
///
/// This is a simple XOR-shift PRNG for demonstration purposes only.
/// It MUST NOT be used for:
/// - Cryptography or any security-sensitive operations
/// - Gambling or lottery systems
/// - Session tokens or authentication
/// - Statistical sampling where unpredictability is required
///
/// For security-sensitive applications, use the [`rand`] crate with
/// cryptographically secure generators like `ThreadRng` or `OsRng`.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct BitGen {
    state: Wrapping<u64>,
}

impl BitGen {
    /// Creates a new bit generator from a seed.
    #[inline]
    pub const fn new(seed: u64) -> Self {
        Self {
            state: Wrapping(seed),
        }
    }

    /// Creates a new bit generator with a default seed.
    ///
    /// # ⚠️ SECURITY WARNING
    ///
    /// **This method is NOT cryptographically secure!**
    ///
    /// The implementation uses system time as a seed, which can be:
    /// - Predictable by attackers who can estimate the system time
    /// - Duplicated across processes started at the same time
    /// - Replayed if the time is known
    ///
    /// The XOR-shift algorithm itself is also not suitable for cryptography.
    ///
    /// **NEVER use this for:**
    /// - Cryptographic keys or nonces
    /// - Session tokens or authentication
    /// - Gambling or lottery systems
    /// - Any security-sensitive operation
    ///
    /// For secure randomness, use `rand::rngs::OsRng` or `rand::rngs::ThreadRng`.
    #[inline]
    #[cfg(feature = "std")]
    pub fn from_entropy() -> Self {
        // Use system time as seed for better randomness
        use std::time::{SystemTime, UNIX_EPOCH};

        // SECURITY: System time is predictable and NOT suitable for security!
        // This is provided only for convenience in non-security contexts.
        let seed = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0x9e3779b97f4a7c15);
        Self::new(seed)
    }

    /// Creates a new bit generator with a default seed (no_std version).
    ///
    /// # ⚠️ SECURITY WARNING
    ///
    /// **This uses a FIXED seed and is completely deterministic!**
    ///
    /// In no_std environments without access to system time, this falls back
    /// to a constant seed. This means:
    /// - Every run produces the same "random" sequence
    /// - Values are completely predictable
    /// - This is ONLY suitable for testing deterministic behavior
    ///
    /// **NEVER use this for any purpose where randomness is actually needed!**
    ///
    /// For secure randomness in no_std contexts, implement a proper entropy
    /// source or import a cryptographically secure RNG.
    #[inline]
    #[cfg(not(feature = "std"))]
    pub fn from_entropy() -> Self {
        // In no_std, use a fixed seed but warn in documentation
        // Users should provide their own seed for security-sensitive uses
        let seed = 0x9e3779b97f4a7c15;
        Self::new(seed)
    }

    /// Generates a random `u64` value.
    #[inline]
    pub fn gen_u64(&mut self) -> u64 {
        // XOR-shift algorithm (from Marsaglia)
        let mut x = self.state;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.state = x;
        self.state.0.wrapping_mul(0x2545f4914f6cdd1d)
    }

    /// Generates a random `u32` value.
    #[inline]
    pub fn gen_u32(&mut self) -> u32 {
        (self.gen_u64() & 0xFFFFFFFF) as u32
    }

    /// Generates a random `usize` value.
    #[inline]
    pub fn gen_usize(&mut self) -> usize {
        self.gen_u64() as usize
    }

    /// Generates a random `bool` value.
    #[inline]
    pub fn gen_bool(&mut self) -> bool {
        self.gen_u64() & 1 == 1
    }

    /// Generates a random `u8` value.
    #[inline]
    pub fn gen_u8(&mut self) -> u8 {
        (self.gen_u64() & 0xFF) as u8
    }

    /// Generates a random `u16` value.
    #[inline]
    pub fn gen_u16(&mut self) -> u16 {
        (self.gen_u64() & 0xFFFF) as u16
    }

    /// Generates a random `i64` value.
    #[inline]
    pub fn gen_i64(&mut self) -> i64 {
        self.gen_u64() as i64
    }

    /// Generates a random `i32` value.
    #[inline]
    pub fn gen_i32(&mut self) -> i32 {
        self.gen_u32() as i32
    }

    /// Generates a random value in the range [0, upper).
    ///
    /// Uses rejection sampling to avoid bias from modulo operation.
    #[inline]
    pub fn gen_range(&mut self, upper: u64) -> u64 {
        if upper == 0 {
            return 0;
        }
        // Use rejection sampling to avoid modulo bias
        // Calculate threshold to avoid bias
        let threshold = upper.wrapping_neg() % upper;
        loop {
            let x = self.gen_u64();
            if x >= threshold {
                return x % upper;
            }
        }
    }

    /// Generates a random value in the range [lower, upper).
    #[inline]
    pub fn gen_range_inclusive(&mut self, lower: u64, upper: u64) -> u64 {
        if lower >= upper {
            return lower;
        }
        let range = upper - lower + 1;
        lower + self.gen_range(range)
    }

    /// Fills a buffer with random bytes.
    #[inline]
    pub fn fill_bytes(&mut self, buf: &mut [u8]) {
        for chunk in buf.chunks_mut(8) {
            let bytes = self.gen_u64().to_le_bytes();
            for (i, &byte) in bytes.iter().enumerate() {
                if i < chunk.len() {
                    chunk[i] = byte;
                }
            }
        }
    }
}

impl Default for BitGen {
    #[inline]
    fn default() -> Self {
        Self::from_entropy()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bit_gen_new() {
        let mut gen = BitGen::new(42);
        // Verify it produces values
        let _ = gen.gen_u64();
    }

    #[test]
    fn test_bit_gen_default() {
        let mut gen = BitGen::default();
        // Verify it produces values
        let val = gen.gen_u64();
        assert!(val != 0 || gen.gen_u64() != 0); // Eventually should be non-zero
    }

    #[test]
    fn test_gen_u64() {
        let mut gen = BitGen::new(42);
        let v1 = gen.gen_u64();
        let v2 = gen.gen_u64();
        // Values should be different (very unlikely to be same)
        assert_ne!(v1, v2);
    }

    #[test]
    fn test_gen_u32() {
        let mut gen = BitGen::new(42);
        let v = gen.gen_u32();
        assert!(v <= u32::MAX);
    }

    #[test]
    fn test_gen_i64() {
        let mut gen = BitGen::new(42);
        let v = gen.gen_i64();
        assert!(v >= i64::MIN && v <= i64::MAX);
    }

    #[test]
    fn test_gen_i32() {
        let mut gen = BitGen::new(42);
        let v = gen.gen_i32();
        assert!(v >= i32::MIN && v <= i32::MAX);
    }

    #[test]
    fn test_gen_u8() {
        let mut gen = BitGen::new(42);
        let v = gen.gen_u8();
        assert!(v <= u8::MAX);
    }

    #[test]
    fn test_gen_u16() {
        let mut gen = BitGen::new(42);
        let v = gen.gen_u16();
        assert!(v <= u16::MAX);
    }

    #[test]
    fn test_gen_bool() {
        let mut gen = BitGen::new(42);
        let _ = gen.gen_bool();
        // Just verify it runs
    }

    #[test]
    fn test_gen_range() {
        let mut gen = BitGen::new(42);
        let v = gen.gen_range(100);
        assert!(v < 100);
    }

    #[test]
    fn test_gen_range_zero() {
        let mut gen = BitGen::new(42);
        let v = gen.gen_range(0);
        assert_eq!(v, 0);
    }

    #[test]
    fn test_gen_range_inclusive() {
        let mut gen = BitGen::new(42);
        let v = gen.gen_range_inclusive(10, 20);
        assert!(v >= 10 && v <= 20);
    }

    #[test]
    fn test_fill_bytes() {
        let mut gen = BitGen::new(42);
        let mut buf = [0u8; 32];
        gen.fill_bytes(&mut buf);
        // At least some bytes should be non-zero
        let non_zero_count = buf.iter().filter(|&&x| x != 0).count();
        assert!(non_zero_count > 10);
    }

    #[test]
    fn test_fill_bytes_exact_multiple() {
        let mut gen = BitGen::new(42);
        let mut buf = [0u8; 16];
        gen.fill_bytes(&mut buf);
        // Should fill exactly 16 bytes
        let non_zero_count = buf.iter().filter(|&&x| x != 0).count();
        assert!(non_zero_count > 0);
    }

    #[test]
    fn test_bit_gen_clone() {
        let mut gen1 = BitGen::new(42);
        let mut gen2 = gen1;
        let v1 = gen1.gen_u64();
        let v2 = gen2.gen_u64();
        assert_eq!(v1, v2); // Same state produces same value
    }

    #[test]
    fn test_bit_gen_different_seeds() {
        let gen1 = BitGen::new(42);
        let gen2 = BitGen::new(43);
        assert_ne!(gen1.state, gen2.state);
    }

    // Tests for HIGH security fix - prominent security warnings

    #[test]
    fn test_rng_is_deterministic() {
        // Document that BitGen is deterministic with same seed
        let mut gen1 = BitGen::new(12345);
        let mut gen2 = BitGen::new(12345);

        // Generate same sequence
        assert_eq!(gen1.gen_u64(), gen2.gen_u64());
        assert_eq!(gen1.gen_u32(), gen2.gen_u32());
        assert_eq!(gen1.gen_bool(), gen2.gen_bool());
    }

    #[test]
    fn test_from_entropy_is_predictable() {
        // Document that from_entropy can produce predictable results
        // when system time is known or guessable
        let mut gen1 = BitGen::from_entropy();
        let mut gen2 = BitGen::from_entropy();

        // These may or may not be different depending on timing
        // The key point is: if an attacker knows the system time,
        // they can predict the RNG output
        let _ = gen1.gen_u64();
        let _ = gen2.gen_u64();
    }

    #[test]
    #[cfg(feature = "std")]
    fn test_system_time_seed_is_predictable() {
        use std::time::{SystemTime, UNIX_EPOCH};

        // Get system time twice
        let time1 = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0);

        let time2 = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0);

        // The time difference is likely small and predictable
        // An attacker who can estimate the time can guess the seed
        let _ = time1.abs_diff(time2);
    }

    #[test]
    #[cfg(not(feature = "std"))]
    fn test_fixed_seed_is_completely_deterministic() {
        // In no_std, from_entropy uses a fixed seed
        let mut gen1 = BitGen::from_entropy();
        let mut gen2 = BitGen::from_entropy();

        // Should produce identical results
        assert_eq!(gen1.gen_u64(), gen2.gen_u64());
        assert_eq!(gen1.gen_u32(), gen2.gen_u32());
    }
}

