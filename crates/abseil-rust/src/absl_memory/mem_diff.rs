//! Memory difference module - comparing and diffing memory regions.

use alloc::vec::Vec;

/// Represents a difference between two memory regions.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MemDiff {
    /// The regions are identical.
    Identical,
    /// Different lengths.
    LengthMismatch { a_len: usize, b_len: usize },
    /// Byte differs at position.
    ByteDiff { offset: usize, a_val: u8, b_val: u8 },
    /// Multiple differences found.
    MultipleDiffs(Vec<(usize, u8, u8)>),
}

impl MemDiff {
    /// Returns true if the regions are identical.
    pub fn is_identical(&self) -> bool {
        matches!(self, MemDiff::Identical)
    }

    /// Returns the number of differences found.
    pub fn diff_count(&self) -> usize {
        match self {
            MemDiff::Identical => 0,
            MemDiff::LengthMismatch { .. } => 1,
            MemDiff::ByteDiff { .. } => 1,
            MemDiff::MultipleDiffs(diffs) => diffs.len(),
        }
    }
}

/// Compares two memory regions and returns detailed difference information.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_memory::memdiff;
///
/// let a = b"hello world";
/// let b = b"hello earth";
/// let diff = memdiff(a, b);
/// assert!(!diff.is_identical());
/// ```
pub fn memdiff(a: &[u8], b: &[u8]) -> MemDiff {
    if a.len() != b.len() {
        return MemDiff::LengthMismatch {
            a_len: a.len(),
            b_len: b.len(),
        };
    }

    let mut diffs = Vec::new();
    for i in 0..a.len() {
        if a[i] != b[i] {
            diffs.push((i, a[i], b[i]));
        }
    }

    match diffs.len() {
        0 => MemDiff::Identical,
        1 => {
            let (offset, a_val, b_val) = diffs[0];
            MemDiff::ByteDiff {
                offset,
                a_val,
                b_val,
            }
        }
        _ => MemDiff::MultipleDiffs(diffs),
    }
}

/// Finds the first differing byte between two memory regions.
///
/// Returns None if they are identical (up to the min length).
///
/// # Examples
///
/// ```rust
/// use abseil::absl_memory::find_first_diff;
///
/// let a = b"hello world";
/// let b = b"hello earth";
/// assert_eq!(find_first_diff(a, b), Some((6, b'w', b'e')));
/// ```
pub fn find_first_diff(a: &[u8], b: &[u8]) -> Option<(usize, u8, u8)> {
    let min_len = a.len().min(b.len());
    for i in 0..min_len {
        if a[i] != b[i] {
            return Some((i, a[i], b[i]));
        }
    }
    if a.len() != b.len() {
        Some((
            min_len,
            a.get(min_len).copied().unwrap_or(0),
            b.get(min_len).copied().unwrap_or(0),
        ))
    } else {
        None
    }
}

/// Counts matching bytes between two memory regions.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_memory::count_matching_bytes;
///
/// let a = b"hello world";
/// let b = b"hello earth";
/// assert_eq!(count_matching_bytes(a, b), 5); // "hello"
/// ```
pub fn count_matching_bytes(a: &[u8], b: &[u8]) -> usize {
    a.iter()
        .zip(b.iter())
        .take_while(|(x, y)| x == y)
        .count()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memdiff_identical() {
        let a = b"hello";
        let b = b"hello";
        let diff = memdiff(a, b);
        assert!(diff.is_identical());
    }

    #[test]
    fn test_memdiff_byte_diff() {
        let a = b"hello";
        let b = b"hallo";
        let diff = memdiff(a, b);
        assert!(!diff.is_identical());
        assert_eq!(diff.diff_count(), 1);
    }

    #[test]
    fn test_memdiff_length_mismatch() {
        let a = b"hello";
        let b = b"hello world";
        let diff = memdiff(a, b);
        assert!(!diff.is_identical());
    }

    #[test]
    fn test_find_first_diff() {
        let a = b"hello world";
        let b = b"hello earth";
        let diff = find_first_diff(a, b);
        assert_eq!(diff, Some((6, b'w', b'e')));
    }

    #[test]
    fn test_find_first_diff_identical() {
        let a = b"hello";
        let b = b"hello";
        let diff = find_first_diff(a, b);
        assert!(diff.is_none());
    }

    #[test]
    fn test_count_matching_bytes() {
        let a = b"hello world";
        let b = b"hello earth";
        assert_eq!(count_matching_bytes(a, b), 6); // "hello "
    }
}
