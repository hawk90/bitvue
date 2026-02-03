//! Casting utilities.

use core::fmt::Display;

/// Safely casts between integer types.
///
/// Returns None if the value doesn't fit in the target type.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_numeric::casting::safe_cast;
///
/// assert_eq!(safe_cast::<i32, i64>(5i64), Some(5i32));
/// assert_eq!(safe_cast::<i32, i64>(i64::MAX), None);
/// assert_eq!(safe_cast::<u32, i32>(-1i32), None);
/// ```
#[inline]
pub fn safe_cast<Src: Copy, Dst: Copy + TryFrom<Src>>(value: Src) -> Option<Dst> {
    Dst::try_from(value).ok()
}

/// Casts a value, clamping to the target type's range.
///
/// Note: This is a simplified implementation that falls back to default
/// when the value doesn't fit. For proper clamping with min/max values,
/// use the saturating_cast macro or implement type-specific logic.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_numeric::casting::cast_clamp;
///
/// // When value fits, it converts directly
/// assert_eq!(cast_clamp::<i32, i64>(50), 50);
///
/// // When value overflows, returns default
/// assert_eq!(cast_clamp::<i64, i32>(i64::MAX), i32::default());
/// ```
#[inline]
pub fn cast_clamp<Src: Copy, Dst: Copy + TryFrom<Src> + Default>(value: Src) -> Dst {
    Dst::try_from(value).unwrap_or_else(|_| Dst::default())
}

/// Checks if a value is within a range (inclusive).
///
/// # Examples
///
/// ```rust
/// use abseil::absl_numeric::casting::in_range;
///
/// assert!(in_range(5, 0, 10));
/// assert!(!in_range(15, 0, 10));
/// assert!(in_range(0, 0, 10));
/// assert!(in_range(10, 0, 10));
/// ```
#[inline]
pub const fn in_range<T: Copy + PartialOrd>(value: T, min: T, max: T) -> bool {
    value >= min && value <= max
}
