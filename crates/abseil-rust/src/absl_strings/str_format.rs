//! String formatting utilities.
//!
//! This module provides type-safe string formatting with a syntax similar to
//! Abseil's `absl::StrFormat`.
//!
//! # Example
//!
//! ```rust
//! use abseil::absl_strings::str_format::{Spec, Stream};
//!
//! // Using Spec (similar to absl::StrFormat)
//! let spec = Spec::new();
//! let args: &[&dyn std::fmt::Display] = &[&"world"];
//! assert_eq!(spec.format("Hello, {}!", args), "Hello, world!");
//!
//! // Using Stream for building strings incrementally
//! let stream = Stream::new().append("Hello").append(", ").append("world!");
//! assert_eq!(stream.to_string(), "Hello, world!");
//!
//! // Using the macro (import from crate root)
//! // use abseil::str_format;
//! // assert_eq!(str_format!("{} + {} = {}", 1, 2, 3), "1 + 2 = 3");
//! ```

use core::fmt;

/// Format specification for string formatting.
///
/// This struct provides methods for building formatted strings.
#[derive(Debug, Clone, Default)]
pub struct Spec {
    args: Vec<FormatArg>,
}

impl Spec {
    /// Creates a new empty Spec.
    pub const fn new() -> Self {
        Self { args: Vec::new() }
    }

    /// Formats a string with the given arguments.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use abseil::absl_strings::str_format::Spec;
    ///
    /// let spec = Spec::new();
    /// let args: &[&dyn std::fmt::Display] = &[&"world"];
    /// assert_eq!(spec.format("Hello, {}!", args), "Hello, world!");
    /// let args: &[&dyn std::fmt::Display] = &[&"1", &"2", &"3"];
    /// assert_eq!(spec.format("{} + {} = {}", args), "1 + 2 = 3");
    /// ```
    ///
    /// # Security
    ///
    /// This function validates that the number of placeholders matches the number
    /// of arguments to prevent format string injection attacks.
    pub fn format(&self, pattern: &str, args: &[&dyn fmt::Display]) -> String {
        let mut result = String::with_capacity(pattern.len() * 2);
        let bytes = pattern.as_bytes();
        let mut i = 0;
        let mut arg_idx = 0;
        let mut placeholder_count = 0;

        // First pass: count placeholders (excluding escaped {{)
        while i < bytes.len() {
            if bytes[i] == b'{' {
                // Check for escaped brace {{
                if i + 1 < bytes.len() && bytes[i + 1] == b'{' {
                    i += 2;
                    continue;
                }
                // Count potential placeholder
                placeholder_count += 1;
                i += 1;
                // Skip to after the closing brace if present
                while i < bytes.len() && bytes[i] != b'}' {
                    i += 1;
                }
                if i < bytes.len() {
                    i += 1;
                }
            } else {
                i += 1;
            }
        }

        // Validate argument count
        if placeholder_count != args.len() {
            return format!("<format error: {} placeholders, {} args>",
                placeholder_count, args.len());
        }

        // Second pass: actual formatting
        i = 0;
        while i < bytes.len() {
            if bytes[i] == b'{' {
                // Check for escaped brace {{
                if i + 1 < bytes.len() && bytes[i + 1] == b'{' {
                    result.push('{');
                    i += 2;
                    continue;
                }
                // Format placeholder - find closing }
                if i + 1 < bytes.len() && bytes[i + 1] == b'}' {
                    // Only use argument if we haven't exceeded args
                    if arg_idx < args.len() {
                        if let Some(&arg) = args.get(arg_idx) {
                            result.push_str(&format!("{}", arg));
                        }
                        arg_idx += 1;
                    }
                    i += 2;
                    continue;
                }
            }
            if bytes[i] == b'}' {
                // Check for escaped brace }}
                if i + 1 < bytes.len() && bytes[i + 1] == b'}' {
                    result.push('}');
                    i += 2;
                    continue;
                }
                // Unmatched closing brace - ignore for safety
            }
            // Regular character
            result.push(bytes[i] as char);
            i += 1;
        }

        result
    }
}

/// Format argument wrapper.
#[derive(Clone)]
pub struct FormatArg {
    value: String,
}

impl FormatArg {
    /// Creates a new FormatArg from any Display type.
    pub fn new<T: fmt::Display>(value: T) -> Self {
        Self {
            value: format!("{}", value),
        }
    }
}

impl fmt::Display for FormatArg {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.value)
    }
}

impl fmt::Debug for FormatArg {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.value)
    }
}

/// String formatting macro.
///
/// This macro provides a convenient way to format strings with arguments.
///
/// # Examples
///
/// ```rust
/// use abseil::str_format;
///
/// assert_eq!(str_format!("Hello, {}!", "world"), "Hello, world!");
/// assert_eq!(str_format!("{} + {} = {}", 1, 2, 3), "1 + 2 = 3");
/// assert_eq!(str_format!("π ≈ {:.2}", 3.14159), "π ≈ 3.14");
/// ```
#[macro_export]
macro_rules! str_format {
    ($($arg:tt)*) => {{
        format!($($arg)*)
    }};
}

/// Appends formatted text to a string.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_strings::str_format::append;
///
/// let mut s = String::from("Hello");
/// let args: &[&dyn std::fmt::Display] = &[&"world"];
/// append(&mut s, ", {}!", args);
/// assert_eq!(s, "Hello, world!");
/// ```
pub fn append(dst: &mut String, pattern: &str, args: &[&dyn fmt::Display]) {
    dst.push_str(&Spec::new().format(pattern, args));
}

/// Appends a single formatted value to a string.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_strings::str_format::appendv;
///
/// let mut s = String::from("Count: ");
/// appendv(&mut s, 42);
/// assert_eq!(s, "Count: 42");
/// ```
pub fn appendv<T: fmt::Display>(dst: &mut String, value: T) {
    dst.push_str(&format!("{}", value));
}

/// Returns the length of the formatted string without actually creating it.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_strings::str_format::formatted_len;
///
/// let args: &[&dyn std::fmt::Display] = &[&"world"];
/// assert_eq!(formatted_len("Hello, {}!", args), 13);
/// ```
pub fn formatted_len(pattern: &str, args: &[&dyn fmt::Display]) -> usize {
    Spec::new().format(pattern, args).len()
}

/// Formats arguments with a specified delimiter between each.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_strings::str_format::format_with;
///
/// assert_eq!(format_with(", ", &["a", "b", "c"]), "a, b, c");
/// assert_eq!(format_with(" - ", &[1, 2, 3]), "1 - 2 - 3");
/// ```
pub fn format_with<T: fmt::Display>(delimiter: &str, items: &[T]) -> String {
    if items.is_empty() {
        return String::new();
    }

    let mut result = String::new();
    for (i, item) in items.iter().enumerate() {
        if i > 0 {
            result.push_str(delimiter);
        }
        result.push_str(&format!("{}", item));
    }
    result
}

/// Formats arguments with a prefix and suffix.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_strings::str_format::format_delimited;
///
/// assert_eq!(format_delimited("[", "]", ", ", &[1, 2, 3]), "[1, 2, 3]");
/// assert_eq!(format_delimited("(", ")", " & ", &["a", "b"]), "(a & b)");
/// ```
pub fn format_delimited<T: fmt::Display>(
    prefix: &str,
    suffix: &str,
    delimiter: &str,
    items: &[T],
) -> String {
    if items.is_empty() {
        return format!("{}{}", prefix, suffix);
    }

    let mut result = String::from(prefix);
    for (i, item) in items.iter().enumerate() {
        if i > 0 {
            result.push_str(delimiter);
        }
        result.push_str(&format!("{}", item));
    }
    result.push_str(suffix);
    result
}

/// Stream-like formatting adapter.
///
/// This allows building formatted strings incrementally.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_strings::str_format::Stream;
///
/// let stream = Stream::new()
///     .append("Hello")
///     .append(", ")
///     .append("world!");
/// assert_eq!(stream.to_string(), "Hello, world!");
/// ```
#[derive(Debug, Clone, Default)]
pub struct Stream {
    buffer: String,
}

impl Stream {
    /// Creates a new empty Stream.
    pub fn new() -> Self {
        Self {
            buffer: String::new(),
        }
    }

    /// Appends a value to the stream.
    pub fn append<T: fmt::Display>(mut self, value: T) -> Self {
        self.buffer.push_str(&format!("{}", value));
        self
    }

    /// Appends a formatted string to the stream.
    pub fn append_format(mut self, pattern: &str, args: &[&dyn fmt::Display]) -> Self {
        self.buffer.push_str(&Spec::new().format(pattern, args));
        self
    }

    /// Returns the current length of the stream.
    pub fn len(&self) -> usize {
        self.buffer.len()
    }

    /// Returns true if the stream is empty.
    pub fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }

    /// Clears the stream.
    pub fn clear(&mut self) {
        self.buffer.clear();
    }

    /// Converts the stream to a String.
    pub fn to_string(&self) -> String {
        self.buffer.clone()
    }
}

impl fmt::Display for Stream {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.buffer)
    }
}

/// Converts a size in bytes to a human-readable string.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_strings::str_format::human_readable_size;
///
/// assert_eq!(human_readable_size(0), "0 B");
/// assert_eq!(human_readable_size(1024), "1.00 KiB");
/// assert_eq!(human_readable_size(1024 * 1024), "1.00 MiB");
/// assert_eq!(human_readable_size(1536), "1.50 KiB");
/// ```
pub fn human_readable_size(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KiB", "MiB", "GiB", "TiB", "PiB", "EiB"];

    if bytes == 0 {
        return "0 B".to_string();
    }

    let mut size = bytes as f64;
    let mut unit_index = 0;

    while size >= 1024.0 && unit_index < UNITS.len() - 1 {
        size /= 1024.0;
        unit_index += 1;
    }

    if unit_index == 0 {
        format!("{} {}", bytes, UNITS[0])
    } else {
        format!("{:.2} {}", size, UNITS[unit_index])
    }
}

/// Converts a duration in seconds to a human-readable string.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_strings::str_format::human_readable_duration;
///
/// assert_eq!(human_readable_duration(0.0), "0s");
/// assert_eq!(human_readable_duration(1.5), "1.5s");
/// assert_eq!(human_readable_duration(65.0), "1m 5s");
/// assert_eq!(human_readable_duration(3665.0), "1h 1m 5s");
/// ```
pub fn human_readable_duration(seconds: f64) -> String {
    if seconds == 0.0 {
        return "0s".to_string();
    }

    let mut remaining = seconds;
    let mut parts = Vec::new();

    let hours = (remaining / 3600.0).floor() as i64;
    if hours > 0 {
        parts.push(format!("{}h", hours));
        remaining -= hours as f64 * 3600.0;
    }

    let minutes = (remaining / 60.0).floor() as i64;
    if minutes > 0 {
        parts.push(format!("{}m", minutes));
        remaining -= minutes as f64 * 60.0;
    }

    // Always show seconds when we have hours/minutes, or when non-zero
    let secs = remaining;
    if secs > 0.0 || parts.is_empty() || !parts.is_empty() {
        if secs.fract() == 0.0 {
            parts.push(format!("{}s", secs as i64));
        } else {
            parts.push(format!("{:.1}s", secs));
        }
    }

    parts.join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spec_format() {
        let spec = Spec::new();
        let args: &[&dyn fmt::Display] = &[&"world"];
        assert_eq!(spec.format("Hello, {}!", args), "Hello, world!");
        let args: &[&dyn fmt::Display] = &[&"1", &"2", &"3"];
        assert_eq!(
            spec.format("{} + {} = {}", args),
            "1 + 2 = 3"
        );
    }

    #[test]
    fn test_spec_format_numbers() {
        let spec = Spec::new();
        let args: &[&dyn fmt::Display] = &[&"42"];
        assert_eq!(
            spec.format("Value: {}", args),
            "Value: 42"
        );
    }

    #[test]
    fn test_format_arg() {
        let arg = FormatArg::new(42);
        assert_eq!(format!("{}", arg), "42");
    }

    #[test]
    fn test_append() {
        let mut s = String::from("Hello");
        let args: &[&dyn fmt::Display] = &[&"world"];
        append(&mut s, ", {}!", args);
        assert_eq!(s, "Hello, world!");
    }

    #[test]
    fn test_appendv() {
        let mut s = String::from("Count: ");
        appendv(&mut s, 42);
        assert_eq!(s, "Count: 42");
    }

    #[test]
    fn test_formatted_len() {
        let args: &[&dyn fmt::Display] = &[&"world"];
        assert_eq!(formatted_len("Hello, {}!", args), 13);
        let args: &[&dyn fmt::Display] = &[&"test"];
        assert_eq!(formatted_len("{}", args), 4);
    }

    #[test]
    fn test_format_with() {
        assert_eq!(format_with(", ", &["a", "b", "c"] as &[&str]), "a, b, c");
        assert_eq!(format_with(" - ", &[1, 2, 3] as &[i32]), "1 - 2 - 3");
        assert_eq!(format_with(", ", &["single"] as &[&str]), "single");
    }

    #[test]
    fn test_format_with_empty() {
        let args: &[&str] = &[];
        assert_eq!(format_with(", ", args), "");
    }

    #[test]
    fn test_format_delimited() {
        assert_eq!(format_delimited("[", "]", ", ", &[1, 2, 3] as &[i32]), "[1, 2, 3]");
        assert_eq!(format_delimited("(", ")", " & ", &["a", "b"] as &[&str]), "(a & b)");
    }

    #[test]
    fn test_format_delimited_empty() {
        assert_eq!(format_delimited("[", "]", ", ", &[] as &[i32]), "[]");
    }

    #[test]
    fn test_stream() {
        let stream = Stream::new()
            .append("Hello")
            .append(", ")
            .append("world!");

        assert_eq!(stream.to_string(), "Hello, world!");
        assert_eq!(stream.len(), 13);
        assert!(!stream.is_empty());
    }

    #[test]
    fn test_stream_empty() {
        let stream = Stream::new();
        assert_eq!(stream.to_string(), "");
        assert_eq!(stream.len(), 0);
        assert!(stream.is_empty());
    }

    #[test]
    fn test_stream_clear() {
        let mut stream = Stream::new().append("Hello");
        assert_eq!(stream.len(), 5);
        stream.clear();
        assert_eq!(stream.len(), 0);
        assert!(stream.is_empty());
    }

    #[test]
    fn test_human_readable_size() {
        assert_eq!(human_readable_size(0), "0 B");
        assert_eq!(human_readable_size(1), "1 B");
        assert_eq!(human_readable_size(512), "512 B");
        assert_eq!(human_readable_size(1024), "1.00 KiB");
        assert_eq!(human_readable_size(1536), "1.50 KiB");
        assert_eq!(human_readable_size(1024 * 1024), "1.00 MiB");
        assert_eq!(human_readable_size(1024 * 1024 * 1024), "1.00 GiB");
    }

    #[test]
    fn test_human_readable_duration() {
        assert_eq!(human_readable_duration(0.0), "0s");
        assert_eq!(human_readable_duration(1.5), "1.5s");
        assert_eq!(human_readable_duration(60.0), "1m 0s");
        assert_eq!(human_readable_duration(65.0), "1m 5s");
        assert_eq!(human_readable_duration(3665.0), "1h 1m 5s");
        assert_eq!(human_readable_duration(3600.0), "1h 0s");
    }

    #[test]
    fn test_stream_display() {
        let stream = Stream::new().append("test");
        assert_eq!(format!("{}", stream), "test");
    }
}
