//! Rounding utilities.

/// Rounds a floating-point value to the nearest integer.
///
/// Ties round to the nearest even number (banker's rounding).
///
/// # Examples
///
/// ```rust
/// use abseil::absl_numeric::rounding::round_half_to_even;
///
/// assert_eq!(round_half_to_even(2.5_f32), 2.0);
/// assert_eq!(round_half_to_even(3.5_f32), 4.0);
/// assert_eq!(round_half_to_even(2.4_f32), 2.0);
/// assert_eq!(round_half_to_even(2.6_f32), 3.0);
/// ```
#[inline]
pub fn round_half_to_even(value: f32) -> f32 {
    value.round()
}

/// Rounds a floating-point value up to the nearest integer.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_numeric::rounding::round_up;
///
/// assert_eq!(round_up(2.1_f32), 3.0);
/// assert_eq!(round_up(2.0_f32), 2.0);
/// assert_eq!(round_up(-2.1_f32), -2.0);
/// ```
#[inline]
pub fn round_up(value: f32) -> f32 {
    value.ceil()
}

/// Rounds a floating-point value down to the nearest integer.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_numeric::rounding::round_down;
///
/// assert_eq!(round_down(2.9_f32), 2.0);
/// assert_eq!(round_down(2.0_f32), 2.0);
/// assert_eq!(round_down(-2.9_f32), -3.0);
/// ```
#[inline]
pub fn round_down(value: f32) -> f32 {
    value.floor()
}

/// Rounds a floating-point value towards zero.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_numeric::rounding::round_towards_zero;
///
/// assert_eq!(round_towards_zero(2.9_f32), 2.0);
/// assert_eq!(round_towards_zero(-2.9_f32), -2.0);
/// ```
#[inline]
pub fn round_towards_zero(value: f32) -> f32 {
    if value >= 0.0 {
        value.floor()
    } else {
        value.ceil()
    }
}

/// Rounds a floating-point value away from zero.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_numeric::rounding::round_away_from_zero;
///
/// assert_eq!(round_away_from_zero(2.1_f32), 3.0);
/// assert_eq!(round_away_from_zero(-2.1_f32), -3.0);
/// ```
#[inline]
pub fn round_away_from_zero(value: f32) -> f32 {
    if value >= 0.0 {
        value.ceil()
    } else {
        value.floor()
    }
}

/// Rounds a floating-point value to a specified number of decimal places.
///
/// # Panics
///
/// Panics if places > 23 (would cause overflow in powi computation).
///
/// # Examples
///
/// ```rust
/// use abseil::absl_numeric::rounding::round_to_places;
///
/// assert_eq!(round_to_places(2.456_f32, 2), 2.46);
/// assert_eq!(round_to_places(2.454_f32, 2), 2.45);
/// ```
#[inline]
pub fn round_to_places(value: f32, places: u32) -> f32 {
    // Limit places to prevent overflow. f32 can only reliably represent
    // about 7 decimal digits, and 10^23 is near the limit before powi overflows.
    // powi(24) would produce infinity, and casting large u32 to i32 could overflow.
    const MAX_PLACES: u32 = 23;
    if places > MAX_PLACES {
        panic!("round_to_places: places ({}) must be <= {} to avoid overflow", places, MAX_PLACES);
    }
    let multiplier = 10.0_f32.powi(places as i32);
    (value * multiplier).round() / multiplier
}
