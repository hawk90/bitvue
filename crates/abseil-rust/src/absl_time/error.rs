//! Error types for time operations

use core::time::Duration as StdDuration;

/// Errors that can occur during time operations.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TimeError {
    /// The time string is malformed or invalid.
    InvalidFormat(String),
    /// The time value is out of range.
    OutOfRange,
    /// The time zone is invalid or unknown.
    InvalidTimeZone,
    /// The duration cannot be converted to the target unit.
    DurationConversionFailed,
}

impl core::fmt::Display for TimeError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            TimeError::InvalidFormat(msg) => write!(f, "Invalid time format: {}", msg),
            TimeError::OutOfRange => write!(f, "Time value is out of range"),
            TimeError::InvalidTimeZone => write!(f, "Invalid or unknown time zone"),
            TimeError::DurationConversionFailed => write!(f, "Failed to convert duration"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for TimeError {}
