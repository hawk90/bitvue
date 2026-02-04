//! Natural sort implementation for strings.
//!
//! Natural sort orders strings containing numbers in a way that makes sense
//! to humans, e.g., "file2.txt" comes before "file10.txt".
//!
//! # Examples
//!
//! ```
//! use abseil::absl_sorting::natural_sort;
//!
//! let mut files = vec!["file10.txt", "file2.txt", "file1.txt"];
//! natural_sort(&mut files);
//! assert_eq!(files, vec!["file1.txt", "file2.txt", "file10.txt"]);
//! ```

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;
use core::cmp::Ordering;

/// Represents a segment of a string - either text or a number
#[derive(Debug, Clone, PartialEq)]
enum Segment {
    Text(String),
    Number(u64),
}

impl PartialOrd for Segment {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Eq for Segment {}

impl Ord for Segment {
    fn cmp(&self, other: &Self) -> Ordering {
        match (self, other) {
            // Numbers come before text
            (Segment::Number(_), Segment::Text(_)) => Ordering::Less,
            (Segment::Text(_), Segment::Number(_)) => Ordering::Greater,
            // Compare numbers numerically
            (Segment::Number(a), Segment::Number(b)) => a.cmp(b),
            // Compare text lexicographically (case-insensitive)
            (Segment::Text(a), Segment::Text(b)) => a.to_lowercase().cmp(&b.to_lowercase()),
        }
    }
}

/// Splits a string into natural sort segments (text and numbers)
fn parse_segments(s: &str) -> Vec<Segment> {
    let mut segments = Vec::new();
    let mut current_text = String::new();
    let mut current_number = String::new();
    let mut in_number = false;

    for ch in s.chars() {
        if ch.is_ascii_digit() {
            if !in_number {
                if !current_text.is_empty() {
                    segments.push(Segment::Text(current_text.clone()));
                    current_text.clear();
                }
                in_number = true;
            }
            current_number.push(ch);
        } else {
            if in_number {
                if !current_number.is_empty() {
                    if let Ok(n) = current_number.parse::<u64>() {
                        segments.push(Segment::Number(n));
                    }
                    current_number.clear();
                }
                in_number = false;
            }
            current_text.push(ch);
        }
    }

    // Don't forget the last segment
    if in_number {
        if !current_number.is_empty() {
            if let Ok(n) = current_number.parse::<u64>() {
                segments.push(Segment::Number(n));
            }
        }
    } else if !current_text.is_empty() {
        segments.push(Segment::Text(current_text));
    }

    segments
}

/// Compares two strings using natural sort order
pub fn natural_cmp(a: &str, b: &str) -> Ordering {
    let a_segments = parse_segments(a);
    let b_segments = parse_segments(b);

    // Compare segment by segment
    for (seg_a, seg_b) in a_segments.iter().zip(b_segments.iter()) {
        match seg_a.cmp(seg_b) {
            Ordering::Equal => continue,
            ordering => return ordering,
        }
    }

    // If all common segments are equal, the shorter one comes first
    a_segments.len().cmp(&b_segments.len())
}

/// Sorts a slice of strings using natural sort order
pub fn natural_sort(slice: &mut [&str]) {
    slice.sort_by(|a, b| natural_cmp(a, b));
}

/// Sorts a slice of String using natural sort order
pub fn natural_sort_string(slice: &mut [String]) {
    slice.sort_by(|a, b| natural_cmp(a, b));
}

/// Sorts a slice of strings using natural sort order with a custom key function
pub fn natural_sort_by<T, F>(slice: &mut [T], mut key: F)
where
    F: FnMut(&T) -> &str,
{
    slice.sort_by(|a, b| natural_cmp(key(a), key(b)));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_segments_simple() {
        let segments = parse_segments("file10.txt");
        assert_eq!(segments.len(), 3);
        assert_eq!(segments[0], Segment::Text("file".to_string()));
        assert_eq!(segments[1], Segment::Number(10));
        assert_eq!(segments[2], Segment::Text(".txt".to_string()));
    }

    #[test]
    fn test_parse_segments_multiple_numbers() {
        let segments = parse_segments("v1.2.3");
        assert_eq!(segments.len(), 6);
        assert_eq!(segments[0], Segment::Text("v".to_string()));
        assert_eq!(segments[1], Segment::Number(1));
        assert_eq!(segments[2], Segment::Text(".".to_string()));
        assert_eq!(segments[3], Segment::Number(2));
        assert_eq!(segments[4], Segment::Text(".".to_string()));
        assert_eq!(segments[5], Segment::Number(3));
    }

    #[test]
    fn test_natural_cmp_files() {
        assert_eq!(natural_cmp("file1.txt", "file2.txt"), Ordering::Less);
        assert_eq!(natural_cmp("file2.txt", "file10.txt"), Ordering::Less);
        assert_eq!(natural_cmp("file10.txt", "file10.txt"), Ordering::Equal);
    }

    #[test]
    fn test_natural_cmp_versions() {
        assert_eq!(natural_cmp("v1.2", "v1.10"), Ordering::Less);
        assert_eq!(natural_cmp("v1.10", "v2.0"), Ordering::Less);
        assert_eq!(natural_cmp("v2.0", "v2.0"), Ordering::Equal);
    }

    #[test]
    fn test_natural_sort() {
        let mut files = ["file10.txt", "file2.txt", "file1.txt"];
        natural_sort(&mut files);
        assert_eq!(files, ["file1.txt", "file2.txt", "file10.txt"]);
    }

    #[test]
    fn test_natural_sort_string() {
        let mut files = vec![
            "file10.txt".to_string(),
            "file2.txt".to_string(),
            "file1.txt".to_string(),
        ];
        natural_sort_string(&mut files);
        assert_eq!(
            files,
            vec![
                "file1.txt".to_string(),
                "file2.txt".to_string(),
                "file10.txt".to_string(),
            ]
        );
    }

    #[test]
    fn test_natural_sort_by() {
        let mut files = [("a", "file10.txt"), ("b", "file2.txt"), ("c", "file1.txt")];
        natural_sort_by(&mut files, |(_, name)| *name);
        assert_eq!(files[0].1, "file1.txt");
        assert_eq!(files[1].1, "file2.txt");
        assert_eq!(files[2].1, "file10.txt");
    }

    #[test]
    fn test_natural_sort_case_insensitive() {
        let mut files = ["File10.txt", "file2.txt", "FILE1.txt"];
        natural_sort(&mut files);
        assert_eq!(files, ["FILE1.txt", "file2.txt", "File10.txt"]);
    }

    #[test]
    fn test_natural_sort_complex() {
        let mut items = ["item1a", "item10", "item1", "item2", "item20a", "item2a"];
        natural_sort(&mut items);
        assert_eq!(
            items,
            ["item1", "item1a", "item2", "item2a", "item10", "item20a"]
        );
    }
}
