//! String to number conversion utilities.
//!
//! Provides functions for converting strings to numbers with error handling.

use core::fmt;

/// Maximum allowed string length for number parsing to prevent DoS attacks.
/// This limits the number of digits that can be parsed, preventing
/// excessive CPU usage on extremely long strings.
pub const MAX_PARSE_LENGTH: usize = 1024;

/// Error type for number parsing failures.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ParseError {
    /// The string was empty.
    Empty,
    /// The string contained invalid characters.
    Invalid,
    /// The number was out of range for the target type.
    OutOfRange,
    /// The number had an invalid format.
    InvalidFormat,
    /// The string was too long to parse safely.
    TooLong,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParseError::Empty => write!(f, "empty string"),
            ParseError::Invalid => write!(f, "invalid character"),
            ParseError::OutOfRange => write!(f, "out of range"),
            ParseError::InvalidFormat => write!(f, "invalid format"),
            ParseError::TooLong => write!(f, "string too long (max {} characters)", MAX_PARSE_LENGTH),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for ParseError {}

/// Parses a string as an i8.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_strings::numbers::parse_i8;
///
/// assert_eq!(parse_i8("42"), Ok(42));
/// assert_eq!(parse_i8("-128"), Ok(-128));
/// assert!(parse_i8("129").is_err()); // Out of range
/// ```
pub fn parse_i8(s: &str) -> Result<i8, ParseError> {
    parse_signed(s)
}

/// Parses a string as an i16.
pub fn parse_i16(s: &str) -> Result<i16, ParseError> {
    parse_signed(s)
}

/// Parses a string as an i32.
pub fn parse_i32(s: &str) -> Result<i32, ParseError> {
    parse_signed(s)
}

/// Parses a string as an i64.
pub fn parse_i64(s: &str) -> Result<i64, ParseError> {
    parse_signed(s)
}

/// Parses a string as an i128.
pub fn parse_i128(s: &str) -> Result<i128, ParseError> {
    parse_signed(s)
}

/// Parses a string as an isize.
pub fn parse_isize(s: &str) -> Result<isize, ParseError> {
    parse_signed(s)
}

/// Parses a string as a u8.
pub fn parse_u8(s: &str) -> Result<u8, ParseError> {
    parse_unsigned(s)
}

/// Parses a string as a u16.
pub fn parse_u16(s: &str) -> Result<u16, ParseError> {
    parse_unsigned(s)
}

/// Parses a string as a u32.
pub fn parse_u32(s: &str) -> Result<u32, ParseError> {
    parse_unsigned(s)
}

/// Parses a string as a u64.
pub fn parse_u64(s: &str) -> Result<u64, ParseError> {
    parse_unsigned(s)
}

/// Parses a string as a u128.
pub fn parse_u128(s: &str) -> Result<u128, ParseError> {
    parse_unsigned(s)
}

/// Parses a string as a usize.
pub fn parse_usize(s: &str) -> Result<usize, ParseError> {
    parse_unsigned(s)
}

/// Parses a string as a f32.
pub fn parse_f32(s: &str) -> Result<f32, ParseError> {
    parse_float(s)
}

/// Parses a string as a f64.
pub fn parse_f64(s: &str) -> Result<f64, ParseError> {
    parse_float(s)
}

/// Generic signed integer parsing function.
fn parse_signed<T: core::ops::Neg<Output = T> + TryFrom<i64>>(s: &str) -> Result<T, ParseError> {
    if s.is_empty() {
        return Err(ParseError::Empty);
    }

    // Check string length to prevent DoS attacks
    if s.len() > MAX_PARSE_LENGTH {
        return Err(ParseError::TooLong);
    }

    let bytes = s.as_bytes();
    let is_negative = bytes[0] == b'-';
    let is_positive = bytes[0] == b'+';
    let start = if is_negative || is_positive { 1 } else { 0 };

    if start >= bytes.len() {
        return Err(ParseError::InvalidFormat);
    }

    let mut result: i64 = 0;
    for &byte in &bytes[start..] {
        if !byte.is_ascii_digit() {
            return Err(ParseError::Invalid);
        }
        let digit = (byte - b'0') as i64;
        result = result.checked_mul(10).ok_or(ParseError::OutOfRange)?;
        result = result.checked_add(digit).ok_or(ParseError::OutOfRange)?;
    }

    if is_negative {
        result = result.checked_neg().ok_or(ParseError::OutOfRange)?;
    }

    T::try_from(result).map_err(|_| ParseError::OutOfRange)
}

/// Generic unsigned integer parsing function.
fn parse_unsigned<T: TryFrom<u64>>(s: &str) -> Result<T, ParseError> {
    if s.is_empty() {
        return Err(ParseError::Empty);
    }

    // Check string length to prevent DoS attacks
    if s.len() > MAX_PARSE_LENGTH {
        return Err(ParseError::TooLong);
    }

    let bytes = s.as_bytes();
    let start = if bytes[0] == b'+' { 1 } else { 0 };

    if start >= bytes.len() {
        return Err(ParseError::InvalidFormat);
    }

    let mut result: u64 = 0;
    for &byte in &bytes[start..] {
        if !byte.is_ascii_digit() {
            return Err(ParseError::Invalid);
        }
        let digit = (byte - b'0') as u64;
        result = result.checked_mul(10).ok_or(ParseError::OutOfRange)?;
        result = result.checked_add(digit).ok_or(ParseError::OutOfRange)?;
    }

    T::try_from(result).map_err(|_| ParseError::OutOfRange)
}

/// Generic floating-point parsing function.
fn parse_float<T: core::str::FromStr>(s: &str) -> Result<T, ParseError> {
    if s.is_empty() {
        return Err(ParseError::Empty);
    }

    // Check string length to prevent DoS attacks
    if s.len() > MAX_PARSE_LENGTH {
        return Err(ParseError::TooLong);
    }

    s.parse::<T>().map_err(|_| ParseError::Invalid)
}

/// Converts a number to a hexadecimal string.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_strings::numbers::to_hex;
///
/// assert_eq!(to_hex(255u64), "ff");
/// assert_eq!(to_hex(0x1234u64), "1234");
/// ```
pub fn to_hex<T: Into<u64>>(value: T) -> String {
    let v = value.into();
    format!("{:x}", v)
}

/// Converts a hexadecimal string to a number.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_strings::numbers::from_hex;
///
/// assert_eq!(from_hex::<u32>("ff"), Ok(255));
/// assert!(from_hex::<u32>("xyz").is_err());
/// ```
pub fn from_hex<T: TryFrom<u64>>(s: &str) -> Result<T, ParseError> {
    if s.is_empty() {
        return Err(ParseError::Empty);
    }

    let bytes = s.as_bytes();
    let start = if bytes.get(0).map_or(false, |&b| b == b'0' && bytes.get(1).map_or(false, |&b| b == b'x' || b == b'X')) {
        2
    } else {
        0
    };

    if start >= bytes.len() {
        return Err(ParseError::InvalidFormat);
    }

    let mut result: u64 = 0;
    for &byte in &bytes[start..] {
        let digit = if byte.is_ascii_digit() {
            (byte - b'0') as u64
        } else if byte.is_ascii_lowercase() {
            (byte - b'a' + 10) as u64
        } else if byte.is_ascii_uppercase() {
            (byte - b'A' + 10) as u64
        } else {
            return Err(ParseError::Invalid);
        };

        if digit >= 16 {
            return Err(ParseError::Invalid);
        }

        result = result.checked_mul(16).ok_or(ParseError::OutOfRange)?;
        result = result.checked_add(digit).ok_or(ParseError::OutOfRange)?;
    }

    T::try_from(result).map_err(|_| ParseError::OutOfRange)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_i8() {
        assert_eq!(parse_i8("42"), Ok(42));
        assert_eq!(parse_i8("-128"), Ok(-128));
        assert_eq!(parse_i8("127"), Ok(127));
        assert!(parse_i8("128").is_err()); // Out of range
        assert!(parse_i8("-129").is_err()); // Out of range
        assert!(parse_i8("").is_err()); // Empty
        assert!(parse_i8("abc").is_err()); // Invalid
    }

    #[test]
    fn test_parse_i32() {
        assert_eq!(parse_i32("123456"), Ok(123456));
        assert_eq!(parse_i32("-99999"), Ok(-99999));
        assert!(parse_i32("abc").is_err());
    }

    #[test]
    fn test_parse_u8() {
        assert_eq!(parse_u8("42"), Ok(42));
        assert_eq!(parse_u8("255"), Ok(255));
        assert!(parse_u8("256").is_err()); // Out of range
        assert!(parse_u8("-1").is_err()); // Invalid (unsigned)
    }

    #[test]
    fn test_parse_u32() {
        assert_eq!(parse_u32("123456"), Ok(123456));
        assert!(parse_u32("-1").is_err());
    }

    #[test]
    fn test_parse_f32() {
        assert_eq!(parse_f32("3.14"), Ok(3.14));
        assert_eq!(parse_f32("-2.5"), Ok(-2.5));
        assert!(parse_f32("abc").is_err());
    }

    #[test]
    fn test_parse_f64() {
        assert_eq!(parse_f64("3.14159"), Ok(3.14159));
        assert_eq!(parse_f64("-2.71828"), Ok(-2.71828));
        assert!(parse_f64("abc").is_err());
    }

    #[test]
    fn test_parse_with_sign() {
        assert_eq!(parse_i32("+42"), Ok(42));
        assert_eq!(parse_i32("-42"), Ok(-42));
    }

    #[test]
    fn test_to_hex() {
        assert_eq!(to_hex(255u8), "ff");
        assert_eq!(to_hex(0x1234u32), "1234");
        assert_eq!(to_hex(0u32), "0");
    }

    #[test]
    fn test_from_hex() {
        assert_eq!(from_hex::<u32>("ff"), Ok(255));
        assert_eq!(from_hex::<u32>("1234"), Ok(4660));
        assert_eq!(from_hex::<u32>("0xFF"), Ok(255));
        assert!(from_hex::<u32>("xyz").is_err());
    }

    #[test]
    fn test_parse_error_display() {
        assert_eq!(format!("{}", ParseError::Empty), "empty string");
        assert_eq!(format!("{}", ParseError::Invalid), "invalid character");
        assert_eq!(format!("{}", ParseError::OutOfRange), "out of range");
    }

    // Edge case tests

    #[test]
    fn test_parse_boundary_values() {
        // Maximum values for each type
        assert_eq!(parse_u8("255"), Ok(255u8));
        assert_eq!(parse_i8("127"), Ok(127i8));
        assert_eq!(parse_i8("-128"), Ok(-128i8));

        // Overflow cases
        assert!(parse_u8("256").is_err());
        assert!(parse_i8("128").is_err());
        assert!(parse_i8("-129").is_err());

        // Large unsigned values
        assert_eq!(parse_u64("18446744073709551615"), Ok(u64::MAX));
        assert!(parse_u64("18446744073709551616").is_err()); // Overflow
    }

    #[test]
    fn test_parse_zero() {
        assert_eq!(parse_i8("0"), Ok(0));
        assert_eq!(parse_i32("0"), Ok(0));
        assert_eq!(parse_i64("0"), Ok(0));
        assert_eq!(parse_u8("0"), Ok(0));
        assert_eq!(parse_u32("0"), Ok(0));
        assert_eq!(parse_u64("0"), Ok(0));
        assert_eq!(parse_f32("0"), Ok(0.0));
        assert_eq!(parse_f64("0"), Ok(0.0));
    }

    #[test]
    fn test_parse_single_digit() {
        assert_eq!(parse_i32("7"), Ok(7));
        assert_eq!(parse_i32("-3"), Ok(-3));
        assert_eq!(parse_i32("+5"), Ok(5));
    }

    #[test]
    fn test_parse_whitespace_handling() {
        // Note: Current implementation may or may not handle whitespace
        // These tests document the current behavior
        assert!(parse_i32(" 42").is_err()); // Leading space not allowed
        assert!(parse_i32("42 ").is_err()); // Trailing space not allowed
        assert!(parse_i32("4 2").is_err()); // Space in middle not allowed
    }

    #[test]
    fn test_parse_empty_sign_only() {
        // Sign without number
        assert!(parse_i32("+").is_err());
        assert!(parse_i32("-").is_err());
    }

    #[test]
    fn test_hex_boundary_values() {
        // Maximum hex values for each type
        assert_eq!(from_hex::<u8>("ff"), Ok(255u8));
        assert_eq!(from_hex::<u16>("ffff"), Ok(65535u16));
        assert_eq!(from_hex::<u32>("ffffffff"), Ok(u32::MAX));

        // Overflow cases
        assert!(from_hex::<u8>("100").is_err());
        assert!(from_hex::<u16>("10000").is_err());
    }

    #[test]
    fn test_hex_case_insensitive() {
        assert_eq!(from_hex::<u32>("FF"), Ok(255));
        assert_eq!(from_hex::<u32>("Ff"), Ok(255));
        assert_eq!(from_hex::<u32>("fF"), Ok(255));
        assert_eq!(from_hex::<u32>("ff"), Ok(255));
    }

    #[test]
    fn test_hex_prefix() {
        assert_eq!(from_hex::<u32>("0xff"), Ok(255));
        assert_eq!(from_hex::<u32>("0XFF"), Ok(255));
        assert_eq!(from_hex::<u32>("0x0"), Ok(0));
    }

    // Edge case tests for MEDIUM security fix - string parsing length limits

    #[test]
    fn test_parse_too_long_signed() {
        // Test that strings exceeding MAX_PARSE_LENGTH are rejected
        let long_string = "1".repeat(MAX_PARSE_LENGTH + 1);
        assert_eq!(parse_i32(&long_string), Err(ParseError::TooLong));
        assert_eq!(parse_i64(&long_string), Err(ParseError::TooLong));
        assert_eq!(parse_u32(&long_string), Err(ParseError::TooLong));
        assert_eq!(parse_u64(&long_string), Err(ParseError::TooLong));
    }

    #[test]
    fn test_parse_too_long_unsigned() {
        let long_string = "1".repeat(MAX_PARSE_LENGTH + 1);
        assert_eq!(parse_u32(&long_string), Err(ParseError::TooLong));
        assert_eq!(parse_u64(&long_string), Err(ParseError::TooLong));
    }

    #[test]
    fn test_parse_too_long_float() {
        let long_string = format!("1.{}", "0".repeat(MAX_PARSE_LENGTH));
        assert_eq!(parse_f32(&long_string), Err(ParseError::TooLong));
        assert_eq!(parse_f64(&long_string), Err(ParseError::TooLong));
    }

    #[test]
    fn test_parse_at_max_length() {
        // Test that strings exactly at MAX_PARSE_LENGTH don't get TooLong error
        // They may still overflow due to value, but that's a different error
        let max_string = "9".repeat(MAX_PARSE_LENGTH);
        let result = parse_u64(&max_string);
        // Should NOT be TooLong error - length is acceptable
        assert_ne!(result, Err(ParseError::TooLong));
        // But it will overflow due to value
        assert_eq!(result, Err(ParseError::OutOfRange));

        let result2 = parse_i64(&max_string);
        assert_ne!(result2, Err(ParseError::TooLong));
        assert_eq!(result2, Err(ParseError::OutOfRange));
    }

    #[test]
    fn test_parse_within_length_limit() {
        // Normal-length strings should work fine
        let normal_string = "1234567890".repeat(10); // 100 characters, well under limit
        // The string "1234567890" repeated 10 times overflows i64, so it should error
        assert!(parse_i64(&normal_string).is_err());
        // But a smaller number should work
        let small_string = "1".repeat(100); // 100 ones
        assert!(parse_u64(&small_string).is_err()); // Overflows u64
        // A 20-digit number that fits in u64 should work
        let valid_string = "12345678901234567890"; // 20 digits, fits in u64
        assert_eq!(parse_u64(valid_string), Ok(12345678901234567890u64));
    }

    #[test]
    fn test_empty_string_still_returns_empty_error() {
        // Empty strings should still return Empty error, not TooLong
        assert_eq!(parse_i32(""), Err(ParseError::Empty));
        assert_eq!(parse_u32(""), Err(ParseError::Empty));
        assert_eq!(parse_f64(""), Err(ParseError::Empty));
    }

    #[test]
    fn test_parse_error_too_long_display() {
        let error = ParseError::TooLong;
        let error_string = format!("{}", error);
        assert!(error_string.contains("too long"));
        assert!(error_string.contains(&format!("{}", MAX_PARSE_LENGTH)));
    }

    #[test]
    fn test_length_limit_prevents_dos() {
        // Verify that extremely long strings are rejected efficiently
        // without iterating through all characters

        // Create a string just over the limit
        let oversize = "9".repeat(MAX_PARSE_LENGTH + 100);

        // Should return TooLong error without excessive processing
        let result = parse_i64(&oversize);
        assert_eq!(result, Err(ParseError::TooLong));

        // Same for other types
        assert_eq!(parse_u32(&oversize), Err(ParseError::TooLong));
        assert_eq!(parse_f32(&oversize), Err(ParseError::TooLong));
    }

    #[test]
    fn test_signed_with_length_limit() {
        // Test signed parsing with length limit
        let long_negative = "-9".repeat(MAX_PARSE_LENGTH + 1);
        assert_eq!(parse_i64(&long_negative), Err(ParseError::TooLong));

        let long_positive = "+9".repeat(MAX_PARSE_LENGTH + 1);
        assert_eq!(parse_i64(&long_positive), Err(ParseError::TooLong));
    }

    #[test]
    fn test_unsigned_with_length_limit() {
        // Test unsigned parsing with length limit
        let long_with_plus = "+9".repeat(MAX_PARSE_LENGTH + 1);
        assert_eq!(parse_u64(&long_with_plus), Err(ParseError::TooLong));
    }

    #[test]
    fn test_zero_length_string() {
        // Edge case: string of length 0 should return Empty error
        assert_eq!(parse_i32(""), Err(ParseError::Empty));
        assert_eq!(parse_u32(""), Err(ParseError::Empty));
    }

    #[test]
    fn test_single_char_strings() {
        // Single character strings should work
        assert_eq!(parse_i32("1"), Ok(1));
        assert_eq!(parse_u32("5"), Ok(5));
        assert_eq!(parse_i32("-"), Err(ParseError::InvalidFormat)); // Just a sign is invalid
    }
}
