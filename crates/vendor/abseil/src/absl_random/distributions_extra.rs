//! Extra probability distributions - Normal, Exponential, Poisson, Gamma, Beta, ChiSquared, Cauchy, Laplace, LogNormal, Weibull.

use crate::absl_random::bit_gen::BitGen;

/// Normal (Gaussian) distribution.
#[derive(Clone, Debug)]
pub struct Normal {
    mean: f64,
    std_dev: f64,
    has_spare: bool,
    spare: f64,
}

impl Normal {
    /// Creates a new normal distribution with the given mean and standard deviation.
    pub const fn new(mean: f64, std_dev: f64) -> Self {
        Self {
            mean,
            std_dev,
            has_spare: false,
            spare: 0.0,
        }
    }

    /// Creates a standard normal distribution (mean=0, std_dev=1).
    pub const fn standard() -> Self {
        Self::new(0.0, 1.0)
    }

    /// Samples from the distribution using the Box-Muller transform.
    pub fn sample(&mut self, rng: &mut BitGen) -> f64 {
        if self.has_spare {
            self.has_spare = false;
            return self.mean + self.std_dev * self.spare;
        }

        let u1 = rng.gen_u64() as f64 / (u64::MAX as f64 + 1.0);
        let u2 = rng.gen_u64() as f64 / (u64::MAX as f64 + 1.0);

        // Avoid log(0)
        let u1 = if u1 == 0.0 { 1e-300 } else { u1 };

        let mag = (-2.0 * u1.ln()).sqrt();
        let z0 = mag * (core::f64::consts::PI * 2.0 * u2).cos();
        let z1 = mag * (core::f64::consts::PI * 2.0 * u2).sin();

        self.spare = z1;
        self.has_spare = true;

        self.mean + self.std_dev * z0
    }
}

/// Exponential distribution.
#[derive(Clone, Debug)]
pub struct Exponential {
    lambda: f64,
}

impl Exponential {
    /// Creates a new exponential distribution with rate parameter lambda.
    pub const fn new(lambda: f64) -> Self {
        Self { lambda }
    }

    /// Samples from the distribution.
    pub fn sample(&self, rng: &mut BitGen) -> f64 {
        let u = rng.gen_u64() as f64 / (u64::MAX as f64 + 1.0);
        // Avoid log(0)
        let u = if u == 0.0 { 1e-300 } else { u };
        -u.ln() / self.lambda
    }
}

/// Poisson distribution.
#[derive(Clone, Debug)]
pub struct Poisson {
    lambda: f64,
}

impl Poisson {
    /// Creates a new Poisson distribution with parameter lambda.
    pub const fn new(lambda: f64) -> Self {
        Self { lambda }
    }

    /// Samples from the distribution using Knuth's algorithm.
    pub fn sample(&self, rng: &mut BitGen) -> u64 {
        if self.lambda < 30.0 {
            // Knuth's algorithm for small lambda
            let mut k = 0u64;
            let mut p = 1.0;
            let l = (-self.lambda).exp();

            loop {
                k += 1;
                p *= rng.gen_u64() as f64 / (u64::MAX as f64 + 1.0);
                if p <= l {
                    return k - 1;
                }
            }
        } else {
            // Rejection sampling for large lambda
            let mut k = 0i64;
            let c = 0.767 - 3.36 / self.lambda;
            let beta = core::f64::consts::PI / (3.0 * self.lambda).sqrt();
            let alpha = beta * self.lambda;
            let k_log_lambda = (self.lambda).ln();

            loop {
                let u = rng.gen_u64() as f64 / (u64::MAX as f64 + 1.0);
                let x = (alpha - (1.0 - u) / (beta * u).ln()).max(0.0);
                let n = (x + 0.5).floor() as i64;

                let v = rng.gen_u64() as f64 / (u64::MAX as f64 + 1.0);
                let y = (alpha - beta * x).exp();

                let lhs = (beta * (x - n)).exp() * (1.0 + v * y * y);
                let rhs = (n as f64 * k_log_lambda - (self.lambda).lgamma().0)
                    / (n as f64 * (self.lambda).ln() - (self.lambda).lgamma().0).exp();

                if lhs <= rhs {
                    return n as u64;
                }
                k = n;
            }
        }
    }
}

/// Gamma distribution.
#[derive(Clone, Debug)]
pub struct Gamma {
    shape: f64,
    scale: f64,
}

impl Gamma {
    /// Creates a new gamma distribution with shape and scale parameters.
    pub const fn new(shape: f64, scale: f64) -> Self {
        Self { shape, scale }
    }

    /// Samples from the distribution.
    pub fn sample(&self, rng: &mut BitGen) -> f64 {
        if self.shape < 1.0 {
            // Marsaglia and Tsang's method for shape < 1
            let u = rng.gen_u64() as f64 / (u64::MAX as f64 + 1.0);
            return self.sample_with_shape(self.shape + 1.0, rng) * u.powf(1.0 / self.shape);
        }
        self.sample_with_shape(self.shape, rng)
    }

    fn sample_with_shape(&self, shape: f64, rng: &mut BitGen) -> f64 {
        let d = shape - 1.0 / 3.0;
        let c = (1.0 / 3.0) / d.sqrt();

        loop {
            let mut x;
            let mut v;
            loop {
                x = Normal::standard().sample(rng);
                v = (1.0 + c * x).powi(3);
                if v > 0.0 {
                    break;
                }
            }

            let u = rng.gen_u64() as f64 / (u64::MAX as f64 + 1.0);
            if u < 1.0 - 0.0331 * (x * x).powi(2) {
                return d * v * self.scale;
            }

            if (u.ln()) < 0.5 * x * x + d * (1.0 - v + v.ln()) {
                return d * v * self.scale;
            }
        }
    }
}

/// Beta distribution.
#[derive(Clone, Debug)]
pub struct Beta {
    alpha: f64,
    beta: f64,
}

impl Beta {
    /// Creates a new beta distribution with alpha and beta parameters.
    pub const fn new(alpha: f64, beta: f64) -> Self {
        Self { alpha, beta }
    }

    /// Samples from the distribution.
    pub fn sample(&self, rng: &mut BitGen) -> f64 {
        let gamma1 = Gamma::new(self.alpha, 1.0).sample(rng);
        let gamma2 = Gamma::new(self.beta, 1.0).sample(rng);
        gamma1 / (gamma1 + gamma2)
    }
}

/// Chi-squared distribution.
#[derive(Clone, Debug)]
pub struct ChiSquared {
    degrees: f64,
}

impl ChiSquared {
    /// Creates a new chi-squared distribution with given degrees of freedom.
    pub const fn new(degrees: f64) -> Self {
        Self { degrees }
    }

    /// Samples from the distribution.
    pub fn sample(&self, rng: &mut BitGen) -> f64 {
        Gamma::new(self.degrees / 2.0, 2.0).sample(rng)
    }
}

/// Cauchy distribution.
#[derive(Clone, Debug)]
pub struct Cauchy {
    location: f64,
    scale: f64,
}

impl Cauchy {
    /// Creates a new Cauchy distribution with location and scale.
    pub const fn new(location: f64, scale: f64) -> Self {
        Self { location, scale }
    }

    /// Creates a standard Cauchy distribution.
    pub const fn standard() -> Self {
        Self::new(0.0, 1.0)
    }

    /// Samples from the distribution.
    pub fn sample(&self, rng: &mut BitGen) -> f64 {
        let u = rng.gen_u64() as f64 / (u64::MAX as f64 + 1.0);
        self.location + self.scale * (core::f64::consts::PI * (u - 0.5)).tan()
    }
}

/// Laplace distribution.
#[derive(Clone, Debug)]
pub struct Laplace {
    location: f64,
    scale: f64,
}

impl Laplace {
    /// Creates a new Laplace distribution with location and scale.
    pub const fn new(location: f64, scale: f64) -> Self {
        Self { location, scale }
    }

    /// Samples from the distribution.
    pub fn sample(&self, rng: &mut BitGen) -> f64 {
        let u = rng.gen_u64() as f64 / (u64::MAX as f64 + 1.0) - 0.5;
        self.location - self.scale * u.signum() * (1.0 - 2.0 * u.abs()).ln()
    }
}

/// Log-normal distribution.
#[derive(Clone, Debug)]
pub struct LogNormal {
    mean: f64,
    std_dev: f64,
}

impl LogNormal {
    /// Creates a new log-normal distribution with mean and std_dev of the underlying normal.
    pub const fn new(mean: f64, std_dev: f64) -> Self {
        Self { mean, std_dev }
    }

    /// Samples from the distribution.
    pub fn sample(&self, rng: &mut BitGen) -> f64 {
        Normal::new(self.mean, self.std_dev).sample(rng).exp()
    }
}

/// Weibull distribution.
#[derive(Clone, Debug)]
pub struct Weibull {
    shape: f64,
    scale: f64,
}

impl Weibull {
    /// Creates a new Weibull distribution with shape and scale parameters.
    pub const fn new(shape: f64, scale: f64) -> Self {
        Self { shape, scale }
    }

    /// Samples from the distribution.
    pub fn sample(&self, rng: &mut BitGen) -> f64 {
        let u = rng.gen_u64() as f64 / (u64::MAX as f64 + 1.0);
        // Avoid log(0)
        let u = if u == 0.0 { 1e-300 } else { u };
        self.scale * (-u.ln()).powf(1.0 / self.shape)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normal_distribution() {
        let mut dist = Normal::new(0.0, 1.0);
        let mut rng = BitGen::new(42);

        for _ in 0..100 {
            let val = dist.sample(&mut rng);
            // Should be within reasonable range (6 sigma)
            assert!(val > -6.0 && val < 6.0);
        }
    }

    #[test]
    fn test_exponential_distribution() {
        let dist = Exponential::new(1.0);
        let mut rng = BitGen::new(42);

        for _ in 0..100 {
            let val = dist.sample(&mut rng);
            assert!(val >= 0.0);
        }
    }

    #[test]
    fn test_poisson_distribution() {
        let dist = Poisson::new(5.0);
        let mut rng = BitGen::new(42);

        for _ in 0..100 {
            let val = dist.sample(&mut rng);
            assert!(val < 100); // Should be reasonable
        }
    }

    #[test]
    fn test_gamma_distribution() {
        let dist = Gamma::new(2.0, 1.0);
        let mut rng = BitGen::new(42);

        for _ in 0..100 {
            let val = dist.sample(&mut rng);
            assert!(val >= 0.0);
        }
    }

    #[test]
    fn test_beta_distribution() {
        let dist = Beta::new(2.0, 5.0);
        let mut rng = BitGen::new(42);

        for _ in 0..100 {
            let val = dist.sample(&mut rng);
            assert!(val >= 0.0 && val <= 1.0);
        }
    }

    #[test]
    fn test_chisquared_distribution() {
        let dist = ChiSquared::new(5.0);
        let mut rng = BitGen::new(42);

        for _ in 0..100 {
            let val = dist.sample(&mut rng);
            assert!(val >= 0.0);
        }
    }

    #[test]
    fn test_cauchy_distribution() {
        let dist = Cauchy::new(0.0, 1.0);
        let mut rng = BitGen::new(42);

        // Cauchy has heavy tails, so values can be large
        let mut has_small = false;
        let mut has_large = false;

        for _ in 0..1000 {
            let val = dist.sample(&mut rng);
            if val.abs() < 1.0 {
                has_small = true;
            }
            if val.abs() > 10.0 {
                has_large = true;
            }
        }

        assert!(has_small || has_large);
    }

    #[test]
    fn test_laplace_distribution() {
        let dist = Laplace::new(0.0, 1.0);
        let mut rng = BitGen::new(42);

        for _ in 0..100 {
            let val = dist.sample(&mut rng);
            // Most values should be within reasonable range
            assert!(val > -20.0 && val < 20.0);
        }
    }

    #[test]
    fn test_lognormal_distribution() {
        let dist = LogNormal::new(0.0, 1.0);
        let mut rng = BitGen::new(42);

        for _ in 0..100 {
            let val = dist.sample(&mut rng);
            assert!(val > 0.0);
        }
    }

    #[test]
    fn test_weibull_distribution() {
        let dist = Weibull::new(2.0, 1.0);
        let mut rng = BitGen::new(42);

        for _ in 0..100 {
            let val = dist.sample(&mut rng);
            assert!(val >= 0.0);
        }
    }
}
