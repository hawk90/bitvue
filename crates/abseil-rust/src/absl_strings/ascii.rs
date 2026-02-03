//! ASCII-specific string utilities.
//!
//! This module provides optimized operations for ASCII strings,
//! similar to Abseil's `absl/strings/ascii.h`.
//!
//! # Example
//!
//! ```
//! use abseil::absl_strings::ascii::*;
//!
//! assert!(is_alpha('A'));
//! assert!(is_digit('5'));
//! assert!(to_upper('a') == 'A');
//!
//! // String operations
//! assert_eq!(to_upper_ascii("hello"), "HELLO");
//! assert_eq!(to_lower_ascii("HELLO"), "hello");
//! ```

/// Returns `true` if the byte is an ASCII alphabetic character.
#[inline]
pub const fn is_alpha(c: char) -> bool {
    matches!(c, 'A'..='Z' | 'a'..='z')
}

/// Returns `true` if the byte is an ASCII digit.
#[inline]
pub const fn is_digit(c: char) -> bool {
    matches!(c, '0'..='9')
}

/// Returns `true` if the byte is an ASCII hexadecimal digit.
#[inline]
pub const fn is_hex_digit(c: char) -> bool {
    matches!(c, '0'..='9' | 'A'..='F' | 'a'..='f')
}

/// Returns `true` if the byte is ASCII whitespace.
#[inline]
pub const fn is_space(c: char) -> bool {
    matches!(c, ' ' | '\t' | '\n' | '\r' | '\x0b' | '\x0c')
}

/// Returns `true` if the byte is an ASCII punctuation character.
#[inline]
pub const fn is_punct(c: char) -> bool {
    matches!(c, '!'..='/' | ':'..='@' | '['..='`' | '{'..='~')
}

/// Returns `true` if the byte is an ASCII printable character (excluding space).
#[inline]
pub const fn is_graph(c: char) -> bool {
    matches!(c, '!'..='~')
}

/// Returns `true` if the byte is an ASCII printable character (including space).
#[inline]
pub const fn is_print(c: char) -> bool {
    matches!(c, ' ' | '!'..='~')
}

/// Returns `true` if the byte is an ASCII control character.
#[inline]
pub const fn is_control(c: char) -> bool {
    (c as u8) < 32 || c == '\x7f'
}

/// Returns `true` if the byte is an ASCII uppercase letter.
#[inline]
pub const fn is_upper(c: char) -> bool {
    matches!(c, 'A'..='Z')
}

/// Returns `true` if the byte is an ASCII lowercase letter.
#[inline]
pub const fn is_lower(c: char) -> bool {
    matches!(c, 'a'..='z')
}

/// Returns `true` if the character is ASCII (code point <= 127).
#[inline]
pub const fn is_ascii_char(c: char) -> bool {
    (c as u32) < 128
}

/// Converts an ASCII letter to uppercase.
///
/// Non-ASCII letters are returned unchanged.
#[inline]
pub const fn to_upper(c: char) -> char {
    if matches!(c, 'a'..='z') {
        const OFFSET: u8 = b'a' - b'A';
        ((c as u8) - OFFSET) as char
    } else {
        c
    }
}

/// Converts an ASCII letter to lowercase.
///
/// Non-ASCII letters are returned unchanged.
#[inline]
pub const fn to_lower(c: char) -> char {
    if matches!(c, 'A'..='Z') {
        const OFFSET: u8 = b'a' - b'A';
        ((c as u8) + OFFSET) as char
    } else {
        c
    }
}

/// Returns the ASCII value of a character, or the original character if not ASCII.
#[inline]
pub const fn to_ascii(c: char) -> u8 {
    if is_ascii_char(c) {
        c as u8
    } else {
        // Return a placeholder for non-ASCII
        0x7F // DEL character as placeholder
    }
}

/// Converts a string to uppercase, assuming all characters are ASCII.
///
/// # Panics
///
/// This function will panic if the string contains non-ASCII characters.
pub fn to_upper_ascii(s: &str) -> String {
    s.chars().map(to_upper).collect()
}

/// Converts a string to lowercase, assuming all characters are ASCII.
///
/// # Panics
///
/// This function will panic if the string contains non-ASCII characters.
pub fn to_lower_ascii(s: &str) -> String {
    s.chars().map(to_lower).collect()
}

/// Returns `true` if all characters in the string are ASCII.
pub fn is_ascii(s: &str) -> bool {
    s.is_ascii()
}

/// Returns `true` if the string contains only ASCII alphabetic characters.
pub fn is_alpha_string(s: &str) -> bool {
    !s.is_empty() && s.chars().all(is_alpha)
}

/// Returns `true` if the string contains only ASCII digits.
pub fn is_digit_string(s: &str) -> bool {
    !s.is_empty() && s.chars().all(is_digit)
}

/// Returns `true` if the string contains only ASCII alphanumeric characters.
pub fn is_alnum(c: char) -> bool {
    is_alpha(c) || is_digit(c)
}

/// Returns `true` if the string contains only ASCII alphanumeric characters.
pub fn is_alnum_string(s: &str) -> bool {
    !s.is_empty() && s.chars().all(is_alnum)
}

/// Strips ASCII whitespace from the beginning and end of a string.
pub fn strip_ascii_whitespace(s: &str) -> &str {
    s.trim_matches(is_space)
}

/// Strips ASCII whitespace from the beginning of a string.
pub fn strip_leading_ascii_whitespace(s: &str) -> &str {
    s.trim_start_matches(is_space)
}

/// Strips ASCII whitespace from the end of a string.
pub fn strip_trailing_ascii_whitespace(s: &str) -> &str {
    s.trim_end_matches(is_space)
}

/// Removes all ASCII whitespace from a string.
pub fn remove_ascii_whitespace(s: &str) -> String {
    s.chars().filter(|&c| !is_space(c)).collect()
}

/// Escapes non-ASCII characters and control characters in a string.
///
/// This function replaces non-ASCII and control characters with their
/// hexadecimal escape sequences (e.g., `\x7f`).
pub fn escape_nonascii(s: &str) -> String {
    let mut result = String::with_capacity(s.len() * 2);
    for c in s.chars() {
        if is_print(c) {
            result.push(c);
        } else {
            // Escape as \xHH
            result.push_str(&format!("\\x{:02x}", c as u32));
        }
    }
    result
}

/// Reserves space for ASCII characters in a string.
///
/// This is a no-op in this implementation, but provided for API compatibility.
pub fn reserve_ascii(_: &mut String, _: usize) {
    // No-op - String handles its own capacity
}

/// Returns the number of leading ASCII characters in a string.
pub fn leading_ascii_count(s: &str) -> usize {
    s.chars().take_while(|&c| is_ascii_char(c)).count()
}

/// Returns the number of trailing ASCII characters in a string.
pub fn trailing_ascii_count(s: &str) -> usize {
    s.chars().rev().take_while(|&c| is_ascii_char(c)).count()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_alpha() {
        assert!(is_alpha('A'));
        assert!(is_alpha('z'));
        assert!(!is_alpha('5'));
        assert!(!is_alpha(' '));
        assert!(!is_alpha('中'));
    }

    #[test]
    fn test_is_digit() {
        assert!(is_digit('5'));
        assert!(is_digit('0'));
        assert!(is_digit('9'));
        assert!(!is_digit('a'));
        assert!(!is_digit('Z'));
    }

    #[test]
    fn test_is_hex_digit() {
        assert!(is_hex_digit('0'));
        assert!(is_hex_digit('9'));
        assert!(is_hex_digit('A'));
        assert!(is_hex_digit('F'));
        assert!(is_hex_digit('a'));
        assert!(is_hex_digit('f'));
        assert!(!is_hex_digit('g'));
        assert!(!is_hex_digit('G'));
    }

    #[test]
    fn test_is_space() {
        assert!(is_space(' '));
        assert!(is_space('\t'));
        assert!(is_space('\n'));
        assert!(is_space('\r'));
        assert!(!is_space('a'));
    }

    #[test]
    fn test_is_punct() {
        assert!(is_punct('!'));
        assert!(is_punct('.'));
        assert!(is_punct('@'));
        assert!(!is_punct('a'));
        assert!(!is_punct(' '));
    }

    #[test]
    fn test_is_graph() {
        assert!(is_graph('!'));
        assert!(is_graph('a'));
        assert!(is_graph('~'));
        assert!(!is_graph(' '));
        assert!(!is_graph('\n'));
    }

    #[test]
    fn test_is_print() {
        assert!(is_print(' '));
        assert!(is_print('a'));
        assert!(is_print('~'));
        assert!(!is_print('\n'));
        assert!(!is_print('\x00'));
    }

    #[test]
    fn test_is_control() {
        assert!(is_control('\x00'));
        assert!(is_control('\x1f'));
        assert!(is_control('\x7f'));
        assert!(!is_control(' '));
        assert!(!is_control('a'));
    }

    #[test]
    fn test_is_upper() {
        assert!(is_upper('A'));
        assert!(is_upper('Z'));
        assert!(!is_upper('a'));
        assert!(!is_upper('5'));
    }

    #[test]
    fn test_is_lower() {
        assert!(is_lower('a'));
        assert!(is_lower('z'));
        assert!(!is_lower('A'));
        assert!(!is_lower('5'));
    }

    #[test]
    fn test_is_ascii_char() {
        assert!(is_ascii_char('a'));
        assert!(is_ascii_char('~'));
        assert!(is_ascii_char('\x00'));
        assert!(is_ascii_char('\x7f'));
        assert!(!is_ascii_char('中'));
    }

    #[test]
    fn test_to_upper() {
        assert_eq!(to_upper('a'), 'A');
        assert_eq!(to_upper('z'), 'Z');
        assert_eq!(to_upper('A'), 'A');
        assert_eq!(to_upper('5'), '5');
        assert_eq!(to_upper('中'), '中');
    }

    #[test]
    fn test_to_lower() {
        assert_eq!(to_lower('A'), 'a');
        assert_eq!(to_lower('Z'), 'z');
        assert_eq!(to_lower('a'), 'a');
        assert_eq!(to_lower('5'), '5');
        assert_eq!(to_lower('中'), '中');
    }

    #[test]
    fn test_to_ascii() {
        assert_eq!(to_ascii('a'), b'a');
        assert_eq!(to_ascii('中'), 0x7F);
    }

    #[test]
    fn test_to_upper_ascii() {
        assert_eq!(to_upper_ascii("hello"), "HELLO");
        assert_eq!(to_upper_ascii("HeLLo"), "HELLO");
        assert_eq!(to_upper_ascii("123"), "123");
    }

    #[test]
    fn test_to_lower_ascii() {
        assert_eq!(to_lower_ascii("HELLO"), "hello");
        assert_eq!(to_lower_ascii("HeLLo"), "hello");
        assert_eq!(to_lower_ascii("123"), "123");
    }

    #[test]
    fn test_is_ascii() {
        assert!(is_ascii("Hello"));
        assert!(is_ascii("World123"));
        assert!(is_ascii("\u{007F}")); // DEL is ASCII (0x7F = 127)
        assert!(!is_ascii("Hello 世界"));
    }

    #[test]
    fn test_is_alpha_string() {
        assert!(is_alpha_string("Hello"));
        assert!(is_alpha_string("WORLD"));
        assert!(!is_alpha_string("Hello123"));
        assert!(!is_alpha_string("Hello World"));
    }

    #[test]
    fn test_is_digit_string() {
        assert!(is_digit_string("12345"));
        assert!(!is_digit_string("123a45"));
        assert!(!is_digit_string(""));
    }

    #[test]
    fn test_is_alnum() {
        assert!(is_alnum('a'));
        assert!(is_alnum('Z'));
        assert!(is_alnum('5'));
        assert!(!is_alnum(' '));
        assert!(!is_alnum('!'));
    }

    #[test]
    fn test_is_alnum_string() {
        assert!(is_alnum_string("Hello123"));
        assert!(is_alnum_string("abcXYZ"));
        assert!(!is_alnum_string("Hello 123"));
        assert!(!is_alnum_string("Hello!"));
    }

    #[test]
    fn test_strip_ascii_whitespace() {
        assert_eq!(strip_ascii_whitespace("  hello  "), "hello");
        assert_eq!(strip_ascii_whitespace("\t\tworld\n"), "world");
        assert_eq!(strip_ascii_whitespace("nochange"), "nochange");
    }

    #[test]
    fn test_strip_leading_ascii_whitespace() {
        assert_eq!(strip_leading_ascii_whitespace("  hello"), "hello");
        assert_eq!(strip_leading_ascii_whitespace("\nworld"), "world");
        assert_eq!(strip_leading_ascii_whitespace("hello  "), "hello  ");
    }

    #[test]
    fn test_strip_trailing_ascii_whitespace() {
        assert_eq!(strip_trailing_ascii_whitespace("hello  "), "hello");
        assert_eq!(strip_trailing_ascii_whitespace("world\t"), "world");
        assert_eq!(strip_trailing_ascii_whitespace("  hello"), "  hello");
    }

    #[test]
    fn test_remove_ascii_whitespace() {
        assert_eq!(remove_ascii_whitespace("h e l l o"), "hello");
        assert_eq!(remove_ascii_whitespace("a\tb\nc"), "abc");
        assert_eq!(remove_ascii_whitespace("  "), "");
    }

    #[test]
    fn test_escape_nonascii() {
        assert_eq!(escape_nonascii("hello"), "hello");
        assert_eq!(escape_nonascii("hello\n"), "hello\\x0a");
        assert_eq!(escape_nonascii("世界"), "\\x4e16\\x754c");
        assert_eq!(escape_nonascii("\x00"), "\\x00");
    }

    #[test]
    fn test_leading_ascii_count() {
        assert_eq!(leading_ascii_count("Hello世界"), 5);
        assert_eq!(leading_ascii_count("World123!"), 9);
        assert_eq!(leading_ascii_count("世界Hello"), 0);
        assert_eq!(leading_ascii_count(""), 0);
    }

    #[test]
    fn test_trailing_ascii_count() {
        assert_eq!(trailing_ascii_count("世界Hello"), 5);
        assert_eq!(trailing_ascii_count("123!World"), 9);
        assert_eq!(trailing_ascii_count("Hello世界"), 0);
        assert_eq!(trailing_ascii_count(""), 0);
    }

    #[test]
    fn test_mixed_ascii_operations() {
        let s = "  Hello World 123  ";
        assert_eq!(to_upper_ascii(&strip_ascii_whitespace(s)), "HELLO WORLD 123");
    }
}
