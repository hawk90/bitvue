//! Character set matching utilities.
//!
//! This module provides character set matching similar to Abseil's `absl::strings::CharSet`.
//!
//! # Example
//!
//! ```rust
//! use abseil::absl_strings::charset::{CharSet, CharSetBuilder};
//!
//! let alnum = CharSet::Alnum;
//! assert!(alnum.matches('A'));
//! assert!(alnum.matches('5'));
//! assert!(!alnum.matches(' '));
//!
//! // Create custom charset
//! let custom = CharSetBuilder::new().add_range('x', 'z').build();
//! assert!(custom.matches('x'));
//! ```

use core::fmt;
use std::collections::HashSet;

/// A set of characters for efficient matching.
///
/// CharSet uses an enum representation for common character sets
/// and a fallback for custom sets.
pub enum CharSet {
    /// Alphanumeric characters
    Alnum,
    /// Alphabetic characters
    Alpha,
    /// Digits
    Digit,
    /// Hexadecimal digits
    XDigit,
    /// Whitespace
    Whitespace,
    /// Uppercase letters
    Upper,
    /// Lowercase letters
    Lower,
    /// Punctuation
    Punctuation,
    /// Control characters
    Control,
    /// Graphical characters
    Graph,
    /// Printable characters
    Print,
    /// ASCII characters
    Ascii,
    /// Blank characters (space or tab)
    Blank,
    /// Custom character set (stores characters in a Vec)
    Custom(Vec<char>),
}

impl CharSet {
    /// Returns `true` if the character is in the set.
    ///
    /// # Example
    ///
    /// ```
    /// use abseil::absl_strings::charset::CharSet;
    ///
    /// let digits = CharSet::Digit;
    /// assert!(digits.matches('5'));
    /// assert!(!digits.matches('a'));
    /// ```
    #[inline]
    pub fn matches(&self, c: char) -> bool {
        match self {
            CharSet::Alnum => c.is_ascii_alphanumeric(),
            CharSet::Alpha => c.is_ascii_alphabetic(),
            CharSet::Digit => c.is_ascii_digit(),
            CharSet::XDigit => c.is_ascii_hexdigit(),
            CharSet::Whitespace => c.is_ascii_whitespace(),
            CharSet::Upper => c.is_ascii_uppercase(),
            CharSet::Lower => c.is_ascii_lowercase(),
            CharSet::Punctuation => {
                matches!(c, '!'..='/' | ':'..='@' | '['..='`' | '{'..='~')
            }
            CharSet::Control => (c as u8) < 32 || c == '\x7f',
            CharSet::Graph => matches!(c, '!'..='~'),
            CharSet::Print => matches!(c, ' ' | '!'..='~'),
            CharSet::Ascii => (c as u32) < 128,
            CharSet::Blank => c == ' ' || c == '\t',
            CharSet::Custom(chars) => chars.contains(&c),
        }
    }

    /// Returns `true` if all characters in the string match this set.
    ///
    /// # Example
    ///
    /// ```
    /// use abseil::absl_strings::charset::CharSet;
    ///
    /// let digits = CharSet::Digit;
    /// assert!(digits.matches_all("12345"));
    /// assert!(!digits.matches_all("123a5"));
    /// ```
    pub fn matches_all(&self, s: &str) -> bool {
        !s.is_empty() && s.chars().all(|c| self.matches(c))
    }

    /// Returns `true` if any character in the string matches this set.
    ///
    /// # Example
    ///
    /// ```
    /// use abseil::absl_strings::charset::CharSet;
    ///
    /// let digits = CharSet::Digit;
    /// assert!(digits.matches_any("abc123"));
    /// assert!(!digits.matches_any("abc"));
    /// ```
    pub fn matches_any(&self, s: &str) -> bool {
        s.chars().any(|c| self.matches(c))
    }

    /// Counts characters in the string that match this set.
    ///
    /// # Example
    ///
    /// ```
    /// use abseil::absl_strings::charset::{CharSet, CharSetBuilder};
    ///
    /// let vowels = CharSetBuilder::new().add_chars("aeiouAEIOU").build();
    /// assert_eq!(vowels.count("hello world"), 3); // e, o, o
    /// ```
    pub fn count(&self, s: &str) -> usize {
        s.chars().filter(|&c| self.matches(c)).count()
    }

    /// Finds the first character in the string that matches this set.
    ///
    /// # Example
    ///
    /// ```
    /// use abseil::absl_strings::charset::CharSet;
    ///
    /// let digits = CharSet::Digit;
    /// assert_eq!(digits.find("abc123"), Some('1'));
    /// assert_eq!(digits.find("abc"), None);
    /// ```
    pub fn find(&self, s: &str) -> Option<char> {
        s.chars().find(|&c| self.matches(c))
    }

    /// Finds the first character in the string that does NOT match this set.
    ///
    /// # Example
    ///
    /// ```
    /// use abseil::absl_strings::charset::CharSet;
    ///
    /// let digits = CharSet::Digit;
    /// assert_eq!(digits.find_not("123abc"), Some('a'));
    /// assert_eq!(digits.find_not("123"), None);
    /// ```
    pub fn find_not(&self, s: &str) -> Option<char> {
        s.chars().find(|&c| !self.matches(c))
    }

    /// Alphanumeric characters: [A-Za-z0-9].
    pub const fn alnum() -> Self {
        CharSet::Alnum
    }

    /// Alphabetic characters: [A-Za-z].
    pub const fn alpha() -> Self {
        CharSet::Alpha
    }

    /// Digits: [0-9].
    pub const fn digit() -> Self {
        CharSet::Digit
    }

    /// Hexadecimal digits: [0-9A-Fa-f].
    pub const fn xdigit() -> Self {
        CharSet::XDigit
    }

    /// Whitespace characters: space, tab, newline, return, form feed, vertical tab.
    pub const fn whitespace() -> Self {
        CharSet::Whitespace
    }

    /// Uppercase letters: [A-Z].
    pub const fn upper() -> Self {
        CharSet::Upper
    }

    /// Lowercase letters: [a-z].
    pub const fn lower() -> Self {
        CharSet::Lower
    }

    /// Punctuation characters: [!-/:-@[-`{-~].
    pub const fn punctuation() -> Self {
        CharSet::Punctuation
    }

    /// Control characters: [\x00-\x1F\x7F].
    pub const fn control() -> Self {
        CharSet::Control
    }

    /// Graphical characters (visible, printable except space).
    pub const fn graph() -> Self {
        CharSet::Graph
    }

    /// Printable characters (including space).
    pub const fn print() -> Self {
        CharSet::Print
    }

    /// ASCII characters (code point <= 127).
    pub const fn ascii() -> Self {
        CharSet::Ascii
    }

    /// Blank characters (space or tab only).
    pub const fn blank() -> Self {
        CharSet::Blank
    }

    /// Creates a builder for constructing a custom CharSet.
    pub fn builder() -> CharSetBuilder {
        CharSetBuilder::new()
    }

    /// Negates this character set, matching characters NOT in the set.
    ///
    /// Returns a custom CharSet with the negated character set.
    ///
    /// # Example
    ///
    /// ```
    /// use abseil::absl_strings::charset::CharSet;
    ///
    /// let digits = CharSet::digit();
    /// let not_digits = digits.negate();
    /// assert!(!not_digits.matches('5'));
    /// assert!(not_digits.matches('a'));
    /// ```
    pub fn negate(&self) -> Self {
        // Collect all ASCII characters that don't match this set
        // Use HashSet for O(1) membership tracking, then sort for deterministic output
        let mut seen = HashSet::new();
        for c in b'\x00'..=b'\x7f' {
            let c = c as char;
            if !self.matches(c) {
                seen.insert(c);
            }
        }
        let mut chars: Vec<char> = seen.into_iter().collect();
        chars.sort_unstable();
        CharSet::Custom(chars)
    }

    /// Combines two character sets with OR logic.
    ///
    /// A character matches if it's in either set.
    ///
    /// # Example
    ///
    /// ```
    /// use abseil::absl_strings::charset::CharSet;
    ///
    /// let alnum = CharSet::alpha().or(&CharSet::digit());
    /// assert!(alnum.matches('A'));
    /// assert!(alnum.matches('5'));
    /// assert!(!alnum.matches(' '));
    /// ```
    pub fn or(&self, other: &CharSet) -> Self {
        // Collect all characters that match either set
        // Use HashSet for O(1) deduplication
        let mut seen = HashSet::new();
        for c in b'\x00'..=b'\x7f' {
            let c = c as char;
            if self.matches(c) || other.matches(c) {
                seen.insert(c);
            }
        }
        let mut chars: Vec<char> = seen.into_iter().collect();
        chars.sort_unstable();
        CharSet::Custom(chars)
    }

    /// Combines two character sets with AND logic.
    ///
    /// A character matches only if it's in both sets.
    ///
    /// # Example
    ///
    /// ```
    /// use abseil::absl_strings::charset::{CharSet, CharSetBuilder};
    ///
    /// let lower = CharSet::lower();
    /// let vowels = CharSetBuilder::new().add_chars("aeiou").build();
    /// let lower_vowels = lower.and(&vowels);
    /// assert!(lower_vowels.matches('a'));
    /// assert!(lower_vowels.matches('e'));
    /// assert!(!lower_vowels.matches('A'));
    /// assert!(!lower_vowels.matches('x'));
    /// ```
    pub fn and(&self, other: &CharSet) -> Self {
        // Collect all characters that match both sets
        let mut seen = HashSet::new();
        for c in b'\x00'..=b'\x7f' {
            let c = c as char;
            if self.matches(c) && other.matches(c) {
                seen.insert(c);
            }
        }
        let mut chars: Vec<char> = seen.into_iter().collect();
        chars.sort_unstable();
        CharSet::Custom(chars)
    }

    /// Combines two character sets with XOR logic.
    ///
    /// A character matches if it's in exactly one of the sets.
    ///
    /// # Example
    ///
    /// ```
    /// use abseil::absl_strings::charset::{CharSet, CharSetBuilder};
    ///
    /// let lower = CharSet::lower();
    /// let vowels = CharSetBuilder::new().add_chars("aeiouAEIOU").build();
    /// let lower_xor_vowels = lower.xor(&vowels);
    /// assert!(!lower_xor_vowels.matches('a'));  // in both lower and vowels
    /// assert!(lower_xor_vowels.matches('E'));   // in vowels only (not lower)
    /// assert!(lower_xor_vowels.matches('b'));   // in lower only (not vowels)
    /// ```
    pub fn xor(&self, other: &CharSet) -> Self {
        // Collect all characters that match exactly one set
        let mut seen = HashSet::new();
        for c in b'\x00'..=b'\x7f' {
            let c = c as char;
            let in_self = self.matches(c);
            let in_other = other.matches(c);
            if in_self ^ in_other {
                seen.insert(c);
            }
        }
        let mut chars: Vec<char> = seen.into_iter().collect();
        chars.sort_unstable();
        CharSet::Custom(chars)
    }
}

impl fmt::Debug for CharSet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CharSet::Alnum => write!(f, "CharSet(Alnum)"),
            CharSet::Alpha => write!(f, "CharSet(Alpha)"),
            CharSet::Digit => write!(f, "CharSet(Digit)"),
            CharSet::XDigit => write!(f, "CharSet(XDigit)"),
            CharSet::Whitespace => write!(f, "CharSet(Whitespace)"),
            CharSet::Upper => write!(f, "CharSet(Upper)"),
            CharSet::Lower => write!(f, "CharSet(Lower)"),
            CharSet::Punctuation => write!(f, "CharSet(Punctuation)"),
            CharSet::Control => write!(f, "CharSet(Control)"),
            CharSet::Graph => write!(f, "CharSet(Graph)"),
            CharSet::Print => write!(f, "CharSet(Print)"),
            CharSet::Ascii => write!(f, "CharSet(Ascii)"),
            CharSet::Blank => write!(f, "CharSet(Blank)"),
            CharSet::Custom(chars) => write!(f, "CharSet(Custom({} chars))", chars.len()),
        }
    }
}

impl Clone for CharSet {
    fn clone(&self) -> Self {
        match self {
            CharSet::Alnum => CharSet::Alnum,
            CharSet::Alpha => CharSet::Alpha,
            CharSet::Digit => CharSet::Digit,
            CharSet::XDigit => CharSet::XDigit,
            CharSet::Whitespace => CharSet::Whitespace,
            CharSet::Upper => CharSet::Upper,
            CharSet::Lower => CharSet::Lower,
            CharSet::Punctuation => CharSet::Punctuation,
            CharSet::Control => CharSet::Control,
            CharSet::Graph => CharSet::Graph,
            CharSet::Print => CharSet::Print,
            CharSet::Ascii => CharSet::Ascii,
            CharSet::Blank => CharSet::Blank,
            CharSet::Custom(chars) => CharSet::Custom(chars.clone()),
        }
    }
}

/// A builder for creating character sets.
///
/// # Example
///
/// ```
/// use abseil::absl_strings::charset::CharSetBuilder;
///
/// let set = CharSetBuilder::new()
///     .add_range('a', 'z')
///     .add_range('A', 'Z')
///     .add_chars("0123456789")
///     .build();
///
/// assert!(set.matches('a'));
/// assert!(set.matches('Z'));
/// assert!(set.matches('5'));
/// assert!(!set.matches(' '));
/// ```
pub struct CharSetBuilder {
    chars: HashSet<char>,
}

impl CharSetBuilder {
    /// Creates a new CharSetBuilder.
    pub fn new() -> Self {
        Self {
            chars: HashSet::new(),
        }
    }

    /// Adds a range of characters to the set.
    ///
    /// This is O(range_size) instead of O(range_size * current_size).
    pub fn add_range(mut self, start: char, end: char) -> Self {
        for c in start..=end {
            self.chars.insert(c);
        }
        self
    }

    /// Adds specific characters to the set.
    ///
    /// This is O(chars.len()) instead of O(chars.len() * current_size).
    pub fn add_chars(mut self, chars: &str) -> Self {
        for c in chars.chars() {
            self.chars.insert(c);
        }
        self
    }

    /// Builds the CharSet.
    ///
    /// Converts the internal HashSet to a sorted Vec for deterministic ordering.
    pub fn build(self) -> CharSet {
        let mut chars: Vec<char> = self.chars.into_iter().collect();
        chars.sort_unstable();
        CharSet::Custom(chars)
    }
}

impl Default for CharSetBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Trait for character set matching operations.
///
/// This trait allows different types to implement character set matching.
pub trait CharSetMatcher {
    /// Returns true if the character is in the set.
    fn matches(&self, c: char) -> bool;

    /// Returns true if all characters in the string match this set.
    fn matches_all(&self, s: &str) -> bool {
        !s.is_empty() && s.chars().all(|c| self.matches(c))
    }

    /// Returns true if any character in the string matches this set.
    fn matches_any(&self, s: &str) -> bool {
        s.chars().any(|c| self.matches(c))
    }

    /// Counts characters in the string that match this set.
    fn count(&self, s: &str) -> usize {
        s.chars().filter(|&c| self.matches(c)).count()
    }
}

impl CharSetMatcher for CharSet {
    fn matches(&self, c: char) -> bool {
        self.matches(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_alnum() {
        let set = CharSet::alnum();
        assert!(set.matches('A'));
        assert!(set.matches('z'));
        assert!(set.matches('5'));
        assert!(!set.matches(' '));
    }

    #[test]
    fn test_alpha() {
        let set = CharSet::alpha();
        assert!(set.matches('A'));
        assert!(set.matches('z'));
        assert!(!set.matches('5'));
    }

    #[test]
    fn test_digit() {
        let set = CharSet::digit();
        assert!(set.matches('5'));
        assert!(!set.matches('a'));
    }

    #[test]
    fn test_xdigit() {
        let set = CharSet::xdigit();
        assert!(set.matches('0'));
        assert!(set.matches('9'));
        assert!(set.matches('A'));
        assert!(set.matches('F'));
        assert!(set.matches('a'));
        assert!(set.matches('f'));
        assert!(!set.matches('g'));
        assert!(!set.matches('G'));
    }

    #[test]
    fn test_whitespace() {
        let set = CharSet::whitespace();
        assert!(set.matches(' '));
        assert!(set.matches('\t'));
        assert!(set.matches('\n'));
        assert!(!set.matches('a'));
    }

    #[test]
    fn test_upper() {
        let set = CharSet::upper();
        assert!(set.matches('A'));
        assert!(!set.matches('a'));
    }

    #[test]
    fn test_lower() {
        let set = CharSet::lower();
        assert!(set.matches('a'));
        assert!(!set.matches('A'));
    }

    #[test]
    fn test_punctuation() {
        let set = CharSet::punctuation();
        assert!(set.matches('!'));
        assert!(set.matches('.'));
        assert!(set.matches('@'));
        assert!(!set.matches('a'));
        assert!(!set.matches(' '));
    }

    #[test]
    fn test_control() {
        let set = CharSet::control();
        assert!(set.matches('\x00'));
        assert!(set.matches('\x1f'));
        assert!(set.matches('\x7f'));
        assert!(!set.matches(' '));
        assert!(!set.matches('a'));
    }

    #[test]
    fn test_graph() {
        let set = CharSet::graph();
        assert!(set.matches('!'));
        assert!(set.matches('a'));
        assert!(set.matches('~'));
        assert!(!set.matches(' '));
        assert!(!set.matches('\n'));
    }

    #[test]
    fn test_print() {
        let set = CharSet::print();
        assert!(set.matches(' '));
        assert!(set.matches('a'));
        assert!(set.matches('~'));
        assert!(!set.matches('\n'));
        assert!(!set.matches('\x00'));
    }

    #[test]
    fn test_ascii() {
        let set = CharSet::ascii();
        assert!(set.matches('a'));
        assert!(set.matches('~'));
        assert!(set.matches('\x00'));
        assert!(set.matches('\x7f'));
        assert!(!set.matches('中'));
    }

    #[test]
    fn test_blank() {
        let set = CharSet::blank();
        assert!(set.matches(' '));
        assert!(set.matches('\t'));
        assert!(!set.matches('\n'));
        assert!(!set.matches('a'));
    }

    #[test]
    fn test_char_set_builder() {
        let set = CharSetBuilder::new()
            .add_range('a', 'z')
            .add_range('A', 'Z')
            .add_chars("0123456789")
            .build();

        assert!(set.matches('a'));
        assert!(set.matches('Z'));
        assert!(set.matches('5'));
        assert!(!set.matches(' '));
    }

    #[test]
    fn test_matches_all() {
        let digits = CharSet::digit();
        assert!(digits.matches_all("12345"));
        assert!(!digits.matches_all("123a5"));
        assert!(!digits.matches_all("")); // empty string
    }

    #[test]
    fn test_matches_any() {
        let digits = CharSet::digit();
        assert!(digits.matches_any("abc123"));
        assert!(!digits.matches_any("abc"));
        assert!(!digits.matches_any("")); // empty string
    }

    #[test]
    fn test_count() {
        let vowels = CharSetBuilder::new().add_chars("aeiouAEIOU").build();
        assert_eq!(vowels.count("hello world"), 3); // e, o, o
        assert_eq!(vowels.count("AEIOU"), 5);
        assert_eq!(vowels.count("xyz"), 0);
    }

    #[test]
    fn test_find() {
        let digits = CharSet::digit();
        assert_eq!(digits.find("abc123"), Some('1'));
        assert_eq!(digits.find("abc"), None);
    }

    #[test]
    fn test_find_not() {
        let digits = CharSet::digit();
        assert_eq!(digits.find_not("123abc"), Some('a'));
        assert_eq!(digits.find_not("123"), None);
    }

    #[test]
    fn test_negate() {
        let digits = CharSet::digit();
        let not_digits = digits.negate();
        assert!(!not_digits.matches('5'));
        assert!(not_digits.matches('a'));
        assert!(not_digits.matches(' '));
    }

    #[test]
    fn test_or() {
        let alnum = CharSet::alpha().or(&CharSet::digit());
        assert!(alnum.matches('A'));
        assert!(alnum.matches('5'));
        assert!(!alnum.matches(' '));
    }

    #[test]
    fn test_and() {
        let lower = CharSet::lower();
        let vowels = CharSetBuilder::new().add_chars("aeiou").build();
        let lower_vowels = lower.and(&vowels);
        assert!(lower_vowels.matches('a'));
        assert!(lower_vowels.matches('e'));
        assert!(!lower_vowels.matches('A'));
        assert!(!lower_vowels.matches('x'));
    }

    #[test]
    fn test_xor() {
        let lower = CharSet::lower();
        let vowels = CharSetBuilder::new().add_chars("aeiouAEIOU").build();
        let lower_xor_vowels = lower.xor(&vowels);
        assert!(!lower_xor_vowels.matches('a')); // in both lower and vowels
        assert!(lower_xor_vowels.matches('E')); // in vowels only (not lower)
        assert!(lower_xor_vowels.matches('b')); // in lower only (not vowels)
    }

    #[test]
    fn test_clone() {
        let set1 = CharSet::digit();
        let set2 = set1.clone();
        let set3 = CharSetBuilder::new().add_chars("xyz").build();

        assert!(set2.matches('5'));
        assert!(set3.matches('x'));
    }

    #[test]
    fn test_complex_operations() {
        let alnum = CharSet::alnum();
        let vowels = CharSetBuilder::new().add_chars("aeiouAEIOU").build();
        let alnum_not_vowels = alnum.and(&vowels.negate());

        assert!(alnum_not_vowels.matches('b'));
        assert!(alnum_not_vowels.matches('B'));
        assert!(alnum_not_vowels.matches('5'));
        assert!(!alnum_not_vowels.matches('a'));
        assert!(!alnum_not_vowels.matches('E'));
        assert!(!alnum_not_vowels.matches(' '));
    }
}
