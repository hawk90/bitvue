//! CRC lookup tables for fast computation.
//!
//! This module provides pre-computed lookup tables for CRC-32 and CRC-32C,
//! which can significantly speed up CRC computation.

/// CRC-32C lookup table for fast computation.
///
/// This table can be used to speed up CRC-32C calculations.
pub const CRC32C_TABLE: [u32; 256] = generate_crc32c_table();

const fn generate_crc32c_table() -> [u32; 256] {
    let mut table = [0u32; 256];
    let mut i = 0;
    while i < 256 {
        let mut crc = (i as u32) << 24;
        let mut j = 0;
        while j < 8 {
            if crc & 0x80000000 != 0 {
                crc = (crc << 1) ^ 0x1edc6f41;
            } else {
                crc <<= 1;
            }
            j += 1;
        }
        table[i as usize] = crc.reverse_bits();
        i += 1;
    }
    table
}

/// CRC-32 lookup table for fast computation.
///
/// This table can be used to speed up CRC-32 calculations.
pub const CRC32_TABLE: [u32; 256] = generate_crc32_table();

const fn generate_crc32_table() -> [u32; 256] {
    let mut table = [0u32; 256];
    let mut i = 0;
    while i < 256 {
        let mut crc = (i as u32) << 24;
        let mut j = 0;
        while j < 8 {
            if crc & 0x80000000 != 0 {
                crc = (crc << 1) ^ 0x04c11db7;
            } else {
                crc <<= 1;
            }
            j += 1;
        }
        table[i as usize] = crc.reverse_bits();
        i += 1;
    }
    table
}

/// Computes CRC-32C using the lookup table.
///
/// # Examples
///
/// ```
/// use abseil::absl_crc::tables::crc32c_fast;
///
/// let data = b"123456789";
/// let crc = crc32c_fast(data);
/// assert_eq!(crc, 0xe3069283);
/// ```
pub fn crc32c_fast(data: &[u8]) -> u32 {
    let mut crc = 0xffffffff;
    for &byte in data {
        let index = ((crc as u8) ^ byte) as usize;
        crc = (crc >> 8) ^ CRC32C_TABLE[index];
    }
    !crc
}

/// Computes CRC-32 using the lookup table.
///
/// # Examples
///
/// ```
/// use abseil::absl_crc::tables::crc32_fast;
///
/// let data = b"123456789";
/// let crc = crc32_fast(data);
/// assert_eq!(crc, 0xcbf43926);
/// ```
pub fn crc32_fast(data: &[u8]) -> u32 {
    let mut crc = 0xffffffff;
    for &byte in data {
        let index = ((crc as u8) ^ byte) as usize;
        crc = (crc >> 8) ^ CRC32_TABLE[index];
    }
    !crc
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_crc32_fast() {
        let data = b"123456789";
        let crc = crc32_fast(data);
        assert_eq!(crc, 0xcbf43926);
    }

    #[test]
    fn test_crc32_fast_large_data() {
        let data = vec![0x42u8; 10000];
        let crc = crc32_fast(&data);
        assert_ne!(crc, 0);
    }

    #[test]
    fn test_crc32c_fast() {
        let data = b"123456789";
        let crc = crc32c_fast(data);
        assert_eq!(crc, 0xe3069283);
    }

    #[test]
    fn test_crc32c_table_generation() {
        // Verify the table is generated at compile time
        assert_eq!(CRC32C_TABLE.len(), 256);
        // First entry should be 0
        assert_eq!(CRC32C_TABLE[0], 0);
        // Table should have varied values
        let mut seen_different = false;
        let mut i = 1;
        while i < 256 {
            if CRC32C_TABLE[i] != CRC32C_TABLE[0] {
                seen_different = true;
                break;
            }
            i += 1;
        }
        assert!(seen_different);
    }

    #[test]
    fn test_crc32_table_generation() {
        // Verify the table is generated at compile time
        assert_eq!(CRC32_TABLE.len(), 256);
        assert_eq!(CRC32_TABLE[0], 0);
    }
}
