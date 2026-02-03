//! Time utilities.
//!
//! This module provides time utilities similar to Abseil's `absl/time` directory.
//! Rust's standard library already provides many time utilities through `core::time`
//! and `std::time`, but this module provides additional compatibility helpers
//! and Abseil-specific utilities.
//!
//! # Overview
//!
//! Time utilities provide common time operations and helper functions that enhance
//! Rust's built-in time system. These include:
//!
//! - Time conversion utilities
//! - Time formatting and parsing
//! - Time validation functions
//! - Time zone abstraction
//! - Duration formatting helpers
//!
//! # Modules
//!
//! - [`civil_time`] - Civil time (date/time) for calendar operations
//! - [`duration`] - Duration for representing time spans
//!
//! # Examples
//!
//! ```rust
//! use abseil::absl_time::{format_duration, parse_iso8601};
//!
//! // Format a duration
//! let duration = core::time::Duration::from_secs(3661);
//! assert_eq!(format_duration(duration), "1h 1m 1s");
//!
//! // Parse an ISO 8601 timestamp
//! let timestamp = "2024-01-15T10:30:00Z";
//! let parsed = parse_iso8601(timestamp)?;
//! ```

pub mod civil_time;
pub mod duration;

// Error types
pub mod error;

// Timestamp and timezone utilities
pub mod timestamp;

// Duration conversion utilities
pub mod duration_conv;

// ISO 8601 parsing and formatting
pub mod iso8601;

// Date and time validation
pub mod validation;

// Time interval utilities
pub mod instrument;

// Clock and format utilities
pub mod utilities;

// Re-exports from civil_time module
pub use civil_time::{CivilDay, CivilHour, CivilMinute, CivilSecond, CivilYear, CivilMonth};

// Re-exports from duration module
pub use duration::Duration;

// Re-exports from error module
pub use error::TimeError;

// Re-exports from timestamp module
pub use timestamp::{TimeZoneOffset, Timestamp};

// Re-exports from duration_conv module
pub use duration_conv::{
    duration_from_millis, duration_from_nanos, duration_from_seconds, duration_to_micros,
    duration_to_millis, duration_to_nanos, duration_to_seconds,
};

// Re-exports from iso8601 module
pub use iso8601::{format_duration, format_iso8601, parse_iso8601};

// Re-exports from validation module
pub use validation::{
    days_in_month, is_leap_year, to_12_hour, to_24_hour, validate_date, validate_time,
};

// Re-exports from instrument module
pub use instrument::{
    Deadline, FormatOptions, Interval, MockClock, Stopwatch,
};

// Re-exports from utilities module
pub use utilities::{
    clamp_timestamp, compare_timestamps, format_with_options, is_within, max_timestamp,
    min_timestamp, parse_unix_timestamp, parse_unix_timestamp_millis, round_to_day,
    round_to_hour, round_to_minute, round_to_second, timestamp_diff, to_unix_timestamp_string,
    Clock, SystemClock,
};
