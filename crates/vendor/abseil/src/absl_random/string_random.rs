//! String and bytes random generation.

use alloc::string::String;

use crate::absl_random::bit_gen::BitGen;

/// Generates a random string of the given length.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_random::random_string;
///
/// let mut rng = BitGen::new(42);
/// let s = random_string(10, &mut rng);
/// assert_eq!(s.len(), 10);
/// ```
pub fn random_string(len: usize, rng: &mut BitGen) -> String {
    (0..len)
        .map(|_| super::random_alphanumeric(rng))
        .collect()
}

/// Generates a random string from the given character set.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_random::random_string_from;
///
/// let mut rng = BitGen::new(42);
/// let s = random_string_from("abcdef", 10, &mut rng);
/// assert_eq!(s.len(), 10);
/// ```
pub fn random_string_from(chars: &str, len: usize, rng: &mut BitGen) -> String {
    if chars.is_empty() {
        return String::new();
    }
    (0..len)
        .map(|_| super::random_char(chars, rng))
        .collect()
}

/// Generates a random bytes array of the given length.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_random::random_bytes;
///
/// let mut rng = BitGen::new(42);
/// let bytes = random_bytes(16, &mut rng);
/// assert_eq!(bytes.len(), 16);
/// ```
pub fn random_bytes(len: usize, rng: &mut BitGen) -> Vec<u8> {
    (0..len).map(|_| rng.gen_u8()).collect()
}

/// Fills a slice with random bytes.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_random::fill_bytes;
///
/// let mut rng = BitGen::new(42);
/// let mut buffer = [0u8; 16];
/// fill_bytes(&mut buffer, &mut rng);
/// ```
pub fn fill_bytes(slice: &mut [u8], rng: &mut BitGen) {
    for byte in slice.iter_mut() {
        *byte = rng.gen_u8();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_random_string() {
        let mut rng = BitGen::new(42);
        let s = random_string(20, &mut rng);
        assert_eq!(s.len(), 20);
        for c in s.chars() {
            assert!(c.is_ascii_alphanumeric());
        }
    }

    #[test]
    fn test_random_string_from() {
        let mut rng = BitGen::new(42);
        let s = random_string_from("abc", 10, &mut rng);
        assert_eq!(s.len(), 10);
        for c in s.chars() {
            assert!(c == 'a' || c == 'b' || c == 'c');
        }
    }

    #[test]
    fn test_random_bytes() {
        let mut rng = BitGen::new(42);
        let bytes = random_bytes(32, &mut rng);
        assert_eq!(bytes.len(), 32);
    }

    #[test]
    fn test_fill_bytes() {
        let mut rng = BitGen::new(42);
        let mut buffer = [0u8; 16];
        fill_bytes(&mut buffer, &mut rng);
        // Not necessarily all different, but they were set
        assert_ne!(buffer, [0u8; 16]);
    }
}
