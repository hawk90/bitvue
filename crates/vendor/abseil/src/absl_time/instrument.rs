//! Time interval utilities - Interval, Stopwatch, Deadline

use core::time::Duration as StdDuration;
use super::timestamp::Timestamp;

/// A time interval representing a span between two timestamps.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Interval {
    pub start: Timestamp,
    pub end: Timestamp,
}

impl Interval {
    /// Creates a new interval.
    #[inline]
    pub const fn new(start: Timestamp, end: Timestamp) -> Self {
        Self { start, end }
    }

    /// Returns the duration of this interval.
    #[inline]
    pub fn duration(&self) -> StdDuration {
        let diff_secs = self.end.seconds - self.start.seconds;
        let diff_nanos = self.end.nanos as i64 - self.start.nanos as i64;
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

    /// Checks if this interval contains a timestamp.
    #[inline]
    pub const fn contains(&self, ts: Timestamp) -> bool {
        ts.seconds >= self.start.seconds
            && ts.seconds <= self.end.seconds
    }

    /// Checks if this interval overlaps with another.
    #[inline]
    pub const fn overlaps(&self, other: &Interval) -> bool {
        self.start.seconds <= other.end.seconds
            && self.end.seconds >= other.start.seconds
    }
}

/// A stopwatch for measuring elapsed time.
#[derive(Clone, Copy, Debug)]
pub struct Stopwatch {
    start: Option<Timestamp>,
    elapsed: StdDuration,
}

impl Stopwatch {
    /// Creates a new stopwatch.
    #[inline]
    pub const fn new() -> Self {
        Self {
            start: None,
            elapsed: StdDuration::ZERO,
        }
    }

    /// Starts the stopwatch.
    #[cfg(feature = "std")]
    #[inline]
    pub fn start(&mut self) {
        self.start = Some(Timestamp::now());
    }

    /// Stops the stopwatch and returns the elapsed time.
    #[cfg(feature = "std")]
    #[inline]
    pub fn stop(&mut self) -> StdDuration {
        if let Some(start) = self.start {
            let now = Timestamp::now();
            let elapsed = now.to_duration().unwrap_or_default();
            self.elapsed = elapsed;
            self.start = None;
            self.elapsed
        } else {
            self.elapsed
        }
    }

    /// Resets the stopwatch.
    #[inline]
    pub fn reset(&mut self) {
        self.start = None;
        self.elapsed = StdDuration::ZERO;
    }

    /// Gets the elapsed time without stopping.
    #[cfg(feature = "std")]
    #[inline]
    pub fn elapsed(&self) -> StdDuration {
        if let Some(start) = self.start {
            let now = Timestamp::now();
            let dur = now.to_duration().unwrap_or_default();
            dur
        } else {
            self.elapsed
        }
    }

    /// Restarts the stopwatch.
    #[cfg(feature = "std")]
    #[inline]
    pub fn restart(&mut self) {
        self.elapsed = StdDuration::ZERO;
        self.start = Some(Timestamp::now());
    }
}

impl Default for Stopwatch {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

/// A deadline that can expire.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Deadline {
    timestamp: Timestamp,
}

impl Deadline {
    /// Creates a new deadline from a timestamp.
    #[inline]
    pub const fn new(ts: Timestamp) -> Self {
        Self { timestamp: ts }
    }

    /// Creates a deadline from a duration in the future.
    #[cfg(feature = "std")]
    #[inline]
    pub fn from_duration(duration: StdDuration) -> Self {
        let now = Timestamp::now();
        Self {
            timestamp: now.add(duration),
        }
    }

    /// Returns the deadline timestamp.
    #[inline]
    pub const fn timestamp(&self) -> Timestamp {
        self.timestamp
    }

    /// Checks if the deadline has passed.
    #[cfg(feature = "std")]
    #[inline]
    pub fn has_passed(&self) -> bool {
        Timestamp::now().seconds >= self.timestamp.seconds
    }

    /// Returns the time remaining until the deadline.
    #[cfg(feature = "std")]
    #[inline]
    pub fn remaining(&self) -> Option<StdDuration> {
        let now = Timestamp::now();
        if now.seconds < self.timestamp.seconds {
            Some(
                self.timestamp.to_duration().unwrap_or_default()
                    - now.to_duration().unwrap_or_default(),
            )
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_interval() {
        let start = Timestamp::from_seconds(1000);
        let end = Timestamp::from_seconds(2000);
        let interval = Interval::new(start, end);

        assert_eq!(interval.duration().as_secs(), 1000);
        assert!(interval.contains(Timestamp::from_seconds(1500)));
        assert!(!interval.contains(Timestamp::from_seconds(500)));
    }

    #[test]
    fn test_interval_overlaps() {
        let a = Interval::new(Timestamp::from_seconds(1000), Timestamp::from_seconds(2000));
        let b = Interval::new(Timestamp::from_seconds(1500), Timestamp::from_seconds(2500));
        let c = Interval::new(Timestamp::from_seconds(3000), Timestamp::from_seconds(4000));

        assert!(a.overlaps(&b));
        assert!(!a.overlaps(&c));
    }

    #[test]
    fn test_stopwatch() {
        let sw = Stopwatch::new();
        assert_eq!(sw.elapsed(), StdDuration::ZERO);

        sw.reset();
        assert_eq!(sw.elapsed(), StdDuration::ZERO);
    }

    #[test]
    fn test_stopwatch_default() {
        let sw = Stopwatch::default();
        assert_eq!(sw.elapsed(), StdDuration::ZERO);
    }

    #[test]
    fn test_deadline() {
        let ts = Timestamp::from_seconds(1000);
        let deadline = Deadline::new(ts);

        assert_eq!(deadline.timestamp(), ts);
        assert_eq!(deadline.timestamp().seconds(), 1000);
    }
}
