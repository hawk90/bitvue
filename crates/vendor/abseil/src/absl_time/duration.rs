//! Duration utilities.
//!
//! Provides a Duration type for representing time spans with nanosecond precision.

use core::fmt;
use core::ops::{Add, AddAssign, Sub, SubAssign};
use core::time::Duration as StdDuration;

/// A duration representing a span of time.
///
/// Similar to `std::time::Duration` but with additional utility methods
/// and more ergonomic API for time calculations.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_time::duration::Duration;
///
/// let d = Duration::from_seconds(5);
/// assert_eq!(d.seconds(), 5);
/// assert_eq!(d.millis(), 5000);
/// ```
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Duration {
    // Duration in nanoseconds
    nanos: i64,
}

impl Duration {
    /// The maximum representable duration.
    pub const MAX: Duration = Duration { nanos: i64::MAX };

    /// The minimum representable duration.
    pub const MIN: Duration = Duration { nanos: i64::MIN };

    /// A duration of zero.
    pub const ZERO: Duration = Duration { nanos: 0 };

    /// Creates a new Duration from the specified number of nanoseconds.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use abseil::absl_time::duration::Duration;
    ///
    /// let d = Duration::from_nanos(1_000_000_000); // 1 second
    /// ```
    #[inline]
    pub const fn from_nanos(nanos: i64) -> Self {
        Duration { nanos }
    }

    /// Creates a new Duration from the specified number of microseconds.
    ///
    /// # Panics
    ///
    /// Panics in debug mode if the multiplication overflows. In release mode,
    /// overflow wraps around. Use [`checked_from_micros`](Self::checked_from_micros)
    /// for a version that returns None on overflow.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use abseil::absl_time::duration::Duration;
    ///
    /// let d = Duration::from_micros(1_000_000); // 1 second
    /// ```
    #[inline]
    pub const fn from_micros(micros: i64) -> Self {
        Duration {
            nanos: micros * 1000,
        }
    }

    /// Creates a new Duration from the specified number of milliseconds.
    ///
    /// # Panics
    ///
    /// Panics in debug mode if the multiplication overflows. In release mode,
    /// overflow wraps around. Use [`checked_from_millis`](Self::checked_from_millis)
    /// for a version that returns None on overflow.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use abseil::absl_time::duration::Duration;
    ///
    /// let d = Duration::from_millis(1000); // 1 second
    /// ```
    #[inline]
    pub const fn from_millis(millis: i64) -> Self {
        Duration {
            nanos: millis * 1_000_000,
        }
    }

    /// Creates a new Duration from the specified number of seconds.
    ///
    /// # Panics
    ///
    /// Panics in debug mode if the multiplication overflows. In release mode,
    /// overflow wraps around. Use [`checked_from_seconds`](Self::checked_from_seconds)
    /// for a version that returns None on overflow.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use abseil::absl_time::duration::Duration;
    ///
    /// let d = Duration::from_seconds(60);
    /// ```
    #[inline]
    pub const fn from_seconds(seconds: i64) -> Self {
        Duration {
            nanos: seconds * 1_000_000_000,
        }
    }

    /// Creates a new Duration from the specified number of minutes.
    ///
    /// # Panics
    ///
    /// Panics in debug mode if the multiplication overflows. In release mode,
    /// overflow wraps around. Use [`checked_from_minutes`](Self::checked_from_minutes)
    /// for a version that returns None on overflow.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use abseil::absl_time::duration::Duration;
    ///
    /// let d = Duration::from_minutes(60); // 1 hour
    /// ```
    #[inline]
    pub const fn from_minutes(minutes: i64) -> Self {
        Duration {
            nanos: minutes * 60_000_000_000,
        }
    }

    /// Creates a new Duration from the specified number of hours.
    ///
    /// # Panics
    ///
    /// Panics in debug mode if the multiplication overflows. In release mode,
    /// overflow wraps around. Use [`checked_from_hours`](Self::checked_from_hours)
    /// for a version that returns None on overflow.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use abseil::absl_time::duration::Duration;
    ///
    /// let d = Duration::from_hours(24); // 1 day
    /// ```
    #[inline]
    pub const fn from_hours(hours: i64) -> Self {
        Duration {
            nanos: hours * 3_600_000_000_000,
        }
    }

    /// Creates a new Duration from microseconds, returning None on overflow.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use abseil::absl_time::duration::Duration;
    ///
    /// assert!(Duration::checked_from_micros(1_000_000).is_some());
    /// assert!(Duration::checked_from_micros(i64::MAX).is_none()); // Would overflow
    /// ```
    #[inline]
    pub const fn checked_from_micros(micros: i64) -> Option<Self> {
        match micros.checked_mul(1000) {
            Some(nanos) => Some(Duration { nanos }),
            None => None,
        }
    }

    /// Creates a new Duration from milliseconds, returning None on overflow.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use abseil::absl_time::duration::Duration;
    ///
    /// assert!(Duration::checked_from_millis(1000).is_some());
    /// assert!(Duration::checked_from_millis(i64::MAX).is_none()); // Would overflow
    /// ```
    #[inline]
    pub const fn checked_from_millis(millis: i64) -> Option<Self> {
        match millis.checked_mul(1_000_000) {
            Some(nanos) => Some(Duration { nanos }),
            None => None,
        }
    }

    /// Creates a new Duration from seconds, returning None on overflow.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use abseil::absl_time::duration::Duration;
    ///
    /// assert!(Duration::checked_from_seconds(60).is_some());
    /// assert!(Duration::checked_from_seconds(i64::MAX).is_none()); // Would overflow
    /// ```
    #[inline]
    pub const fn checked_from_seconds(seconds: i64) -> Option<Self> {
        match seconds.checked_mul(1_000_000_000) {
            Some(nanos) => Some(Duration { nanos }),
            None => None,
        }
    }

    /// Creates a new Duration from minutes, returning None on overflow.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use abseil::absl_time::duration::Duration;
    ///
    /// assert!(Duration::checked_from_minutes(60).is_some());
    /// assert!(Duration::checked_from_minutes(i64::MAX).is_none()); // Would overflow
    /// ```
    #[inline]
    pub const fn checked_from_minutes(minutes: i64) -> Option<Self> {
        match minutes.checked_mul(60_000_000_000) {
            Some(nanos) => Some(Duration { nanos }),
            None => None,
        }
    }

    /// Creates a new Duration from hours, returning None on overflow.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use abseil::absl_time::duration::Duration;
    ///
    /// assert!(Duration::checked_from_hours(24).is_some());
    /// assert!(Duration::checked_from_hours(i64::MAX).is_none()); // Would overflow
    /// ```
    #[inline]
    pub const fn checked_from_hours(hours: i64) -> Option<Self> {
        match hours.checked_mul(3_600_000_000_000) {
            Some(nanos) => Some(Duration { nanos }),
            None => None,
        }
    }

    /// Returns the total number of whole nanoseconds in this duration.
    #[inline]
    pub const fn nanos(&self) -> i64 {
        self.nanos
    }

    /// Returns the total number of whole microseconds in this duration.
    #[inline]
    pub const fn micros(&self) -> i64 {
        self.nanos / 1000
    }

    /// Returns the total number of whole milliseconds in this duration.
    #[inline]
    pub const fn millis(&self) -> i64 {
        self.nanos / 1_000_000
    }

    /// Returns the total number of whole seconds in this duration.
    #[inline]
    pub const fn seconds(&self) -> i64 {
        self.nanos / 1_000_000_000
    }

    /// Returns the total number of whole minutes in this duration.
    #[inline]
    pub const fn minutes(&self) -> i64 {
        self.nanos / 60_000_000_000
    }

    /// Returns the total number of whole hours in this duration.
    #[inline]
    pub const fn hours(&self) -> i64 {
        self.nanos / 3_600_000_000_000
    }

    /// Returns the fractional part of the duration in nanoseconds.
    #[inline]
    pub const fn subsec_nanos(&self) -> i64 {
        self.nanos % 1_000_000_000
    }

    /// Returns the fractional part of the duration in microseconds.
    #[inline]
    pub const fn subsec_micros(&self) -> i64 {
        (self.nanos / 1000) % 1_000_000
    }

    /// Returns the fractional part of the duration in milliseconds.
    #[inline]
    pub const fn subsec_millis(&self) -> i64 {
        (self.nanos / 1_000_000) % 1000
    }

    /// Returns `true` if this duration is zero.
    #[inline]
    pub const fn is_zero(&self) -> bool {
        self.nanos == 0
    }

    /// Returns `true` if this duration is negative.
    #[inline]
    pub const fn is_negative(&self) -> bool {
        self.nanos < 0
    }

    /// Computes the absolute value of this duration.
    #[inline]
    pub const fn abs(&self) -> Duration {
        Duration {
            nanos: if self.nanos < 0 {
                -self.nanos
            } else {
                self.nanos
            },
        }
    }

    /// Checked duration addition. Computes `self + other`,
    /// returning None if an overflow occurred.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use abseil::absl_time::duration::Duration;
    ///
    /// let d1 = Duration::from_seconds(1);
    /// let d2 = Duration::from_seconds(2);
    /// let sum = d1.checked_add(d2).unwrap();
    /// assert_eq!(sum.seconds(), 3);
    /// ```
    #[inline]
    pub const fn checked_add(&self, other: Duration) -> Option<Duration> {
        match self.nanos.checked_add(other.nanos) {
            Some(nanos) => Some(Duration { nanos }),
            None => None,
        }
    }

    /// Saturating duration addition. Computes `self + other`,
    /// returning [`Duration::MAX`] if an overflow occurred.
    #[inline]
    pub const fn saturating_add(&self, other: Duration) -> Duration {
        match self.nanos.checked_add(other.nanos) {
            Some(nanos) => Duration { nanos },
            None => Duration::MAX,
        }
    }

    /// Checked duration subtraction. Computes `self - other`,
    /// returning None if an overflow occurred.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use abseil::absl_time::duration::Duration;
    ///
    /// let d1 = Duration::from_seconds(5);
    /// let d2 = Duration::from_seconds(2);
    /// let diff = d1.checked_sub(d2).unwrap();
    /// assert_eq!(diff.seconds(), 3);
    /// ```
    #[inline]
    pub const fn checked_sub(&self, other: Duration) -> Option<Duration> {
        match self.nanos.checked_sub(other.nanos) {
            Some(nanos) => Some(Duration { nanos }),
            None => None,
        }
    }

    /// Saturating duration subtraction. Computes `self - other`,
    /// returning [`Duration::MIN`] if an overflow occurred.
    #[inline]
    pub const fn saturating_sub(&self, other: Duration) -> Duration {
        match self.nanos.checked_sub(other.nanos) {
            Some(nanos) => Duration { nanos },
            None => Duration::MIN,
        }
    }

    /// Creates a duration from a standard library Duration.
    #[inline]
    pub fn from_std_duration(std: StdDuration) -> Option<Duration> {
        let nanos = std.as_nanos();
        i64::try_from(nanos).ok().map(|nanos| Duration { nanos })
    }

    /// Converts this duration to a standard library Duration if possible.
    #[inline]
    pub fn to_std_duration(&self) -> Option<StdDuration> {
        if self.nanos >= 0 {
            Some(StdDuration::from_nanos(self.nanos as u64))
        } else {
            None
        }
    }
}

impl Default for Duration {
    #[inline]
    fn default() -> Self {
        Duration::ZERO
    }
}

impl Add for Duration {
    type Output = Duration;

    /// Adds two durations.
    ///
    /// # Panics
    ///
    /// Panics in debug mode if the addition overflows. In release mode,
    /// overflow wraps around. Use [`checked_add`](Self::checked_add) or
    /// [`saturating_add`](Self::saturating_add) for safer alternatives.
    #[inline]
    fn add(self, other: Duration) -> Duration {
        Duration {
            nanos: self.nanos + other.nanos,
        }
    }
}

impl AddAssign for Duration {
    #[inline]
    fn add_assign(&mut self, other: Duration) {
        self.nanos += other.nanos;
    }
}

impl Sub for Duration {
    type Output = Duration;

    /// Subtracts two durations.
    ///
    /// # Panics
    ///
    /// Panics in debug mode if the subtraction overflows. In release mode,
    /// overflow wraps around. Use [`checked_sub`](Self::checked_sub) or
    /// [`saturating_sub`](Self::saturating_sub) for safer alternatives.
    #[inline]
    fn sub(self, other: Duration) -> Duration {
        Duration {
            nanos: self.nanos - other.nanos,
        }
    }
}

impl SubAssign for Duration {
    #[inline]
    fn sub_assign(&mut self, other: Duration) {
        self.nanos -= other.nanos;
    }
}

impl fmt::Debug for Duration {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Duration({} ns)", self.nanos)
    }
}

impl From<StdDuration> for Duration {
    #[inline]
    fn from(std: StdDuration) -> Self {
        Duration::from_std_duration(std).unwrap_or_else(|| {
            // For very large durations, use the maximum value
            Duration::MAX
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_duration_from_nanos() {
        let d = Duration::from_nanos(1_000_000_000);
        assert_eq!(d.seconds(), 1);
    }

    #[test]
    fn test_duration_from_micros() {
        let d = Duration::from_micros(1_000_000);
        assert_eq!(d.seconds(), 1);
    }

    #[test]
    fn test_duration_from_millis() {
        let d = Duration::from_millis(1000);
        assert_eq!(d.seconds(), 1);
    }

    #[test]
    fn test_duration_from_seconds() {
        let d = Duration::from_seconds(60);
        assert_eq!(d.minutes(), 1);
    }

    #[test]
    fn test_duration_from_minutes() {
        let d = Duration::from_minutes(60);
        assert_eq!(d.hours(), 1);
    }

    #[test]
    fn test_duration_from_hours() {
        let d = Duration::from_hours(24);
        assert_eq!(d.seconds(), 86400);
    }

    #[test]
    fn test_duration_subsec() {
        let d = Duration::from_nanos(1_500_000_001); // 1.500000001 seconds
        assert_eq!(d.seconds(), 1);
        assert_eq!(d.subsec_nanos(), 500_000_001);
        assert_eq!(d.subsec_micros(), 500_000);
        assert_eq!(d.subsec_millis(), 500);
    }

    #[test]
    fn test_duration_is_zero() {
        assert!(Duration::ZERO.is_zero());
        assert!(!Duration::from_nanos(1).is_zero());
    }

    #[test]
    fn test_duration_is_negative() {
        assert!(Duration::from_nanos(-1).is_negative());
        assert!(!Duration::from_nanos(1).is_negative());
        assert!(!Duration::ZERO.is_negative());
    }

    #[test]
    fn test_duration_abs() {
        let d = Duration::from_nanos(-100);
        assert_eq!(d.abs(), Duration::from_nanos(100));
    }

    #[test]
    fn test_duration_add() {
        let d1 = Duration::from_seconds(1);
        let d2 = Duration::from_seconds(2);
        let sum = d1 + d2;
        assert_eq!(sum.seconds(), 3);
    }

    #[test]
    fn test_duration_sub() {
        let d1 = Duration::from_seconds(5);
        let d2 = Duration::from_seconds(2);
        let diff = d1 - d2;
        assert_eq!(diff.seconds(), 3);
    }

    #[test]
    fn test_duration_saturating_add() {
        let d = Duration::MAX;
        let result = d.saturating_add(Duration::from_nanos(1));
        assert_eq!(result, Duration::MAX);
    }

    #[test]
    fn test_duration_saturating_sub() {
        let d = Duration::MIN;
        let result = d.saturating_sub(Duration::from_nanos(1));
        assert_eq!(result, Duration::MIN);
    }

    #[test]
    fn test_duration_equality() {
        let d1 = Duration::from_seconds(1);
        let d2 = Duration::from_seconds(1);
        assert_eq!(d1, d2);
    }

    #[test]
    fn test_duration_ordering() {
        let d1 = Duration::from_seconds(1);
        let d2 = Duration::from_seconds(2);
        assert!(d1 < d2);
    }

    #[test]
    fn test_duration_from_std() {
        let std = StdDuration::from_secs(1);
        let d = Duration::from_std_duration(std).unwrap();
        assert_eq!(d.seconds(), 1);
    }

    #[test]
    fn test_duration_to_std() {
        let d = Duration::from_seconds(1);
        let std = d.to_std_duration().unwrap();
        assert_eq!(std.as_secs(), 1);
    }

    #[test]
    fn test_duration_negative_to_std() {
        let d = Duration::from_seconds(-1);
        assert!(d.to_std_duration().is_none());
    }

    #[test]
    fn test_duration_default() {
        let d = Duration::default();
        assert_eq!(d, Duration::ZERO);
    }

    // Edge case tests for MEDIUM security fix - Duration overflow in conversions

    #[test]
    fn test_checked_from_micros_normal() {
        assert!(Duration::checked_from_micros(1_000_000).is_some());
        let d = Duration::checked_from_micros(1_000_000).unwrap();
        assert_eq!(d.seconds(), 1);
    }

    #[test]
    fn test_checked_from_micros_overflow() {
        // i64::MAX / 1000 would still overflow when multiplied by 1000
        // The max safe value is i64::MAX / 1000 = 9223372036854775
        let max_safe = i64::MAX / 1000;
        assert!(Duration::checked_from_micros(max_safe).is_some());
        assert!(Duration::checked_from_micros(max_safe + 1).is_none());
        assert!(Duration::checked_from_micros(i64::MAX).is_none());
    }

    #[test]
    fn test_checked_from_micros_zero() {
        assert!(Duration::checked_from_micros(0).is_some());
        let d = Duration::checked_from_micros(0).unwrap();
        assert_eq!(d.nanos(), 0);
    }

    #[test]
    fn test_checked_from_micros_negative() {
        // Negative values should work (wrapping in debug mode in actual multiplication)
        assert!(Duration::checked_from_micros(-1000).is_some());
        let d = Duration::checked_from_micros(-1000).unwrap();
        assert_eq!(d.nanos(), -1_000_000);
    }

    #[test]
    fn test_checked_from_millis_normal() {
        assert!(Duration::checked_from_millis(1000).is_some());
        let d = Duration::checked_from_millis(1000).unwrap();
        assert_eq!(d.seconds(), 1);
    }

    #[test]
    fn test_checked_from_millis_overflow() {
        // The max safe value is i64::MAX / 1_000_000 = 9223372036854
        let max_safe = i64::MAX / 1_000_000;
        assert!(Duration::checked_from_millis(max_safe).is_some());
        assert!(Duration::checked_from_millis(max_safe + 1).is_none());
        assert!(Duration::checked_from_millis(i64::MAX).is_none());
    }

    #[test]
    fn test_checked_from_seconds_normal() {
        assert!(Duration::checked_from_seconds(60).is_some());
        let d = Duration::checked_from_seconds(60).unwrap();
        assert_eq!(d.minutes(), 1);
    }

    #[test]
    fn test_checked_from_seconds_overflow() {
        // The max safe value is i64::MAX / 1_000_000_000 = 9
        let max_safe = i64::MAX / 1_000_000_000;
        assert!(Duration::checked_from_seconds(max_safe).is_some());
        assert!(Duration::checked_from_seconds(max_safe + 1).is_none());
        assert!(Duration::checked_from_seconds(i64::MAX).is_none());
    }

    #[test]
    fn test_checked_from_minutes_normal() {
        assert!(Duration::checked_from_minutes(60).is_some());
        let d = Duration::checked_from_minutes(60).unwrap();
        assert_eq!(d.hours(), 1);
    }

    #[test]
    fn test_checked_from_minutes_overflow() {
        // The max safe value is i64::MAX / 60_000_000_000 = 153
        let max_safe = i64::MAX / 60_000_000_000;
        assert!(Duration::checked_from_minutes(max_safe).is_some());
        assert!(Duration::checked_from_minutes(max_safe + 1).is_none());
        assert!(Duration::checked_from_minutes(i64::MAX).is_none());
    }

    #[test]
    fn test_checked_from_hours_normal() {
        assert!(Duration::checked_from_hours(24).is_some());
        let d = Duration::checked_from_hours(24).unwrap();
        assert_eq!(d.seconds(), 86400);
    }

    #[test]
    fn test_checked_from_hours_overflow() {
        // The max safe value is i64::MAX / 3_600_000_000_000 = 2
        let max_safe = i64::MAX / 3_600_000_000_000;
        assert!(Duration::checked_from_hours(max_safe).is_some());
        assert!(Duration::checked_from_hours(max_safe + 1).is_none());
        assert!(Duration::checked_from_hours(i64::MAX).is_none());
    }

    #[test]
    fn test_checked_methods_with_max_values() {
        // Test that the maximum safe values produce correct results
        let max_micros = i64::MAX / 1000;
        let d1 = Duration::checked_from_micros(max_micros).unwrap();
        assert_eq!(d1.nanos(), max_micros * 1000);

        let max_millis = i64::MAX / 1_000_000;
        let d2 = Duration::checked_from_millis(max_millis).unwrap();
        assert_eq!(d2.nanos(), max_millis * 1_000_000);

        let max_seconds = i64::MAX / 1_000_000_000;
        let d3 = Duration::checked_from_seconds(max_seconds).unwrap();
        assert_eq!(d3.nanos(), max_seconds * 1_000_000_000);
    }

    #[test]
    fn test_checked_from_methods_boundary_values() {
        // Test boundary values around overflow points
        // i64::MAX = 9223372036854775807
        // Max safe seconds = 9223372036854775807 / 1_000_000_000 = 9223372036
        let max_seconds = i64::MAX / 1_000_000_000;
        assert!(Duration::checked_from_seconds(max_seconds).is_some());
        assert!(Duration::checked_from_seconds(max_seconds + 1).is_none());

        // Max safe minutes = 9223372036854775807 / 60_000_000_000 = 153722867
        let max_minutes = i64::MAX / 60_000_000_000;
        assert!(Duration::checked_from_minutes(max_minutes).is_some());
        assert!(Duration::checked_from_minutes(max_minutes + 1).is_none());

        // Max safe hours = 9223372036854775807 / 3_600_000_000_000 = 2562047
        let max_hours = i64::MAX / 3_600_000_000_000;
        assert!(Duration::checked_from_hours(max_hours).is_some());
        assert!(Duration::checked_from_hours(max_hours + 1).is_none());
    }

    #[test]
    fn test_checked_methods_consistency_with_unchecked() {
        // Verify that for values that don't overflow, checked and unchecked match
        let test_values = [0, 1, 1000, 1_000_000, 60];

        for &val in &test_values {
            if let Some(checked) = Duration::checked_from_micros(val) {
                assert_eq!(checked, Duration::from_micros(val));
            }
            if let Some(checked) = Duration::checked_from_millis(val) {
                assert_eq!(checked, Duration::from_millis(val));
            }
            if let Some(checked) = Duration::checked_from_seconds(val) {
                assert_eq!(checked, Duration::from_seconds(val));
            }
            if let Some(checked) = Duration::checked_from_minutes(val) {
                assert_eq!(checked, Duration::from_minutes(val));
            }
            if let Some(checked) = Duration::checked_from_hours(val) {
                assert_eq!(checked, Duration::from_hours(val));
            }
        }
    }
}
