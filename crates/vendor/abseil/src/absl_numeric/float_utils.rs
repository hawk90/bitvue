//! Floating-point utilities.

/// Checks if a floating-point value is NaN (Not a Number).
///
/// # Examples
///
/// ```rust
/// use abseil::absl_numeric::float_utils::is_nan;
///
/// assert!(is_nan(f32::NAN));
/// assert!(!is_nan(0.0_f32));
/// assert!(!is_nan(f32::INFINITY));
/// ```
#[inline]
pub const fn is_nan(value: f32) -> bool {
    value != value
}

/// Checks if a floating-point value is finite (not NaN or infinity).
///
/// # Examples
///
/// ```rust
/// use abseil::absl_numeric::float_utils::is_finite;
///
/// assert!(is_finite(0.0_f32));
/// assert!(is_finite(1.0_f32));
/// assert!(!is_finite(f32::NAN));
/// assert!(!is_finite(f32::INFINITY));
/// ```
#[inline]
pub const fn is_finite(value: f32) -> bool {
    value == value && value.abs() < f32::MAX
}

/// Checks if a floating-point value is infinite.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_numeric::float_utils::is_infinite;
///
/// assert!(is_infinite(f32::INFINITY));
/// assert!(is_infinite(f32::NEG_INFINITY));
/// assert!(!is_infinite(0.0_f32));
/// assert!(!is_infinite(f32::NAN));
/// ```
#[inline]
pub const fn is_infinite(value: f32) -> bool {
    value == f32::INFINITY || value == f32::NEG_INFINITY
}

/// Returns the sign of a value.
///
/// Returns -1.0 for negative, 0.0 for zero, 1.0 for positive.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_numeric::float_utils::sign;
///
/// assert_eq!(sign(-5.0_f32), -1.0);
/// assert_eq!(sign(0.0_f32), 0.0);
/// assert_eq!(sign(5.0_f32), 1.0);
/// ```
#[inline]
pub fn sign(value: f32) -> f32 {
    if value > 0.0 {
        1.0
    } else if value < 0.0 {
        -1.0
    } else {
        0.0
    }
}

/// Copies the sign from one value to another.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_numeric::float_utils::copy_sign;
///
/// assert_eq!(copy_sign(5.0_f32, -1.0_f32), -5.0);
/// assert_eq!(copy_sign(-5.0_f32, 1.0_f32), 5.0);
/// ```
#[inline]
pub fn copy_sign(magnitude: f32, sign: f32) -> f32 {
    if (magnitude > 0.0) == (sign > 0.0) {
        magnitude
    } else {
        -magnitude
    }
}

/// Computes the next representable floating-point value.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_numeric::float_utils::next_after;
///
/// assert!(next_after(1.0_f32, 2.0) > 1.0);
/// assert!(next_after(1.0_f32, 0.0) < 1.0);
/// ```
#[inline]
pub fn next_after(value: f32, direction: f32) -> f32 {
    if value == direction {
        value
    } else if value.is_nan() || direction.is_nan() {
        f32::NAN
    } else if value == 0.0 {
        if direction > 0.0 {
            f32::MIN_POSITIVE
        } else {
            -f32::MIN_POSITIVE
        }
    } else if (value > 0.0) == (direction > value) {
        value.abs().next_after(f32::MAX) * value.signum()
    } else {
        value.abs().next_after(0.0) * value.signum()
    }
}
