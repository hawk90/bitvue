//! Error types for flag parsing

/// Error type for flag parsing failures.
///
/// This error is returned when flag parsing encounters invalid input.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FlagsError {
    /// Unknown flag encountered.
    UnknownFlag(String),
    /// Missing value for a flag that requires one.
    MissingValue(String),
    /// Invalid value for a flag.
    InvalidValue {
        /// The flag that received the invalid value.
        flag: String,
        /// The invalid value that was provided.
        value: String,
        /// Expected format description.
        expected: String,
    },
    /// Flag specified multiple times.
    DuplicateFlag(String),
    /// Conflicting flags specified.
    ConflictingFlags {
        /// The first flag.
        flag1: String,
        /// The conflicting second flag.
        flag2: String,
    },
}

impl core::fmt::Display for FlagsError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            FlagsError::UnknownFlag(flag) => write!(f, "Unknown flag: {}", flag),
            FlagsError::MissingValue(flag) => write!(f, "Missing value for flag: {}", flag),
            FlagsError::InvalidValue { flag, value, expected } => {
                write!(f, "Invalid value '{}' for flag {}: {}", value, flag, expected)
            }
            FlagsError::DuplicateFlag(flag) => write!(f, "Flag specified multiple times: {}", flag),
            FlagsError::ConflictingFlags { flag1, flag2 } => {
                write!(f, "Conflicting flags: {} and {} cannot be used together", flag1, flag2)
            }
        }
    }
}

impl core::error::Error for FlagsError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_flags_error_display() {
        let err = FlagsError::UnknownFlag("--unknown".to_string());
        assert!(format!("{}", err).contains("Unknown flag"));
    }
}
