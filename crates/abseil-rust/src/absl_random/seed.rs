//! Seed module - seed types and creation.

/// Seed type for random number generators.
pub type Seed = u64;

/// Creates a new seed from a value.
#[inline]
pub const fn seed_from(value: u64) -> Seed {
    value
}

/// Creates a seed from the current time (placeholder).
///
/// In no_std environments, this returns a fixed seed.
/// With std, this would use system time.
#[inline]
pub fn seed_from_time() -> Seed {
    // Fixed seed for no_std
    0x123456789ABCDEF
}

/// Creates a seed from an array of bytes.
///
/// Uses a simple hash of the bytes to create a seed.
#[inline]
pub fn seed_from_bytes(bytes: &[u8]) -> Seed {
    let mut hash: Seed = 0xCBF29CE484222325;
    for &byte in bytes {
        hash = hash.wrapping_mul(0x100000001B3);
        hash ^= byte as Seed;
    }
    hash
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_seed_from_bytes() {
        let seed1 = seed_from_bytes(b"test");
        let seed2 = seed_from_bytes(b"test");
        assert_eq!(seed1, seed2);

        let seed3 = seed_from_bytes(b"different");
        assert_ne!(seed1, seed3);
    }
}
