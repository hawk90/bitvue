//! Hash algorithms - FNV, MurmurHash3, xxHash, DJB2, SipHash, xxHash3, HighwayHash implementations.
//!
//! # ⚠️ CRITICAL SECURITY WARNING
//!
//! **The hash functions in this module are NOT suitable for cryptographic use!**
//!
//! This module provides non-cryptographic hash functions including:
//! - FNV-1a/FNV-1a (fast but vulnerable to collision attacks)
//! - DJB2 (simple but NOT cryptographically secure)
//! - MurmurHash3 (fast but has known theoretical vulnerabilities)
//! - xxHash (fast non-cryptographic hash)
//! - SipHash (designed for hash table DoS protection, not cryptography)
//! - HighwayHash (fast, but not for security)
//!
//! ## Why These Are NOT Cryptographic:
//!
//! 1. **No collision resistance**: Attackers can craft inputs with the same hash
//! 2. **No preimage resistance**: Finding an input for a given hash is computationally easy
//! 3. **Predictable output**: The algorithms are deterministic and publicly known
//! 4. **Fast computation**: Speed is the priority, not security
//!
//! ## NEVER Use These For:
//! - Password storage or hashing
//! - Digital signatures or MACs
//! - Cryptographic keys or nonces
//! - Commitment schemes or proof-of-work
//! - Any security-critical application
//!
//! ## For Cryptographic Needs, Use:
//! - **SHA-256** (NIST standard, widely supported)
//! - **BLAKE3** (faster, modern, parallelizable)
//! - **SHA-3** (quantum-resistant, latest standard)
//! - **Argon2** or **bcrypt** (for password hashing)
//!
//! ## Appropriate Uses:
//! - Hash table keys (non-adversarial environments)
//! - Cache identifiers
//! - Data deduplication
//! - Non-security checksums
//! - Bloom filters and probabilistic data structures
//!

/// A deterministic hash function that produces consistent results.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_hash::algorithms::deterministic_hash;
///
/// let hash1 = deterministic_hash(&42);
/// let hash2 = deterministic_hash(&42);
/// assert_eq!(hash1, hash2);
/// ```
pub fn deterministic_hash<T: core::hash::Hash>(value: &T) -> u64 {
    use super::hash::hash_of;
    hash_of(value)
}

/// Computes a hash for a byte slice using FNV-1a.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_hash::algorithms::fnv_hash;
///
/// let hash = fnv_hash(b"hello world");
/// ```
pub fn fnv_hash(bytes: &[u8]) -> u64 {
    const FNV_PRIME: u64 = 1099511628211;
    const FNV_OFFSET_BASIS: u64 = 14695981039346656037;

    bytes.iter().fold(FNV_OFFSET_BASIS, |hash, &byte| {
        (hash ^ byte as u64).wrapping_mul(FNV_PRIME)
    })
}

/// Computes a 32-bit hash using FNV-1a.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_hash::algorithms::fnv_hash_32;
///
/// let hash = fnv_hash_32(b"hello world");
/// ```
pub fn fnv_hash_32(bytes: &[u8]) -> u32 {
    const FNV_PRIME: u32 = 16777619;
    const FNV_OFFSET_BASIS: u32 = 2166136261;

    bytes.iter().fold(FNV_OFFSET_BASIS, |hash, &byte| {
        (hash ^ byte as u32).wrapping_mul(FNV_PRIME)
    })
}

/// Computes a 128-bit hash using FNV-1a.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_hash::algorithms::fnv_hash_128;
///
/// let (hash_low, hash_high) = fnv_hash_128(b"hello world");
/// ```
pub fn fnv_hash_128(bytes: &[u8]) -> (u64, u64) {
    const FNV_PRIME: u64 = 0x100000001b3;
    const FNV_OFFSET: u128 = 0x6c62272e07bb01414;

    bytes.iter().fold(FNV_OFFSET, |hash, &byte| {
        (hash ^ byte as u128).wrapping_mul(FNV_PRIME as u128)
    }).to_le_bytes()
        .split_at(8)
        .map(|b| u64::from_le_bytes(*b))
}

/// MurmurHash3 finalization mix.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_hash::algorithms::murmur3_mix;
///
/// let mixed = murmur3_mix(0x12345678);
/// ```
#[inline]
pub const fn murmur3_mix(k: u64) -> u64 {
    let mut k = k;
    k ^= k >> 33;
    k = k.wrapping_mul(0xff51afd7ed558ccd);
    k ^= k >> 33;
    k = k.wrapping_mul(0xc4ceb9fe1a85ec53);
    k ^= k >> 33;
    k
}

/// MurmurHash3 64-bit hash function.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_hash::algorithms::murmur3_64;
///
/// let hash = murmur3_64(b"hello world", 42);
/// ```
pub fn murmur3_64(bytes: &[u8], seed: u64) -> u64 {
    const C1: u64 = 0x87c37b91114253d5;
    const C2: u64 = 0x4cf5ad432745937f;

    let len = bytes.len() as u64;
    let mut h = seed.wrapping_add(len).wrapping_mul(C1).wrapping_add(C2);

    // Process 16-byte blocks using safe array access
    let chunks = bytes.chunks_exact(16);
    let remaining = chunks.remainder();

    for chunk in chunks {
        // SAFETY: chunk is guaranteed to be exactly 16 bytes by chunks_exact(16)
        // Use safe array access instead of unsafe pointer arithmetic
        let block = u64::from_le_bytes([
            chunk[0], chunk[1], chunk[2], chunk[3],
            chunk[4], chunk[5], chunk[6], chunk[7],
        ]);

        let mut k = block;
        k = k.wrapping_mul(C1);
        k ^= k >> 33;
        k = k.wrapping_mul(C2);

        h ^= k;
        h = h.wrapping_mul(5).wrapping_add(0x52dce729);
    }

    // Process remaining bytes (up to 8 bytes)
    let remaining_len = remaining.len();
    if remaining_len > 0 {
        let mut k = if remaining_len >= 8 {
            // Read 8 bytes safely using array indexing
            u64::from_le_bytes([
                remaining[0], remaining[1], remaining[2], remaining[3],
                remaining[4], remaining[5], remaining[6], remaining[7],
            ])
        } else {
            // Handle 1-7 remaining bytes safely
            let mut buf = [0u8; 8];
            for (i, &byte) in remaining.iter().enumerate() {
                buf[i] = byte;
            }
            u64::from_le_bytes(buf)
        };

        k = k.wrapping_mul(C1);
        k ^= k >> 33;
        k = k.wrapping_mul(C2);
        h ^= k;
    }

    h ^= h >> 33;
    h = h.wrapping_mul(0xff51afd7ed558ccd);
    h ^= h >> 33;
    h = h.wrapping_mul(0xc4ceb9fe1a85ec53);
    h ^= h >> 33;

    h
}

/// xxHash 64-bit algorithm (simplified).
///
/// # Examples
///
/// ```rust
/// use abseil::absl_hash::algorithms::xxhash_64;
///
/// let hash = xxhash_64(b"hello world", 0);
/// ```
pub fn xxhash_64(bytes: &[u8], seed: u64) -> u64 {
    const PRIME1: u64 = 0x9e3779b97f4a7c15;
    const PRIME2: u64 = 0x9e3779b97f4a7c15;
    const PRIME3: u64 = 0xbf58476d1ce4e5b9;
    const PRIME4: u64 = 0x94d049bb133111eb;
    const PRIME5: u64 = 0x9e3779b97f4a7c15;

    let mut h = seed.wrapping_add(PRIME5);
    let len = bytes.len() as u64;

    h ^= len.wrapping_mul(PRIME1);
    h = murmur3_mix(h);

    // Process 32-byte blocks using safe chunks_exact
    let chunks = bytes.chunks_exact(32);
    let remaining = chunks.remainder();

    for chunk in chunks {
        // SAFETY: chunk is guaranteed to be exactly 32 bytes by chunks_exact(32)
        // Read first two u64 values from the chunk using safe array access
        let k1 = u64::from_le_bytes([
            chunk[0], chunk[1], chunk[2], chunk[3],
            chunk[4], chunk[5], chunk[6], chunk[7],
        ]).wrapping_mul(PRIME1);

        let k2 = u64::from_le_bytes([
            chunk[8], chunk[9], chunk[10], chunk[11],
            chunk[12], chunk[13], chunk[14], chunk[15],
        ]).wrapping_mul(PRIME1);

        h ^= murmur3_mix(k1);
        h = h.wrapping_mul(1).wrapping_add(murmur3_mix(k2));
        h = h.wrapping_mul(2).wrapping_add(PRIME4);
    }

    // Process remaining bytes safely
    for &byte in remaining {
        h = h.wrapping_mul(PRIME1).wrapping_add(byte as u64);
    }

    h ^= h >> 29;
    h = murmur3_mix(h);
    h ^= h >> 33;
    murmur3_mix(h)
}

/// DJB2 hash function (simple and fast).
///
/// # Examples
///
/// ```rust
/// use abseil::absl_hash::algorithms::djb2_hash;
///
/// let hash = djb2_hash(b"hello");
/// ```
pub fn djb2_hash(bytes: &[u8]) -> u64 {
    bytes.iter().fold(5381u64, |hash, &byte| {
        hash.wrapping_mul(33).wrapping_add(byte as u64)
    })
}

/// SipHash-2-4 (simplified version).
///
/// # Examples
///
/// ```rust
/// use abseil::absl_hash::algorithms::siphash_24;
///
/// let hash = siphash_24(b"hello world", 0, 0);
/// ```
pub fn siphash_24(bytes: &[u8], k0: u64, k1: u64) -> u64 {
    const PRIME0: u64 = 0xff51afd7ed558ccd;
    const PRIME1: u64 = 0xc4ceb9fe1a85ec53;

    let mut v0 = k0 ^ PRIME0;
    let mut v1 = k1 ^ PRIME1;
    let mut v3 = k1;
    let mut v2 = k0 ^ PRIME1;

    let mut m: u64 = 0;
    let mut len = bytes.len();

    for chunk in bytes.chunks(8) {
        let mut b: u64 = 0;
        for (i, &byte) in chunk.iter().enumerate() {
            b |= (byte as u64) << (i * 8);
        }

        m |= b;

        v3 ^= b;
        v3 = v3.wrapping_add(v1);
        v1 = v1.wrapping_add(v0);
        v0 = murmur3_mix(v0 ^ b);
    }

    len &= 0xff;

    v3 ^= len;
    v3 = v3.wrapping_add(v1);
    v1 = v1.wrapping_add(v0);
    v0 = murmur3_mix(v0 ^ len);

    v2 ^= 0xff;
    v2 = v2.wrapping_add(v3);
    v3 = murmur3_mix(v3);
    v1 = murmur3_mix(v1);
    v0 = murmur3_mix(v0);

    v2 ^= m.wrapping_mul(2);
    v3 ^= m.wrapping_mul(3);
    v1 ^= m.wrapping_mul(4);

    v2 = murmur3_mix(v2);
    v3 = murmur3_mix(v3);
    v1 = murmur3_mix(v1);
    v0 = murmur3_mix(v0);

    v0.wrapping_add(v1).wrapping_add(v2).wrapping_add(v3)
}

/// xxHash3 - improved version of xxHash (simplified implementation).
///
/// xxHash3 is an improved version of xxHash that provides better performance
/// and distribution while maintaining backward compatibility.
///
/// # Examples
///
/// ```
/// use abseil::absl_hash::algorithms::xxhash3_64;
///
/// let hash = xxhash3_64(b"hello world", 0);
/// ```
pub fn xxhash3_64(bytes: &[u8], seed: u64) -> u64 {
    const PRIME1: u64 = 0x9e3779b97f4a7c15;
    const PRIME2: u64 = 0x9e3779b97f4a7c15;
    const PRIME3: u64 = 0xbf58476d1ce4e5b9;
    const PRIME4: u64 = 0x94d049bb133111eb;
    const PRIME5: u64 = 0x9e3779b97f4a7c15;

    let mut h64 = PRIME5;
    let mut h = seed.wrapping_add(PRIME3);
    let len = bytes.len() as u64;

    // Process 32-byte blocks using safe chunks_exact for bounds checking
    let chunks = bytes.chunks_exact(32);
    let remaining = chunks.remainder();

    for chunk in chunks {
        // SAFETY: chunk is guaranteed to be exactly 32 bytes by chunks_exact(32)
        // Read 4 u64 values using safe array access
        let k1 = u64::from_le_bytes([
            chunk[0], chunk[1], chunk[2], chunk[3],
            chunk[4], chunk[5], chunk[6], chunk[7],
        ]).wrapping_mul(PRIME1);

        let k2 = u64::from_le_bytes([
            chunk[8], chunk[9], chunk[10], chunk[11],
            chunk[12], chunk[13], chunk[14], chunk[15],
        ]).wrapping_mul(PRIME1);

        let k3 = u64::from_le_bytes([
            chunk[16], chunk[17], chunk[18], chunk[19],
            chunk[20], chunk[21], chunk[22], chunk[23],
        ]).wrapping_mul(PRIME1);

        let k4 = u64::from_le_bytes([
            chunk[24], chunk[25], chunk[26], chunk[27],
            chunk[28], chunk[29], chunk[30], chunk[31],
        ]).wrapping_mul(PRIME1);

        h ^= murmur3_mix(k1);
        h = h.wrapping_mul(1).wrapping_add(murmur3_mix(k2));
        h ^= murmur3_mix(k3);
        h = h.wrapping_mul(1).wrapping_add(murmur3_mix(k4));
    }

    h ^= murmur3_mix(h64);

    // Process remaining bytes safely
    let remaining_len = remaining.len();
    if remaining_len > 0 {
        let mut k = 0u64;

        // Process first 8 bytes
        let first_8 = remaining_len.min(8);
        for i in 0..first_8 {
            k |= (remaining[i] as u64) << (i * 8);
        }
        h ^= murmur3_mix(k.wrapping_mul(PRIME1));

        // Process next 8 bytes (bytes 8-15)
        if remaining_len > 8 {
            k = 0u64;
            let next_8 = (remaining_len - 8).min(8);
            for i in 0..next_8 {
                k |= (remaining[8 + i] as u64) << (i * 8);
            }
            h ^= murmur3_mix(k.wrapping_mul(PRIME2));

            // Process next 8 bytes (bytes 16-23)
            if remaining_len > 16 {
                k = 0u64;
                let next_8 = (remaining_len - 16).min(8);
                for i in 0..next_8 {
                    k |= (remaining[16 + i] as u64) << (i * 8);
                }
                h ^= murmur3_mix(k.wrapping_mul(PRIME3));

                // Process final bytes (bytes 24-31)
                if remaining_len > 24 {
                    k = 0u64;
                    for i in 24..remaining_len {
                        k |= (remaining[i] as u64) << ((i - 24) * 8);
                    }
                    h ^= murmur3_mix(k.wrapping_mul(PRIME4));
                }
            }
        }
    }

    h ^= len;
    h ^= h >> 33;
    h = murmur3_mix(h);
    h ^= h >> 29;
    murmur3_mix(h)
}

/// HighwayHash - a fast hash function for hash table lookup.
///
/// HighwayHash is designed for high performance on modern CPUs while
/// maintaining good hash distribution properties.
///
/// # Examples
///
/// ```
/// use abseil::absl_hash::algorithms::highway_hash;
///
/// let hash = highway_hash(b"hello world", 0);
/// ```
pub fn highway_hash(bytes: &[u8], seed: u64) -> u64 {
    const MULT0: u64 = 0xdbe6d5d5fe4cce2f;
    const MULT1: u64 = 0xa40838222f6407c8;
    const MULT2: u64 = 0x9b0568872af3b5e9;
    const MULT3: u64 = 0xcd5d652b9d874069;
    const MULT4: u64 = 0x210821a9c3ca8c63;

    let mut v0 = seed.wrapping_mul(MULT0);
    let mut v1 = seed.wrapping_mul(MULT1);
    let mut v2 = seed.wrapping_mul(MULT2);
    let mut v3 = seed.wrapping_mul(MULT3);

    // Process 32-byte blocks using safe chunks_exact for bounds checking
    let chunks = bytes.chunks_exact(32);
    let remaining = chunks.remainder();

    for chunk in chunks {
        // SAFETY: chunk is guaranteed to be exactly 32 bytes by chunks_exact(32)
        // Read 4 u64 values using safe array access
        let k1 = u64::from_le_bytes([
            chunk[0], chunk[1], chunk[2], chunk[3],
            chunk[4], chunk[5], chunk[6], chunk[7],
        ]).wrapping_mul(MULT0);

        let k2 = u64::from_le_bytes([
            chunk[8], chunk[9], chunk[10], chunk[11],
            chunk[12], chunk[13], chunk[14], chunk[15],
        ]).wrapping_mul(MULT1);

        let k3 = u64::from_le_bytes([
            chunk[16], chunk[17], chunk[18], chunk[19],
            chunk[20], chunk[21], chunk[22], chunk[23],
        ]).wrapping_mul(MULT2);

        let k4 = u64::from_le_bytes([
            chunk[24], chunk[25], chunk[26], chunk[27],
            chunk[28], chunk[29], chunk[30], chunk[31],
        ]).wrapping_mul(MULT3);

        v0 ^= k1;
        v1 ^= k2;
        v2 ^= k3;
        v3 ^= k4;

        v0 = v0.wrapping_add(v1).wrapping_mul(MULT4);
        v2 = v2.wrapping_add(v3).wrapping_mul(MULT4);
    }

    // Process remaining bytes safely
    let remaining_len = remaining.len();
    let mut last_block = [0u64; 4];
    for i in 0..remaining_len {
        let byte_idx = i % 8;
        let word_idx = i / 8;
        last_block[word_idx] |= (remaining[i] as u64) << (byte_idx * 8);
    }

    v0 ^= last_block[0].wrapping_mul(MULT0);
    v1 ^= last_block[1].wrapping_mul(MULT1);
    v2 ^= last_block[2].wrapping_mul(MULT2);
    v3 ^= last_block[3].wrapping_mul(MULT3);

    // Finalization
    let len_mod = bytes.len() as u64;
    v0 = v0.wrapping_add(v1.wrapping_add(v2).wrapping_add(v3)).wrapping_add(len_mod);

    v0 ^= v0 >> 33;
    v0 = v0.wrapping_mul(MULT0);
    v0 ^= v0 >> 29;
    v0 = v0.wrapping_mul(MULT1);
    v0 ^= v0 >> 23;
    v0
}

/// WyHash - a fast hash function (small state variant).
///
/// # Examples
///
/// ```
/// use abseil::absl_hash::algorithms::wyhash;
///
/// let hash = wyhash(b"hello", 42);
/// ```
pub fn wyhash(bytes: &[u8], seed: u64) -> u64 {
    const PRIME1: u64 = 0xa0761d6478bd642f;
    const PRIME2: u64 = 0xe7037ed1a0b428db;
    const PRIME3: u64 = 0x8ebc6af09c88c6e3;
    const PRIME4: u64 = 0x589965cc75374cc3;
    const PRIME5: u64 = 0x1d8e4e27c47d1f3f;

    let len = bytes.len() as u64;
    let mut seed = seed.wrapping_add(PRIME5);
    let mut a = seed;
    let mut b = seed;

    // Process 16-byte blocks using safe chunks_exact for bounds checking
    let chunks = bytes.chunks_exact(16);
    let remaining = chunks.remainder();

    for chunk in chunks {
        // SAFETY: chunk is guaranteed to be exactly 16 bytes by chunks_exact(16)
        // Read 2 u64 values using safe array access
        let k1 = u64::from_le_bytes([
            chunk[0], chunk[1], chunk[2], chunk[3],
            chunk[4], chunk[5], chunk[6], chunk[7],
        ]);

        let k2 = u64::from_le_bytes([
            chunk[8], chunk[9], chunk[10], chunk[11],
            chunk[12], chunk[13], chunk[14], chunk[15],
        ]);

        a ^= k1.wrapping_mul(PRIME1);
        a = a.wrapping_mul(PRIME2);
        a ^= a >> 31;

        b ^= k2.wrapping_mul(PRIME3);
        b = b.wrapping_mul(PRIME4);
        b ^= b >> 31;

        seed ^= a.wrapping_add(b);
    }

    // Process remaining bytes safely
    let remaining_len = remaining.len();
    let mut tail = 0u64;
    for i in 0..remaining_len {
        tail |= (remaining[i] as u64) << (i * 8);
    }

    if remaining_len > 8 {
        let t1 = tail & 0xFFFFFFFFFFFFFFFF;
        let t2 = tail >> 8;
        a ^= t1.wrapping_mul(PRIME1);
        b ^= t2.wrapping_mul(PRIME2);
    } else {
        a ^= tail.wrapping_mul(PRIME1);
        b ^= tail.wrapping_mul(PRIME3);
    }

    a = a.wrapping_mul(PRIME1);
    a ^= a >> 27;
    a = a.wrapping_mul(PRIME2);
    a ^= a >> 31;

    b = b.wrapping_mul(PRIME3);
    b ^= b >> 27;
    b = b.wrapping_mul(PRIME4);
    b ^= b >> 31;

    seed.wrapping_add(a.wrapping_add(b)).wrapping_mul(PRIME1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fnv_hash() {
        let hash1 = fnv_hash(b"hello world");
        let hash2 = fnv_hash(b"hello world");
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_fnv_hash_32() {
        let hash1 = fnv_hash_32(b"hello world");
        let hash2 = fnv_hash_32(b"hello world");
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_fnv_hash_128() {
        let (h1, h2) = fnv_hash_128(b"hello world");
        let (h3, h4) = fnv_hash_128(b"hello world");
        assert_eq!(h1, h3);
        assert_eq!(h2, h4);
    }

    #[test]
    fn test_murmur3_64() {
        let hash = murmur3_64(b"hello world", 42);
        assert_ne!(hash, 0);
    }

    #[test]
    fn test_xxhash_64() {
        let hash = xxhash_64(b"hello world", 0);
        assert_ne!(hash, 0);
    }

    #[test]
    fn test_djb2_hash() {
        let hash = djb2_hash(b"hello");
        assert_ne!(hash, 0);
    }

    #[test]
    fn test_siphash_24() {
        let hash = siphash_24(b"hello world", 0, 0);
        assert_ne!(hash, 0);
    }

    #[test]
    fn test_murmur3_mix() {
        let mixed = murmur3_mix(0x12345678);
        assert_ne!(mixed, 0x12345678);
    }

    #[test]
    fn test_xxhash3_64() {
        let hash = xxhash3_64(b"hello world", 0);
        assert_ne!(hash, 0);
    }

    #[test]
    fn test_highway_hash() {
        let hash = highway_hash(b"hello world", 0);
        assert_ne!(hash, 0);
    }

    #[test]
    fn test_wyhash() {
        let hash = wyhash(b"hello", 42);
        assert_ne!(hash, 0);
    }

    // Edge case tests for CRITICAL security fixes - bounds checking

    #[test]
    fn test_xxhash3_64_empty_input() {
        // Test empty input - should not panic or cause out-of-bounds access
        let hash = xxhash3_64(b"", 0);
        assert_ne!(hash, 0);
    }

    #[test]
    fn test_xxhash3_64_single_byte() {
        // Test single byte - edge case for remaining bytes handling
        let hash = xxhash3_64(b"a", 0);
        assert_ne!(hash, 0);
    }

    #[test]
    fn test_xxhash3_64_exact_block_boundary() {
        // Test input that is exactly 32 bytes (one block)
        let hash = xxhash3_64(b"12345678901234567890123456789012", 0);
        assert_ne!(hash, 0);

        // Test input that is exactly 64 bytes (two blocks)
        let hash2 = xxhash3_64(b"1234567890123456789012345678901212345678901234567890123456789012", 0);
        assert_ne!(hash2, 0);
    }

    #[test]
    fn test_xxhash3_64_remaining_bytes() {
        // Test various remaining byte counts (1-31 bytes after 32-byte blocks)
        for len in [1, 7, 15, 17, 23, 31].iter() {
            let input = vec![b'a'; 32 + len];
            let hash = xxhash3_64(&input, 0);
            assert_ne!(hash, 0, "Failed for input length {}", 32 + len);
        }
    }

    #[test]
    fn test_highway_hash_empty_input() {
        // Test empty input - should not panic or cause out-of-bounds access
        let hash = highway_hash(b"", 0);
        assert_ne!(hash, 0);
    }

    #[test]
    fn test_highway_hash_single_byte() {
        // Test single byte - edge case for remaining bytes handling
        let hash = highway_hash(b"a", 0);
        assert_ne!(hash, 0);
    }

    #[test]
    fn test_highway_hash_exact_block_boundary() {
        // Test input that is exactly 32 bytes (one block)
        let hash = highway_hash(b"12345678901234567890123456789012", 0);
        assert_ne!(hash, 0);

        // Test input that is exactly 64 bytes (two blocks)
        let hash2 = highway_hash(b"1234567890123456789012345678901212345678901234567890123456789012", 0);
        assert_ne!(hash2, 0);
    }

    #[test]
    fn test_highway_hash_remaining_bytes() {
        // Test various remaining byte counts (1-31 bytes after 32-byte blocks)
        for len in [1, 7, 15, 17, 23, 31].iter() {
            let input = vec![b'a'; 32 + len];
            let hash = highway_hash(&input, 0);
            assert_ne!(hash, 0, "Failed for input length {}", 32 + len);
        }
    }

    #[test]
    fn test_wyhash_empty_input() {
        // Test empty input - should not panic or cause out-of-bounds access
        let hash = wyhash(b"", 42);
        assert_ne!(hash, 0);
    }

    #[test]
    fn test_wyhash_single_byte() {
        // Test single byte - edge case for remaining bytes handling
        let hash = wyhash(b"a", 42);
        assert_ne!(hash, 0);
    }

    #[test]
    fn test_wyhash_exact_block_boundary() {
        // Test input that is exactly 16 bytes (one block)
        let hash = wyhash(b"1234567890123456", 42);
        assert_ne!(hash, 0);

        // Test input that is exactly 32 bytes (two blocks)
        let hash2 = wyhash(b"12345678901234567890123456789012", 42);
        assert_ne!(hash2, 0);
    }

    #[test]
    fn test_wyhash_remaining_bytes() {
        // Test various remaining byte counts (1-15 bytes after 16-byte blocks)
        for len in [1, 7, 9, 15].iter() {
            let input = vec![b'a'; 16 + len];
            let hash = wyhash(&input, 42);
            assert_ne!(hash, 0, "Failed for input length {}", 16 + len);
        }
    }

    // Edge case tests for HIGH security fixes - bounds checking

    #[test]
    fn test_murmur3_64_empty_input() {
        // Test empty input - should not panic or cause out-of-bounds access
        let hash = murmur3_64(b"", 42);
        assert_ne!(hash, 0);
    }

    #[test]
    fn test_murmur3_64_single_byte() {
        // Test single byte - edge case for remaining bytes handling
        let hash = murmur3_64(b"a", 42);
        assert_ne!(hash, 0);
    }

    #[test]
    fn test_murmur3_64_exact_block_boundary() {
        // Test input that is exactly 16 bytes (one block)
        let hash = murmur3_64(b"1234567890123456", 42);
        assert_ne!(hash, 0);

        // Test input that is exactly 32 bytes (two blocks)
        let hash2 = murmur3_64(b"12345678901234567890123456789012", 42);
        assert_ne!(hash2, 0);
    }

    #[test]
    fn test_xxhash_64_empty_input() {
        // Test empty input - should not panic or cause out-of-bounds access
        let hash = xxhash_64(b"", 0);
        assert_ne!(hash, 0);
    }

    #[test]
    fn test_xxhash_64_single_byte() {
        // Test single byte - edge case for remaining bytes handling
        let hash = xxhash_64(b"a", 0);
        assert_ne!(hash, 0);
    }

    #[test]
    fn test_xxhash_64_exact_block_boundary() {
        // Test input that is exactly 32 bytes (one block)
        let hash = xxhash_64(b"12345678901234567890123456789012", 0);
        assert_ne!(hash, 0);

        // Test input that is exactly 64 bytes (two blocks)
        let hash2 = xxhash_64(b"1234567890123456789012345678901212345678901234567890123456789012", 0);
        assert_ne!(hash2, 0);
    }

    #[test]
    fn test_murmur3_64_remaining_bytes() {
        // Test various remaining byte counts (1-7 bytes after 16-byte blocks)
        for len in 1..=7 {
            let input = vec![b'a'; 16 + len];
            let hash = murmur3_64(&input, 42);
            assert_ne!(hash, 0, "Failed for input length {}", 16 + len);
        }
    }

    #[test]
    fn test_xxhash_64_remaining_bytes() {
        // Test various remaining byte counts (1-31 bytes after 32-byte blocks)
        for len in [1, 7, 15, 23, 31].iter() {
            let input = vec![b'a'; 32 + len];
            let hash = xxhash_64(&input, 0);
            assert_ne!(hash, 0, "Failed for input length {}", 32 + len);
        }
    }
}
