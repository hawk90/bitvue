//! Random number distributions.
//!
//! Provides various random number distributions for generating random values
//! following common probability distributions.

use core::ops::RangeInclusive;

/// Uniform distribution over a range.
///
/// Generates random values uniformly distributed in a given range.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_random::distributions::Uniform;
///
/// // Example usage with a type that implements the BitGen trait:
/// // let mut rng = MyBitGen::new(42);
/// // let dist = Uniform::<f64>::new(1.0..=100.0);
/// // let value = dist.sample(&mut rng);
/// // assert!(value >= 1.0 && value <= 100.0);
/// ```
#[derive(Clone, Copy, Debug)]
pub struct Uniform<T> {
    low: T,
    high: T,
}

impl Uniform<f64> {
    /// Creates a new uniform distribution over the given range.
    #[inline]
    pub fn new(range: RangeInclusive<f64>) -> Self {
        Uniform {
            low: *range.start(),
            high: *range.end(),
        }
    }

    /// Samples a value from this distribution using the given RNG.
    pub fn sample<B: BitGen>(&self, rng: &mut B) -> f64 {
        let range = self.high - self.low;
        self.low + range * rng.gen_f64()
    }
}

impl Uniform<f32> {
    /// Creates a new uniform distribution over the given range.
    #[inline]
    pub fn new(range: RangeInclusive<f32>) -> Self {
        Uniform {
            low: *range.start(),
            high: *range.end(),
        }
    }

    /// Samples a value from this distribution using the given RNG.
    pub fn sample<B: BitGen>(&self, rng: &mut B) -> f32 {
        let range = self.high - self.low;
        self.low + range * rng.gen_f32()
    }
}

impl Uniform<i64> {
    /// Creates a new uniform distribution over the given range.
    #[inline]
    pub fn new(range: RangeInclusive<i64>) -> Self {
        Uniform {
            low: *range.start(),
            high: *range.end(),
        }
    }

    /// Samples a value from this distribution using the given RNG.
    ///
    /// Uses rejection sampling to avoid modulo bias when the range
    /// doesn't evenly divide i64::MAX.
    pub fn sample<B: BitGen>(&self, rng: &mut B) -> i64 {
        let range = self.high - self.low + 1;
        if range <= 0 {
            return self.low;
        }

        // For signed integers, use u64 for rejection sampling
        // then convert to i64
        let range_u64 = range as u64;

        // Rejection sampling to avoid modulo bias
        // Calculate the largest multiple of range that fits in u64
        let max_accept = u64::MAX - (u64::MAX % range_u64) - 1;

        loop {
            let random = rng.gen_u64();
            if random <= max_accept {
                return self.low + ((random % range_u64) as i64);
            }
            // Reject and try again
        }
    }
}

impl Uniform<u64> {
    /// Creates a new uniform distribution over the given range.
    #[inline]
    pub fn new(range: RangeInclusive<u64>) -> Self {
        Uniform {
            low: *range.start(),
            high: *range.end(),
        }
    }

    /// Samples a value from this distribution using the given RNG.
    ///
    /// Uses rejection sampling to avoid modulo bias when the range
    /// doesn't evenly divide u64::MAX.
    pub fn sample<B: BitGen>(&self, rng: &mut B) -> u64 {
        let range = self.high - self.low + 1;
        if range == 0 {
            return self.low;
        }

        // Rejection sampling to avoid modulo bias
        // Calculate the largest multiple of range that fits in u64
        let max_accept = u64::MAX - (u64::MAX % range) - 1;

        loop {
            let random = rng.gen_u64();
            if random <= max_accept {
                return self.low + (random % range);
            }
            // Reject and try again
        }
    }
}

impl Uniform<i32> {
    /// Creates a new uniform distribution over the given range.
    #[inline]
    pub fn new(range: RangeInclusive<i32>) -> Self {
        Uniform {
            low: *range.start(),
            high: *range.end(),
        }
    }

    /// Samples a value from this distribution using the given RNG.
    ///
    /// Uses rejection sampling to avoid modulo bias when the range
    /// doesn't evenly divide the random value's range.
    pub fn sample<B: BitGen>(&self, rng: &mut B) -> i32 {
        let range = (self.high - self.low + 1) as i64;
        if range <= 0 {
            return self.low;
        }

        // Use u64 for rejection sampling
        let range_u64 = range as u64;

        // Rejection sampling to avoid modulo bias
        // Calculate the largest multiple of range that fits in u64
        let max_accept = u64::MAX - (u64::MAX % range_u64) - 1;

        loop {
            let random = rng.gen_u64();
            if random <= max_accept {
                return self.low + ((random % range_u64) as i32);
            }
            // Reject and try again
        }
    }
}

impl Uniform<u32> {
    /// Creates a new uniform distribution over the given range.
    #[inline]
    pub fn new(range: RangeInclusive<u32>) -> Self {
        Uniform {
            low: *range.start(),
            high: *range.end(),
        }
    }

    /// Samples a value from this distribution using the given RNG.
    ///
    /// Uses rejection sampling to avoid modulo bias when the range
    /// doesn't evenly divide the random value's range.
    pub fn sample<B: BitGen>(&self, rng: &mut B) -> u32 {
        // Use u64 to handle the case where range would overflow u32
        let low = self.low as u64;
        let high = self.high as u64;
        let range = high - low + 1;
        if range == 0 {
            return self.low;
        }

        // Rejection sampling to avoid modulo bias
        // Calculate the largest multiple of range that fits in u64
        let max_accept = u64::MAX - (u64::MAX % range) - 1;

        loop {
            let random = rng.gen_u64();
            if random <= max_accept {
                return self.low.wrapping_add((random % range) as u32);
            }
            // Reject and try again
        }
    }
}

impl Uniform<usize> {
    /// Creates a new uniform distribution over the given range.
    #[inline]
    pub fn new(range: RangeInclusive<usize>) -> Self {
        Uniform {
            low: *range.start(),
            high: *range.end(),
        }
    }

    /// Samples a value from this distribution using the given RNG.
    ///
    /// Uses rejection sampling to avoid modulo bias when the range
    /// doesn't evenly divide the random value's range.
    pub fn sample<B: BitGen>(&self, rng: &mut B) -> usize {
        let range = self.high - self.low + 1;
        if range == 0 {
            return self.low;
        }

        // Use u64 for rejection sampling (works for both 32-bit and 64-bit usize)
        let range_u64 = range as u64;

        // Rejection sampling to avoid modulo bias
        // Calculate the largest multiple of range that fits in u64
        let max_accept = u64::MAX - (u64::MAX % range_u64) - 1;

        loop {
            let random = rng.gen_u64();
            if random <= max_accept {
                return self.low + ((random % range_u64) as usize);
            }
            // Reject and try again
        }
    }
}

/// Bernoulli distribution for boolean values.
///
/// Generates random booleans with a given probability of being true.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_random::distributions::Bernoulli;
///
/// // Example usage with a type that implements the BitGen trait:
/// // let mut rng = MyBitGen::new(42);
/// // let dist = Bernoulli::new(0.5); // 50% chance of true
/// // let value = dist.sample(&mut rng);
/// ```
#[derive(Clone, Copy, Debug)]
pub struct Bernoulli {
    probability: f64,
}

impl Bernoulli {
    /// Creates a new Bernoulli distribution with the given probability.
    ///
    /// # Panics
    ///
    /// Panics if `probability` is not in the range [0.0, 1.0].
    #[inline]
    pub fn new(probability: f64) -> Self {
        assert!(probability >= 0.0 && probability <= 1.0);
        Bernoulli { probability }
    }

    /// Samples a value from this distribution using the given RNG.
    #[inline]
    pub fn sample<B: BitGen>(&self, rng: &mut B) -> bool {
        rng.gen_f64() < self.probability
    }
}

/// Exponential distribution.
///
/// Generates random numbers following an exponential distribution
/// with the given rate parameter (lambda).
///
/// # Examples
///
/// ```rust
/// use abseil::absl_random::distributions::Exponential;
///
/// // let mut rng = MyBitGen::new(42);
/// // let dist = Exponential::new(1.0); // lambda = 1.0
/// // let value = dist.sample(&mut rng);
/// // assert!(value >= 0.0);
/// ```
#[derive(Clone, Copy, Debug)]
pub struct Exponential {
    lambda: f64,
}

impl Exponential {
    /// Creates a new exponential distribution with the given rate parameter.
    ///
    /// # Panics
    ///
    /// Panics if `lambda <= 0.0`.
    #[inline]
    pub fn new(lambda: f64) -> Self {
        assert!(lambda > 0.0);
        Exponential { lambda }
    }

    /// Samples a value from this distribution using the given RNG.
    #[inline]
    pub fn sample<B: BitGen>(&self, rng: &mut B) -> f64 {
        let u = rng.gen_f64();
        // Handle the case where u is 0.0 (avoid log(0))
        let u = if u == 0.0 { f64::MIN_POSITIVE } else { u };
        -u.ln() / self.lambda
    }
}

/// Normal (Gaussian) distribution.
///
/// Generates random numbers following a normal distribution
/// with the given mean and standard deviation.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_random::distributions::Normal;
///
/// // let mut rng = MyBitGen::new(42);
/// // let dist = Normal::new(0.0, 1.0); // mean=0, std=1
/// // let value = dist.sample(&mut rng);
/// ```
#[derive(Clone, Copy, Debug)]
pub struct Normal {
    mean: f64,
    std: f64,
}

impl Normal {
    /// Creates a new normal distribution with the given mean and standard deviation.
    ///
    /// # Panics
    ///
    /// Panics if `std <= 0.0` or `std` is NaN.
    #[inline]
    pub fn new(mean: f64, std: f64) -> Self {
        assert!(std > 0.0);
        Normal { mean, std }
    }

    /// Creates a standard normal distribution (mean=0, std=1).
    #[inline]
    pub fn standard() -> Self {
        Normal::new(0.0, 1.0)
    }

    /// Samples a value from this distribution using the given RNG.
    ///
    /// Uses the Box-Muller transform to generate normally distributed values.
    #[inline]
    pub fn sample<B: BitGen>(&self, rng: &mut B) -> f64 {
        // Box-Muller transform
        let u1 = rng.gen_f64();
        let u2 = rng.gen_f64();
        // Handle edge case where u1 is 0
        let u1 = if u1 == 0.0 { f64::MIN_POSITIVE } else { u1 };
        let z0 = (-2.0 * u1.ln()).sqrt() * (2.0 * core::f64::consts::PI * u2).cos();
        self.mean + self.std * z0
    }
}

/// Poisson distribution.
///
/// Generates random numbers following a Poisson distribution
/// with the given mean (lambda).
///
/// # Examples
///
/// ```rust
/// use abseil::absl_random::distributions::Poisson;
///
/// // let mut rng = MyBitGen::new(42);
/// // let dist = Poisson::new(5.0);
/// // let value = dist.sample(&mut rng);
/// // assert!(value >= 0);
/// ```
#[derive(Clone, Copy, Debug)]
pub struct Poisson {
    lambda: f64,
}

impl Poisson {
    /// Creates a new Poisson distribution with the given mean.
    ///
    /// # Panics
    ///
    /// Panics if `lambda <= 0.0`.
    #[inline]
    pub fn new(lambda: f64) -> Self {
        assert!(lambda > 0.0);
        Poisson { lambda }
    }

    /// Samples a value from this distribution using the given RNG.
    ///
    /// Uses Knuth's algorithm for small lambda.
    #[inline]
    pub fn sample<B: BitGen>(&self, rng: &mut B) -> u64 {
        // Knuth's algorithm
        let exp_lambda = (-self.lambda).exp();
        let mut result = 0u64;
        let mut p = 1.0;

        loop {
            p *= rng.gen_f64();
            if p <= exp_lambda {
                return result;
            }
            result += 1;
        }
    }
}

/// Geometric distribution.
///
/// Generates the number of trials until the first success
/// in a sequence of Bernoulli trials.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_random::distributions::Geometric;
///
/// // let mut rng = MyBitGen::new(42);
/// // let dist = Geometric::new(0.5); // 50% success probability
/// // let value = dist.sample(&mut rng);
/// // assert!(value >= 1);
/// ```
#[derive(Clone, Copy, Debug)]
pub struct Geometric {
    p: f64,
}

impl Geometric {
    /// Creates a new geometric distribution with the given success probability.
    ///
    /// # Panics
    ///
    /// Panics if `p <= 0.0` or `p > 1.0`.
    #[inline]
    pub fn new(p: f64) -> Self {
        assert!(p > 0.0 && p <= 1.0);
        Geometric { p }
    }

    /// Samples a value from this distribution using the given RNG.
    #[inline]
    pub fn sample<B: BitGen>(&self, rng: &mut B) -> u64 {
        // Inverse transform sampling
        let u = rng.gen_f64();
        // Handle edge case where u is 0 or 1
        let u = if u <= 0.0 {
            f64::MIN_POSITIVE
        } else if u >= 1.0 {
            1.0 - f64::MIN_POSITIVE
        } else {
            u
        };
        1 + (-u.ln() / (-self.p).ln()).floor() as u64
    }
}

/// Trait for types that can generate random bits.
///
/// This trait is implemented by random number generators.
pub trait BitGen {
    /// Generates a random u64 value.
    fn gen_u64(&mut self) -> u64;

    /// Generates a random u32 value.
    fn gen_u32(&mut self) -> u32 {
        self.gen_u64() as u32
    }

    /// Generates a random f64 value in the range [0.0, 1.0).
    fn gen_f64(&mut self) -> f64 {
        const UPPER_BITS: u64 = 0x3FF << 52;
        let bits = self.gen_u64();
        let exponent = UPPER_BITS | (bits >> 12);
        f64::from_bits(exponent) - 1.0
    }

    /// Generates a random f32 value in the range [0.0, 1.0).
    fn gen_f32(&mut self) -> f32 {
        self.gen_f64() as f32
    }

    /// Generates a random boolean with 50% probability.
    fn gen_bool(&mut self) -> bool {
        self.gen_u64() & 1 == 0
    }

    /// Fills a byte slice with random data.
    fn fill_bytes(&mut self, dest: &mut [u8]) {
        for chunk in dest.chunks_mut(8) {
            let rand = self.gen_u64();
            let bytes = rand.to_le_bytes();
            let len = chunk.len().min(8);
            chunk[..len].copy_from_slice(&bytes[..len]);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestRng {
        state: u64,
    }

    impl TestRng {
        fn new(seed: u64) -> Self {
            TestRng { state: seed }
        }
    }

    impl BitGen for TestRng {
        fn gen_u64(&mut self) -> u64 {
            self.state = self
                .state
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            self.state
        }
    }

    #[test]
    fn test_uniform_f64_range() {
        let mut rng = TestRng::new(42);
        let dist = Uniform::<f64>::new(1.0..=100.0);

        for _ in 0..100 {
            let value = dist.sample(&mut rng);
            assert!(value >= 1.0 && value <= 100.0);
        }
    }

    #[test]
    fn test_uniform_i64_range() {
        let mut rng = TestRng::new(42);
        let dist = Uniform::<i64>::new(10..=20);

        for _ in 0..100 {
            let value = dist.sample(&mut rng);
            assert!(value >= 10 && value <= 20);
        }
    }

    #[test]
    fn test_uniform_u64_range() {
        let mut rng = TestRng::new(42);
        let dist = Uniform::<u64>::new(10..=20);

        for _ in 0..100 {
            let value = dist.sample(&mut rng);
            assert!(value >= 10 && value <= 20);
        }
    }

    #[test]
    fn test_uniform_i32_range() {
        let mut rng = TestRng::new(42);
        let dist = Uniform::<i32>::new(10..=20);

        for _ in 0..100 {
            let value = dist.sample(&mut rng);
            assert!(value >= 10 && value <= 20);
        }
    }

    #[test]
    fn test_uniform_u32_range() {
        let mut rng = TestRng::new(42);
        let dist = Uniform::<u32>::new(10..=20);

        for _ in 0..100 {
            let value = dist.sample(&mut rng);
            assert!(value >= 10 && value <= 20);
        }
    }

    #[test]
    fn test_uniform_usize_range() {
        let mut rng = TestRng::new(42);
        let dist = Uniform::<usize>::new(10..=20);

        for _ in 0..100 {
            let value = dist.sample(&mut rng);
            assert!(value >= 10 && value <= 20);
        }
    }

    #[test]
    fn test_bernoulli_probability() {
        let mut rng = TestRng::new(42);
        let dist = Bernoulli::new(0.0);
        assert!(!dist.sample(&mut rng));
        assert!(!dist.sample(&mut rng));

        let dist = Bernoulli::new(1.0);
        assert!(dist.sample(&mut rng));
        assert!(dist.sample(&mut rng));
    }

    #[test]
    fn test_exponential_positive() {
        let mut rng = TestRng::new(42);
        let dist = Exponential::new(1.0);

        for _ in 0..100 {
            let value = dist.sample(&mut rng);
            assert!(value >= 0.0);
        }
    }

    #[test]
    fn test_exponential_mean() {
        let mut rng = TestRng::new(42);
        let dist = Exponential::new(2.0);

        // With lambda=2, mean should be 0.5
        let sum: f64 = (0..1000).map(|_| dist.sample(&mut rng)).sum();
        let mean = sum / 1000.0;
        // Should be close to 0.5 (within 20% for randomness)
        assert!(mean > 0.3 && mean < 0.7);
    }

    #[test]
    fn test_normal_distribution() {
        let mut rng = TestRng::new(42);
        let dist = Normal::new(0.0, 1.0);

        let mut within_1_sigma = 0;
        let mut within_2_sigma = 0;
        let mut within_3_sigma = 0;

        for _ in 0..1000 {
            let value = dist.sample(&mut rng);
            if value.abs() <= 1.0 {
                within_1_sigma += 1;
            }
            if value.abs() <= 2.0 {
                within_2_sigma += 1;
            }
            if value.abs() <= 3.0 {
                within_3_sigma += 1;
            }
        }

        // Rough check for normal distribution properties
        let total = 1000;
        assert!(within_1_sigma as f64 / total as f64 > 0.6); // ~68%
        assert!(within_2_sigma as f64 / total as f64 > 0.9); // ~95%
        assert!(within_3_sigma as f64 / total as f64 > 0.98); // ~99.7%
    }

    #[test]
    fn test_poisson_range() {
        let mut rng = TestRng::new(42);
        let dist = Poisson::new(5.0);

        for _ in 0..100 {
            let value = dist.sample(&mut rng);
            assert!(value >= 0);
        }
    }

    #[test]
    fn test_poisson_mean_approx() {
        let mut rng = TestRng::new(42);
        let dist = Poisson::new(10.0);

        let sum: u64 = (0..1000).map(|_| dist.sample(&mut rng)).sum();
        let mean = sum as f64 / 1000.0;
        // Should be close to 10 (within 30% for randomness)
        assert!(mean > 7.0 && mean < 13.0);
    }

    #[test]
    fn test_geometric_range() {
        let mut rng = TestRng::new(42);
        let dist = Geometric::new(0.5);

        for _ in 0..100 {
            let value = dist.sample(&mut rng);
            assert!(value >= 1);
        }
    }

    #[test]
    fn test_bernoulli_invalid_probability() {
        assert!(Bernoulli::new(0.0).probability >= 0.0);
        assert!(Bernoulli::new(1.0).probability <= 1.0);
    }

    #[test]
    #[should_panic]
    fn test_exponential_invalid_lambda() {
        Exponential::new(0.0);
    }

    #[test]
    #[should_panic]
    fn test_normal_invalid_std() {
        Normal::new(0.0, 0.0);
    }

    #[test]
    #[should_panic]
    fn test_poisson_invalid_lambda() {
        Poisson::new(0.0);
    }

    #[test]
    #[should_panic]
    fn test_geometric_invalid_p() {
        Geometric::new(0.0);
    }

    #[test]
    fn test_bit_gen_u64() {
        let mut rng = TestRng::new(42);
        let _ = rng.gen_u64();
        let _ = rng.gen_u32();
        let _ = rng.gen_f64();
        let _ = rng.gen_f32();
        let _ = rng.gen_bool();
    }

    #[test]
    fn test_fill_bytes() {
        let mut rng = TestRng::new(42);
        let mut buffer = [0u8; 16];
        rng.fill_bytes(&mut buffer);

        // With a fixed seed, should get deterministic output
        let mut rng2 = TestRng::new(42);
        let mut buffer2 = [0u8; 16];
        rng2.fill_bytes(&mut buffer2);

        assert_eq!(buffer, buffer2);
    }

    #[test]
    fn test_uniform_single_value() {
        let mut rng = TestRng::new(42);
        let dist = Uniform::<i64>::new(42..=42);
        assert_eq!(dist.sample(&mut rng), 42);
    }

    #[test]
    fn test_uniform_full_range() {
        let mut rng = TestRng::new(42);
        let dist = Uniform::<u32>::new(0..=u32::MAX);

        for _ in 0..10 {
            let _ = dist.sample(&mut rng);
        }
    }

    // Edge case tests for MEDIUM security fix - modulo bias in random distributions

    #[test]
    fn test_uniform_u64_no_bias_with_odd_range() {
        let mut rng = TestRng::new(42);
        let dist = Uniform::<u64>::new(0..=2); // range = 3 (doesn't divide u64::MAX)

        // Collect many samples
        let mut counts = [0u64; 3];
        for _ in 0..1000 {
            let val = dist.sample(&mut rng);
            assert!(val >= 0 && val <= 2);
            counts[val as usize] += 1;
        }

        // All values should be represented (statistical test - high probability of all values appearing)
        // For a proper unbiased distribution with 1000 samples over 3 values,
        // the expected count is ~333 per value with a std dev of ~18.8
        // Even with 3 std devs (99.7% confidence), we should have >277 samples per value
        for count in &counts {
            assert!(
                *count > 100,
                "Value not sufficiently represented: {}",
                count
            );
        }
    }

    #[test]
    fn test_uniform_i64_no_bias_with_odd_range() {
        let mut rng = TestRng::new(42);
        let dist = Uniform::<i64>::new(0..=4); // range = 5 (doesn't divide u64::MAX)

        let mut counts = [0u64; 5];
        for _ in 0..500 {
            let val = dist.sample(&mut rng);
            assert!(val >= 0 && val <= 4);
            counts[val as usize] += 1;
        }

        // All values should be represented
        for count in &counts {
            assert!(*count > 20, "Value not sufficiently represented: {}", count);
        }
    }

    #[test]
    fn test_uniform_u32_no_bias_with_prime_range() {
        let mut rng = TestRng::new(42);
        let dist = Uniform::<u32>::new(0..=6); // range = 7 (prime, doesn't divide u64::MAX)

        let mut counts = [0u64; 7];
        for _ in 0..700 {
            let val = dist.sample(&mut rng);
            assert!(val >= 0 && val <= 6);
            counts[val as usize] += 1;
        }

        // All values should be represented
        for count in &counts {
            assert!(*count > 30, "Value not sufficiently represented: {}", count);
        }
    }

    #[test]
    fn test_uniform_i32_no_bias_with_small_range() {
        let mut rng = TestRng::new(42);
        let dist = Uniform::<i32>::new(0..=2); // range = 3 (doesn't divide u64::MAX)

        let mut counts = [0u64; 3];
        for _ in 0..300 {
            let val = dist.sample(&mut rng);
            assert!(val >= 0 && val <= 2);
            counts[val as usize] += 1;
        }

        // All values should be represented
        for count in &counts {
            assert!(*count > 30, "Value not sufficiently represented: {}", count);
        }
    }

    #[test]
    fn test_uniform_usize_no_bias_with_odd_range() {
        let mut rng = TestRng::new(42);
        let dist = Uniform::<usize>::new(0..=2); // range = 3 (doesn't divide u64::MAX)

        let mut counts = [0usize; 3];
        for _ in 0..300 {
            let val = dist.sample(&mut rng);
            assert!(val >= 0 && val <= 2);
            counts[val] += 1;
        }

        // All values should be represented
        for count in &counts {
            assert!(*count > 30, "Value not sufficiently represented: {}", count);
        }
    }

    #[test]
    fn test_uniform_rejection_sampling_terminates() {
        // Test that rejection sampling always terminates
        let mut rng = TestRng::new(42);

        // Worst case: range = 2^63 + 1 (half of u64::MAX + 1)
        // This should still terminate with high probability
        let dist = Uniform::<u64>::new(0..=((u64::MAX / 2) + 1));

        for _ in 0..100 {
            let val = dist.sample(&mut rng);
            assert!(val >= 0 && val <= (u64::MAX / 2) + 1);
        }
    }

    #[test]
    fn test_uniform_edge_cases() {
        let mut rng = TestRng::new(42);

        // Test range = 1 (single value)
        let dist1 = Uniform::<u64>::new(5..=5);
        assert_eq!(dist1.sample(&mut rng), 5);

        // Test range = 2
        let dist2 = Uniform::<u64>::new(0..=1);
        for _ in 0..10 {
            let val = dist2.sample(&mut rng);
            assert!(val == 0 || val == 1);
        }

        // Test range near maximum
        let dist3 = Uniform::<u64>::new(u64::MAX - 1..=u64::MAX);
        for _ in 0..10 {
            let val = dist3.sample(&mut rng);
            assert!(val == u64::MAX - 1 || val == u64::MAX);
        }
    }

    #[test]
    fn test_uniform_signed_range_works() {
        let mut rng = TestRng::new(42);
        let dist = Uniform::<i64>::new(-10..=10);

        for _ in 0..100 {
            let val = dist.sample(&mut rng);
            assert!(val >= -10 && val <= 10);
        }
    }

    #[test]
    fn test_uniform_deterministic_with_seed() {
        let mut rng1 = TestRng::new(12345);
        let mut rng2 = TestRng::new(12345);

        let dist = Uniform::<u64>::new(0..=100);

        for _ in 0..10 {
            assert_eq!(dist.sample(&mut rng1), dist.sample(&mut rng2));
        }
    }
}
