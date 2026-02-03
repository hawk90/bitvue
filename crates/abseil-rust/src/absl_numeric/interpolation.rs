//! Interpolation utilities.

/// Linear interpolation between two values.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_numeric::interpolation::lerp;
///
/// assert_eq!(lerp(0.0, 10.0, 0.5), 5.0);
/// assert_eq!(lerp(0.0, 10.0, 0.25), 2.5);
/// assert_eq!(lerp(0.0, 10.0, 0.75), 7.5);
/// ```
#[inline]
pub fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

/// Linear interpolation between two integer values.
///
/// Returns an integer, rounding towards zero.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_numeric::interpolation::lerp_int;
///
/// assert_eq!(lerp_int(0, 10, 5, 100), 50);
/// assert_eq!(lerp_int(0, 100, 25, 100), 25);
/// ```
#[inline]
pub fn lerp_int(a: i32, b: i32, t: i32, scale: i32) -> i32 {
    a + (b - a) * t / scale
}

/// Smooth interpolation (smoothstep).
///
/// Uses Hermite interpolation for smooth transitions.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_numeric::interpolation::smoothstep;
///
/// assert_eq!(smoothstep(0.0), 0.0);
/// assert_eq!(smoothstep(0.5), 0.5);
/// assert_eq!(smoothstep(1.0), 1.0);
/// assert!(smoothstep(0.25) < smoothstep(0.5));
/// ```
#[inline]
pub fn smoothstep(t: f32) -> f32 {
    t * t * (3.0 - 2.0 * t)
}

/// Smoother interpolation (smootherstep).
///
/// Uses quintic Hermite interpolation for smoother transitions.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_numeric::interpolation::smootherstep;
///
/// assert_eq!(smootherstep(0.0), 0.0);
/// assert_eq!(smootherstep(0.5), 0.5);
/// assert_eq!(smootherstep(1.0), 1.0);
/// ```
#[inline]
pub fn smootherstep(t: f32) -> f32 {
    t * t * t * (t * (t * 6.0 - 15.0) + 10.0)
}

/// Inverse lerp - finds t where lerp(a, b, t) = value.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_numeric::interpolation::inverse_lerp;
///
/// assert_eq!(inverse_lerp(0.0, 10.0, 5.0), 0.5);
/// assert_eq!(inverse_lerp(0.0, 10.0, 2.5), 0.25);
/// assert_eq!(inverse_lerp(0.0, 10.0, 10.0), 1.0);
/// ```
#[inline]
pub fn inverse_lerp(a: f32, b: f32, value: f32) -> f32 {
    if a == b {
        0.0
    } else {
        (value - a) / (b - a)
    }
}

/// Remaps a value from one range to another.
///
/// # Panics
///
/// Panics if `from_min == from_max` (division by zero).
///
/// # Examples
///
/// ```rust
/// use abseil::absl_numeric::interpolation::remap;
///
/// // Map [0, 10] to [0, 100]
/// assert_eq!(remap(5.0, 0.0, 10.0, 0.0, 100.0), 50.0);
///
/// // Map [0, 1] to [-1, 1]
/// assert_eq!(remap(0.5, 0.0, 1.0, -1.0, 1.0), 0.0);
/// ```
#[inline]
pub fn remap(value: f32, from_min: f32, from_max: f32, to_min: f32, to_max: f32) -> f32 {
    let from_range = from_max - from_min;
    if from_range == 0.0 {
        // When source range is zero, return the midpoint of target range
        // This is a reasonable default that avoids NaN/Inf
        return (to_min + to_max) / 2.0;
    }
    to_min + (value - from_min) * (to_max - to_min) / from_range
}

/// Clamps and remaps a value.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_numeric::interpolation::remap_clamp;
///
/// assert_eq!(remap_clamp(15.0, 0.0, 10.0, 0.0, 100.0), 100.0);
/// assert_eq!(remap_clamp(-5.0, 0.0, 10.0, 0.0, 100.0), 0.0);
/// ```
#[inline]
pub fn remap_clamp(
    value: f32,
    from_min: f32,
    from_max: f32,
    to_min: f32,
    to_max: f32,
) -> f32 {
    let clamped = crate::absl_numeric::comparison::clamp(value, from_min, from_max);
    remap(clamped, from_min, from_max, to_min, to_max)
}
