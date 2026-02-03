//! Date and time validation, calendar utilities

use super::error::TimeError;

/// Validates a time value.
///
/// Returns `Ok(())` if the time components are valid.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_time::validate_time;
///
/// assert!(validate_time(12, 30, 45).is_ok());
/// assert!(validate_time(25, 0, 0).is_err());
/// ```
#[inline]
pub const fn validate_time(hour: u32, minute: u32, second: u32) -> Result<(), TimeError> {
    if hour > 23 {
        return Err(TimeError::OutOfRange);
    }
    if minute > 59 {
        return Err(TimeError::OutOfRange);
    }
    if second > 59 {
        return Err(TimeError::OutOfRange);
    }
    Ok(())
}

/// Validates a date value.
///
/// Returns `Ok(())` if the date components are valid.
#[inline]
pub const fn validate_date(year: i64, month: u32, day: u32) -> Result<(), TimeError> {
    if year < 0 {
        return Err(TimeError::OutOfRange);
    }
    if month < 1 || month > 12 {
        return Err(TimeError::OutOfRange);
    }
    if day < 1 || day > 31 {
        return Err(TimeError::OutOfRange);
    }
    Ok(())
}

/// Checks if a year is a leap year (Gregorian calendar).
///
/// # Examples
///
/// ```rust
/// use abseil::absl_time::is_leap_year;
///
/// assert!(!is_leap_year(2023));
/// assert!(is_leap_year(2024));
/// assert!(is_leap_year(2000));
/// assert!(!is_leap_year(1900));
/// ```
#[inline]
pub const fn is_leap_year(year: i64) -> bool {
    (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
}

/// Returns the number of days in a month.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_time::days_in_month;
///
/// assert_eq!(days_in_month(2024, 2).unwrap(), 29);
/// assert_eq!(days_in_month(2023, 2).unwrap(), 28);
/// assert_eq!(days_in_month(2024, 4).unwrap(), 30);
/// ```
#[inline]
pub const fn days_in_month(year: i64, month: u32) -> Result<u32, TimeError> {
    if month < 1 || month > 12 {
        return Err(TimeError::OutOfRange);
    }

    let days = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if is_leap_year(year) {
                29
            } else {
                28
            }
        }
        _ => return Err(TimeError::OutOfRange),
    };

    Ok(days)
}

/// Converts a 12-hour clock time to 24-hour format.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_time::to_24_hour;
///
/// assert_eq!(to_24_hour(3, 30, true), (15, 30));
/// assert_eq!(to_24_hour(3, 30, false), (3, 30));
/// assert_eq!(to_24_hour(12, 0, false), (0, 0));
/// ```
#[inline]
pub const fn to_24_hour(hour: u32, minute: u32, is_pm: bool) -> (u32, u32) {
    if is_pm {
        if hour == 12 {
            (12, minute)
        } else {
            (hour + 12, minute)
        }
    } else {
        if hour == 12 {
            (0, minute)
        } else {
            (hour, minute)
        }
    }
}

/// Converts a 24-hour clock time to 12-hour format.
///
/// Returns (hour_12, minute, is_pm).
///
/// # Examples
///
/// ```rust
/// use abseil::absl_time::to_12_hour;
///
/// assert_eq!(to_12_hour(15, 30), (3, 30, true));
/// assert_eq!(to_12_hour(3, 30), (3, 30, false));
/// assert_eq!(to_12_hour(0, 0), (12, 0, false));
/// ```
#[inline]
pub const fn to_12_hour(hour: u32, minute: u32) -> (u32, u32, bool) {
    if hour == 0 {
        (12, minute, false)
    } else if hour < 12 {
        (hour, minute, false)
    } else if hour == 12 {
        (12, minute, true)
    } else {
        (hour - 12, minute, true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_time() {
        assert!(validate_time(0, 0, 0).is_ok());
        assert!(validate_time(12, 30, 45).is_ok());
        assert!(validate_time(23, 59, 59).is_ok());
        assert!(validate_time(24, 0, 0).is_err());
        assert!(validate_time(12, 60, 0).is_err());
        assert!(validate_time(12, 0, 60).is_err());
    }

    #[test]
    fn test_validate_date() {
        assert!(validate_date(2024, 1, 15).is_ok());
        assert!(validate_date(2024, 12, 31).is_ok());
        assert!(validate_date(2024, 0, 1).is_err());
        assert!(validate_date(2024, 13, 1).is_err());
        assert!(validate_date(2024, 1, 0).is_err());
        assert!(validate_date(2024, 1, 32).is_err());
        assert!(validate_date(-1, 1, 1).is_err());
    }

    #[test]
    fn test_is_leap_year() {
        assert!(!is_leap_year(2023));
        assert!(is_leap_year(2024));
        assert!(is_leap_year(2000));
        assert!(!is_leap_year(1900));
        assert!(is_leap_year(1600));
    }

    #[test]
    fn test_days_in_month() {
        assert_eq!(days_in_month(2024, 1).unwrap(), 31);
        assert_eq!(days_in_month(2024, 2).unwrap(), 29);
        assert_eq!(days_in_month(2023, 2).unwrap(), 28);
        assert_eq!(days_in_month(2024, 4).unwrap(), 30);
        assert_eq!(days_in_month(2024, 12).unwrap(), 31);
        assert!(days_in_month(2024, 13).is_err());
    }

    #[test]
    fn test_to_24_hour() {
        assert_eq!(to_24_hour(3, 30, false), (3, 30));
        assert_eq!(to_24_hour(3, 30, true), (15, 30));
        assert_eq!(to_24_hour(12, 0, false), (0, 0));
        assert_eq!(to_24_hour(12, 0, true), (12, 0));
        assert_eq!(to_24_hour(11, 59, true), (23, 59));
    }

    #[test]
    fn test_to_12_hour() {
        assert_eq!(to_12_hour(0, 0), (12, 0, false));
        assert_eq!(to_12_hour(3, 30), (3, 30, false));
        assert_eq!(to_12_hour(12, 0), (12, 0, true));
        assert_eq!(to_12_hour(15, 30), (3, 30, true));
        assert_eq!(to_12_hour(23, 59), (11, 59, true));
    }
}
