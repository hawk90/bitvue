//! Hash state for combining hash values.
//!
//! This module provides hash utilities similar to Abseil's `absl/hash` directory,
//! including HashState for combining hash values, and various hash-related utilities.

use core::hash::{Hash, Hasher};

/// A simple hasher for no_std compatibility (internal implementation).
///
/// # Security Warning
///
/// **This hasher is NOT suitable for security-critical applications.**
///
/// The `SimpleHasher` uses an FNV-1a style hash which is fast but has known
/// weaknesses:
///
/// - **Hash Flooding Vulnerability**: An attacker can craft inputs that cause
///   excessive hash collisions, leading to denial-of-service attacks on hash tables
/// - **Poor Avalanche Effect**: Similar inputs may produce similar hashes
/// - **Predictable Output**: The mixing function is simple and predictable
///
/// For security-sensitive applications (cryptography, authentication, etc.),
/// use a cryptographically secure hash function like SHA-256 or BLAKE3 instead.
///
/// For internal use (non-adversarial environments like caching), this hasher
/// provides good performance and acceptable distribution.
#[derive(Clone, Copy, Default)]
struct SimpleHasher {
    state: u64,
}

impl SimpleHasher {
    #[inline]
    const fn new() -> Self {
        Self {
            state: 0x9e3779b97f4a7c15,
        }
    }
}

impl Hasher for SimpleHasher {
    #[inline]
    fn finish(&self) -> u64 {
        self.state
    }

    #[inline]
    fn write(&mut self, bytes: &[u8]) {
        // FNV-1a style hashing
        for &byte in bytes {
            self.state ^= byte as u64;
            self.state = self.state.wrapping_mul(0x100000001b3);
        }
    }

    #[inline]
    fn write_u8(&mut self, i: u8) {
        self.state ^= i as u64;
        self.state = self.state.wrapping_mul(0x100000001b3);
    }

    #[inline]
    fn write_u16(&mut self, i: u16) {
        self.write(&i.to_le_bytes());
    }

    #[inline]
    fn write_u32(&mut self, i: u32) {
        self.write(&i.to_le_bytes());
    }

    #[inline]
    fn write_u64(&mut self, i: u64) {
        self.write(&i.to_le_bytes());
    }

    #[inline]
    fn write_usize(&mut self, i: usize) {
        self.write(&(i as u64).to_le_bytes());
    }

    #[inline]
    fn write_i8(&mut self, i: i8) {
        self.write_u8(i as u8);
    }

    #[inline]
    fn write_i16(&mut self, i: i16) {
        self.write_u16(i as u16);
    }

    #[inline]
    fn write_i32(&mut self, i: i32) {
        self.write_u32(i as u32);
    }

    #[inline]
    fn write_i64(&mut self, i: i64) {
        self.write_u64(i as u64);
    }

    #[inline]
    fn write_isize(&mut self, i: isize) {
        self.write_usize(i as usize);
    }
}

/// A hash state for combining hash values.
///
/// Similar to Abseil's `absl::HashState`, this type is used to combine
/// multiple hash values using a mixing function.
#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub struct HashState {
    state: u64,
}

impl HashState {
    /// Creates a new hash state with the given initial value.
    #[inline]
    pub const fn new(state: u64) -> Self {
        Self { state }
    }

    /// Creates a hash state combining the hash of one or more values.
    #[inline]
    pub fn combine<T: Hash + ?Sized>(values: &[&T]) -> u64 {
        let mut state = Self::default();
        for &value in values {
            state = state.update(value);
        }
        state.finalize()
    }

    /// Updates the hash state with a new value.
    #[inline]
    pub fn update<T: Hash + ?Sized>(mut self, value: &T) -> Self {
        let hash = hash_of(value);
        self.state = Self::mix(self.state, hash);
        self
    }

    /// Updates the hash state with a raw hash value.
    #[inline]
    pub fn update_raw(mut self, hash: u64) -> Self {
        self.state = Self::mix(self.state, hash);
        self
    }

    /// Updates the hash state with multiple values.
    #[inline]
    pub fn update_many<T: Hash + ?Sized>(mut self, values: &[&T]) -> Self {
        for &value in values {
            self = self.update(value);
        }
        self
    }

    /// Finalizes and returns the hash value.
    #[inline]
    pub fn finalize(self) -> u64 {
        self.state
    }

    /// Mixes two hash values using a similar approach to Abseil.
    ///
    /// This uses the "Mix" operation from Abseil's hashing algorithm.
    #[inline]
    fn mix(a: u64, b: u64) -> u64 {
        // Similar to Abseil's mixing function
        // Uses multiplicative hashing with rotation
        let m = 0x9e3779b97f4a7c15;
        a.wrapping_mul(m).wrapping_add(b).rotate_left(31)
    }
}

// Implement Hash for HashState for convenience
impl Hash for HashState {
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.state.hash(state);
    }
}

/// Computes the hash of a value using a simple hasher.
///
/// # Examples
///
/// ```
/// use abseil::absl_hash::hash::hash_of;
///
/// let hash = hash_of(&42);
/// let hash2 = hash_of(&42);
/// assert_eq!(hash, hash2);
/// ```
#[inline]
pub fn hash_of<T: Hash + ?Sized>(value: &T) -> u64 {
    let mut hasher = SimpleHasher::new();
    value.hash(&mut hasher);
    hasher.finish()
}

/// Computes a combined hash of multiple values.
///
/// # Examples
///
/// ```
/// use abseil::absl_hash::hash::hash_combine;
///
/// let hash = hash_combine(&[&1, &2, &3]);
/// let hash2 = hash_combine(&[&1, &2, &3]);
/// assert_eq!(hash, hash2);
/// ```
#[inline]
pub fn hash_combine<T: Hash + ?Sized>(values: &[&T]) -> u64 {
    HashState::combine(values)
}

/// Combines two hash values into one.
///
/// # Examples
///
/// ```
/// use abseil::absl_hash::hash::mix_hashes;
///
/// let hash1 = 12345u64;
/// let hash2 = 67890u64;
/// let combined = mix_hashes(hash1, hash2);
/// ```
#[inline]
pub fn mix_hashes(a: u64, b: u64) -> u64 {
    HashState::mix(a, b)
}

/// Piecewise hasher for streaming hash computation.
///
/// # Examples
///
/// ```
/// use abseil::absl_hash::hash::PiecewiseHasher;
///
/// let mut hasher = PiecewiseHasher::new();
/// hasher.update(&42);
/// hasher.update(&"hello");
/// let hash = hasher.finish();
/// ```
#[derive(Clone, Default)]
pub struct PiecewiseHasher {
    state: HashState,
}

impl PiecewiseHasher {
    /// Creates a new piecewise hasher.
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a new piecewise hasher with an initial state.
    #[inline]
    pub fn with_state(state: u64) -> Self {
        Self {
            state: HashState::new(state),
        }
    }

    /// Updates the hash with a value.
    #[inline]
    pub fn update<T: Hash + ?Sized>(&mut self, value: &T) {
        self.state = self.state.update(value);
    }

    /// Updates the hash with a raw hash value.
    #[inline]
    pub fn update_raw(&mut self, hash: u64) {
        self.state = self.state.update_raw(hash);
    }

    /// Finalizes and returns the hash value.
    #[inline]
    pub fn finish(self) -> u64 {
        self.state.finalize()
    }

    /// Resets the hasher to its initial state.
    #[inline]
    pub fn reset(&mut self) {
        self.state = HashState::default();
    }
}

impl Hasher for PiecewiseHasher {
    #[inline]
    fn finish(&self) -> u64 {
        self.state.finalize()
    }

    #[inline]
    fn write(&mut self, bytes: &[u8]) {
        self.state = self.state.update(bytes);
    }
}

/// Hash builder for custom hash types.
///
/// # Examples
///
/// ```
/// use abseil::absl_hash::hash::HashBuilder;
///
/// let hash = HashBuilder::new()
///     .add(&42)
///     .add(&"hello")
///     .build();
/// ```
#[derive(Clone, Default)]
pub struct HashBuilder {
    state: HashState,
}

impl HashBuilder {
    /// Creates a new hash builder.
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a new hash builder with a seed.
    #[inline]
    pub fn with_seed(seed: u64) -> Self {
        Self {
            state: HashState::new(seed),
        }
    }

    /// Adds a value to the hash.
    #[inline]
    pub fn add<T: Hash + ?Sized>(mut self, value: &T) -> Self {
        self.state = self.state.update(value);
        self
    }

    /// Adds multiple values to the hash.
    #[inline]
    pub fn add_many<T: Hash + ?Sized>(mut self, values: &[&T]) -> Self {
        self.state = self.state.update_many(values);
        self
    }

    /// Adds a raw hash value to the hash.
    #[inline]
    pub fn add_raw(mut self, hash: u64) -> Self {
        self.state = self.state.update_raw(hash);
        self
    }

    /// Builds and returns the final hash value.
    #[inline]
    pub fn build(self) -> u64 {
        self.state.finalize()
    }

    /// Returns the current hash state without consuming the builder.
    #[inline]
    pub fn current_hash(&self) -> u64 {
        self.state.finalize()
    }
}

/// Fingerprint type for 128-bit hash values.
#[derive(Clone, Copy, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Fingerprint {
    pub low: u64,
    pub high: u64,
}

impl Fingerprint {
    /// Creates a new fingerprint from low and high parts.
    #[inline]
    pub const fn new(low: u64, high: u64) -> Self {
        Self { low, high }
    }

    /// Creates a fingerprint from a single value.
    #[inline]
    pub fn from_value<T: Hash + ?Sized>(value: &T) -> Self {
        let hash = hash_of(value);
        // Use different mixing functions for low and high parts
        let low = hash.wrapping_mul(0x9e3779b97f4a7c15);
        let high = hash.wrapping_mul(0x85ebca6b).rotate_left(31);
        Self { low, high }
    }

    /// Combines two fingerprints.
    #[inline]
    pub fn combine(&self, other: &Fingerprint) -> Fingerprint {
        Fingerprint {
            low: mix_hashes(self.low, other.low),
            high: mix_hashes(self.high, other.high),
        }
    }

    /// Returns the fingerprint as a 128-bit value.
    #[inline]
    pub fn as_u128(&self) -> u128 {
        ((self.high as u128) << 64) | (self.low as u128)
    }

    /// Creates a fingerprint from a u128 value.
    #[inline]
    pub fn from_u128(value: u128) -> Self {
        Self {
            low: value as u64,
            high: (value >> 64) as u64,
        }
    }
}

impl core::fmt::Debug for Fingerprint {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "Fingerprint({:016x}{:016x})", self.high, self.low)
    }
}

impl core::fmt::Display for Fingerprint {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{:016x}{:016x}", self.high, self.low)
    }
}

/// A simple deterministic hasher for testing.
///
/// This hasher produces predictable hash values for testing purposes.
#[derive(Clone, Copy, Default)]
pub struct TestHasher {
    state: u64,
}

impl TestHasher {
    /// Creates a new test hasher.
    #[inline]
    pub const fn new() -> Self {
        Self { state: 0 }
    }

    /// Creates a new test hasher with an initial state.
    #[inline]
    pub const fn with_state(state: u64) -> Self {
        Self { state }
    }
}

impl Hasher for TestHasher {
    #[inline]
    fn finish(&self) -> u64 {
        self.state
    }

    #[inline]
    fn write(&mut self, bytes: &[u8]) {
        // Simple deterministic hash
        for &byte in bytes {
            self.state = self.state.wrapping_mul(31).wrapping_add(byte as u64);
        }
    }
}

/// Returns a hash value for a slice using a deterministic algorithm.
///
/// # Examples
///
/// ```
/// use abseil::absl_hash::hash::hash_slice;
///
/// let slice = &[1, 2, 3, 4, 5];
/// let hash = hash_slice(slice);
/// ```
#[inline]
pub fn hash_slice<T: Hash>(slice: &[T]) -> u64 {
    let mut state = HashState::default();
    for item in slice {
        state = state.update(item);
    }
    state.finalize()
}

/// Returns a hash value for a pair of values.
///
/// # Examples
///
/// ```
/// use abseil::absl_hash::hash::hash_pair;
///
/// let hash = hash_pair(&1, &2);
/// let hash2 = hash_pair(&1, &2);
/// assert_eq!(hash, hash2);
/// ```
#[inline]
pub fn hash_pair<A: Hash + ?Sized, B: Hash + ?Sized>(a: &A, b: &B) -> u64 {
    HashState::default().update(a).update(b).finalize()
}

/// Returns a hash value for a triple of values.
#[inline]
pub fn hash_triple<A: Hash + ?Sized, B: Hash + ?Sized, C: Hash + ?Sized>(
    a: &A,
    b: &B,
    c: &C,
) -> u64 {
    HashState::default()
        .update(a)
        .update(b)
        .update(c)
        .finalize()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_state_new() {
        let state = HashState::new(42);
        assert_eq!(state.finalize(), 42);
    }

    #[test]
    fn test_hash_state_default() {
        let state = HashState::default();
        assert_eq!(state.finalize(), 0);
    }

    #[test]
    fn test_hash_state_update() {
        let state = HashState::default();
        let state = state.update(&42);
        let state = state.update(&"hello");
        // Just verify it runs and produces some hash
        assert_ne!(state.finalize(), 0);
    }

    #[test]
    fn test_hash_state_combine() {
        let hash = HashState::combine(&[&1, &2, &3]);
        // Just verify it runs and produces some hash
        assert_ne!(hash, 0);
    }

    #[test]
    fn test_hash_state_clone() {
        let state1 = HashState::new(42);
        let state2 = state1;
        assert_eq!(state1.finalize(), 42);
        assert_eq!(state2.finalize(), 42);
    }

    #[test]
    fn test_hash_state_update_raw() {
        let state = HashState::default();
        let state = state.update_raw(12345);
        assert_ne!(state.finalize(), 0);
    }

    #[test]
    fn test_hash_state_update_many() {
        let state = HashState::default();
        let values: Vec<&i32> = vec![&1, &2, &3, &4, &5];
        let state = state.update_many(&values);
        assert_ne!(state.finalize(), 0);
    }

    #[test]
    fn test_hash_of() {
        let hash1 = hash_of(&42);
        let hash2 = hash_of(&42);
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_hash_of_different() {
        let hash1 = hash_of(&42);
        let hash2 = hash_of(&43);
        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_hash_combine() {
        let hash1 = hash_combine(&[&1, &2, &3]);
        let hash2 = hash_combine(&[&1, &2, &3]);
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_mix_hashes() {
        let hash1 = mix_hashes(12345, 67890);
        assert_ne!(hash1, 0);
        // Mixing should be order-independent for first argument
        let hash2 = mix_hashes(54321, 67890);
        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_piecewise_hasher() {
        let mut hasher = PiecewiseHasher::new();
        hasher.update(&42);
        hasher.update(&"hello");
        let hash = hasher.finish();
        assert_ne!(hash, 0);
    }

    #[test]
    fn test_piecewise_hasher_with_state() {
        let hasher = PiecewiseHasher::with_state(12345);
        let hash = hasher.finish();
        assert_eq!(hash, 12345);
    }

    #[test]
    fn test_piecewise_hasher_reset() {
        let mut hasher = PiecewiseHasher::new();
        hasher.update(&42);
        hasher.reset();
        let hash = hasher.finish();
        assert_eq!(hash, 0);
    }

    #[test]
    fn test_hash_builder() {
        let hash = HashBuilder::new().add(&42).add(&"hello").build();
        assert_ne!(hash, 0);
    }

    #[test]
    fn test_hash_builder_with_seed() {
        let hash1 = HashBuilder::with_seed(12345).add(&42).build();
        let hash2 = HashBuilder::with_seed(12345).add(&42).build();
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_hash_builder_current_hash() {
        let builder = HashBuilder::new().add(&42);
        let hash1 = builder.current_hash();
        let hash2 = builder.add(&"hello").build();
        assert_ne!(hash1, 0);
        assert_ne!(hash2, 0);
    }

    #[test]
    fn test_fingerprint_new() {
        let fp = Fingerprint::new(123, 456);
        assert_eq!(fp.low, 123);
        assert_eq!(fp.high, 456);
    }

    #[test]
    fn test_fingerprint_from_value() {
        let fp = Fingerprint::from_value(&42);
        assert_ne!(fp.low, 0);
        assert_ne!(fp.high, 0);
    }

    #[test]
    fn test_fingerprint_combine() {
        let fp1 = Fingerprint::new(1, 2);
        let fp2 = Fingerprint::new(3, 4);
        let combined = fp1.combine(&fp2);
        assert_ne!(combined.low, 0);
        assert_ne!(combined.high, 0);
    }

    #[test]
    fn test_fingerprint_as_u128() {
        let fp = Fingerprint::new(0x1111111111111111, 0x2222222222222222);
        let val = fp.as_u128();
        assert_eq!(val, 0x22222222222222221111111111111111);
    }

    #[test]
    fn test_fingerprint_from_u128() {
        let val: u128 = 0x22222222222222221111111111111111;
        let fp = Fingerprint::from_u128(val);
        assert_eq!(fp.high, 0x2222222222222222);
        assert_eq!(fp.low, 0x1111111111111111);
    }

    #[test]
    fn test_test_hasher() {
        let mut hasher = TestHasher::new();
        hasher.write(&[1, 2, 3, 4, 5]);
        let hash = hasher.finish();
        assert_ne!(hash, 0);
    }

    #[test]
    fn test_test_hasher_with_state() {
        let hasher = TestHasher::with_state(42);
        assert_eq!(hasher.finish(), 42);
    }

    #[test]
    fn test_hash_slice() {
        let slice = &[1, 2, 3, 4, 5];
        let hash = hash_slice(slice);
        assert_ne!(hash, 0);
    }

    #[test]
    fn test_hash_pair() {
        let hash1 = hash_pair(&1, &2);
        let hash2 = hash_pair(&1, &2);
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_hash_triple() {
        let hash1 = hash_triple(&1, &2, &3);
        let hash2 = hash_triple(&1, &2, &3);
        assert_eq!(hash1, hash2);
    }

    // Edge case tests

    #[test]
    fn test_hash_empty_inputs() {
        // Hash of empty slice
        let hash1 = hash_slice(&[] as &[i32]);
        let hash2 = hash_slice(&[] as &[i32]);
        // Empty slice should hash consistently
        assert_eq!(hash1, hash2);

        // Empty string should also hash consistently
        let hash_str1 = hash_of("");
        let hash_str2 = hash_of("");
        assert_eq!(hash_str1, hash_str2);
    }

    #[test]
    fn test_hash_single_element() {
        // Single element should hash consistently
        let hash1 = hash_slice(&[42]);
        let hash2 = hash_slice(&[42]);
        assert_eq!(hash1, hash2);
        // And different from empty
        let hash_empty = hash_slice(&[] as &[i32]);
        assert_ne!(hash1, hash_empty);
    }

    #[test]
    fn test_hash_large_inputs() {
        // Large slice should hash without issues
        let large_slice: [u8; 1024] = [0u8; 1024];
        let hash = hash_slice(&large_slice);
        assert_ne!(hash, 0);

        // Different large inputs should hash differently
        let large_slice2: [u8; 1024] = [1u8; 1024];
        let hash2 = hash_slice(&large_slice2);
        assert_ne!(hash, hash2);
    }

    #[test]
    fn test_hash_max_min_values() {
        // Test with extreme numeric values
        assert_ne!(hash_of(&i64::MIN), hash_of(&i64::MAX));
        assert_ne!(hash_of(&u64::MIN), hash_of(&u64::MAX));
        assert_ne!(hash_of(&i128::MIN), hash_of(&i128::MAX));
        assert_ne!(hash_of(&u128::MIN), hash_of(&u128::MAX));
    }

    #[test]
    fn test_hash_state_update_with_zero() {
        // Updating with zero values should still change hash
        let state = HashState::new(0x12345678);
        let state = state.update(&0);
        assert_ne!(state.finalize(), 0x12345678);
    }
}
