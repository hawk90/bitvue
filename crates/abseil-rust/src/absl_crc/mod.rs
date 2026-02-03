//! CRC (Cyclic Redundancy Check) utilities.
//!
//! This module provides CRC checksum computation (similar to Abseil's `absl/crc`)
//! which is used for error detection in data transmission and storage.
//!
//! # Overview
//!
//! CRC algorithms compute checksums that can detect accidental changes
//! to data. Common CRC standards include CRC-32, CRC-32C, CRC-64, etc.
//!
//! # Modules
//!
//! - [`crc32`] - CRC-32 checksums
//! - [`crc64`] - CRC-64 checksums
//! - [`variants`] - CRC algorithm variants and parameters
//! - [`computer`] - Stateful CRC computers
//! - [`tables`] - Lookup tables for fast computation
//! - [`file_formats`] - File format verification (PNG, ZIP, GZIP, etc.)
//! - [`utils`] - Utility functions (hex conversion, bit manipulation)
//!
//! # Examples
//!
//! ```rust
//! use abseil::absl_crc;
//!
//! // Simple CRC-32 computation
//! let data = b"Hello, world!";
//! let checksum = absl_crc::crc32::crc32(data);
//! println!("CRC-32: {:#x}", checksum);
//!
//! // Using CRC-32C (Castagnoli)
//! let checksum_c = absl_crc::crc32::crc32c(data);
//! println!("CRC-32C: {:#x}", checksum_c);
//!
//! // Using a specific variant
//! use absl_crc::variants::{Crc32Variant, crc32_variant};
//! let checksum_mpeg2 = crc32_variant(data, Crc32Variant::Mpeg2);
//! println!("CRC-32/MPEG-2: {:#x}", checksum_mpeg2);
//! ```

pub mod crc32;
pub mod crc64;
pub mod variants;
pub mod computer;
pub mod tables;
pub mod file_formats;
pub mod utils;

// Re-exports for convenience
pub use crc32::{crc32, crc32c};
pub use crc64::crc64;
pub use variants::{
    CrcAlgorithm, CrcParams, Crc32Variant, Crc64Variant,
    CRC32_MPEG_2, CRC32_BZIP2, CRC32_POSIX, CRC32_JAMCRC, CRC32_XFER,
    Crc32Presets, Crc64Presets, Crc16Presets, Crc8Presets,
};
pub use computer::{CrcComputer, TableCrcComputer, StreamingCrc, CrcBuilder};
pub use tables::{crc32_fast, crc32c_fast, CRC32_TABLE, CRC32C_TABLE};
pub use file_formats::{
    verify_png_chunk, compute_png_chunk_crc,
    verify_zip_entry, compute_zip_crc,
    verify_gzip, compute_gzip_crc,
    verify_ethernet_frame, compute_ethernet_crc,
    verify_scsi_data, compute_scsi_crc,
};
pub use utils::{
    reflect_bits, reflect_u64, reflect_u32, reflect_u16, reflect_u8,
    swap_bytes_u32, swap_bytes_u64, swap_bytes_u16,
    crc_to_hex, crc64_to_hex, hex_to_crc, hex_to_crc64,
    crc_hamming_distance, crc64_hamming_distance,
    crc_xor, crc64_xor, crc_negate, crc64_negate,
    crc_is_zero, crc64_is_zero, crc_match,
    crc_algorithm_width, is_crc32, is_crc64,
};

// Legacy function wrappers for backward compatibility
use crate::variants::CrcAlgorithm;
use crate::computer::CrcComputer;

/// Computes CRC using the specified algorithm.
///
/// # Examples
///
/// ```
/// use abseil::absl_crc::{compute_crc, CrcAlgorithm};
///
/// let data = b"Hello, world!";
/// let crc = compute_crc(CrcAlgorithm::Crc32, data);
/// println!("CRC-32: {:#x}", crc);
/// ```
pub fn compute_crc(algorithm: CrcAlgorithm, data: &[u8]) -> u64 {
    match algorithm {
        CrcAlgorithm::Crc32 => crc32::crc32(data) as u64,
        CrcAlgorithm::Crc32C => crc32::crc32c(data) as u64,
        CrcAlgorithm::Crc64Ecma | CrcAlgorithm::Crc64Iso => crc64::crc64(data),
        CrcAlgorithm::Crc16 => crc16(data, 0x8005, 0x0000, true, true) as u64,
        CrcAlgorithm::Crc16Ccitt => crc16(data, 0x1021, 0xffff, false, false) as u64,
        CrcAlgorithm::Crc8 => crc8(data) as u64,
    }
}

/// Computes CRC-32 with the specified variant.
///
/// # Examples
///
/// ```
/// use abseil::absl_crc::{crc32_variant, Crc32Variant};
///
/// let data = b"123456789";
/// let crc = crc32_variant(data, Crc32Variant::Mpeg2);
/// ```
pub fn crc32_variant(data: &[u8], variant: Crc32Variant) -> u32 {
    variant.params().compute(data) as u32
}

/// Computes CRC-32/MPEG-2.
pub fn crc32_mpeg2(data: &[u8]) -> u32 {
    CRC32_MPEG_2.compute(data) as u32
}

/// Computes CRC-32/BZIP2.
pub fn crc32_bzip2(data: &[u8]) -> u32 {
    CRC32_BZIP2.compute(data) as u32
}

/// Computes CRC-32/POSIX.
pub fn crc32_posix(data: &[u8]) -> u32 {
    CRC32_POSIX.compute(data) as u32
}

/// Computes CRC-32/JAMCRC.
pub fn crc32_jamcrc(data: &[u8]) -> u32 {
    CRC32_JAMCRC.compute(data) as u32
}

/// Computes CRC-32/XFER.
pub fn crc32_xfer(data: &[u8]) -> u32 {
    CRC32_XFER.compute(data) as u32
}

/// Computes CRC-64 with the specified variant.
pub fn crc64_variant(data: &[u8], variant: Crc64Variant) -> u64 {
    variant.params().compute(data)
}

/// Computes CRC-64-Jones.
pub fn crc64_jones(data: &[u8]) -> u64 {
    Crc64Variant::Jones.params().compute(data)
}

/// Computes CRC-16 with standard parameters.
pub fn crc16_standard(data: &[u8]) -> u16 {
    crc16(data, 0x8005, 0x0000, true, true)
}

/// Computes CRC-16-CCITT with standard parameters.
pub fn crc16_ccitt(data: &[u8]) -> u16 {
    crc16(data, 0x1021, 0xffff, false, false)
}

/// Computes CRC-8 with standard parameters.
pub fn crc8_standard(data: &[u8]) -> u8 {
    crc8(data)
}

/// CRC-16 computation.
fn crc16(data: &[u8], poly: u16, init: u16, reflect_in: bool, reflect_out: bool) -> u16 {
    let mut crc = init;

    for &byte in data {
        let byte = if reflect_in {
            byte.reverse_bits() as u16
        } else {
            byte as u16
        };

        crc ^= byte << 8;

        for _ in 0..8 {
            if crc & 0x8000 != 0 {
                crc = (crc << 1) ^ poly;
            } else {
                crc <<= 1;
            }
        }
    }

    if reflect_out {
        crc.reverse_bits()
    } else {
        crc
    }
}

/// CRC-8 computation.
fn crc8(data: &[u8]) -> u8 {
    const POLY: u8 = 0x07;
    let mut crc: u8 = 0x00;

    for &byte in data {
        crc ^= byte;
        for _ in 0..8 {
            if crc & 0x80 != 0 {
                crc = (crc << 1) ^ POLY;
            } else {
                crc <<= 1;
            }
        }
    }

    crc
}

/// Computes the CRC of a slice in chunks.
pub fn compute_crc_chunked(data: &[u8], chunk_size: usize, algorithm: CrcAlgorithm) -> u64 {
    let mut computer = CrcComputer::new(algorithm);

    for chunk in data.chunks(chunk_size) {
        computer.update(chunk);
    }

    computer.finalize()
}

/// Combines two CRC values of the same algorithm.
pub fn combine_crcs(crc1: u32, crc2: u32, len2: usize, poly: u32, bits: u8) -> u32 {
    if bits != 32 {
        return crc1 ^ crc2;
    }

    let mut crc1 = crc1;
    let mut len = len2;

    for _ in 0..len {
        let temp = crc1 ^ 0x80000000;
        crc1 = if temp & 0x80000000 != 0 {
            (temp << 1) ^ poly
        } else {
            temp << 1
        };
    }

    crc1 ^ crc2
}

/// Combines multiple CRC values computed in parallel.
pub fn combine_crc_parallel(crcs: &[u32], lengths: &[usize], poly: u32) -> u32 {
    if crcs.is_empty() {
        return 0;
    }

    let mut result = crcs[0];
    let mut accumulated_len = 0;

    for i in 1..crcs.len() {
        let len = lengths.get(i).copied().unwrap_or(0);
        accumulated_len += len;

        let mut crc = crcs[i - 1];
        for _ in 0..accumulated_len {
            let temp = crc ^ 0x80000000;
            crc = if temp & 0x80000000 != 0 {
                (temp << 1) ^ poly
            } else {
                temp << 1
            };
        }

        result ^= crc ^ crcs[i];
    }

    result
}

/// Splits data into chunks and computes CRCs in parallel (conceptual).
pub fn crc_parallel_chunks(data: &[u8], chunk_size: usize, algorithm: CrcAlgorithm) -> u64 {
    let mut results = alloc::vec::Vec::new();
    let mut lengths = alloc::vec::Vec::new();

    for chunk in data.chunks(chunk_size) {
        let crc = compute_crc(algorithm, chunk);
        results.push(crc as u32);
        lengths.push(chunk.len());
    }

    match algorithm {
        CrcAlgorithm::Crc32 | CrcAlgorithm::Crc32C => {
            let poly = algorithm.polynomial() as u32;
            combine_crc_parallel(&results, &lengths, poly) as u64
        }
        _ => {
            results.iter().fold(0u64, |acc, &crc| acc ^ crc as u64)
        }
    }
}

/// Validates data using a CRC checksum.
pub fn validate_crc(data: &[u8], checksum: u64, algorithm: CrcAlgorithm) -> bool {
    compute_crc(algorithm, data) == checksum
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::variants::Crc32Variant;

    #[test]
    fn test_compute_crc_crc32() {
        let data = b"123456789";
        let crc = compute_crc(CrcAlgorithm::Crc32, data);
        assert_eq!(crc, 0xcbf43926);
    }

    #[test]
    fn test_compute_crc_crc32c() {
        let data = b"123456789";
        let crc = compute_crc(CrcAlgorithm::Crc32C, data);
        assert_eq!(crc, 0xe3069283);
    }

    #[test]
    fn test_crc16_standard() {
        let data = b"123456789";
        let crc = crc16_standard(data);
        assert_ne!(crc, 0);
    }

    #[test]
    fn test_crc16_ccitt() {
        let data = b"123456789";
        let crc = crc16_ccitt(data);
        assert_ne!(crc, 0);
    }

    #[test]
    fn test_crc8_standard() {
        let data = b"123456789";
        let crc = crc8_standard(data);
        assert_ne!(crc, 0);
    }

    #[test]
    fn test_validate_crc_valid() {
        let data = b"123456789";
        let checksum = 0xcbf43926;
        assert!(validate_crc(data, checksum, CrcAlgorithm::Crc32));
    }

    #[test]
    fn test_validate_crc_invalid() {
        let data = b"123456789";
        let checksum = 0x12345678;
        assert!(!validate_crc(data, checksum, CrcAlgorithm::Crc32));
    }

    #[test]
    fn test_compute_crc_chunked() {
        let data = b"123456789";
        let crc1 = compute_crc_chunked(data, 3, CrcAlgorithm::Crc32);
        let crc2 = compute_crc(CrcAlgorithm::Crc32, data);
        assert_eq!(crc1, crc2);
    }

    #[test]
    fn test_crc32_variant_standard() {
        let data = b"123456789";
        let crc = crc32_variant(data, Crc32Variant::Standard);
        assert_eq!(crc, 0xcbf43926);
    }

    #[test]
    fn test_crc32_variant_castagnoli() {
        let data = b"123456789";
        let crc = crc32_variant(data, Crc32Variant::Castagnoli);
        assert_eq!(crc, 0xe3069283);
    }

    #[test]
    fn test_crc32_mpeg2() {
        let data = b"123456789";
        let crc = crc32_mpeg2(data);
        assert_ne!(crc, 0);
    }

    #[test]
    fn test_crc32_bzip2() {
        let data = b"123456789";
        let crc = crc32_bzip2(data);
        assert_ne!(crc, 0);
    }

    #[test]
    fn test_crc32_posix() {
        let data = b"123456789";
        let crc = crc32_posix(data);
        assert_ne!(crc, 0);
    }

    #[test]
    fn test_crc32_jamcrc() {
        let data = b"123456789";
        let crc = crc32_jamcrc(data);
        let standard = crc32_variant(data, Crc32Variant::Standard);
        assert_ne!(crc, standard);
    }

    #[test]
    fn test_crc32_xfer() {
        let data = b"123456789";
        let crc = crc32_xfer(data);
        assert_ne!(crc, 0);
    }

    #[test]
    fn test_crc64_variant_ecma() {
        let data = b"123456789";
        let crc = crc64_variant(data, Crc64Variant::Ecma);
        assert_eq!(crc, 0x6c40df5f0b497347);
    }

    #[test]
    fn test_crc64_jones() {
        let data = b"123456789";
        let crc = crc64_jones(data);
        assert_ne!(crc, 0);
    }

    #[test]
    fn test_combine_crc_parallel() {
        let crcs = vec![0x12345678, 0x9abcdef0, 0x11111111];
        let lengths = vec![10, 20, 15];
        let combined = combine_crc_parallel(&crcs, &lengths, 0x04c11db7);
        assert_ne!(combined, 0);
    }

    #[test]
    fn test_combine_crc_parallel_empty() {
        let crcs: Vec<u32> = vec![];
        let lengths: Vec<usize> = vec![];
        let combined = combine_crc_parallel(&crcs, &lengths, 0x04c11db7);
        assert_eq!(combined, 0);
    }

    #[test]
    fn test_crc_parallel_chunks() {
        let data = b"The quick brown fox jumps over the lazy dog";
        let crc_parallel = crc_parallel_chunks(data, 4, CrcAlgorithm::Crc32);
        let crc_serial = compute_crc(CrcAlgorithm::Crc32, data);
        assert_eq!(crc_parallel, crc_serial);
    }
}
