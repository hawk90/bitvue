//! Comparison utilities.

use core::ops::RangeInclusive;

/// Clamps a value between a minimum and maximum.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_utility::comparison::clamp;
///
/// assert_eq!(clamp(5, 0, 10), 5);
/// assert_eq!(clamp(-5, 0, 10), 0);
/// assert_eq!(clamp(15, 0, 10), 10);
/// ```
#[inline]
pub const fn clamp<T: Copy + PartialOrd>(value: T, min: T, max: T) -> T {
    if value < min {
        min
    } else if value > max {
        max
    } else {
        value
    }
}

/// Returns the minimum of two values.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_utility::comparison::min;
///
/// assert_eq!(min(5, 10), 5);
/// assert_eq!(min(10, 5), 5);
/// ```
#[inline]
pub const fn min<T: Copy + PartialOrd>(a: T, b: T) -> T {
    if a < b { a } else { b }
}

/// Returns the maximum of two values.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_utility::comparison::max;
///
/// assert_eq!(max(5, 10), 10);
/// assert_eq!(max(10, 5), 10);
/// ```
#[inline]
pub const fn max<T: Copy + PartialOrd>(a: T, b: T) -> T {
    if a > b { a } else { b }
}

/// Returns the minimum of two values using `Ord` trait.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_utility::comparison::cmp_min;
///
/// assert_eq!(cmp_min(&5, &10), &5);
/// ```
#[inline]
pub fn cmp_min<'a, T: Ord>(a: &'a T, b: &'a T) -> &'a T {
    if a <= b { a } else { b }
}

/// Returns the maximum of two values using `Ord` trait.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_utility::comparison::cmp_max;
///
/// assert_eq!(cmp_max(&5, &10), &10);
/// ```
#[inline]
pub fn cmp_max<'a, T: Ord>(a: &'a T, b: &'a T) -> &'a T {
    if a >= b { a } else { b }
}

/// Returns the median of three values.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_utility::comparison::median;
///
/// assert_eq!(median(1, 5, 3), 3);
/// assert_eq!(median(5, 1, 3), 3);
/// ```
#[inline]
pub const fn median<T: Copy + PartialOrd>(a: T, b: T, c: T) -> T {
    if a < b {
        if b < c {
            b
        } else if a < c {
            c
        } else {
            a
        }
    } else {
        // a >= b
        if a < c {
            a
        } else if b < c {
            c
        } else {
            b
        }
    }
}

/// Checks if a value is between two bounds (exclusive).
///
/// # Examples
///
/// ```rust
/// use abseil::absl_utility::comparison::is_between;
///
/// assert!(is_between(5, 0, 10));
/// assert!(!is_between(0, 0, 10));
/// assert!(!is_between(10, 0, 10));
/// ```
#[inline]
pub const fn is_between<T: PartialOrd>(value: T, min: T, max: T) -> bool {
    value > min && value < max
}

/// Checks if a value is in the given inclusive range.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_utility::comparison::is_in_range;
///
/// assert!(is_in_range(5, 0..=10));
/// assert!(is_in_range(0, 0..=10));
/// assert!(is_in_range(10, 0..=10));
/// assert!(!is_in_range(11, 0..=10));
/// ```
#[inline]
pub fn is_in_range<T: PartialOrd>(value: T, range: RangeInclusive<T>) -> bool {
    range.contains(&value)
}
