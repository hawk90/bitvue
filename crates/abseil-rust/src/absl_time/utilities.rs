//! Clock trait and format options - Clock, SystemClock, MockClock, FormatOptions, format_with_options

use alloc::string::String;
use alloc::format;
use core::time::Duration as StdDuration;
use super::timestamp::Timestamp;
use super::instrument::{Deadline, Stopwatch, Interval};

/// A clock that provides the current time.
pub trait Clock {
    /// Returns the current time.
    fn now(&self) -> Timestamp;

    /// Returns the current time as a Unix timestamp (seconds).
    fn now_seconds(&self) -> i64 {
        self.now().seconds()
    }
}

/// System clock using the actual system time.
#[derive(Clone, Copy, Debug, Default)]
#[cfg(feature = "std")]
pub struct SystemClock;

#[cfg(feature = "std")]
impl Clock for SystemClock {
    #[inline]
    fn now(&self) -> Timestamp {
        Timestamp::now()
    }
}

/// A mock clock for testing with deterministic time.
#[derive(Clone, Debug)]
pub struct MockClock {
    current: Timestamp,
}

impl MockClock {
    /// Creates a new mock clock starting at the given timestamp.
    #[inline]
    pub const fn new(start: Timestamp) -> Self {
        Self { current: start }
    }

    /// Advances the clock by a duration.
    #[inline]
    pub fn advance(&mut self, duration: StdDuration) {
        self.current = self.current.add(duration);
    }

    /// Sets the clock to a specific timestamp.
    #[inline]
    pub fn set_time(&mut self, ts: Timestamp) {
        self.current = ts;
    }
}

impl Default for MockClock {
    #[inline]
    fn default() -> Self {
        Self::new(Timestamp::from_seconds(0))
    }
}

impl Clock for MockClock {
    #[inline]
    fn now(&self) -> Timestamp {
        self.current
    }
}

/// Format options for time formatting.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FormatOptions {
    /// Include date component.
    pub include_date: bool,
    /// Include time component.
    pub include_time: bool,
    /// Include nanoseconds.
    pub include_nanos: bool,
    /// Use 12-hour format (if false, uses 24-hour).
    pub use_12_hour: bool,
    /// Include timezone offset.
    pub include_tz: bool,
}

impl FormatOptions {
    /// Creates a new format options with default values.
    #[inline]
    pub const fn new() -> Self {
        Self {
            include_date: true,
            include_time: true,
            include_nanos: false,
            use_12_hour: false,
            include_tz: true,
        }
    }

    /// Sets whether to include the date.
    #[inline]
    pub const fn date(mut self, include: bool) -> Self {
        self.include_date = include;
        self
    }

    /// Sets whether to include the time.
    #[inline]
    pub const fn time(mut self, include: bool) -> Self {
        self.include_time = include;
        self
    }

    /// Sets whether to include nanoseconds.
    #[inline]
    pub const fn nanos(mut self, include: bool) -> Self {
        self.include_nanos = include;
        self
    }

    /// Sets whether to use 12-hour format.
    #[inline]
    pub const fn format_12_hour(mut self, use_12: bool) -> Self {
        self.use_12_hour = use_12;
        self
    }

    /// Sets whether to include timezone.
    #[inline]
    pub const fn timezone(mut self, include: bool) -> Self {
        self.include_tz = include;
        self
    }

    /// Creates options for date only.
    #[inline]
    pub const fn date_only() -> Self {
        Self {
            include_date: true,
            include_time: false,
            include_nanos: false,
            use_12_hour: false,
            include_tz: false,
        }
    }

    /// Creates options for time only.
    #[inline]
    pub const fn time_only() -> Self {
        Self {
            include_date: false,
            include_time: true,
            include_nanos: false,
            use_12_hour: false,
            include_tz: false,
        }
    }
}

/// Formats a timestamp with custom options.
pub fn format_with_options(ts: Timestamp, options: FormatOptions) -> String {
    let mut result = String::new();

    let total_days = ts.seconds / 86400;
    let year = 1970 + (total_days / 365) as i64;
    let month = 1;
    let day = (total_days % 365) as u32 + 1;

    let remaining_secs = ts.seconds % 86400;
    let hour = (remaining_secs / 3600) as u32;
    let minute = ((remaining_secs % 3600) / 60) as u32;
    let second = (remaining_secs % 60) as u32;

    if options.include_date {
        result.push_str(&format!("{:04}-{:02}-{:02}", year, month, day));
    }

    if options.include_time {
        if options.include_date {
            result.push(' ');
        }

        let (display_hour, is_pm) = if options.use_12_hour {
            let (h, _, pm) = to_12_hour(hour, minute);
            (h, pm)
        } else {
            (hour, false)
        };

        result.push_str(&format!("{:02}:{:02}:{:02}", display_hour, minute, second));

        if options.use_12_hour {
            result.push_str(if is_pm { " PM" } else { " AM" });
        }

        if options.include_nanos && ts.nanos > 0 {
            result.push_str(&format!(".{:09}", ts.nanos));
        }
    }

    if options.include_tz {
        result.push_str(" UTC");
    }

    result
}

/// Parses a Unix timestamp (seconds since epoch).
#[inline]
pub const fn parse_unix_timestamp(seconds: i64) -> Timestamp {
    Timestamp::from_seconds(seconds)
}

/// Parses a Unix timestamp in milliseconds.
#[inline]
pub const fn parse_unix_timestamp_millis(millis: i64) -> Timestamp {
    Timestamp::from_millis(millis)
}

/// Converts a timestamp to a Unix timestamp string.
#[inline]
pub fn to_unix_timestamp_string(ts: Timestamp) -> String {
    format!("{}", ts.seconds)
}

/// Compares two timestamps.
#[inline]
pub const fn compare_timestamps(a: Timestamp, b: Timestamp) -> core::cmp::Ordering {
    if a.seconds < b.seconds {
        core::cmp::Ordering::Less
    } else if a.seconds > b.seconds {
        core::cmp::Ordering::Greater
    } else if a.nanos < b.nanos {
        core::cmp::Ordering::Less
    } else if a.nanos > b.nanos {
        core::cmp::Ordering::Greater
    } else {
        core::cmp::Ordering::Equal
    }
}

/// Returns the earlier of two timestamps.
#[inline]
pub const fn min_timestamp(a: Timestamp, b: Timestamp) -> Timestamp {
    if compare_timestamps(a, b) == core::cmp::Ordering::Less {
        a
    } else {
        b
    }
}

/// Returns the later of two timestamps.
#[inline]
pub const fn max_timestamp(a: Timestamp, b: Timestamp) -> Timestamp {
    if compare_timestamps(a, b) == core::cmp::Ordering::Greater {
        a
    } else {
        b
    }
}

/// Clamps a timestamp between a minimum and maximum.
#[inline]
pub const fn clamp_timestamp(ts: Timestamp, min: Timestamp, max: Timestamp) -> Timestamp {
    min_timestamp(max_timestamp(ts, min), max)
}

/// Calculates the difference between two timestamps.
#[inline]
pub const fn timestamp_diff(a: Timestamp, b: Timestamp) -> StdDuration {
    let diff_secs = a.seconds - b.seconds;
    let diff_nanos = a.nanos as i64 - b.nanos as i64;
    let total_nanos = diff_secs * 1_000_000_000 + diff_nanos;

    if total_nanos > 0 {
        StdDuration::new(
            (total_nanos / 1_000_000_000) as u64,
            (total_nanos % 1_000_000_000) as u32,
        )
    } else {
        StdDuration::ZERO
    }
}

/// Checks if a timestamp is within a duration of now.
#[cfg(feature = "std")]
#[inline]
pub fn is_within(ts: Timestamp, duration: StdDuration) -> bool {
    let now = Timestamp::now();
    let diff = timestamp_diff(now, ts);
    diff <= duration
}

/// Rounds a timestamp down to the nearest second.
#[inline]
pub const fn round_to_second(ts: Timestamp) -> Timestamp {
    Timestamp::from_seconds(ts.seconds)
}

/// Rounds a timestamp down to the nearest minute.
#[inline]
pub const fn round_to_minute(ts: Timestamp) -> Timestamp {
    Timestamp::from_seconds((ts.seconds / 60) * 60)
}

/// Rounds a timestamp down to the nearest hour.
#[inline]
pub const fn round_to_hour(ts: Timestamp) -> Timestamp {
    Timestamp::from_seconds((ts.seconds / 3600) * 3600)
}

/// Rounds a timestamp down to the nearest day.
#[inline]
pub const fn round_to_day(ts: Timestamp) -> Timestamp {
    Timestamp::from_seconds((ts.seconds / 86400) * 86400)
}

// Re-export the 12-hour conversion function for use in format_with_options
const fn to_12_hour(hour: u32, minute: u32) -> (u32, u32, bool) {
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
    fn test_mock_clock() {
        let mut clock = MockClock::new(Timestamp::from_seconds(1000));
        assert_eq!(clock.now().seconds(), 1000);

        clock.advance(StdDuration::from_secs(100));
        assert_eq!(clock.now().seconds(), 1100);

        clock.set_time(Timestamp::from_seconds(2000));
        assert_eq!(clock.now().seconds(), 2000);
    }

    #[test]
    fn test_mock_clock_default() {
        let clock = MockClock::default();
        assert_eq!(clock.now().seconds(), 0);
    }

    #[test]
    fn test_format_options() {
        let ts = Timestamp::new(1705315800, 500_000_000);

        let date_only = format_with_options(ts, FormatOptions::date_only());
        assert!(date_only.contains("2024"));

        let time_only = format_with_options(ts, FormatOptions::time_only());
        assert!(!time_only.contains("2024"));
        assert!(time_only.contains(":"));
    }

    #[test]
    fn test_format_options_builder() {
        let opts = FormatOptions::new()
            .date(true)
            .time(true)
            .nanos(false)
            .format_12_hour(false)
            .timezone(true);

        assert!(opts.include_date);
        assert!(opts.include_time);
        assert!(!opts.include_nanos);
        assert!(!opts.use_12_hour);
        assert!(opts.include_tz);
    }

    #[test]
    fn test_parse_unix_timestamp() {
        let ts = parse_unix_timestamp(1705315800);
        assert_eq!(ts.seconds(), 1705315800);

        let ts_millis = parse_unix_timestamp_millis(1705315800500);
        assert_eq!(ts_millis.seconds(), 1705315800);
        assert_eq!(ts_millis.nanos(), 500_000_000);
    }

    #[test]
    fn test_to_unix_timestamp_string() {
        let ts = Timestamp::from_seconds(1705315800);
        assert_eq!(to_unix_timestamp_string(ts), "1705315800");
    }

    #[test]
    fn test_compare_timestamps() {
        let a = Timestamp::from_seconds(1000);
        let b = Timestamp::from_seconds(2000);
        let c = Timestamp::from_seconds(1000);

        assert_eq!(compare_timestamps(a, b), core::cmp::Ordering::Less);
        assert_eq!(compare_timestamps(b, a), core::cmp::Ordering::Greater);
        assert_eq!(compare_timestamps(a, c), core::cmp::Ordering::Equal);
    }

    #[test]
    fn test_min_max_timestamp() {
        let a = Timestamp::from_seconds(1000);
        let b = Timestamp::from_seconds(2000);

        assert_eq!(min_timestamp(a, b), a);
        assert_eq!(max_timestamp(a, b), b);
    }

    #[test]
    fn test_clamp_timestamp() {
        let min = Timestamp::from_seconds(1000);
        let max = Timestamp::from_seconds(2000);

        assert_eq!(clamp_timestamp(Timestamp::from_seconds(500), min, max), min);
        assert_eq!(clamp_timestamp(Timestamp::from_seconds(1500), min, max), Timestamp::from_seconds(1500));
        assert_eq!(clamp_timestamp(Timestamp::from_seconds(2500), min, max), max);
    }

    #[test]
    fn test_timestamp_diff() {
        let a = Timestamp::from_seconds(2000);
        let b = Timestamp::from_seconds(1000);
        let diff = timestamp_diff(a, b);

        assert_eq!(diff.as_secs(), 1000);
    }

    #[test]
    fn test_round_to_second() {
        let ts = Timestamp::new(1000, 500_000_000);
        let rounded = round_to_second(ts);
        assert_eq!(rounded.seconds(), 1000);
        assert_eq!(rounded.nanos(), 0);
    }

    #[test]
    fn test_round_to_minute() {
        let ts = Timestamp::new(3661, 0);
        let rounded = round_to_minute(ts);
        assert_eq!(rounded.seconds(), 3600);
    }

    #[test]
    fn test_round_to_hour() {
        let ts = Timestamp::new(7200, 0);
        let rounded = round_to_hour(ts);
        assert_eq!(rounded.seconds(), 7200);

        let ts2 = Timestamp::new(7300, 0);
        let rounded2 = round_to_hour(ts2);
        assert_eq!(rounded2.seconds(), 7200);
    }

    #[test]
    fn test_round_to_day() {
        let ts = Timestamp::new(100000, 0);
        let rounded = round_to_day(ts);
        assert_eq!(rounded.seconds(), 86400);
    }
}
