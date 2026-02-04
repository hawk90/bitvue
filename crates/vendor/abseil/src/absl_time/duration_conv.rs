//! Duration conversion utilities

use core::time::Duration as StdDuration;

/// Converts a duration to seconds (truncates).
#[inline]
pub const fn duration_to_seconds(dur: StdDuration) -> u64 {
    dur.as_secs()
}

/// Converts a duration to milliseconds.
#[inline]
pub const fn duration_to_millis(dur: StdDuration) -> u128 {
    dur.as_secs() as u128 * 1000 + dur.subsec_millis() as u128
}

/// Converts a duration to microseconds.
#[inline]
pub const fn duration_to_micros(dur: StdDuration) -> u128 {
    dur.as_secs() as u128 * 1_000_000 + dur.subsec_micros() as u128
}

/// Converts a duration to nanoseconds.
#[inline]
pub const fn duration_to_nanos(dur: StdDuration) -> u128 {
    dur.as_secs() as u128 * 1_000_000_000 + dur.subsec_nanos() as u128
}

/// Creates a duration from seconds.
#[inline]
pub const fn duration_from_seconds(secs: u64) -> StdDuration {
    StdDuration::new(secs, 0)
}

/// Creates a duration from milliseconds.
#[inline]
pub const fn duration_from_millis(millis: u128) -> StdDuration {
    StdDuration::new((millis / 1000) as u64, ((millis % 1000) * 1_000_000) as u32)
}

/// Creates a duration from nanoseconds.
#[inline]
pub const fn duration_from_nanos(nanos: u128) -> StdDuration {
    StdDuration::new(
        (nanos / 1_000_000_000) as u64,
        (nanos % 1_000_000_000) as u32,
    )
}
