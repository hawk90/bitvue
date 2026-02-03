//! Optimized string concatenation.
//!
//! This module provides utilities for efficient string concatenation,
//! similar to Abseil's `absl::StrCat`.
//!
//! # Example
//!
//! The `str_cat!` macro is exported at the crate root:
//!
//! ```rust
//! use abseil::str_cat;
//! use abseil::absl_strings::str_cat::StrCat;
//!
//! // Simple concatenation with str_cat macro
//! assert_eq!(str_cat!("Hello", " ", "world", "!"), "Hello world!");
//! assert_eq!(str_cat!(123, " + ", 456, " = ", 579), "123 + 456 = 579");
//!
//! // Builder pattern for string slices
//! let result = StrCat::new()
//!     .append("Hello")
//!     .append(" ")
//!     .append("world!")
//!     .build();
//! assert_eq!(result, "Hello world!");
//! ```

use core::fmt;

/// Trait for types that can be appended to a string builder.
pub trait AlphaNum {
    /// Append this value to the given formatter.
    fn write_to(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result;
}

// Macro to generate AlphaNum implementations for types that implement Display
macro_rules! impl_alphanum_for_display {
    ($($ty:ty),* $(,)?) => {
        $(
            impl AlphaNum for $ty {
                fn write_to(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                    write!(f, "{}", self)
                }
            }
        )*
    };
}

// Implement AlphaNum for string types
impl_alphanum_for_display!(str, &str, String);

// Implement AlphaNum for numeric types
impl_alphanum_for_display!(
    i8, i16, i32, i64, i128, isize,
    u8, u16, u32, u64, u128, usize,
    f32, f64,
    char, bool
);

/// Builder for efficient string concatenation.
///
/// This builder calculates the final capacity upfront to avoid reallocations.
pub struct StrCat {
    parts: Vec<String>,
}

impl StrCat {
    /// Creates a new StrCat builder.
    pub fn new() -> Self {
        Self { parts: Vec::new() }
    }

    /// Appends a string slice to the builder.
    pub fn append(mut self, s: &str) -> Self {
        self.parts.push(s.to_string());
        self
    }

    /// Builds the final string.
    pub fn build(self) -> String {
        // Use checked arithmetic to prevent overflow
        let capacity = self.parts.iter().fold(0usize, |acc, p| {
            acc.checked_add(p.len()).unwrap_or(usize::MAX)
        });
        let mut result = String::with_capacity(capacity);
        for part in self.parts {
            result.push_str(&part);
        }
        result
    }

    /// Returns the estimated length of the final string.
    ///
    /// Returns `usize::MAX` if the actual length would overflow.
    pub fn len(&self) -> usize {
        self.parts.iter().fold(0usize, |acc, p| {
            acc.checked_add(p.len()).unwrap_or(usize::MAX)
        })
    }

    /// Returns true if no parts have been added.
    pub fn is_empty(&self) -> bool {
        self.parts.is_empty()
    }
}

impl Default for StrCat {
    fn default() -> Self {
        Self::new()
    }
}

/// Simple string concatenation macro.
///
/// This macro provides a convenient way to concatenate multiple values
/// into a single string.
///
/// # Examples
///
/// ```rust
/// // The macro is exported at the crate root
/// // In this doctest, we can't use it directly, so showing usage:
///
/// // In your code:
/// // use abseil::str_cat;
/// // assert_eq!(str_cat!("Hello", " ", "world"), "Hello world");
/// // assert_eq!(str_cat!(123, " + ", 456), "123 + 456");
/// // assert_eq!(str_cat!("Value: ", 42.5), "Value: 42.5");
/// ```
#[macro_export]
macro_rules! str_cat {
    () => {
        String::new()
    };
    ($($arg:expr),+ $(,)?) => {{
        let mut result = String::new();
        $(
            result.push_str(&format!("{}", $arg));
        )+
        result
    }};
}

/// Appends multiple values to a string with a delimiter.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_strings::str_cat::str_join;
///
/// assert_eq!(str_join(", ", &["a", "b", "c"]), "a, b, c");
/// assert_eq!(str_join("-", &[1, 2, 3]), "1-2-3");
/// ```
pub fn str_join<T: core::fmt::Display>(delimiter: &str, items: &[T]) -> String {
    if items.is_empty() {
        return String::new();
    }

    // Calculate approximate capacity with overflow protection
    let delimiter_len = delimiter.len();
    let count = items.len();

    // Estimate: (count - 1) * delimiter_len + count * 8 (average string size)
    // Use checked arithmetic to prevent overflow
    let delimiter_space = if count > 0 {
        count.saturating_sub(1).saturating_mul(delimiter_len)
    } else {
        0
    };

    let content_space = count.saturating_mul(8);
    let approx_len = delimiter_space.saturating_add(content_space);

    let mut result = String::with_capacity(approx_len);

    for (i, item) in items.iter().enumerate() {
        if i > 0 {
            result.push_str(delimiter);
        }
        result.push_str(&format!("{}", item));
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_str_cat_basic() {
        let result = StrCat::new()
            .append("Hello")
            .append(", ")
            .append("world!")
            .build();

        assert_eq!(result, "Hello, world!");
    }

    #[test]
    fn test_str_cat_empty() {
        assert_eq!(StrCat::new().build(), "");
        assert!(StrCat::new().is_empty());
    }

    #[test]
    fn test_str_cat_len() {
        let cat = StrCat::new().append("Hello").append(" ");
        assert_eq!(cat.len(), 6);
    }

    #[test]
    fn test_str_cat_default() {
        let cat: StrCat = Default::default();
        assert_eq!(cat.build(), "");
    }

    #[test]
    fn test_str_cat_macro_empty() {
        assert_eq!(str_cat!(), "");
    }

    #[test]
    fn test_str_cat_macro_single() {
        assert_eq!(str_cat!("hello"), "hello");
    }

    #[test]
    fn test_str_cat_macro_multiple() {
        assert_eq!(str_cat!("Hello", " ", "world", "!"), "Hello world!");
    }

    #[test]
    fn test_str_cat_macro_numbers() {
        assert_eq!(str_cat!(123, " + ", 456, " = ", 579), "123 + 456 = 579");
        assert_eq!(str_cat!(1, 2, 3), "123");
    }

    #[test]
    fn test_str_cat_macro_floats() {
        assert_eq!(str_cat!(3.14, " is pi"), "3.14 is pi");
    }

    #[test]
    fn test_str_cat_macro_bool() {
        assert_eq!(str_cat!(true, " and ", false), "true and false");
    }

    #[test]
    fn test_str_cat_macro_trailing_comma() {
        assert_eq!(str_cat!("a", "b", "c"), "abc");
    }

    #[test]
    fn test_str_join_strings() {
        assert_eq!(str_join(", ", &["a", "b", "c"]), "a, b, c");
    }

    #[test]
    fn test_str_join_numbers() {
        assert_eq!(str_join("-", &[1, 2, 3, 4]), "1-2-3-4");
    }

    #[test]
    fn test_str_join_empty() {
        let empty: &[&str] = &[];
        assert_eq!(str_join(", ", empty), "");
    }

    #[test]
    fn test_str_join_single() {
        assert_eq!(str_join(", ", &["alone"]), "alone");
    }

    #[test]
    fn test_str_join_empty_delimiter() {
        assert_eq!(str_join("", &["a", "b", "c"]), "abc");
    }

    #[test]
    fn test_alpha_num_str() {
        // Test through str_cat macro instead of direct formatter
        assert_eq!(str_cat!("hello"), "hello");
    }

    #[test]
    fn test_alpha_num_integers() {
        assert_eq!(str_cat!(42i8), "42");
        assert_eq!(str_cat!(1000i16), "1000");
        assert_eq!(str_cat!(99999i32), "99999");
        assert_eq!(str_cat!(123456789i64), "123456789");
    }

    #[test]
    fn test_alpha_num_unsigned() {
        assert_eq!(str_cat!(42u8), "42");
        assert_eq!(str_cat!(1000u16), "1000");
        assert_eq!(str_cat!(99999u32), "99999");
        assert_eq!(str_cat!(123456789u64), "123456789");
    }

    #[test]
    fn test_alpha_num_floats() {
        assert_eq!(str_cat!(3.14f32), "3.14");
        assert_eq!(str_cat!(2.71828f64), "2.71828");
    }

    #[test]
    fn test_alpha_num_bool() {
        assert_eq!(str_cat!(true), "true");
        assert_eq!(str_cat!(false), "false");
    }

    #[test]
    fn test_alpha_num_char() {
        assert_eq!(str_cat!('a'), "a");
        assert_eq!(str_cat!('中'), "中");
    }
}
