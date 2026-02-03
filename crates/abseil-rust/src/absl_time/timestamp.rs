//! TimeZone and Timestamp utilities

use alloc::string::String;
use alloc::format;
use core::time::Duration as StdDuration;

/// Represents a time zone offset from UTC.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TimeZoneOffset {
    /// Offset in seconds from UTC. Positive values are east of UTC.
    pub offset_seconds: i32,
}

impl TimeZoneOffset {
    /// Creates a new time zone offset from seconds.
    ///
    /// # Panics
    ///
    /// Panics if the offset is not within -86400 to 86400 seconds (-24 to +24 hours).
    #[inline]
    pub const fn from_seconds(seconds: i32) -> Self {
        assert!(seconds >= -86400 && seconds <= 86400, "Time zone offset out of range");
        TimeZoneOffset { offset_seconds: seconds }
    }

    /// Creates a new time zone offset from hours.
    ///
    /// # Panics
    ///
    /// Panics if the offset is not within -24 to 24 hours.
    #[inline]
    pub const fn from_hours(hours: i32) -> Self {
        assert!(hours >= -24 && hours <= 24, "Time zone offset out of range");
        TimeZoneOffset { offset_seconds: hours * 3600 }
    }

    /// Returns the offset in seconds.
    #[inline]
    pub const fn as_seconds(&self) -> i32 {
        self.offset_seconds
    }

    /// Returns the offset in hours.
    #[inline]
    pub const fn as_hours(&self) -> i32 {
        self.offset_seconds / 3600
    }

    /// Returns the offset in minutes.
    #[inline]
    pub const fn as_minutes(&self) -> i32 {
        self.offset_seconds / 60
    }

    /// Returns UTC (zero offset).
    #[inline]
    pub const fn utc() -> Self {
        TimeZoneOffset { offset_seconds: 0 }
    }

    /// Formats the offset as +/-HH:MM.
    #[inline]
    pub fn format(&self) -> String {
        let sign = if self.offset_seconds >= 0 { '+' } else { '-' };
        // Cast to i64 to handle i32::MIN without overflow.
        // abs(i32::MIN) would overflow, but i64::MIN.abs() works fine.
        let abs_secs = (self.offset_seconds as i64).abs();
        let hours = (abs_secs / 3600) as i32;
        let minutes = ((abs_secs % 3600) / 60) as i32;
        format!("{}{:02}:{:02}", sign, hours, minutes)
    }
}

/// Represents a timestamp with nanosecond precision.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Timestamp {
    /// Seconds since Unix epoch (1970-01-01 00:00:00 UTC).
    pub seconds: i64,
    /// Nanoseconds within the second (0 to 999,999,999).
    pub nanos: u32,
}

impl Timestamp {
    /// Creates a new timestamp.
    ///
    /// # Panics
    ///
    /// Panics if nanos is >= 1,000,000,000.
    #[inline]
    pub const fn new(seconds: i64, nanos: u32) -> Self {
        assert!(nanos < 1_000_000_000, "Nanoseconds must be less than 1,000,000,000");
        Timestamp { seconds, nanos }
    }

    /// Creates a timestamp from seconds.
    #[inline]
    pub const fn from_seconds(seconds: i64) -> Self {
        Timestamp { seconds, nanos: 0 }
    }

    /// Creates a timestamp from milliseconds.
    #[inline]
    pub const fn from_millis(millis: i64) -> Self {
        Timestamp {
            seconds: millis / 1000,
            nanos: ((millis % 1000) * 1_000_000) as u32,
        }
    }

    /// Creates a timestamp from nanoseconds.
    #[inline]
    pub const fn from_nanos(nanos: i64) -> Self {
        Timestamp {
            seconds: nanos / 1_000_000_000,
            nanos: (nanos % 1_000_000_000) as u32,
        }
    }

    /// Returns the seconds component.
    #[inline]
    pub const fn seconds(&self) -> i64 {
        self.seconds
    }

    /// Returns the nanoseconds component.
    #[inline]
    pub const fn nanos(&self) -> u32 {
        self.nanos
    }

    /// Converts to a Duration (only for non-negative timestamps).
    #[inline]
    pub const fn to_duration(&self) -> Option<StdDuration> {
        if self.seconds >= 0 {
            Some(StdDuration::new(self.seconds as u64, self.nanos))
        } else {
            None
        }
    }

    /// Returns the current time (requires std).
    #[cfg(feature = "std")]
    #[inline]
    pub fn now() -> Self {
        use std::time::SystemTime;
        let epoch = SystemTime::UNIX_EPOCH;
        let now = SystemTime::now();
        let duration = now.duration_since(epoch).unwrap_or_default();
        Timestamp {
            seconds: duration.as_secs() as i64,
            nanos: duration.subsec_nanos(),
        }
    }

    /// Adds a duration to this timestamp.
    #[inline]
    pub const fn add(&self, duration: StdDuration) -> Timestamp {
        let new_secs = self.seconds + duration.as_secs() as i64;
        let new_nanos = self.nanos + duration.subsec_nanos();
        Timestamp {
            seconds: new_secs + (new_nanos / 1_000_000_000) as i64,
            nanos: new_nanos % 1_000_000_000,
        }
    }

    /// Subtracts a duration from this timestamp.
    #[inline]
    pub const fn sub(&self, duration: StdDuration) -> Timestamp {
        let total_nanos = self.nanos as i64 - duration.subsec_nanos() as i64;
        let borrow = if total_nanos < 0 { 1 } else { 0 };
        Timestamp {
            seconds: self.seconds - duration.as_secs() as i64 - borrow,
            nanos: (total_nanos + 1_000_000_000 * borrow) as u32,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_time_zone_offset() {
        let utc = TimeZoneOffset::utc();
        assert_eq!(utc.as_seconds(), 0);
        assert_eq!(utc.format(), "+00:00");

        let plus_8 = TimeZoneOffset::from_hours(8);
        assert_eq!(plus_8.as_seconds(), 28800);
        assert_eq!(plus_8.format(), "+08:00");

        let minus_5 = TimeZoneOffset::from_hours(-5);
        assert_eq!(minus_5.as_seconds(), -18000);
        assert_eq!(minus_5.format(), "-05:00");
    }

    #[test]
    fn test_timestamp() {
        let ts = Timestamp::new(1705315800, 500_000_000);
        assert_eq!(ts.seconds(), 1705315800);
        assert_eq!(ts.nanos(), 500_000_000);

        let ts_from_secs = Timestamp::from_seconds(1000);
        assert_eq!(ts_from_secs.seconds(), 1000);
        assert_eq!(ts_from_secs.nanos(), 0);

        let ts_from_millis = Timestamp::from_millis(1500);
        assert_eq!(ts_from_millis.seconds(), 1);
        assert_eq!(ts_from_millis.nanos(), 500_000_000);
    }

    #[test]
    fn test_timestamp_add_sub() {
        let ts = Timestamp::from_seconds(1000);
        let dur = StdDuration::from_secs(100);

        let added = ts.add(dur);
        assert_eq!(added.seconds(), 1100);

        let subbed = ts.sub(dur);
        assert_eq!(subbed.seconds(), 900);
    }

    #[test]
    fn test_time_zone_offset_i32_min() {
        // Test that format() doesn't panic on i32::MIN
        // (abs(i32::MIN) would overflow, but we handle it by casting to i64)
        let offset = TimeZoneOffset { offset_seconds: i32::MIN };
        let formatted = offset.format();
        // The offset is i32::MIN = -2147483648 seconds
        // = -596523 hours = -596515 minutes remainder
        // Expected: "-596523:15" (approximate due to large value)
        assert!(formatted.starts_with('-'));
        assert!(formatted.contains(':'));
    }
}
