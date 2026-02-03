//! String utilities.

use alloc::string::String;

/// Creates a static string from a string literal.
///
/// This is a marker for strings that are statically allocated.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_utility::string_utils::static_str;
///
/// let s: &'static str = static_str!("hello");
/// ```
#[inline]
pub const fn static_str(s: &'static str) -> &'static str {
    s
}

/// Checks if a string contains only ASCII characters.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_utility::string_utils::is_ascii;
///
/// assert!(is_ascii("hello"));
/// assert!(!is_ascii("hello€"));
/// ```
#[inline]
pub fn is_ascii(s: &str) -> bool {
    s.is_ascii()
}

/// Converts a string to uppercase if possible (ASCII only).
///
/// # Examples
///
/// ```rust
/// use abseil::absl_utility::string_utils::to_ascii_upper;
///
/// assert_eq!(to_ascii_upper("hello"), "HELLO");
/// assert_eq!(to_ascii_upper("hello€"), "hello€");
/// ```
#[inline]
pub fn to_ascii_upper(s: &str) -> String {
    s.bytes()
        .map(|b| if b.is_ascii_lowercase() { b.to_ascii_uppercase() } else { b })
        .map(|b| b as char)
        .collect()
}

/// Converts a string to lowercase if possible (ASCII only).
///
/// # Examples
///
/// ```rust
/// use abseil::absl_utility::string_utils::to_ascii_lower;
///
/// assert_eq!(to_ascii_lower("HELLO"), "hello");
/// assert_eq!(to_ascii_lower("HELLO€"), "HELLO€");
/// ```
#[inline]
pub fn to_ascii_lower(s: &str) -> String {
    s.bytes()
        .map(|b| if b.is_ascii_uppercase() { b.to_ascii_lowercase() } else { b })
        .map(|b| b as char)
        .collect()
}

/// Truncates a string to a maximum length, adding "..." if truncated.
///
/// This function respects UTF-8 character boundaries and will not panic
/// even if the truncation point falls in the middle of a multi-byte character.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_utility::string_utils::truncate;
///
/// assert_eq!(truncate("hello world", 8), "hello...");
/// assert_eq!(truncate("hello", 10), "hello");
/// assert_eq!(truncate("hello€", 7), "he...");  // € is 3 bytes, safe truncation
/// ```
#[inline]
pub fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        return s.to_string();
    }
    let trunc_len = max_len.saturating_sub(3);
    // Find a safe UTF-8 boundary for truncation
    let safe_end = s.char_indices()
        .find(|(pos, _)| *pos > trunc_len)
        .map(|(pos, _)| pos)
        .unwrap_or(trunc_len);
    format!("{}...", &s[..safe_end])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_string_utils() {
        assert!(is_ascii("hello"));
        assert!(!is_ascii("hello€"));
        assert_eq!(to_ascii_upper("hello"), "HELLO");
        assert_eq!(to_ascii_lower("HELLO"), "hello");
        assert_eq!(truncate("hello world", 8), "hello...");
        assert_eq!(truncate("hello", 10), "hello");
    }
}
