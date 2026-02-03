//! Comparison utilities.

use core::cmp::PartialOrd;

/// Clamps a value between a minimum and maximum.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_numeric::comparison::clamp;
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
/// use abseil::absl_numeric::comparison::min;
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
/// use abseil::absl_numeric::comparison::max;
///
/// assert_eq!(max(5, 10), 10);
/// assert_eq!(max(10, 5), 10);
/// ```
#[inline]
pub const fn max<T: Copy + PartialOrd>(a: T, b: T) -> T {
    if a > b { a } else { b }
}

/// Returns the middle value of three numbers.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_numeric::comparison::median;
///
/// assert_eq!(median(1, 5, 3), 3);
/// assert_eq!(median(5, 1, 3), 3);
/// assert_eq!(median(3, 5, 1), 3);
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
    } else if a < c {
        a
    } else if b < c {
        c
    } else {
        b
    }
}
