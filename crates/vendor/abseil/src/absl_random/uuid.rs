//! UUID generation utilities - random_uuid, format_uuid

use alloc::format;
use alloc::string::String;

use crate::absl_random::bit_gen::BitGen;

/// Generates a random UUID v4.
///
/// Returns a 16-byte array representing the UUID.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_random::random_uuid;
///
/// let mut rng = BitGen::new(42);
/// let uuid = random_uuid(&mut rng);
/// assert_eq!(uuid.len(), 16);
/// ```
pub fn random_uuid(rng: &mut BitGen) -> [u8; 16] {
    let mut bytes = [0u8; 16];
    super::fill_bytes(&mut bytes, rng);
    // Set version bits (0100xxxx for v4)
    bytes[6] = (bytes[6] & 0x0F) | 0x40;
    // Set variant bits (10xxxxxx)
    bytes[8] = (bytes[8] & 0x3F) | 0x80;
    bytes
}

/// Formats a UUID as a string.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_random::{random_uuid, format_uuid};
///
/// let mut rng = BitGen::new(42);
/// let uuid = random_uuid(&mut rng);
/// let s = format_uuid(&uuid);
/// assert_eq!(s.len(), 36); // 8-4-4-4-12 format
/// ```
pub fn format_uuid(uuid: &[u8; 16]) -> String {
    format!(
        "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
        uuid[0], uuid[1], uuid[2], uuid[3],
        uuid[4], uuid[5],
        uuid[6], uuid[7],
        uuid[8], uuid[9],
        uuid[10], uuid[11], uuid[12], uuid[13], uuid[14], uuid[15],
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_random_uuid() {
        let mut rng = BitGen::new(42);
        let uuid = random_uuid(&mut rng);
        assert_eq!(uuid.len(), 16);
        // Check version bits (v4)
        assert_eq!(uuid[6] & 0xF0, 0x40);
        // Check variant bits
        assert_eq!(uuid[8] & 0xC0, 0x80);
    }

    #[test]
    fn test_format_uuid() {
        let mut rng = BitGen::new(42);
        let uuid = random_uuid(&mut rng);
        let s = format_uuid(&uuid);
        assert_eq!(s.len(), 36);
        assert_eq!(s.chars().filter(|&c| c == '-').count(), 4);
    }
}
