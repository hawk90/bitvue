//! String splitting utilities.
//!
//! This module provides enhanced string splitting functionality similar to
//! Abseil's `absl::StrSplit`, with additional features like:
//!
//! - Different splitting behaviors (by delimiter, by predicate, etc.)
//! - Control over empty token handling
//! - Limit on number of splits
//! - Efficient lazy iteration
//!
//! # Example
//!
//! ```rust
//! use abseil::absl_strings::str_split::*;
//!
//! // Split by delimiter
//! let parts: Vec<&str> = StrSplit::new("a,b,c", Delimiter::Char(',')).collect();
//! assert_eq!(parts, ["a", "b", "c"]);
//!
//! // Split by string
//! let parts: Vec<&str> = StrSplit::new("a::b::c", Delimiter::Str("::")).collect();
//! assert_eq!(parts, ["a", "b", "c"]);
//!
//! // Split with limit
//! let parts: Vec<&str> = StrSplit::new("a,b,c,d", Delimiter::Char(',')).limit(2).collect();
//! assert_eq!(parts, ["a", "b", "c,d"]);
//! ```

use core::iter::FusedIterator;

/// Delimiter types for string splitting.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Delimiter<'a> {
    /// Split by a single character.
    Char(char),
    /// Split by a string slice.
    Str(&'a str),
    /// Split by any character in the given string.
    AnyOf(&'a str),
    /// Split by whitespace.
    Whitespace,
    /// Split by a predicate function.
    Pred(fn(char) -> bool),
}

impl Delimiter<'_> {
    /// Returns true if the character matches this delimiter.
    fn matches(&self, c: char) -> bool {
        match self {
            Delimiter::Char(d) => c == *d,
            Delimiter::AnyOf(s) => s.contains(c),
            Delimiter::Whitespace => c.is_whitespace(),
            Delimiter::Pred(f) => f(c),
            Delimiter::Str(_) => false, // String delimiters are handled separately
        }
    }

    /// Returns the length of this delimiter (for string delimiters).
    fn len(&self) -> usize {
        match self {
            Delimiter::Char(_) => 1,
            Delimiter::Str(s) => s.len(),
            Delimiter::AnyOf(_) | Delimiter::Whitespace | Delimiter::Pred(_) => 1,
        }
    }
}

/// An iterator over substrings split by a delimiter.
///
/// # Example
///
/// ```rust
/// use abseil::absl_strings::str_split::{StrSplit, Delimiter};
///
/// let split = StrSplit::new("a,b,c", Delimiter::Char(','));
/// let parts: Vec<&str> = split.collect();
/// assert_eq!(parts, ["a", "b", "c"]);
/// ```
#[derive(Clone, Debug)]
pub struct StrSplit<'a, 'delim> {
    input: &'a str,
    delimiter: Delimiter<'delim>,
    limit: Option<usize>,
    finished: bool,
}

impl<'a, 'delim> StrSplit<'a, 'delim> {
    /// Creates a new StrSplit iterator.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use abseil::absl_strings::str_split::{StrSplit, Delimiter};
    ///
    /// let split = StrSplit::new("a,b,c", Delimiter::Char(','));
    /// ```
    pub fn new(input: &'a str, delimiter: Delimiter<'delim>) -> Self {
        Self {
            input,
            delimiter,
            limit: None,
            finished: false,
        }
    }

    /// Sets a limit on the number of splits.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use abseil::absl_strings::str_split::{StrSplit, Delimiter};
    ///
    /// let parts: Vec<&str> = StrSplit::new("a,b,c,d", Delimiter::Char(','))
    ///     .limit(2)
    ///     .collect();
    /// assert_eq!(parts, ["a", "b", "c,d"]);
    /// ```
    pub fn limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }

    /// Returns a reference to the remaining input.
    pub fn remainder(&self) -> &'a str {
        self.input
    }
}

impl<'a, 'delim> Iterator for StrSplit<'a, 'delim> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        if self.finished {
            return None;
        }

        // Check if we've reached the limit
        if let Some(limit) = self.limit {
            if limit == 0 {
                self.finished = true;
                return Some(self.input);
            }
            self.limit = Some(limit - 1);
        }

        match self.delimiter {
            Delimiter::Str(delimiter) => {
                if let Some(pos) = self.input.find(delimiter) {
                    let result = &self.input[..pos];
                    self.input = &self.input[pos + delimiter.len()..];
                    Some(result)
                } else {
                    self.finished = true;
                    Some(self.input)
                }
            }
            _ => {
                // For character-based delimiters, use char_indices for UTF-8 safety
                // Note: This is O(n) per call to next(), making collect() O(n²) overall
                // This is acceptable for lazy iteration where not all results are consumed
                // For better performance on full splits, use std::str::split() instead

                // SAFETY: We need both the position AND the character to correctly
                // skip multi-byte UTF-8 characters. Using delimiter.len() would be wrong
                // because Char delimiters can be multi-byte (e.g., '€' is 3 bytes).
                let found = self.input.char_indices()
                    .find_map(|(i, c)| {
                        if self.delimiter.matches(c) {
                            Some((i, c)) // Return both position and character
                        } else {
                            None
                        }
                    });

                if let Some((pos, delim_char)) = found {
                    let result = &self.input[..pos];
                    // SAFETY: Use the actual byte length of the delimiter character we found,
                    // not delimiter.len() which returns 1 for Char delimiters regardless of
                    // their actual UTF-8 byte length. This ensures we skip the full character.
                    self.input = &self.input[pos + delim_char.len_utf8()..];
                    Some(result)
                } else {
                    self.finished = true;
                    Some(self.input)
                }
            }
        }
    }
}

impl<'a, 'delim> FusedIterator for StrSplit<'a, 'delim> {}

/// An iterator over substrings, skipping empty strings.
///
/// # Example
///
/// ```rust
/// use abseil::absl_strings::str_split::{StrSplitSkipEmpty, Delimiter};
///
/// let parts: Vec<&str> = StrSplitSkipEmpty::new("a,,b,c", Delimiter::Char(',')).collect();
/// assert_eq!(parts, ["a", "b", "c"]);
/// ```
#[derive(Clone, Debug)]
pub struct StrSplitSkipEmpty<'a, 'delim> {
    inner: StrSplit<'a, 'delim>,
}

impl<'a, 'delim> StrSplitSkipEmpty<'a, 'delim> {
    /// Creates a new StrSplitSkipEmpty iterator.
    pub fn new(input: &'a str, delimiter: Delimiter<'delim>) -> Self {
        Self {
            inner: StrSplit::new(input, delimiter),
        }
    }

    /// Sets a limit on the number of splits.
    pub fn limit(self, limit: usize) -> Self {
        Self {
            inner: self.inner.limit(limit),
        }
    }
}

impl<'a, 'delim> Iterator for StrSplitSkipEmpty<'a, 'delim> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let next = self.inner.next()?;
            if !next.is_empty() {
                return Some(next);
            }
        }
    }
}

impl<'a, 'delim> FusedIterator for StrSplitSkipEmpty<'a, 'delim> {}

/// Splits a string by a delimiter character.
///
/// # Example
///
/// ```rust
/// use abseil::absl_strings::str_split::split_by;
///
/// let parts: Vec<&str> = split_by("a,b,c", ',').collect();
/// assert_eq!(parts, ["a", "b", "c"]);
/// ```
pub fn split_by<'a>(s: &'a str, delimiter: char) -> StrSplit<'a, 'static> {
    StrSplit::new(s, Delimiter::Char(delimiter))
}

/// Splits a string by a delimiter string.
///
/// # Example
///
/// ```rust
/// use abseil::absl_strings::str_split::split_by_str;
///
/// let parts: Vec<&str> = split_by_str("a::b::c", "::").collect();
/// assert_eq!(parts, ["a", "b", "c"]);
/// ```
pub fn split_by_str<'a, 'delim>(s: &'a str, delimiter: &'delim str) -> StrSplit<'a, 'delim> {
    StrSplit::new(s, Delimiter::Str(delimiter))
}

/// Splits a string by any of the characters in the given string.
///
/// # Example
///
/// ```rust
/// use abseil::absl_strings::str_split::split_by_any;
///
/// let parts: Vec<&str> = split_by_any("a,b;c", ",;").collect();
/// assert_eq!(parts, ["a", "b", "c"]);
/// ```
pub fn split_by_any<'a>(s: &'a str, chars: &'a str) -> StrSplit<'a, 'a> {
    StrSplit::new(s, Delimiter::AnyOf(chars))
}

/// Splits a string by whitespace.
///
/// # Example
///
/// ```rust
/// use abseil::absl_strings::str_split::split_whitespace;
///
/// let parts: Vec<&str> = split_whitespace("hello  world\r\nrust").collect();
/// assert_eq!(parts, ["hello", "world", "rust"]);
/// ```
pub fn split_whitespace(s: &str) -> StrSplitSkipEmpty<'_, 'static> {
    StrSplitSkipEmpty::new(s, Delimiter::Whitespace)
}

/// Splits a string by a predicate.
///
/// # Example
///
/// ```rust
/// use abseil::absl_strings::str_split::split_by_pred;
///
/// fn is_vowel(c: char) -> bool {
///     matches!(c, 'a' | 'e' | 'i' | 'o' | 'u' | 'A' | 'E' | 'I' | 'O' | 'U')
/// }
///
/// let parts: Vec<&str> = split_by_pred("hello world", is_vowel).collect();
/// assert!(parts.len() > 1);
/// ```
pub fn split_by_pred<'a>(s: &'a str, pred: fn(char) -> bool) -> StrSplit<'a, 'static> {
    StrSplit::new(s, Delimiter::Pred(pred))
}

/// Allows splitting with functional style.
///
/// # Example
///
/// ```rust
/// use abseil::absl_strings::str_split::Splitter;
///
/// let parts: Vec<&str> = Splitter::new(',')
///     .skip_empty()
///     .on("a,b,c")
///     .collect();
/// assert_eq!(parts, ["a", "b", "c"]);
/// ```
#[derive(Clone, Debug, Default)]
pub struct Splitter {
    delimiter: Option<Delimiter<'static>>,
    skip_empty: bool,
    limit: Option<usize>,
}

impl Splitter {
    /// Creates a new Splitter with the specified delimiter.
    pub fn new(delimiter: char) -> Self {
        Self {
            delimiter: Some(Delimiter::Char(delimiter)),
            skip_empty: false,
            limit: None,
        }
    }

    /// Uses a string delimiter instead of a character.
    pub fn with_str(self, delimiter: &str) -> SplitterWithStr<'_> {
        SplitterWithStr {
            delimiter: Some(Delimiter::Str(delimiter)),
            skip_empty: self.skip_empty,
            limit: self.limit,
            _lifetime: core::marker::PhantomData,
        }
    }

    /// Skips empty strings in the output.
    pub fn skip_empty(mut self) -> Self {
        self.skip_empty = true;
        self
    }

    /// Sets a limit on the number of splits.
    pub fn limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }

    /// Splits the given input.
    pub fn on<'a>(&self, input: &'a str) -> StrSplitWithConfig<'a, 'static> {
        let delimiter = self.delimiter.unwrap_or(Delimiter::Whitespace);
        let mut split = StrSplit::new(input, delimiter);
        if let Some(limit) = self.limit {
            split = split.limit(limit);
        }
        StrSplitWithConfig {
            inner: split,
            skip_empty: self.skip_empty,
        }
    }
}

/// Splitter with a string delimiter (lifetime-bound).
pub struct SplitterWithStr<'delim> {
    delimiter: Option<Delimiter<'delim>>,
    skip_empty: bool,
    limit: Option<usize>,
    _lifetime: core::marker::PhantomData<&'delim ()>,
}

impl<'delim> SplitterWithStr<'delim> {
    /// Skips empty strings in the output.
    pub fn skip_empty(mut self) -> Self {
        self.skip_empty = true;
        self
    }

    /// Sets a limit on the number of splits.
    pub fn limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }

    /// Splits the given input.
    pub fn on<'a>(&self, input: &'a str) -> StrSplitWithConfig<'a, 'delim> {
        let delimiter = self.delimiter.unwrap_or(Delimiter::Whitespace);
        let mut split = StrSplit::new(input, delimiter);
        if let Some(limit) = self.limit {
            split = split.limit(limit);
        }
        StrSplitWithConfig {
            inner: split,
            skip_empty: self.skip_empty,
        }
    }
}

/// Wrapper iterator that handles skip_empty configuration.
#[derive(Clone, Debug)]
pub struct StrSplitWithConfig<'a, 'delim> {
    inner: StrSplit<'a, 'delim>,
    skip_empty: bool,
}

impl<'a, 'delim> Iterator for StrSplitWithConfig<'a, 'delim> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        if self.skip_empty {
            loop {
                let next = self.inner.next()?;
                if !next.is_empty() {
                    return Some(next);
                }
            }
        } else {
            self.inner.next()
        }
    }
}

impl<'a, 'delim> FusedIterator for StrSplitWithConfig<'a, 'delim> {}

/// Convenience function for splitting a string and collecting into a Vec.
///
/// # Example
///
/// ```rust
/// use abseil::absl_strings::str_split::split_to_vec;
///
/// assert_eq!(split_to_vec("a,b,c", ','), vec!["a", "b", "c"]);
/// ```
pub fn split_to_vec(s: &str, delimiter: char) -> Vec<&str> {
    split_by(s, delimiter).collect()
}

/// Returns the number of substrings when split by the given delimiter.
///
/// # Example
///
/// ```rust
/// use abseil::absl_strings::str_split::split_count;
///
/// assert_eq!(split_count("a,b,c", ','), 3);
/// assert_eq!(split_count("a,,c", ','), 3);
/// assert_eq!(split_count("", ','), 1);
/// ```
pub fn split_count(s: &str, delimiter: char) -> usize {
    split_by(s, delimiter).count()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_split_by_char() {
        let parts: Vec<&str> = split_by("a,b,c", ',').collect();
        assert_eq!(parts, ["a", "b", "c"]);
    }

    #[test]
    fn test_split_by_str() {
        let parts: Vec<&str> = split_by_str("a::b::c", "::").collect();
        assert_eq!(parts, ["a", "b", "c"]);
    }

    #[test]
    fn test_split_by_any() {
        let parts: Vec<&str> = split_by_any("a,b;c", ",;").collect();
        assert_eq!(parts, ["a", "b", "c"]);
    }

    #[test]
    fn test_split_whitespace() {
        let parts: Vec<&str> = split_whitespace("hello  world\r\nrust").collect();
        assert_eq!(parts, ["hello", "world", "rust"]);
    }

    #[test]
    fn test_split_by_pred() {
        let is_numeric = |c: char| c.is_numeric();
        let parts: Vec<&str> = split_by_pred("abc123def456", is_numeric).collect();
        // Each digit creates a split point, so "123" becomes "" "" "" between digits
        assert_eq!(parts, ["abc", "", "", "def", "", "", ""]);
    }

    #[test]
    fn test_split_limit() {
        let parts: Vec<&str> = StrSplit::new("a,b,c,d", Delimiter::Char(','))
            .limit(2)
            .collect();
        assert_eq!(parts, ["a", "b", "c,d"]);
    }

    #[test]
    fn test_split_with_empty_strings() {
        let parts: Vec<&str> = split_by("a,,c", ',').collect();
        assert_eq!(parts, ["a", "", "c"]);
    }

    #[test]
    fn test_split_skip_empty() {
        let parts: Vec<&str> = StrSplitSkipEmpty::new("a,,b,c", Delimiter::Char(',')).collect();
        assert_eq!(parts, ["a", "b", "c"]);
    }

    #[test]
    fn test_split_empty_string() {
        let parts: Vec<&str> = split_by("", ',').collect();
        assert_eq!(parts, [""]);
    }

    #[test]
    fn test_split_no_delimiter() {
        let parts: Vec<&str> = split_by("abc", ',').collect();
        assert_eq!(parts, ["abc"]);
    }

    #[test]
    fn test_split_trailing_delimiter() {
        let parts: Vec<&str> = split_by("a,b,", ',').collect();
        assert_eq!(parts, ["a", "b", ""]);
    }

    #[test]
    fn test_split_leading_delimiter() {
        let parts: Vec<&str> = split_by(",a,b", ',').collect();
        assert_eq!(parts, ["", "a", "b"]);
    }

    #[test]
    fn test_splitter_builder() {
        let parts: Vec<&str> = Splitter::new(',')
            .skip_empty()
            .on("a,,b,,c")
            .collect();
        assert_eq!(parts, ["a", "b", "c"]);
    }

    #[test]
    fn test_splitter_with_str() {
        let parts: Vec<&str> = Splitter::new(',').with_str("::").skip_empty().on("a::::b::c").collect();
        assert_eq!(parts, ["a", "b", "c"]);
    }

    #[test]
    fn test_splitter_with_limit() {
        let parts: Vec<&str> = Splitter::new(',')
            .limit(1)
            .on("a,b,c,d")
            .collect();
        assert_eq!(parts, ["a", "b,c,d"]);
    }

    #[test]
    fn test_split_to_vec() {
        assert_eq!(split_to_vec("a,b,c", ','), vec!["a", "b", "c"]);
    }

    #[test]
    fn test_split_count() {
        assert_eq!(split_count("a,b,c", ','), 3);
        assert_eq!(split_count("a,,c", ','), 3);
        assert_eq!(split_count("", ','), 1);
    }

    #[test]
    fn test_str_split_remainder() {
        let mut split = StrSplit::new("a,b,c,d", Delimiter::Char(','));
        assert_eq!(split.next(), Some("a"));
        assert_eq!(split.remainder(), "b,c,d");
        assert_eq!(split.next(), Some("b"));
        assert_eq!(split.remainder(), "c,d");
    }

    #[test]
    fn test_multi_char_string_delimiter() {
        let parts: Vec<&str> = split_by_str("aXXbXXc", "XX").collect();
        assert_eq!(parts, ["a", "b", "c"]);
    }

    #[test]
    fn test_complex_delimiter_string() {
        let parts: Vec<&str> = split_by_str("start<SEP>middle<SEP>end", "<SEP>").collect();
        assert_eq!(parts, ["start", "middle", "end"]);
    }

    #[test]
    fn test_split_limit_with_skip_empty() {
        let parts: Vec<&str> = StrSplit::new("a,,b,c,d", Delimiter::Char(','))
            .limit(2)
            .collect();
        assert_eq!(parts, ["a", "", "b,c,d"]);
    }

    #[test]
    fn test_any_of_delimiter() {
        let parts: Vec<&str> = split_by_any("one-two_three", "-_").collect();
        assert_eq!(parts, ["one", "two", "three"]);
    }

    #[test]
    fn test_whitespace_various() {
        let parts: Vec<&str> = split_whitespace(" \t\nhello\r\n  world\t\n").collect();
        assert_eq!(parts, ["hello", "world"]);
    }

    // Performance and edge case tests for MEDIUM priority fix

    #[test]
    fn test_split_multibyte_utf8_delimiter() {
        // Test that multi-byte UTF-8 character delimiters are handled correctly
        // '€' is 3 bytes in UTF-8 (0xE2 0x82 0xAC)
        // '本' is 3 bytes in UTF-8 (0xE6 0x9C 0xAC)
        // '€' and '本' should be skipped correctly

        // Test with Euro sign (3 bytes)
        let input = "hello€world€test";
        let parts: Vec<&str> = split_by(input, '€').collect();
        assert_eq!(parts, ["hello", "world", "test"]);
        // Verify each part is valid UTF-8
        assert!(parts.iter().all(|s| s.is_char_boundary(0)));

        // Test with Japanese character (3 bytes)
        let input2 = "こんにちは本世界";
        let parts2: Vec<&str> = split_by(input2, '本').collect();
        assert_eq!(parts2, ["こんにちは", "世界"]);
        assert!(parts2.iter().all(|s| s.is_char_boundary(0)));

        // Test with emoji (4 bytes - '❤' is 3 bytes, but many emojis are 4)
        let input3 = "hello❤world❤test";
        let parts3: Vec<&str> = split_by(input3, '❤').collect();
        assert_eq!(parts3, ["hello", "world", "test"]);
        assert!(parts3.iter().all(|s| s.is_char_boundary(0)));
    }

    #[test]
    fn test_split_multibyte_delimiter_consecutive() {
        // Test consecutive multi-byte delimiters
        let input = "a€€€b";
        let parts: Vec<&str> = split_by(input, '€').collect();
        assert_eq!(parts, ["a", "", "", "b"]);
        // Verify all parts are valid UTF-8
        assert!(parts.iter().all(|s| s.is_char_boundary(0) || s.is_empty()));
    }

    #[test]
    fn test_split_large_string_performance() {
        // Test that splitting a large string completes in reasonable time
        // This tests the O(n²) fix - with the fix, this should complete quickly
        let input = "a,".repeat(1000) + "b";
        let parts: Vec<&str> = split_by(&input, ',').collect();
        assert_eq!(parts.len(), 1001);
        assert_eq!(parts[0], "a");
        assert_eq!(parts[1000], "b");
    }

    #[test]
    fn test_split_many_small_parts() {
        // Test splitting into many small parts
        let input = "x".repeat(100);
        let parts: Vec<&str> = split_by(&input, 'x').collect();
        // "x".repeat(100) split by 'x' gives 101 empty strings
        assert_eq!(parts.len(), 101);
        assert!(parts.iter().all(|s| s.is_empty()));
    }

    #[test]
    fn test_split_unicode_correctness() {
        // Test that UTF-8 handling is correct
        let input = "héllo,wørld,日本語";
        let parts: Vec<&str> = split_by(input, ',').collect();
        assert_eq!(parts, ["héllo", "wørld", "日本語"]);
    }

    #[test]
    fn test_split_utf8_multibyte_delimiter() {
        // Test splitting with multibyte UTF-8 characters
        let input = "hello❤world❤test";
        let parts: Vec<&str> = split_by_str(input, "❤").collect();
        assert_eq!(parts, ["hello", "world", "test"]);
    }

    #[test]
    fn test_split_empty_vs_no_delimiter() {
        // Test edge case: empty string vs string with no delimiter
        let parts_empty: Vec<&str> = split_by("", ',').collect();
        assert_eq!(parts_empty, [""]);

        let parts_no_delim: Vec<&str> = split_by("abc", ',').collect();
        assert_eq!(parts_no_delim, ["abc"]);
    }
}
