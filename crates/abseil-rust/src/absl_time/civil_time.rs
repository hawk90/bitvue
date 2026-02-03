//! Civil time - date and time for calendar operations.
//!
//! This module provides civil time types (year, month, day, hour, minute, second)
//! for working with calendar dates and times, similar to Abseil's `absl::CivilTime`.
//!
//! # Example
//!
//! ```rust
//! use abseil::absl_time::civil_time::*;
//!
//! let (year, month, day) = CivilDay::from_ymd(2025, 1, 31);
//! assert_eq!(year.year(), 2025);
//! assert_eq!(month.month(), 1);
//! assert_eq!(day.day(), 31);
//! ```

use core::fmt;
use core::ops::{Add, Sub};

/// Error type for civil time validation.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum CivilTimeError {
    /// Month value is out of range (must be 1-12).
    InvalidMonth(i8),
    /// Day value is out of range (must be 1-31).
    InvalidDay(i8),
    /// Hour value is out of range (must be 0-23).
    InvalidHour(i8),
    /// Minute value is out of range (must be 0-59).
    InvalidMinute(i8),
    /// Second value is out of range (must be 0-59).
    InvalidSecond(i8),
    /// Day is invalid for the given month.
    InvalidDayForMonth { month: i8, day: i8 },
    /// Year value is out of range.
    InvalidYear(i16),
}

impl fmt::Display for CivilTimeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CivilTimeError::InvalidMonth(m) => write!(f, "Month must be 1-12, got {}", m),
            CivilTimeError::InvalidDay(d) => write!(f, "Day must be 1-31, got {}", d),
            CivilTimeError::InvalidHour(h) => write!(f, "Hour must be 0-23, got {}", h),
            CivilTimeError::InvalidMinute(m) => write!(f, "Minute must be 0-59, got {}", m),
            CivilTimeError::InvalidSecond(s) => write!(f, "Second must be 0-59, got {}", s),
            CivilTimeError::InvalidDayForMonth { month, day } => {
                write!(f, "Day {} is invalid for month {}", day, month)
            }
            CivilTimeError::InvalidYear(y) => {
                write!(f, "Year must be between {} and {}, got {}", CivilYear::MIN, CivilYear::MAX, y)
            }
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for CivilTimeError {}

/// Civil year (e.g., 2025).
#[repr(transparent)]
#[derive(Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CivilYear(pub i16);

/// Civil month (1-12).
#[repr(transparent)]
#[derive(Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CivilMonth(pub i8);

/// Civil day (1-31).
#[repr(transparent)]
#[derive(Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CivilDay(pub i8);

/// Civil hour (0-23).
#[repr(transparent)]
#[derive(Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CivilHour(pub i8);

/// Civil minute (0-59).
#[repr(transparent)]
#[derive(Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CivilMinute(pub i8);

/// Civil second (0-59).
#[repr(transparent)]
#[derive(Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CivilSecond(pub i8);

// ============================================================================
// CivilYear Implementation
// ============================================================================

impl CivilYear {
    /// Minimum valid year.
    pub const MIN: i16 = -9999;
    /// Maximum valid year.
    pub const MAX: i16 = 9999;

    /// Creates a new `CivilYear`.
    #[inline]
    pub const fn new(year: i16) -> Self {
        Self(year)
    }

    /// Returns the year value.
    #[inline]
    pub const fn year(&self) -> i16 {
        self.0
    }

    /// Returns `true` if this is a leap year.
    pub fn is_leap(&self) -> bool {
        self.0 % 4 == 0 && (self.0 % 100 != 0 || self.0 % 400 == 0)
    }

    /// Returns the number of days in this year.
    pub fn days_in_year(&self) -> u16 {
        if self.is_leap() {
            366
        } else {
            365
        }
    }
}

// ============================================================================
// CivilMonth Implementation
// ============================================================================

impl CivilMonth {
    /// Creates a new `CivilMonth`.
    ///
    /// # Panics
    ///
    /// Panics if `month < 1` or `month > 12`.
    #[inline]
    pub const fn new(month: i8) -> Self {
        assert!(month >= 1 && month <= 12, "Month must be 1-12");
        Self(month)
    }

    /// Creates a new `CivilMonth` without panicking.
    ///
    /// Returns `Err` if `month < 1` or `month > 12`.
    #[inline]
    pub const fn try_new(month: i8) -> Result<Self, CivilTimeError> {
        if month >= 1 && month <= 12 {
            Ok(Self(month))
        } else {
            Err(CivilTimeError::InvalidMonth(month))
        }
    }

    /// Returns the month value (1-12).
    #[inline]
    pub const fn month(&self) -> i8 {
        self.0
    }

    /// Returns `true` if this month is valid for the given day.
    pub const fn valid_for_day(&self, day: i8) -> bool {
        let max_day = match self.0 {
            1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
            4 | 6 | 9 | 11 => 30,
            2 => 29, // February max (including leap years)
            _ => return false,
        };
        day >= 1 && day <= max_day
    }
}

// ============================================================================
// CivilDay Implementation
// ============================================================================

impl CivilDay {
    /// Creates a new `CivilDay`.
    ///
    /// # Panics
    ///
    /// Panics if `day < 1` or `day > 31`.
    #[inline]
    pub const fn new(day: i8) -> Self {
        assert!(day >= 1 && day <= 31, "Day must be 1-31");
        Self(day)
    }

    /// Creates a new `CivilDay` without panicking.
    ///
    /// Returns `Err` if `day < 1` or `day > 31`.
    #[inline]
    pub const fn try_new(day: i8) -> Result<Self, CivilTimeError> {
        if day >= 1 && day <= 31 {
            Ok(Self(day))
        } else {
            Err(CivilTimeError::InvalidDay(day))
        }
    }

    /// Returns the day value (1-31).
    #[inline]
    pub const fn day(&self) -> i8 {
        self.0
    }

    /// Creates a date from year, month, and day.
    ///
    /// # Panics
    ///
    /// Panics if the date is invalid.
    pub fn from_ymd(year: i16, month: i8, day: i8) -> (CivilYear, CivilMonth, CivilDay) {
        Self::try_from_ymd(year, month, day).unwrap()
    }

    /// Creates a date from year, month, and day without panicking.
    ///
    /// Returns `Err` if the date is invalid.
    pub fn try_from_ymd(year: i16, month: i8, day: i8) -> Result<(CivilYear, CivilMonth, CivilDay), CivilTimeError> {
        if month < 1 || month > 12 {
            return Err(CivilTimeError::InvalidMonth(month));
        }
        if day < 1 || day > 31 {
            return Err(CivilTimeError::InvalidDay(day));
        }
        if year < CivilYear::MIN || year > CivilYear::MAX {
            return Err(CivilTimeError::InvalidYear(year));
        }
        let m = CivilMonth::new(month);
        if !m.valid_for_day(day) {
            return Err(CivilTimeError::InvalidDayForMonth { month, day });
        }
        // Special check for February 29th - must be a leap year
        if month == 2 && day == 29 {
            let y = CivilYear(year);
            if !y.is_leap() {
                return Err(CivilTimeError::InvalidDayForMonth { month, day });
            }
        }
        Ok((CivilYear(year), m, CivilDay(day)))
    }
}

// ============================================================================
// CivilHour Implementation
// ============================================================================

impl CivilHour {
    /// Creates a new `CivilHour`.
    ///
    /// # Panics
    ///
    /// Panics if `hour < 0` or `hour > 23`.
    #[inline]
    pub const fn new(hour: i8) -> Self {
        assert!(hour >= 0 && hour <= 23, "Hour must be 0-23");
        Self(hour)
    }

    /// Creates a new `CivilHour` without panicking.
    ///
    /// Returns `Err` if `hour < 0` or `hour > 23`.
    #[inline]
    pub const fn try_new(hour: i8) -> Result<Self, CivilTimeError> {
        if hour >= 0 && hour <= 23 {
            Ok(Self(hour))
        } else {
            Err(CivilTimeError::InvalidHour(hour))
        }
    }

    /// Returns the hour value (0-23).
    #[inline]
    pub const fn hour(&self) -> i8 {
        self.0
    }
}

// ============================================================================
// CivilMinute Implementation
// ============================================================================

impl CivilMinute {
    /// Creates a new `CivilMinute`.
    ///
    /// # Panics
    ///
    /// Panics if `minute < 0` or `minute > 59`.
    #[inline]
    pub const fn new(minute: i8) -> Self {
        assert!(minute >= 0 && minute <= 59, "Minute must be 0-59");
        Self(minute)
    }

    /// Creates a new `CivilMinute` without panicking.
    ///
    /// Returns `Err` if `minute < 0` or `minute > 59`.
    #[inline]
    pub const fn try_new(minute: i8) -> Result<Self, CivilTimeError> {
        if minute >= 0 && minute <= 59 {
            Ok(Self(minute))
        } else {
            Err(CivilTimeError::InvalidMinute(minute))
        }
    }

    /// Returns the minute value (0-59).
    #[inline]
    pub const fn minute(&self) -> i8 {
        self.0
    }
}

// ============================================================================
// CivilSecond Implementation
// ============================================================================

impl CivilSecond {
    /// Creates a new `CivilSecond`.
    ///
    /// # Panics
    ///
    /// Panics if `second < 0` or `second > 59`.
    #[inline]
    pub const fn new(second: i8) -> Self {
        assert!(second >= 0 && second <= 59, "Second must be 0-59");
        Self(second)
    }

    /// Creates a new `CivilSecond` without panicking.
    ///
    /// Returns `Err` if `second < 0` or `second > 59`.
    #[inline]
    pub const fn try_new(second: i8) -> Result<Self, CivilTimeError> {
        if second >= 0 && second <= 59 {
            Ok(Self(second))
        } else {
            Err(CivilTimeError::InvalidSecond(second))
        }
    }

    /// Returns the second value (0-59).
    #[inline]
    pub const fn second(&self) -> i8 {
        self.0
    }
}

// ============================================================================
// Format Implementations
// ============================================================================

impl fmt::Display for CivilYear {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl fmt::Debug for CivilYear {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "CivilYear({})", self.0)
    }
}

impl fmt::Display for CivilMonth {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl fmt::Debug for CivilMonth {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "CivilMonth({})", self.0)
    }
}

impl fmt::Display for CivilDay {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl fmt::Debug for CivilDay {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "CivilDay({})", self.0)
    }
}

impl fmt::Display for CivilHour {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl fmt::Debug for CivilHour {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "CivilHour({})", self.0)
    }
}

impl fmt::Display for CivilMinute {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl fmt::Debug for CivilMinute {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "CivilMinute({})", self.0)
    }
}

impl fmt::Display for CivilSecond {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl fmt::Debug for CivilSecond {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "CivilSecond({})", self.0)
    }
}

// ============================================================================
// Operator Implementations
// ============================================================================

impl Add<i32> for CivilYear {
    type Output = Self;

    fn add(self, rhs: i32) -> Self::Output {
        Self(self.0.saturating_add(rhs as i16))
    }
}

impl Sub<i32> for CivilYear {
    type Output = Self;

    fn sub(self, rhs: i32) -> Self::Output {
        Self(self.0.saturating_sub(rhs as i16))
    }
}

impl Add<i32> for CivilMonth {
    type Output = Self;

    fn add(self, rhs: i32) -> Self::Output {
        let mut result = self.0 as i32 + rhs;
        while result > 12 {
            result -= 12;
        }
        while result < 1 {
            result += 12;
        }
        Self(result as i8)
    }
}

impl Add<i32> for CivilDay {
    type Output = Self;

    fn add(self, rhs: i32) -> Self::Output {
        let mut result = self.0 as i32 + rhs;
        while result > 31 {
            result -= 31;
        }
        while result < 1 {
            result += 31;
        }
        Self(result as i8)
    }
}

// ============================================================================
// From Implementations
// ============================================================================

impl From<i16> for CivilYear {
    fn from(value: i16) -> Self {
        Self(value)
    }
}

impl From<i8> for CivilMonth {
    fn from(value: i8) -> Self {
        Self(value)
    }
}

impl From<i8> for CivilDay {
    fn from(value: i8) -> Self {
        Self(value)
    }
}

impl From<i8> for CivilHour {
    fn from(value: i8) -> Self {
        Self(value)
    }
}

impl From<i8> for CivilMinute {
    fn from(value: i8) -> Self {
        Self(value)
    }
}

impl From<i8> for CivilSecond {
    fn from(value: i8) -> Self {
        Self(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_year() {
        let year = CivilYear::new(2025);
        assert_eq!(year.year(), 2025);
        assert!(!year.is_leap());
        assert_eq!(year.days_in_year(), 365);

        let leap = CivilYear::new(2024);
        assert!(leap.is_leap());
        assert_eq!(leap.days_in_year(), 366);
    }

    #[test]
    fn test_month() {
        let month = CivilMonth::new(6);
        assert_eq!(month.month(), 6);
        assert!(month.valid_for_day(30));
    }

    #[test]
    fn test_day() {
        let day = CivilDay::new(15);
        assert_eq!(day.day(), 15);
    }

    #[test]
    fn test_hour() {
        let hour = CivilHour::new(12);
        assert_eq!(hour.hour(), 12);
    }

    #[test]
    fn test_minute() {
        let minute = CivilMinute::new(30);
        assert_eq!(minute.minute(), 30);
    }

    #[test]
    fn test_second() {
        let second = CivilSecond::new(45);
        assert_eq!(second.second(), 45);
    }

    #[test]
    fn test_from_ymd() {
        let (year, month, day) = CivilDay::from_ymd(2025, 1, 31);
        assert_eq!(year.year(), 2025);
        assert_eq!(month.month(), 1);
        assert_eq!(day.day(), 31);
    }

    #[test]
    fn test_year_add() {
        let year = CivilYear::new(2025);
        let next = year + 1;
        assert_eq!(next.year(), 2026);
    }

    #[test]
    fn test_display() {
        assert_eq!(format!("{}", CivilYear(2025)), "2025");
        assert_eq!(format!("{}", CivilMonth(6)), "6");
        assert_eq!(format!("{}", CivilDay(15)), "15");
    }

    // Tests for MEDIUM fix - try_new variants to avoid panics

    #[test]
    fn test_civil_month_try_new_valid() {
        assert!(CivilMonth::try_new(1).is_ok());
        assert!(CivilMonth::try_new(12).is_ok());
        assert!(CivilMonth::try_new(6).is_ok());
    }

    #[test]
    fn test_civil_month_try_new_invalid() {
        assert!(CivilMonth::try_new(0).is_err());
        assert!(CivilMonth::try_new(-1).is_err());
        assert!(CivilMonth::try_new(13).is_err());
        assert!(CivilMonth::try_new(100).is_err());
    }

    #[test]
    fn test_civil_day_try_new_valid() {
        assert!(CivilDay::try_new(1).is_ok());
        assert!(CivilDay::try_new(31).is_ok());
        assert!(CivilDay::try_new(15).is_ok());
    }

    #[test]
    fn test_civil_day_try_new_invalid() {
        assert!(CivilDay::try_new(0).is_err());
        assert!(CivilDay::try_new(-1).is_err());
        assert!(CivilDay::try_new(32).is_err());
        assert!(CivilDay::try_new(100).is_err());
    }

    #[test]
    fn test_civil_hour_try_new_valid() {
        assert!(CivilHour::try_new(0).is_ok());
        assert!(CivilHour::try_new(23).is_ok());
        assert!(CivilHour::try_new(12).is_ok());
    }

    #[test]
    fn test_civil_hour_try_new_invalid() {
        assert!(CivilHour::try_new(-1).is_err());
        assert!(CivilHour::try_new(24).is_err());
        assert!(CivilHour::try_new(100).is_err());
    }

    #[test]
    fn test_civil_minute_try_new_valid() {
        assert!(CivilMinute::try_new(0).is_ok());
        assert!(CivilMinute::try_new(59).is_ok());
        assert!(CivilMinute::try_new(30).is_ok());
    }

    #[test]
    fn test_civil_minute_try_new_invalid() {
        assert!(CivilMinute::try_new(-1).is_err());
        assert!(CivilMinute::try_new(60).is_err());
        assert!(CivilMinute::try_new(100).is_err());
    }

    #[test]
    fn test_civil_second_try_new_valid() {
        assert!(CivilSecond::try_new(0).is_ok());
        assert!(CivilSecond::try_new(59).is_ok());
        assert!(CivilSecond::try_new(45).is_ok());
    }

    #[test]
    fn test_civil_second_try_new_invalid() {
        assert!(CivilSecond::try_new(-1).is_err());
        assert!(CivilSecond::try_new(60).is_err());
        assert!(CivilSecond::try_new(100).is_err());
    }

    #[test]
    fn test_try_from_ymd_valid() {
        assert!(CivilDay::try_from_ymd(2025, 1, 31).is_ok());
        assert!(CivilDay::try_from_ymd(2024, 2, 29).is_ok()); // Leap year
        assert!(CivilDay::try_from_ymd(2025, 4, 30).is_ok()); // 30 days
    }

    #[test]
    fn test_try_from_ymd_invalid_month() {
        assert!(CivilDay::try_from_ymd(2025, 0, 15).is_err());
        assert!(CivilDay::try_from_ymd(2025, 13, 15).is_err());
    }

    #[test]
    fn test_try_from_ymd_invalid_day_for_month() {
        assert!(CivilDay::try_from_ymd(2025, 4, 31).is_err()); // April has 30 days
        assert!(CivilDay::try_from_ymd(2025, 2, 30).is_err()); // Feb max 29
        assert!(CivilDay::try_from_ymd(2023, 2, 29).is_err()); // Not a leap year
    }

    #[test]
    fn test_try_from_ymd_invalid_day_range() {
        assert!(CivilDay::try_from_ymd(2025, 1, 0).is_err());
        assert!(CivilDay::try_from_ymd(2025, 1, 32).is_err());
    }

    #[test]
    fn test_try_from_ymd_invalid_year() {
        assert!(CivilDay::try_from_ymd(-10000, 1, 15).is_err());
        assert!(CivilDay::try_from_ymd(10000, 1, 15).is_err());
    }

    #[test]
    fn test_civil_time_error_display() {
        assert_eq!(
            format!("{}", CivilTimeError::InvalidMonth(13)),
            "Month must be 1-12, got 13"
        );
        assert_eq!(
            format!("{}", CivilTimeError::InvalidDay(0)),
            "Day must be 1-31, got 0"
        );
        assert_eq!(
            format!("{}", CivilTimeError::InvalidDayForMonth { month: 4, day: 31 }),
            "Day 31 is invalid for month 4"
        );
    }
}
