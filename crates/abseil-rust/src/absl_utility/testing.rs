//! Testing utilities.

/// A utility for approximating equality in floating-point tests.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_utility::testing::approx_eq;
///
/// assert!(approx_eq(1.0, 1.0 + 1e-10, 1e-9));
/// ```
#[inline]
pub fn approx_eq(a: f64, b: f64, epsilon: f64) -> bool {
    (a - b).abs() < epsilon
}

/// A utility for approximating equality with relative tolerance.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_utility::testing::approx_eq_rel;
///
/// assert!(approx_eq_rel(100.0, 100.1, 0.001));
/// ```
#[inline]
pub fn approx_eq_rel(a: f64, b: f64, rel_tol: f64) -> bool {
    let diff = (a - b).abs();
    let max_val = a.abs().max(b.abs());
    diff <= max_val * rel_tol || diff <= f64::EPSILON
}

/// A utility for checking if a value is NaN (Not a Number).
///
/// # Examples
///
/// ```rust
/// use abseil::absl_utility::testing::is_nan;
///
/// assert!(is_nan(f64::NAN));
/// assert!(!is_nan(1.0));
/// ```
#[inline]
pub fn is_nan(value: f64) -> bool {
    value.is_nan()
}

/// A utility for checking if a value is finite (not NaN or infinite).
///
/// # Examples
///
/// ```rust
/// use abseil::absl_utility::testing::is_finite;
///
/// assert!(is_finite(1.0));
/// assert!(!is_finite(f64::NAN));
/// assert!(!is_finite(f64::INFINITY));
/// ```
#[inline]
pub fn is_finite(value: f64) -> bool {
    value.is_finite()
}

/// A utility for checking if a value is infinite.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_utility::testing::is_infinite;
///
/// assert!(is_infinite(f64::INFINITY));
/// assert!(!is_infinite(1.0));
/// ```
#[inline]
pub fn is_infinite(value: f64) -> bool {
    value.is_infinite()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_approx_eq() {
        assert!(approx_eq(1.0, 1.0, 0.0));
        assert!(approx_eq(1.0, 1.0 + 1e-10, 1e-9));
        assert!(!approx_eq(1.0, 2.0, 0.5));
    }

    #[test]
    fn test_approx_eq_rel() {
        assert!(approx_eq_rel(100.0, 100.0, 0.001));
        assert!(approx_eq_rel(100.0, 100.1, 0.001));
        assert!(!approx_eq_rel(100.0, 110.0, 0.001));
    }

    #[test]
    fn test_is_nan() {
        assert!(is_nan(f64::NAN));
        assert!(!is_nan(1.0));
        assert!(!is_nan(f64::INFINITY));
    }

    #[test]
    fn test_is_finite() {
        assert!(is_finite(1.0));
        assert!(!is_finite(f64::NAN));
        assert!(!is_finite(f64::INFINITY));
        assert!(!is_finite(f64::NEG_INFINITY));
    }

    #[test]
    fn test_is_infinite() {
        assert!(is_infinite(f64::INFINITY));
        assert!(is_infinite(f64::NEG_INFINITY));
        assert!(!is_infinite(1.0));
        assert!(!is_infinite(f64::NAN));
    }
}
