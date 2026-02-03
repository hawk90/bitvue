//! Flag validation utilities - validate_flag_range, validate_non_empty, validate_file_extension

use super::error::FlagsError;

/// Validate that flag values meet specified constraints.
///
/// This function performs validation on flag values to ensure they
/// meet specified requirements (e.g., minimum value, maximum value).
///
/// # Examples
///
/// ```rust
/// use abseil::absl_flags::{validate_flag_range, FlagsError};
///
/// // Validate a numeric flag is in range [0, 100]
/// let result = validate_flag_range("count", 50, 0, 100);
/// assert!(result.is_ok());
///
/// let result = validate_flag_range("count", 150, 0, 100);
/// assert!(result.is_err());
/// ```
pub fn validate_flag_range(
    flag: &str,
    value: i64,
    min: i64,
    max: i64,
) -> Result<(), FlagsError> {
    if value < min {
        return Err(FlagsError::InvalidValue {
            flag: flag.to_string(),
            value: value.to_string(),
            expected: format!("value must be at least {}", min),
        });
    }
    if value > max {
        return Err(FlagsError::InvalidValue {
            flag: flag.to_string(),
            value: value.to_string(),
            expected: format!("value must be at most {}", max),
        });
    }
    Ok(())
}

/// Validate that a flag value is not empty.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_flags::{validate_non_empty, FlagsError};
///
/// let result = validate_non_empty("output", "file.txt");
/// assert!(result.is_ok());
///
/// let result = validate_non_empty("output", "");
/// assert!(result.is_err());
/// ```
pub fn validate_non_empty(flag: &str, value: &str) -> Result<(), FlagsError> {
    if value.is_empty() {
        return Err(FlagsError::InvalidValue {
            flag: flag.to_string(),
            value: "".to_string(),
            expected: "value must not be empty".to_string(),
        });
    }
    Ok(())
}

/// Validate that a file path has a specific extension.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_flags::{validate_file_extension, FlagsError};
///
/// let result = validate_file_extension("input", "data.txt", ".txt");
/// assert!(result.is_ok());
///
/// let result = validate_file_extension("input", "data.csv", ".txt");
/// assert!(result.is_err());
/// ```
pub fn validate_file_extension(
    flag: &str,
    path: &str,
    extension: &str,
) -> Result<(), FlagsError> {
    if !path.ends_with(extension) {
        return Err(FlagsError::InvalidValue {
            flag: flag.to_string(),
            value: path.to_string(),
            expected: format!("file must have {} extension", extension),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_flag_range() {
        assert!(validate_flag_range("count", 50, 0, 100).is_ok());
        assert!(validate_flag_range("count", 0, 0, 100).is_ok());
        assert!(validate_flag_range("count", 100, 0, 100).is_ok());

        assert!(validate_flag_range("count", -1, 0, 100).is_err());
        assert!(validate_flag_range("count", 101, 0, 100).is_err());
    }

    #[test]
    fn test_validate_non_empty() {
        assert!(validate_non_empty("output", "file.txt").is_ok());
        assert!(validate_non_empty("output", "").is_err());
    }

    #[test]
    fn test_validate_file_extension() {
        assert!(validate_file_extension("input", "data.txt", ".txt").is_ok());
        assert!(validate_file_extension("input", "data.csv", ".txt").is_err());
    }
}
