//! String escaping utilities.
//!
//! Provides functions for escaping and unescaping strings with special characters.

/// Escapes a C-style string.
///
/// Converts special characters like newlines, tabs, quotes, etc. to their escape sequences.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_strings::escaping::escape_c;
///
/// assert_eq!(escape_c("Hello\nWorld"), r#"Hello\nWorld"#);
/// assert_eq!(escape_c("Quote: \""), r#"Quote: \""#);
/// ```
pub fn escape_c(s: &str) -> String {
    let mut result = String::with_capacity(s.len() * 2);
    for c in s.chars() {
        match c {
            '\n' => result.push_str("\\n"),
            '\r' => result.push_str("\\r"),
            '\t' => result.push_str("\\t"),
            '\\' => result.push_str("\\\\"),
            '\'' => result.push_str("\\'"),
            '"' => result.push_str("\\\""),
            '\0' => result.push_str("\\0"),
            c => result.push(c),
        }
    }
    result
}

/// Error type for unescape failures.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum UnescapeError {
    /// Incomplete escape sequence.
    IncompleteEscape,
    /// Invalid hex digit.
    InvalidHex,
}

impl core::fmt::Display for UnescapeError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            UnescapeError::IncompleteEscape => write!(f, "incomplete escape sequence"),
            UnescapeError::InvalidHex => write!(f, "invalid hex digit"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for UnescapeError {}

/// Unescapes a C-style string.
///
/// Converts escape sequences like `\n`, `\t`, etc. to their actual characters.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_strings::escaping::unescape_c;
///
/// assert_eq!(unescape_c("Hello\\nWorld").unwrap(), "Hello\nWorld");
/// assert_eq!(unescape_c("Tab\\there").unwrap(), "Tab\there");
/// ```
pub fn unescape_c(s: &str) -> Result<String, UnescapeError> {
    let mut result = String::with_capacity(s.len());
    let mut chars = s.char_indices();

    while let Some((_, ch)) = chars.next() {
        if ch == '\\' {
            // Process escape sequence
            let (next_idx, next_ch) = chars.next().ok_or(UnescapeError::IncompleteEscape)?;
            match next_ch {
                'n' => result.push('\n'),
                'r' => result.push('\r'),
                't' => result.push('\t'),
                '\\' => result.push('\\'),
                '\'' => result.push('\''),
                '"' => result.push('"'),
                '0' => result.push('\0'),
                'x' => {
                    // Hex escape \xHH - need to read exactly 2 hex characters
                    // Since \xHH is ASCII, we can work with bytes
                    let remaining = &s[next_idx..];
                    let bytes = remaining.as_bytes();
                    if bytes.len() < 3 {
                        return Err(UnescapeError::IncompleteEscape);
                    }
                    let h1 = bytes[1];
                    let h2 = bytes[2];
                    let high = unhex_byte(h1 as char).ok_or(UnescapeError::InvalidHex)?;
                    let low = unhex_byte(h2 as char).ok_or(UnescapeError::InvalidHex)?;
                    let byte_val = (high << 4) | low;
                    // Only accept ASCII from hex escapes
                    if byte_val > 127 {
                        return Err(UnescapeError::InvalidHex);
                    }
                    result.push(byte_val as char);
                    // Skip past the 2 hex digits
                    chars.next(); // skip first hex digit
                    chars.next(); // skip second hex digit
                }
                _ => {
                    // Unknown escape - preserve backslash and character
                    result.push('\\');
                    result.push(next_ch);
                }
            }
        } else {
            result.push(ch);
        }
    }

    Ok(result)
}

/// Escapes a URL by encoding special characters.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_strings::escaping::escape_url;
///
/// assert_eq!(escape_url("hello world"), "hello%20world");
/// assert_eq!(escape_url("a/b?c=d"), "a%2Fb%3Fc%3Dd");
/// ```
pub fn escape_url(s: &str) -> String {
    let mut result = String::with_capacity(s.len() * 3);
    for byte in s.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
            result.push(byte as char);
        }
            _ => {
                result.push('%');
                result.push(hex_upper(byte >> 4));
                result.push(hex_upper(byte & 0x0F));
            }
        }
    }
    result
}

/// Unescapes a URL by decoding percent-encoded characters.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_strings::escaping::unescape_url;
///
/// assert_eq!(unescape_url("hello%20world").unwrap(), "hello world");
/// assert_eq!(unescape_url("a%2Fb%3Fc%3Dd").unwrap(), "a/b?c=d");
/// ```
pub fn unescape_url(s: &str) -> Result<String, UnescapeError> {
    let mut result = String::with_capacity(s.len());
    let mut chars = s.char_indices();

    while let Some((_, ch)) = chars.next() {
        match ch {
            '%' => {
                // Percent encoding - need to read exactly 2 hex digits
                let remaining = chars.as_str();
                let bytes = remaining.as_bytes();
                if bytes.len() < 2 {
                    return Err(UnescapeError::IncompleteEscape);
                }
                let h1 = bytes[0];
                let h2 = bytes[1];
                let high = unhex_byte(h1 as char).ok_or(UnescapeError::InvalidHex)?;
                let low = unhex_byte(h2 as char).ok_or(UnescapeError::InvalidHex)?;
                let byte_val = (high << 4) | low;
                // Only accept ASCII from percent-encoding
                if byte_val > 127 {
                    return Err(UnescapeError::InvalidHex);
                }
                result.push(byte_val as char);
                // Skip past the 2 hex digits
                chars.next();
                chars.next();
            }
            '+' => result.push(' '),
            _ => result.push(ch),
        }
    }

    Ok(result)
}

/// Escapes HTML special characters.
///
/// Converts `<`, `>`, `&`, `"`, and `'` to their HTML entities.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_strings::escaping::escape_html;
///
/// assert_eq!(escape_html("<div>"), "&lt;div&gt;");
/// assert_eq!(escape_html("a & b"), "a &amp; b");
/// ```
pub fn escape_html(s: &str) -> String {
    let mut result = String::with_capacity(s.len() * 5);
    for c in s.chars() {
        match c {
            '<' => result.push_str("&lt;"),
            '>' => result.push_str("&gt;"),
            '&' => result.push_str("&amp;"),
            '"' => result.push_str("&quot;"),
            '\'' => result.push_str("&apos;"),
            c => result.push(c),
        }
    }
    result
}

/// Unescapes HTML entities.
///
/// Converts HTML entities like `&lt;`, `&gt;`, etc. back to their characters.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_strings::escaping::unescape_html;
///
/// assert_eq!(unescape_html("&lt;div&gt;"), "<div>");
/// assert_eq!(unescape_html("a &amp; b"), "a & b");
/// ```
pub fn unescape_html(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut i = 0;

    while i < s.len() {
        // Check for entities starting with '&'
        if s.as_bytes()[i] == b'&' {
            // Check each entity with proper bounds checking
            if s[i..].starts_with("&lt;") {
                result.push('<');
                i += 4;
            } else if s[i..].starts_with("&gt;") {
                result.push('>');
                i += 4;
            } else if s[i..].starts_with("&amp;") {
                result.push('&');
                i += 5;
            } else if s[i..].starts_with("&quot;") {
                result.push('"');
                i += 6;
            } else if s[i..].starts_with("&apos;") {
                result.push('\'');
                i += 6;
            } else {
                // Not a recognized entity, copy the & as-is
                result.push(s.as_bytes()[i] as char);
                i += 1;
            }
        } else {
            result.push(s.as_bytes()[i] as char);
            i += 1;
        }
    }

    result
}

/// Converts a byte to uppercase hex character.
#[inline]
fn hex_upper(byte: u8) -> char {
    let nibble = byte & 0x0F;
    if nibble < 10 {
        (b'0' + nibble) as char
    } else {
        (b'A' + nibble - 10) as char
    }
}

/// Converts a hex character to its value.
#[inline]
fn unhex_byte(c: char) -> Option<u8> {
    match c {
        '0'..='9' => Some(c as u8 - b'0'),
        'a'..='f' => Some(c as u8 - b'a' + 10),
        'A'..='F' => Some(c as u8 - b'A' + 10),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_escape_c() {
        assert_eq!(escape_c("Hello\nWorld"), r#"Hello\nWorld"#);
        assert_eq!(escape_c("Tab\there"), r#"Tab\there"#);
        assert_eq!(escape_c("Quote: \""), r#"Quote: \""#);
        assert_eq!(escape_c("Backslash: \\"), r#"Backslash: \\"#);
    }

    #[test]
    fn test_unescape_c() {
        assert_eq!(unescape_c("Hello\\nWorld").unwrap(), "Hello\nWorld");
        assert_eq!(unescape_c("Tab\\there").unwrap(), "Tab\there");
        // Quote in string - testing backslash-quote sequence
        let s = "Quote: \\\"";
        assert_eq!(unescape_c(s).unwrap(), "Quote: \"");
    }

    #[test]
    fn test_escape_url() {
        assert_eq!(escape_url("hello world"), "hello%20world");
        assert_eq!(escape_url("a/b?c=d"), "a%2Fb%3Fc%3Dd");
    }

    #[test]
    fn test_unescape_url() {
        assert_eq!(unescape_url("hello%20world").unwrap(), "hello world");
        assert_eq!(unescape_url("a%2Fb%3Fc%3Dd").unwrap(), "a/b?c=d");
        assert!(unescape_url("hello%2").is_err()); // Incomplete hex
    }

    #[test]
    fn test_escape_html() {
        assert_eq!(escape_html("<div>"), "&lt;div&gt;");
        assert_eq!(escape_html("a & b"), "a &amp; b");
        assert_eq!(escape_html("\"quoted\""), "&quot;quoted&quot;");
    }

    #[test]
    fn test_unescape_html() {
        assert_eq!(unescape_html("&lt;div&gt;"), "<div>");
        assert_eq!(unescape_html("a &amp; b"), "a & b");
        assert_eq!(unescape_html("&quot;quoted&quot;"), r#""quoted""#);
    }

    #[test]
    fn test_roundtrip_c() {
        let original = "Hello\tWorld\nTest";
        assert_eq!(unescape_c(&escape_c(original)).unwrap(), original);
    }

    #[test]
    fn test_roundtrip_url() {
        let original = "hello world";
        assert_eq!(unescape_url(&escape_url(original)).unwrap(), original);
    }

    #[test]
    fn test_hex_unhex() {
        assert_eq!(hex_upper(0x0A), 'A');
        assert_eq!(hex_upper(0x0F), 'F');
        assert_eq!(unhex_byte('a'), Some(10));
        assert_eq!(unhex_byte('F'), Some(15));
        assert!(unhex_byte('x').is_none());
    }

    #[test]
    fn test_empty_string() {
        assert_eq!(escape_c(""), "");
        assert_eq!(unescape_c("").unwrap(), "");
    }

    #[test]
    fn test_backslash_handling() {
        // Double backslash becomes single
        let input = "\\\\";
        let result = unescape_c(input).unwrap();
        assert_eq!(result, "\\");
    }
}
