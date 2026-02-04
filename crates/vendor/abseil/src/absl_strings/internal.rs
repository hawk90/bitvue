//! Internal string implementation details.
//!
//! This module contains internal utilities and data structures
//! used by the absl_strings module. These are not part of the
//! public API but are shared across string-related modules.

/// Maximum recommended size for inline string data.
///
/// Strings smaller than this may be stored inline rather than
/// allocating separate storage.
pub const INLINE_STRING_CAPACITY: usize = 23; // Fits in 32 bytes with metadata

/// Default capacity for new string buffers.
pub const DEFAULT_STRING_CAPACITY: usize = 16;

/// Maximum string size before triggering slow path warnings.
pub const MAX_FAST_PATH_SIZE: usize = 1024;

/// Buffer size for string formatting operations.
pub const FORMAT_BUFFER_SIZE: usize = 512;

/// Size of the stack buffer used by Cord.
pub const CORD_STACK_BUFFER_SIZE: usize = 127;

/// ASCII character classification constants.
pub mod ascii {
    /// Check if a byte is an ASCII character (0-127).
    #[inline]
    pub const fn is_ascii(b: u8) -> bool {
        b < 128
    }

    /// Check if a byte is an ASCII digit (0-9).
    #[inline]
    pub const fn is_digit(b: u8) -> bool {
        b >= b'0' && b <= b'9'
    }

    /// Check if a byte is an ASCII uppercase letter (A-Z).
    #[inline]
    pub const fn is_upper(b: u8) -> bool {
        b >= b'A' && b <= b'Z'
    }

    /// Check if a byte is an ASCII lowercase letter (a-z).
    #[inline]
    pub const fn is_lower(b: u8) -> bool {
        b >= b'a' && b <= b'z'
    }

    /// Check if a byte is an ASCII alphabetic character (A-Z, a-z).
    #[inline]
    pub const fn is_alpha(b: u8) -> bool {
        is_upper(b) || is_lower(b)
    }

    /// Check if a byte is an ASCII alphanumeric character (A-Z, a-z, 0-9).
    #[inline]
    pub const fn is_alnum(b: u8) -> bool {
        is_alpha(b) || is_digit(b)
    }

    /// Check if a byte is ASCII whitespace (space, tab, newline, etc.).
    #[inline]
    pub const fn is_space(b: u8) -> bool {
        matches!(b, b' ' | b'\t' | b'\n' | b'\r' | b'\x0b' | b'\x0c')
    }

    /// Check if a byte is ASCII printable (not a control character).
    #[inline]
    pub const fn is_print(b: u8) -> bool {
        b >= b' ' && b <= b'~'
    }

    /// Check if a byte is ASCII punctuation.
    #[inline]
    pub const fn is_punct(b: u8) -> bool {
        matches!(b, b'!'..=b'/' | b':'..=b'@' | b'['..=b'`' | b'{'..=b'~')
    }

    /// Check if a byte is an ASCII control character.
    #[inline]
    pub const fn is_control(b: u8) -> bool {
        b < b' ' || b == 0x7f
    }

    /// Check if a byte is ASCII hexadecimal digit (0-9, a-f, A-F).
    #[inline]
    pub const fn is_xdigit(b: u8) -> bool {
        is_digit(b) || (b >= b'a' && b <= b'f') || (b >= b'A' && b <= b'F')
    }

    /// Convert an ASCII digit byte to its numeric value.
    ///
    /// # Safety
    ///
    /// The byte must be a valid ASCII digit (0-9).
    #[inline]
    pub const fn digit_to_const(b: u8) -> u32 {
        (b - b'0') as u32
    }

    /// Convert an ASCII hexadecimal digit byte to its numeric value.
    ///
    /// Returns None if the byte is not a valid hex digit.
    #[inline]
    pub const fn xdigit_to_const(b: u8) -> Option<u32> {
        if is_digit(b) {
            Some((b - b'0') as u32)
        } else if b >= b'a' && b <= b'f' {
            Some((b - b'a' + 10) as u32)
        } else if b >= b'A' && b <= b'F' {
            Some((b - b'A' + 10) as u32)
        } else {
            None
        }
    }

    /// Convert an uppercase ASCII letter to lowercase.
    #[inline]
    pub const fn to_lower(b: u8) -> u8 {
        if is_upper(b) {
            b + 32
        } else {
            b
        }
    }

    /// Convert a lowercase ASCII letter to uppercase.
    #[inline]
    pub const fn to_upper(b: u8) -> u8 {
        if is_lower(b) {
            b - 32
        } else {
            b
        }
    }

    /// ASCII case-insensitive comparison.
    #[inline]
    pub fn eq_ignore_case(a: u8, b: u8) -> bool {
        to_lower(a) == to_lower(b)
    }
}

/// UTF-8 validation and manipulation utilities.
pub mod utf8 {
    /// Check if a byte is a UTF-8 continuation byte (10xxxxxx).
    #[inline]
    pub const fn is_continuation(b: u8) -> bool {
        (b & 0xC0) == 0x80
    }

    /// Check if a byte is a UTF-8 single-byte character (0xxxxxxx).
    #[inline]
    pub const fn is_single(b: u8) -> bool {
        b & 0x80 == 0
    }

    /// Check if a byte starts a 2-byte UTF-8 sequence (110xxxxx).
    #[inline]
    pub const fn is_lead_2(b: u8) -> bool {
        (b & 0xE0) == 0xC0
    }

    /// Check if a byte starts a 3-byte UTF-8 sequence (1110xxxx).
    #[inline]
    pub const fn is_lead_3(b: u8) -> bool {
        (b & 0xF0) == 0xE0
    }

    /// Check if a byte starts a 4-byte UTF-8 sequence (11110xxx).
    #[inline]
    pub const fn is_lead_4(b: u8) -> bool {
        (b & 0xF8) == 0xF0
    }

    /// Get the expected sequence length from the leading byte.
    ///
    /// Returns 0 if the byte is not a valid leading byte.
    #[inline]
    pub const fn seq_len_from_lead(b: u8) -> usize {
        if is_single(b) {
            1
        } else if is_lead_2(b) {
            2
        } else if is_lead_3(b) {
            3
        } else if is_lead_4(b) {
            4
        } else {
            0
        }
    }

    /// Check if a byte sequence is valid UTF-8.
    #[inline]
    pub fn is_valid_utf8(bytes: &[u8]) -> bool {
        core::str::from_utf8(bytes).is_ok()
    }

    /// Count the number of UTF-8 code points in a byte slice.
    pub fn count_code_points(bytes: &[u8]) -> usize {
        let mut count = 0;
        let mut i = 0;
        while i < bytes.len() {
            let seq_len = seq_len_from_lead(bytes[i]);
            if seq_len == 0 || i + seq_len > bytes.len() {
                // Invalid UTF-8, count remaining as individual bytes
                count += bytes.len() - i;
                break;
            }
            // Verify continuation bytes
            // SAFETY: We already checked i + seq_len <= bytes.len() above,
            // so this range is valid and won't panic or read out of bounds.
            let valid = bytes[i + 1..i + seq_len]
                .iter()
                .all(|&b| is_continuation(b));
            if !valid {
                // Invalid sequence, count as individual bytes
                count += 1;
                i += 1;
            } else {
                count += 1;
                i += seq_len;
            }
        }
        count
    }

    /// Truncate a byte slice to the last complete UTF-8 boundary.
    pub fn truncate_to_boundary(bytes: &[u8], max_len: usize) -> &[u8] {
        if max_len >= bytes.len() {
            return bytes;
        }
        let mut end = max_len;
        while end > 0 && is_continuation(bytes[end]) {
            end -= 1;
        }
        &bytes[..end]
    }

    /// Get the byte length of a code point at the given position.
    ///
    /// Returns 0 if the position is invalid.
    pub fn code_point_len_at(bytes: &[u8], pos: usize) -> usize {
        if pos >= bytes.len() {
            return 0;
        }
        let seq_len = seq_len_from_lead(bytes[pos]);
        if seq_len == 0 || pos + seq_len > bytes.len() {
            return 1; // Invalid, treat as single byte
        }
        // Verify continuation bytes
        let valid = bytes[pos + 1..pos + seq_len]
            .iter()
            .all(|&b| is_continuation(b));
        if valid {
            seq_len
        } else {
            1
        }
    }
}

/// String validation utilities.
pub mod validate {
    /// Check if a string contains only ASCII characters.
    #[inline]
    pub fn is_ascii(s: &str) -> bool {
        s.as_bytes().iter().all(|&b| b < 128)
    }

    /// Check if a string contains only alphanumeric ASCII characters.
    #[inline]
    pub fn is_ascii_alnum(s: &str) -> bool {
        s.as_bytes().iter().all(|&b| super::ascii::is_alnum(b))
    }

    /// Check if a string contains only alphabetic ASCII characters.
    #[inline]
    pub fn is_ascii_alpha(s: &str) -> bool {
        s.as_bytes().iter().all(|&b| super::ascii::is_alpha(b))
    }

    /// Check if a string contains only digit ASCII characters.
    #[inline]
    pub fn is_ascii_digit(s: &str) -> bool {
        s.as_bytes().iter().all(|&b| super::ascii::is_digit(b))
    }

    /// Check if a string contains only printable ASCII characters.
    #[inline]
    pub fn is_ascii_print(s: &str) -> bool {
        s.as_bytes().iter().all(|&b| super::ascii::is_print(b))
    }

    /// Check if a string is empty or contains only whitespace.
    #[inline]
    pub fn is_blank(s: &str) -> bool {
        s.as_bytes().iter().all(|&b| super::ascii::is_space(b))
    }

    /// Check if a string contains valid UTF-8 data.
    #[inline]
    pub fn is_valid_utf8(_s: &str) -> bool {
        true // &str is always valid UTF-8 by construction
    }

    /// Find the first non-ASCII byte position in a string.
    #[inline]
    pub fn find_non_ascii(s: &str) -> Option<usize> {
        s.as_bytes().iter().position(|&b| b >= 128)
    }

    /// Count the number of ASCII characters in a string.
    #[inline]
    pub fn count_ascii(s: &str) -> usize {
        s.as_bytes().iter().filter(|&&b| b < 128).count()
    }
}

/// String comparison utilities.
pub mod compare {
    /// Compare two strings ASCII case-insensitively.
    pub fn eq_ignore_case(a: &str, b: &str) -> bool {
        if a.len() != b.len() {
            return false;
        }
        a.as_bytes()
            .iter()
            .zip(b.as_bytes().iter())
            .all(|(&ca, &cb)| super::ascii::eq_ignore_case(ca, cb))
    }

    /// Compare two strings ASCII case-insensitively, returning ordering.
    pub fn cmp_ignore_case(a: &str, b: &str) -> core::cmp::Ordering {
        let mut a_bytes = a.as_bytes().iter();
        let mut b_bytes = b.as_bytes().iter();

        loop {
            match (a_bytes.next(), b_bytes.next()) {
                (None, None) => return core::cmp::Ordering::Equal,
                (None, Some(_)) => return core::cmp::Ordering::Less,
                (Some(_), None) => return core::cmp::Ordering::Greater,
                (Some(&ca), Some(&cb)) => {
                    let la = super::ascii::to_lower(ca);
                    let lb = super::ascii::to_lower(cb);
                    match la.cmp(&lb) {
                        core::cmp::Ordering::Equal => continue,
                        other => return other,
                    }
                }
            }
        }
    }

    /// Check if a string starts with another string, case-insensitively.
    pub fn starts_with_ignore_case(s: &str, prefix: &str) -> bool {
        if s.len() < prefix.len() {
            return false;
        }
        s.as_bytes()
            .iter()
            .zip(prefix.as_bytes().iter())
            .all(|(&ca, &cb)| super::ascii::eq_ignore_case(ca, cb))
    }

    /// Check if a string ends with another string, case-insensitively.
    pub fn ends_with_ignore_case(s: &str, suffix: &str) -> bool {
        if s.len() < suffix.len() {
            return false;
        }
        let start = s.len() - suffix.len();
        s[start..]
            .as_bytes()
            .iter()
            .zip(suffix.as_bytes().iter())
            .all(|(&ca, &cb)| super::ascii::eq_ignore_case(ca, cb))
    }
}

/// Memory utilities for string operations.
pub mod memory {
    /// Check if a string slice is contiguous and can be treated as &[u8].
    #[inline]
    pub const fn is_contiguous(_s: &str) -> bool {
        true // &str is always contiguous in Rust
    }

    /// Get the byte pointer to string data.
    #[inline]
    pub fn as_ptr(s: &str) -> *const u8 {
        s.as_ptr()
    }

    /// Get the mutable byte pointer to string data.
    ///
    /// # Safety
    ///
    /// The caller must ensure the string is uniquely owned
    /// and the resulting UTF-8 remains valid.
    #[inline]
    pub unsafe fn as_mut_ptr(s: &mut str) -> *mut u8 {
        s.as_bytes_mut().as_mut_ptr()
    }

    /// Calculate the maximum capacity needed to store a string.
    #[inline]
    pub const fn capacity_for(len: usize) -> usize {
        // Round up to next power of two for better alignment
        if len == 0 {
            return 1;
        }
        let mut cap = 1usize;
        while cap < len {
            cap <<= 1;
        }
        cap
    }
}

/// Trimming utilities.
pub mod trim {
    /// Find the start of the non-whitespace region.
    pub fn find_start(s: &str) -> usize {
        s.as_bytes()
            .iter()
            .position(|&b| !super::ascii::is_space(b))
            .unwrap_or(s.len())
    }

    /// Find the end of the non-whitespace region.
    pub fn find_end(s: &str) -> usize {
        s.as_bytes()
            .iter()
            .rposition(|&b| !super::ascii::is_space(b))
            .map(|p| p + 1)
            .unwrap_or(0)
    }

    /// Trim leading whitespace.
    pub fn trim_left(s: &str) -> &str {
        let start = find_start(s);
        &s[start..]
    }

    /// Trim trailing whitespace.
    pub fn trim_right(s: &str) -> &str {
        let end = find_end(s);
        &s[..end]
    }

    /// Trim both leading and trailing whitespace.
    pub fn trim(s: &str) -> &str {
        let start = find_start(s);
        let end = find_end(s);
        &s[start..end]
    }
}

/// String searching utilities.
pub mod search {
    /// Find a substring using a naive algorithm.
    pub fn find_naive(haystack: &str, needle: &str) -> Option<usize> {
        if needle.is_empty() {
            return Some(0);
        }
        if haystack.is_empty() {
            return None;
        }
        let haystack_bytes = haystack.as_bytes();
        let needle_bytes = needle.as_bytes();

        if needle_bytes.len() > haystack_bytes.len() {
            return None;
        }

        let mut i = 0;
        while i <= haystack_bytes.len() - needle_bytes.len() {
            if &haystack_bytes[i..i + needle_bytes.len()] == needle_bytes {
                return Some(i);
            }
            // Advance by at least 1 UTF-8 character
            let seq_len = super::utf8::seq_len_from_lead(haystack_bytes[i]);
            i += seq_len.max(1);
        }
        None
    }

    /// Find the last occurrence of a substring.
    pub fn rfind_naive(haystack: &str, needle: &str) -> Option<usize> {
        if needle.is_empty() {
            return Some(haystack.len());
        }
        let haystack_bytes = haystack.as_bytes();
        let needle_bytes = needle.as_bytes();

        let mut i = haystack_bytes.len().saturating_sub(needle_bytes.len());
        loop {
            if &haystack_bytes[i..i + needle_bytes.len()] == needle_bytes {
                return Some(i);
            }
            if i == 0 {
                break;
            }
            // Go back by at least 1 UTF-8 character
            let mut j = i - 1;
            while j > 0 && super::utf8::is_continuation(haystack_bytes[j]) {
                j -= 1;
            }
            i = j;
        }
        None
    }

    /// Count occurrences of a substring.
    pub fn count_matches(haystack: &str, needle: &str) -> usize {
        if needle.is_empty() {
            return haystack.len() + 1;
        }
        let mut count = 0;
        let mut start = 0;
        while let Some(pos) = find_naive(&haystack[start..], needle) {
            count += 1;
            start += pos + needle.len();
        }
        count
    }

    /// Check if a string contains a substring.
    #[inline]
    pub fn contains(haystack: &str, needle: &str) -> bool {
        find_naive(haystack, needle).is_some()
    }
}

/// Character classification utilities.
pub mod char_class {
    /// Check if a character is whitespace.
    #[inline]
    pub fn is_whitespace(c: char) -> bool {
        c.is_whitespace()
    }

    /// Check if a character is alphanumeric.
    #[inline]
    pub fn is_alphanumeric(c: char) -> bool {
        c.is_alphanumeric()
    }

    /// Check if a character is a control character.
    #[inline]
    pub fn is_control(c: char) -> bool {
        c.is_control()
    }

    /// Check if a character is numeric.
    #[inline]
    pub fn is_numeric(c: char) -> bool {
        c.is_numeric()
    }

    /// Get the general category of a character.
    #[inline]
    pub fn general_category(c: char) -> u32 {
        // Simplified version - just returns a basic category
        // A real implementation would use Unicode data
        match c {
            '0'..='9' => 0,              // Number, Decimal Digit
            'a'..='z' | 'A'..='Z' => 1,  // Letter
            _ if c.is_whitespace() => 2, // Separator
            _ => 3,                      // Other
        }
    }
}

/// Constants for string operations.
pub mod constants {
    /// Common string literals.
    pub mod literals {
        pub const EMPTY: &str = "";
        pub const SPACE: &str = " ";
        pub const NEWLINE: &str = "\n";
        pub const CRLF: &str = "\r\n";
        pub const TAB: &str = "\t";
        pub const COMMA: &str = ",";
        pub const COLON: &str = ":";
        pub const SEMICOLON: &str = ";";
        pub const DOT: &str = ".";
        pub const SLASH: &str = "/";
        pub const BACKSLASH: &str = "\\";
        pub const UNDERSCORE: &str = "_";
        pub const DASH: &str = "-";
        pub const PLUS: &str = "+";
        pub const EQUALS: &str = "=";
        pub const AT: &str = "@";
        pub const HASH: &str = "#";
        pub const DOLLAR: &str = "$";
        pub const PERCENT: &str = "%";
        pub const AMPERSAND: &str = "&";
        pub const PIPE: &str = "|";
        pub const CARET: &str = "^";
        pub const TILDE: &str = "~";
        pub const BANG: &str = "!";
        pub const QUESTION: &str = "?";
        pub const STAR: &str = "*";
        pub const LPAREN: &str = "(";
        pub const RPAREN: &str = ")";
        pub const LBRACKET: &str = "[";
        pub const RBRACKET: &str = "]";
        pub const LBRACE: &str = "{";
        pub const RBRACE: &str = "}";
        pub const LT: &str = "<";
        pub const GT: &str = ">";
        pub const SINGLE_QUOTE: &str = "'";
        pub const DOUBLE_QUOTE: &str = "\"";
        pub const BACKTICK: &str = "`";
    }

    /// Common delimiters.
    pub mod delimiters {
        pub const COMMA: char = ',';
        pub const SEMICOLON: char = ';';
        pub const COLON: char = ':';
        pub const PIPE: char = '|';
        pub const TAB: char = '\t';
        pub const SPACE: char = ' ';
        pub const NEWLINE: char = '\n';
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ASCII tests
    #[test]
    fn test_ascii_is_digit() {
        assert!(ascii::is_digit(b'0'));
        assert!(ascii::is_digit(b'9'));
        assert!(!ascii::is_digit(b'a'));
        assert!(!ascii::is_digit(b' '));
    }

    #[test]
    fn test_ascii_is_alpha() {
        assert!(ascii::is_alpha(b'a'));
        assert!(ascii::is_alpha(b'Z'));
        assert!(!ascii::is_alpha(b'0'));
        assert!(!ascii::is_alpha(b' '));
    }

    #[test]
    fn test_ascii_is_alnum() {
        assert!(ascii::is_alnum(b'a'));
        assert!(ascii::is_alnum(b'Z'));
        assert!(ascii::is_alnum(b'0'));
        assert!(!ascii::is_alnum(b' '));
    }

    #[test]
    fn test_ascii_to_lower() {
        assert_eq!(ascii::to_lower(b'A'), b'a');
        assert_eq!(ascii::to_lower(b'Z'), b'z');
        assert_eq!(ascii::to_lower(b'a'), b'a');
        assert_eq!(ascii::to_lower(b'0'), b'0');
    }

    #[test]
    fn test_ascii_to_upper() {
        assert_eq!(ascii::to_upper(b'a'), b'A');
        assert_eq!(ascii::to_upper(b'z'), b'Z');
        assert_eq!(ascii::to_upper(b'A'), b'A');
        assert_eq!(ascii::to_upper(b'0'), b'0');
    }

    // UTF-8 tests
    #[test]
    fn test_utf8_seq_len_from_lead() {
        assert_eq!(utf8::seq_len_from_lead(0x00), 1);
        assert_eq!(utf8::seq_len_from_lead(0x7F), 1);
        assert_eq!(utf8::seq_len_from_lead(0xC2), 2);
        assert_eq!(utf8::seq_len_from_lead(0xE0), 3);
        assert_eq!(utf8::seq_len_from_lead(0xF0), 4);
        assert_eq!(utf8::seq_len_from_lead(0x80), 0); // Continuation byte
    }

    #[test]
    fn test_utf8_count_code_points() {
        assert_eq!(utf8::count_code_points(b"hello"), 5);
        assert_eq!(utf8::count_code_points("hello".as_bytes()), 5);
        assert_eq!(utf8::count_code_points("你好".as_bytes()), 2);
    }

    #[test]
    fn test_utf8_truncate_to_boundary() {
        let s = "hello world";
        assert_eq!(utf8::truncate_to_boundary(s.as_bytes(), 5), b"hello");
        let s = "你好世界";
        let result = utf8::truncate_to_boundary(s.as_bytes(), 5);
        assert!(result.len() <= 3); // Should truncate to first character
    }

    // Validation tests
    #[test]
    fn test_validate_is_ascii() {
        assert!(validate::is_ascii("hello"));
        assert!(validate::is_ascii("hello world")); // Space is ASCII
        assert!(!validate::is_ascii("你好"));
    }

    #[test]
    fn test_validate_is_ascii_alnum() {
        assert!(validate::is_ascii_alnum("hello123"));
        assert!(!validate::is_ascii_alnum("hello 123")); // Space is not alnum
    }

    #[test]
    fn test_validate_is_blank() {
        assert!(validate::is_blank(""));
        assert!(validate::is_blank("   "));
        assert!(validate::is_blank("\t\n"));
        assert!(!validate::is_blank(" hello "));
    }

    #[test]
    fn test_validate_find_non_ascii() {
        assert_eq!(validate::find_non_ascii("hello"), None);
        assert_eq!(validate::find_non_ascii("hello world"), None); // All ASCII
        assert_eq!(validate::find_non_ascii("你好"), Some(0));
    }

    // Comparison tests
    #[test]
    fn test_compare_eq_ignore_case() {
        assert!(compare::eq_ignore_case("hello", "HELLO"));
        assert!(compare::eq_ignore_case("Hello", "hELLo"));
        assert!(!compare::eq_ignore_case("hello", "world"));
    }

    #[test]
    fn test_compare_starts_with_ignore_case() {
        assert!(compare::starts_with_ignore_case("Hello World", "HELLO"));
        assert!(compare::starts_with_ignore_case("Hello World", "hello"));
        assert!(!compare::starts_with_ignore_case("Hello World", "world"));
    }

    #[test]
    fn test_compare_ends_with_ignore_case() {
        assert!(compare::ends_with_ignore_case("Hello World", "WORLD"));
        assert!(compare::ends_with_ignore_case("Hello World", "world"));
        assert!(!compare::ends_with_ignore_case("Hello World", "hello"));
    }

    // Trim tests
    #[test]
    fn test_trim_trim_left() {
        assert_eq!(trim::trim_left("  hello"), "hello");
        assert_eq!(trim::trim_left("\thello"), "hello");
        assert_eq!(trim::trim_left("hello  "), "hello  ");
    }

    #[test]
    fn test_trim_trim_right() {
        assert_eq!(trim::trim_right("hello  "), "hello");
        assert_eq!(trim::trim_right("hello\t"), "hello");
        assert_eq!(trim::trim_right("  hello"), "  hello");
    }

    #[test]
    fn test_trim_trim() {
        assert_eq!(trim::trim("  hello  "), "hello");
        assert_eq!(trim::trim("\thello\t"), "hello");
        assert_eq!(trim::trim("  hello world  "), "hello world");
    }

    // Search tests
    #[test]
    fn test_search_find_naive() {
        assert_eq!(search::find_naive("hello world", "world"), Some(6));
        assert_eq!(search::find_naive("hello world", "xyz"), None);
        assert_eq!(search::find_naive("hello", ""), Some(0));
    }

    #[test]
    fn test_search_rfind_naive() {
        assert_eq!(search::rfind_naive("hello world hello", "hello"), Some(12));
        assert_eq!(search::rfind_naive("hello world", "xyz"), None);
    }

    #[test]
    fn test_search_count_matches() {
        assert_eq!(search::count_matches("hello hello hello", "hello"), 3);
        assert_eq!(search::count_matches("hello", "xyz"), 0);
        assert_eq!(search::count_matches("", "a"), 0);
        assert_eq!(search::count_matches("aaa", ""), 4); // len + 1 for empty needle
    }

    #[test]
    fn test_search_contains() {
        assert!(search::contains("hello world", "world"));
        assert!(!search::contains("hello world", "xyz"));
    }

    // Memory tests
    #[test]
    fn test_memory_capacity_for() {
        assert_eq!(memory::capacity_for(0), 1);
        assert_eq!(memory::capacity_for(1), 1);
        assert_eq!(memory::capacity_for(5), 8);
        assert_eq!(memory::capacity_for(16), 16);
        assert_eq!(memory::capacity_for(17), 32);
    }

    // Character classification tests
    #[test]
    fn test_char_class_is_whitespace() {
        assert!(char_class::is_whitespace(' '));
        assert!(char_class::is_whitespace('\t'));
        assert!(char_class::is_whitespace('\n'));
        assert!(!char_class::is_whitespace('a'));
    }

    #[test]
    fn test_char_class_is_alphanumeric() {
        assert!(char_class::is_alphanumeric('a'));
        assert!(char_class::is_alphanumeric('Z'));
        assert!(char_class::is_alphanumeric('0'));
        assert!(!char_class::is_alphanumeric(' '));
    }

    #[test]
    fn test_char_class_is_numeric() {
        assert!(char_class::is_numeric('0'));
        assert!(char_class::is_numeric('9'));
        assert!(!char_class::is_numeric('a'));
    }

    // Constants tests
    #[test]
    fn test_constants_literals() {
        assert_eq!(constants::literals::EMPTY, "");
        assert_eq!(constants::literals::SPACE, " ");
        assert_eq!(constants::literals::NEWLINE, "\n");
    }

    #[test]
    fn test_constants_delimiters() {
        assert_eq!(constants::delimiters::COMMA, ',');
        assert_eq!(constants::delimiters::COLON, ':');
        assert_eq!(constants::delimiters::SEMICOLON, ';');
    }
}
