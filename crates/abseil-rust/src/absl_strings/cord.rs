//! Cord - A rope data structure for efficient string manipulation.
//!
//! A `Cord` is a tree-like structure representing a string that allows for
//! efficient operations on large strings without copying. It's similar to a
//! "rope" data structure used in text editors.
//!
//! # When to use Cord
//!
//! - Building large strings incrementally (e.g., logging, file generation)
//! - Frequent concatenations where the intermediate strings would be costly
//! - When you need to mutate (insert/remove) large strings
//!
//! # When NOT to use Cord
//!
//! - For small strings - just use `String` or `&str`
//! - When you need random access by byte index - Cord is optimized for sequential access
//!
//! # Example
//!
//! ```rust
//! use abseil::absl_strings::cord::Cord;
//!
//! let mut cord = Cord::new();
//! cord.append("Hello, ");
//! cord.append("world!");
//!
//! // Efficient concatenation without intermediate allocations
//! assert_eq!(cord.to_str(), "Hello, world!");
//! ```

use core::fmt;
use std::string::String;

/// A rope data structure for efficient string manipulation.
///
/// # Example
///
/// ```rust
/// use abseil::absl_strings::cord::Cord;
///
/// let mut cord = Cord::from("Hello");
/// cord.append(" ");
/// cord.append("world!");
/// assert_eq!(cord.to_str(), "Hello world!");
/// ```
pub struct Cord {
    // Internal representation - we use a simple Vec for chunks
    // In a full implementation, this would be a balanced tree
    chunks: Vec<String>,
}

// SAFETY: Cord is Send and Sync because String is Send+Sync
unsafe impl Send for Cord {}
unsafe impl Sync for Cord {}

impl Cord {
    /// Creates an empty Cord.
    pub fn new() -> Self {
        Self { chunks: Vec::new() }
    }

    /// Creates a Cord from a string slice.
    pub fn from(s: &str) -> Self {
        if s.is_empty() {
            return Self::new();
        }
        Self {
            chunks: vec![s.to_string()],
        }
    }

    /// Creates a Cord from a String.
    pub fn from_string(s: String) -> Self {
        if s.is_empty() {
            return Self::new();
        }
        Self { chunks: vec![s] }
    }

    /// Returns `true` if the Cord is empty.
    pub fn is_empty(&self) -> bool {
        self.chunks.is_empty() || self.chunks.iter().all(|s| s.is_empty())
    }

    /// Returns the total number of bytes in the Cord.
    pub fn size(&self) -> usize {
        self.chunks.iter().map(|s| s.len()).sum()
    }

    /// Appends a string slice to the Cord.
    pub fn append(&mut self, s: &str) {
        if s.is_empty() {
            return;
        }
        self.chunks.push(s.to_string());
    }

    /// Prepends a string slice to the Cord.
    pub fn prepend(&mut self, s: &str) {
        if s.is_empty() {
            return;
        }
        self.chunks.insert(0, s.to_string());
    }

    /// Clears the Cord, making it empty.
    pub fn clear(&mut self) {
        self.chunks.clear();
    }

    /// Converts the Cord to a String.
    pub fn to_string(&self) -> String {
        self.chunks.concat()
    }

    /// Returns the string content as a `&str` if the Cord is simple (single chunk).
    pub fn as_str(&self) -> Option<&str> {
        if self.chunks.len() == 1 {
            self.chunks.get(0).map(|s| s.as_str())
        } else {
            None
        }
    }

    /// Returns the Cord as a string slice, converting if necessary.
    pub fn to_str(&self) -> String {
        self.as_str()
            .map(String::from)
            .unwrap_or_else(|| self.to_string())
    }

    /// Creates an iterator over the chunks of the Cord.
    pub fn chunks(&self) -> ChunksIter<'_> {
        ChunksIter {
            chunks: self.chunks.iter(),
        }
    }

    /// Creates an iterator over the characters of the Cord.
    pub fn chars(&self) -> CharsIter<'_> {
        CharsIter::new(self.chunks())
    }

    /// Creates an iterator over the bytes of the Cord.
    pub fn bytes(&self) -> BytesIter<'_> {
        BytesIter::new(self.chunks())
    }

    /// Returns a substring of the Cord.
    pub fn subview(&self, start: usize, end: usize) -> Cord {
        assert!(start <= end, "start must be <= end");
        assert!(end <= self.size(), "end must be <= self.size()");

        if start == end {
            return Cord::new();
        }

        // For simplicity, just collect and create new Cord
        let mut result = String::new();
        let mut skip = start;
        let mut remaining = end - start;

        for chunk in &self.chunks {
            let chunk_len = chunk.len();
            if skip < chunk_len && remaining > 0 {
                let take_start = skip;
                let take_end = (skip + remaining).min(chunk_len);
                result.push_str(&chunk[take_start..take_end]);
                remaining -= take_end - take_start;
                if remaining == 0 {
                    break;
                }
            }
            skip = skip.saturating_sub(chunk_len);
        }

        Cord::from(result.as_str())
    }

    /// Splits the Cord at the given position.
    pub fn split(self, pos: usize) -> (Cord, Cord) {
        assert!(pos <= self.size(), "split position out of bounds");

        if pos == 0 {
            return (Cord::new(), self);
        }
        if pos == self.size() {
            return (self, Cord::new());
        }

        let left = self.subview(0, pos);
        let right = self.subview(pos, self.size());

        (left, right)
    }

    /// Returns the number of chunks in the Cord.
    pub fn chunk_count(&self) -> usize {
        self.chunks.len()
    }

    /// Appends another Cord to this Cord.
    pub fn append_cord(&mut self, mut other: Cord) {
        if other.is_empty() {
            return;
        }

        self.chunks.append(&mut other.chunks);
        other.chunks.clear();
    }
}

impl Default for Cord {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for Cord {
    fn clone(&self) -> Self {
        Self {
            chunks: self.chunks.clone(),
        }
    }
}

// Note: We deliberately do NOT implement Deref<Target=str> for Cord.
// A Cord is a rope data structure that may consist of multiple chunks.
// Converting to a &str would require either:
// 1. Allocating a new String (violates zero-cost Deref expectation)
// 2. Returning only the first chunk (incorrect - loses data)
// 3. Returning "" for multi-chunk Cords (semantically wrong)
//
// Users should explicitly call to_string(), as_str(), or to_str() as needed.

impl fmt::Display for Cord {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_str())
    }
}

impl fmt::Debug for Cord {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Cord")
            .field("size", &self.size())
            .field("chunk_count", &self.chunk_count())
            .finish()
    }
}

impl PartialEq for Cord {
    fn eq(&self, other: &Self) -> bool {
        self.to_str() == other.to_str()
    }
}

impl Eq for Cord {}

impl PartialOrd for Cord {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Cord {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        self.to_str().cmp(&other.to_str())
    }
}

impl core::hash::Hash for Cord {
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        self.to_str().hash(state);
    }
}

impl From<&str> for Cord {
    fn from(s: &str) -> Self {
        Self::from(s)
    }
}

impl From<String> for Cord {
    fn from(s: String) -> Self {
        Self::from_string(s)
    }
}

impl From<&String> for Cord {
    fn from(s: &String) -> Self {
        Self::from(s.as_str())
    }
}

/// Iterator over Cord chunks.
pub struct ChunksIter<'a> {
    chunks: core::slice::Iter<'a, String>,
}

impl<'a> Iterator for ChunksIter<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        self.chunks.next().map(|s| s.as_str())
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.chunks.size_hint()
    }
}

/// Iterator over Cord characters.
pub struct CharsIter<'a> {
    chunks: ChunksIter<'a>,
    current: Option<core::str::Chars<'a>>,
}

impl<'a> CharsIter<'a> {
    fn new(chunks: ChunksIter<'a>) -> Self {
        Self { chunks, current: None }
    }
}

impl<'a> Iterator for CharsIter<'a> {
    type Item = char;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            // Get next char from current chunk
            if let Some(ref mut chars) = self.current {
                if let Some(ch) = chars.next() {
                    return Some(ch);
                }
            }

            // Move to next chunk
            match self.chunks.next() {
                Some(chunk) => {
                    self.current = Some(chunk.chars());
                }
                None => return None,
            }
        }
    }
}

/// Iterator over Cord bytes.
pub struct BytesIter<'a> {
    chunks: ChunksIter<'a>,
    current: Option<core::slice::Iter<'a, u8>>,
}

impl<'a> BytesIter<'a> {
    fn new(chunks: ChunksIter<'a>) -> Self {
        Self { chunks, current: None }
    }
}

impl<'a> Iterator for BytesIter<'a> {
    type Item = u8;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some(ref mut bytes) = self.current {
                if let Some(byte) = bytes.next() {
                    return Some(*byte);
                }
            }

            match self.chunks.next() {
                Some(chunk) => {
                    self.current = Some(chunk.as_bytes().iter());
                }
                None => return None,
            }
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.current.as_ref().map_or(0, |i| i.len());
        (remaining, self.chunks.size_hint().1.map(|x| x + remaining))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new() {
        let cord = Cord::new();
        assert!(cord.is_empty());
        assert_eq!(cord.size(), 0);
        assert_eq!(cord.chunk_count(), 0);
    }

    #[test]
    fn test_from_str() {
        let cord = Cord::from("Hello, world!");
        assert!(!cord.is_empty());
        assert_eq!(cord.size(), 13);
        assert_eq!(cord.as_str(), Some("Hello, world!"));
        assert_eq!(cord.chunk_count(), 1);
    }

    #[test]
    fn test_from_string() {
        let s = String::from("Hello");
        let cord = Cord::from_string(s);
        assert_eq!(cord.to_str(), "Hello");
    }

    #[test]
    fn test_append() {
        let mut cord = Cord::new();
        cord.append("Hello");
        cord.append(", ");
        cord.append("world!");

        assert_eq!(cord.to_str(), "Hello, world!");
        assert_eq!(cord.size(), 13);
        assert_eq!(cord.chunk_count(), 3);
    }

    #[test]
    fn test_prepend() {
        let mut cord = Cord::from("world!");
        cord.prepend(", ");
        cord.prepend("Hello");

        assert_eq!(cord.to_str(), "Hello, world!");
    }

    #[test]
    fn test_clear() {
        let mut cord = Cord::from("Hello");
        assert!(!cord.is_empty());
        cord.clear();
        assert!(cord.is_empty());
    }

    #[test]
    fn test_to_string() {
        let cord = Cord::from("Hello");
        let s = cord.to_string();
        assert_eq!(s, "Hello");
    }

    #[test]
    fn test_as_str() {
        let simple = Cord::from("Hello");
        assert_eq!(simple.as_str(), Some("Hello"));

        let mut complex = Cord::new();
        complex.append("Hello");
        complex.append(" ");
        complex.append("world");
        assert!(complex.as_str().is_none());
    }

    #[test]
    fn test_chunks() {
        let mut cord = Cord::new();
        cord.append("Hello");
        cord.append(" ");
        cord.append("world");

        let chunks: Vec<&str> = cord.chunks().collect();
        assert_eq!(chunks, ["Hello", " ", "world"]);
        assert_eq!(cord.chunk_count(), 3);
    }

    #[test]
    fn test_chars() {
        let cord = Cord::from("Hello");
        let chars: Vec<char> = cord.chars().collect();
        assert_eq!(chars, ['H', 'e', 'l', 'l', 'o']);
    }

    #[test]
    fn test_bytes() {
        let cord = Cord::from("Hello");
        let bytes: Vec<u8> = cord.bytes().collect();
        assert_eq!(bytes, b"Hello");
    }

    #[test]
    fn test_subview() {
        let cord = Cord::from("Hello, world!");
        let substr = cord.subview(0, 5);
        assert_eq!(substr.to_str(), "Hello");

        let substr2 = cord.subview(7, 12);
        assert_eq!(substr2.to_str(), "world");
    }

    #[test]
    fn test_split() {
        let cord = Cord::from("Hello world");
        let (left, right) = cord.split(5);
        assert_eq!(left.to_str(), "Hello");
        assert_eq!(right.to_str(), " world");
    }

    #[test]
    fn test_append_cord() {
        let mut cord1 = Cord::from("Hello");
        let cord2 = Cord::from(" world");
        cord1.append_cord(cord2);
        assert_eq!(cord1.to_str(), "Hello world");
    }

    #[test]
    fn test_default() {
        let cord = Cord::default();
        assert!(cord.is_empty());
    }

    #[test]
    fn test_clone() {
        let cord = Cord::from("Hello");
        let cloned = cord.clone();
        assert_eq!(cloned.to_str(), "Hello");
    }

    #[test]
    fn test_equality() {
        let a = Cord::from("Hello");
        let b = Cord::from("Hello");
        assert_eq!(a, b);

        let c = Cord::from("World");
        assert_ne!(a, c);
    }

    #[test]
    fn test_ord() {
        let a = Cord::from("aaa");
        let b = Cord::from("bbb");
        assert!(a < b);
    }

    #[test]
    fn test_display() {
        let cord = Cord::from("Hello");
        assert_eq!(format!("{}", cord), "Hello");
    }

    #[test]
    fn test_debug() {
        let cord = Cord::from("Hello");
        let debug = format!("{:?}", cord);
        assert!(debug.contains("Cord"));
        assert!(debug.contains("5"));
    }

    #[test]
    fn test_from_various() {
        let s: &str = "Hello";
        let cord1 = Cord::from(s);

        let owned = String::from("Hello");
        let cord2 = Cord::from(&owned);

        assert_eq!(cord1.to_str(), cord2.to_str());
    }

    #[test]
    fn test_large_string() {
        let large = "a".repeat(10000);
        let cord = Cord::from(&large);
        assert_eq!(cord.size(), 10000);
        assert_eq!(cord.to_str(), large);
    }

    #[test]
    fn test_empty_operations() {
        let mut cord = Cord::new();
        cord.append("");  // Should do nothing
        assert!(cord.is_empty());

        cord.prepend("");  // Should do nothing
        assert!(cord.is_empty());
    }

    #[test]
    fn test_clone_with_chunks() {
        let mut cord1 = Cord::new();
        cord1.append("Hello");
        cord1.append(" ");
        cord1.append("world");

        let cord2 = cord1.clone();
        assert_eq!(cord1.to_str(), cord2.to_str());
        assert_eq!(cord2.chunk_count(), 3);
    }

    #[test]
    fn test_append_empty_cord() {
        let mut cord1 = Cord::from("Hello");
        let cord2 = Cord::new();
        cord1.append_cord(cord2);
        assert_eq!(cord1.to_str(), "Hello");
        // cord2 is moved, so we can't check it anymore
        // The empty cord's chunks were moved into cord1
    }
}
