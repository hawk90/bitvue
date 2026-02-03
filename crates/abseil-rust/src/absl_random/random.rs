//! Random generation utilities - bool, char, and various simple random generators.

use crate::absl_random::bit_gen::BitGen;

/// Generates a random boolean value with the given probability.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_random::random_bool;
///
/// let mut rng = BitGen::new(42);
/// // 50% chance of true
/// let result = random_bool(0.5, &mut rng);
/// ```
#[inline]
pub fn random_bool(probability: f64, rng: &mut BitGen) -> bool {
    let value = rng.gen_u64();
    (value as f64 / (u64::MAX as f64)) < probability
}

/// Generates a random character from the given set.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_random::random_char;
///
/// let mut rng = BitGen::new(42);
/// let c = random_char("abcdef", &mut rng);
/// assert!(c.is_ascii_alphanumeric());
/// ```
pub fn random_char(chars: &str, rng: &mut BitGen) -> char {
    if chars.is_empty() {
        return '\0';
    }
    let bytes = chars.as_bytes();
    let index = rng.gen_usize() % bytes.len();
    bytes[index] as char
}

/// Generates a random ASCII character.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_random::random_ascii;
///
/// let mut rng = BitGen::new(42);
/// let c = random_ascii(&mut rng);
/// assert!(c.is_ascii());
/// ```
#[inline]
pub fn random_ascii(rng: &mut BitGen) -> char {
    (rng.gen_u8() % 128) as char
}

/// Generates a random alphanumeric ASCII character.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_random::random_alphanumeric;
///
/// let mut rng = BitGen::new(42);
/// let c = random_alphanumeric(&mut rng);
/// assert!(c.is_ascii_alphanumeric());
/// ```
#[inline]
pub fn random_alphanumeric(rng: &mut BitGen) -> char {
    const CHARS: &[u8] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
    CHARS[rng.gen_usize() % CHARS.len()] as char
}

/// Generates a random digit (0-9).
///
/// # Examples
///
/// ```rust
/// use abseil::absl_random::random_digit;
///
/// let mut rng = BitGen::new(42);
/// let d = random_digit(&mut rng);
/// assert!(d.is_ascii_digit());
/// ```
#[inline]
pub fn random_digit(rng: &mut BitGen) -> char {
    (b'0' + (rng.gen_u8() % 10)) as char
}

/// Generates a random lowercase letter (a-z).
///
/// # Examples
///
/// ```rust
/// use abseil::absl_random::random_lowercase;
///
/// let mut rng = BitGen::new(42);
/// let c = random_lowercase(&mut rng);
/// assert!(c.is_ascii_lowercase());
/// ```
#[inline]
pub fn random_lowercase(rng: &mut BitGen) -> char {
    (b'a' + (rng.gen_u8() % 26)) as char
}

/// Generates a random uppercase letter (A-Z).
///
/// # Examples
///
/// ```rust
/// use abseil::absl_random::random_uppercase;
///
/// let mut rng = BitGen::new(42);
/// let c = random_uppercase(&mut rng);
/// assert!(c.is_ascii_uppercase());
/// ```
#[inline]
pub fn random_uppercase(rng: &mut BitGen) -> char {
    (b'A' + (rng.gen_u8() % 26)) as char
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_random_char() {
        let mut rng = BitGen::new(42);
        let chars = "abcdef";
        let c = random_char(chars, &mut rng);
        assert!(chars.contains(c));
    }

    #[test]
    fn test_random_ascii() {
        let mut rng = BitGen::new(42);
        for _ in 0..100 {
            let c = random_ascii(&mut rng);
            assert!(c.is_ascii());
        }
    }

    #[test]
    fn test_random_alphanumeric() {
        let mut rng = BitGen::new(42);
        for _ in 0..100 {
            let c = random_alphanumeric(&mut rng);
            assert!(c.is_ascii_alphanumeric());
        }
    }

    #[test]
    fn test_random_digit() {
        let mut rng = BitGen::new(42);
        for _ in 0..100 {
            let d = random_digit(&mut rng);
            assert!(d.is_ascii_digit());
        }
    }

    #[test]
    fn test_random_lowercase() {
        let mut rng = BitGen::new(42);
        for _ in 0..100 {
            let c = random_lowercase(&mut rng);
            assert!(c.is_ascii_lowercase());
        }
    }

    #[test]
    fn test_random_uppercase() {
        let mut rng = BitGen::new(42);
        for _ in 0..100 {
            let c = random_uppercase(&mut rng);
            assert!(c.is_ascii_uppercase());
        }
    }
}
