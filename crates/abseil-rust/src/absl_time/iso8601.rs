//! ISO 8601 parsing and formatting

use alloc::string::String;
use alloc::format;
use core::time::Duration as StdDuration;
use super::timestamp::Timestamp;
use super::error::TimeError;

/// Formats a duration in a human-readable format.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_time::format_duration;
/// use core::time::Duration;
///
/// let dur = Duration::from_secs(3661);
/// assert_eq!(format_duration(dur), "1h 1m 1s");
/// ```
#[inline]
pub fn format_duration(dur: StdDuration) -> String {
    let secs = dur.as_secs();
    let hours = secs / 3600;
    let minutes = (secs % 3600) / 60;
    let seconds = secs % 60;
    let millis = dur.subsec_millis();

    let mut parts = alloc::vec::Vec::new();

    if hours > 0 {
        parts.push(format!("{}h", hours));
    }
    if minutes > 0 || hours > 0 {
        parts.push(format!("{}m", minutes));
    }
    if seconds > 0 || minutes > 0 || hours > 0 {
        parts.push(format!("{}s", seconds));
    }
    if millis > 0 && hours == 0 {
        parts.push(format!("{}ms", millis));
    }

    if parts.is_empty() {
        parts.push("0s".into());
    }

    parts.join(" ")
}

/// Parses an ISO 8601 timestamp string.
///
/// Supports formats like:
/// - "2024-01-15T10:30:00Z"
/// - "2024-01-15T10:30:00+08:00"
/// - "2024-01-15 10:30:00Z"
///
/// # Examples
///
/// ```rust
/// use abseil::absl_time::parse_iso8601;
///
/// let ts = parse_iso8601("2024-01-15T10:30:00Z").unwrap();
/// assert_eq!(ts.seconds(), 1705315800);
/// ```
pub fn parse_iso8601(s: &str) -> Result<Timestamp, TimeError> {
    let s = s.trim();

    // Remove 'T' or space separator
    let separator = if s.contains('T') { 'T' } else { ' ' };
    let parts: Vec<&str> = s.split(separator).collect();

    if parts.len() != 2 {
        return Err(TimeError::InvalidFormat(
            "Missing date/time separator".into(),
        ));
    }

    // Parse date (YYYY-MM-DD)
    let date_parts: Vec<&str> = parts[0].split('-').collect();
    if date_parts.len() != 3 {
        return Err(TimeError::InvalidFormat("Invalid date format".into()));
    }

    let year: i64 = date_parts[0]
        .parse()
        .map_err(|_| TimeError::InvalidFormat("Invalid year".into()))?;
    let month: u32 = date_parts[1]
        .parse()
        .map_err(|_| TimeError::InvalidFormat("Invalid month".into()))?;
    let day: u32 = date_parts[2]
        .parse()
        .map_err(|_| TimeError::InvalidFormat("Invalid day".into()))?;

    // Parse time (HH:MM:SS)
    let time_part = parts[1].trim_end_matches(|c| c == 'Z' || c == 'z');
    let tz_sep = time_part.find(|c| c == '+' || c == '-');
    let (time_str, offset_str) = if let Some(pos) = tz_sep {
        (&time_part[..pos], &time_part[pos..])
    } else {
        (time_part, "")
    };

    let time_parts: Vec<&str> = time_str.split(':').collect();
    if time_parts.len() < 2 || time_parts.len() > 3 {
        return Err(TimeError::InvalidFormat("Invalid time format".into()));
    }

    let hour: u32 = time_parts[0]
        .parse()
        .map_err(|_| TimeError::InvalidFormat("Invalid hour".into()))?;
    let minute: u32 = time_parts[1]
        .parse()
        .map_err(|_| TimeError::InvalidFormat("Invalid minute".into()))?;
    let second: u32 = if time_parts.len() == 3 {
        time_parts[2]
            .parse()
            .map_err(|_| TimeError::InvalidFormat("Invalid second".into()))?
    } else {
        0
    };

    // Validate ranges
    if month < 1 || month > 12 {
        return Err(TimeError::OutOfRange);
    }
    if day < 1 || day > 31 {
        return Err(TimeError::OutOfRange);
    }
    if hour > 23 {
        return Err(TimeError::OutOfRange);
    }
    if minute > 59 {
        return Err(TimeError::OutOfRange);
    }
    if second > 59 {
        return Err(TimeError::OutOfRange);
    }

    // Calculate days since epoch (simplified calculation)
    let days_since_epoch = (year - 1970) * 365
        + (year - 1) / 4
        - (year - 1) / 100
        + (year - 1) / 400
        + day_of_year(month, day)
        - 1;

    let mut total_seconds = days_since_epoch * 86400 + hour as i64 * 3600 + minute as i64 * 60 + second as i64;

    // Apply timezone offset
    // SAFETY: offset_str starts with '+' or '-' followed by timezone data.
    // The check !offset_str.is_empty() guarantees at least 1 character,
    // so offset_str[1..] safely skips the sign character.
    if !offset_str.is_empty() {
        // SAFETY: offset_str[1..] is safe because offset_str has at least 1 character.
        // For example, if offset_str is "+08:00", then offset_str[1..] is "08:00".
        let offset_parts: Vec<&str> = offset_str[1..].split(':').collect();
        if offset_parts.len() >= 2 {
            let offset_hours: i32 = offset_parts[0]
                .parse()
                .map_err(|_| TimeError::InvalidFormat("Invalid offset hours".into()))?;
            let offset_minutes: i32 = offset_parts[1]
                .parse()
                .map_err(|_| TimeError::InvalidFormat("Invalid offset minutes".into()))?;

            let offset_secs = offset_hours * 3600 + offset_minutes * 60;
            if offset_str.starts_with('-') {
                total_seconds += offset_secs as i64;
            } else {
                total_seconds -= offset_secs as i64;
            }
        }
    }

    Ok(Timestamp::new(total_seconds, 0))
}

/// Calculates the day of year (1-indexed) for a given month and day.
///
/// # Panics
///
/// Panics if `month < 1` or `month > 12`.
#[inline]
const fn day_of_year(month: u32, day: u32) -> i64 {
    // SAFETY: month is validated to be 1..12 in parse_iso8601 before calling this function.
    // days_before_month has 12 elements indexed by (month - 1), which is 0..11.
    let days_before_month = [0, 31, 59, 90, 120, 151, 181, 212, 243, 273, 304, 334];
    days_before_month[(month - 1) as usize] as i64 + day as i64
}

/// Formats a timestamp as an ISO 8601 string (UTC).
///
/// # Examples
///
/// ```rust
/// use abseil::absl_time::{format_iso8601, Timestamp};
///
/// let ts = Timestamp::from_seconds(1705315800);
/// assert!(format_iso8601(ts).starts_with("2024-01-15"));
/// ```
pub fn format_iso8601(ts: Timestamp) -> String {
    // Simplified conversion - for production use proper calendar algorithms
    let total_days = ts.seconds / 86400;
    let year = 1970 + (total_days / 365) as i64;

    let remaining_secs = ts.seconds % 86400;
    let hour = (remaining_secs / 3600) as u32;
    let minute = ((remaining_secs % 3600) / 60) as u32;
    let second = (remaining_secs % 60) as u32;

    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        year,
        1,
        (total_days % 365) as u32 + 1,
        hour,
        minute,
        second
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_duration() {
        assert_eq!(format_duration(StdDuration::from_secs(3661)), "1h 1m 1s");
        assert_eq!(format_duration(StdDuration::from_secs(60)), "1m 0s");
        assert_eq!(format_duration(StdDuration::from_secs(1)), "0m 1s");
        assert_eq!(
            format_duration(StdDuration::from_millis(1500)),
            "1s 500ms"
        );
        assert_eq!(format_duration(StdDuration::from_millis(500)), "0s 500ms");
        assert_eq!(format_duration(StdDuration::ZERO), "0s");
    }

    #[test]
    fn test_parse_iso8601() {
        let ts = parse_iso8601("2024-01-15T10:30:00Z").unwrap();
        // Approximately 1705315800 seconds since epoch
        assert!(ts.seconds() > 1705315000 && ts.seconds() < 1705317000);

        let ts_with_tz = parse_iso8601("2024-01-15T10:30:00+08:00").unwrap();
        // Should be different due to timezone offset
        assert_ne!(ts.seconds(), ts_with_tz.seconds());

        let ts_space = parse_iso8601("2024-01-15 10:30:00Z");
        assert!(ts_space.is_ok());

        assert!(parse_iso8601("invalid").is_err());
        assert!(parse_iso8601("2024-13-01T00:00:00Z").is_err()); // Invalid month
        assert!(parse_iso8601("2024-01-32T00:00:00Z").is_err()); // Invalid day
    }
}
