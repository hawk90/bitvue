//! CRC verification for common file formats.
//!
//! This module provides functions for verifying CRC checksums in various
//! file formats like PNG, ZIP, GZIP, Ethernet frames, and SCSI data.

use crate::crc32::{self, crc32c_fast};

/// Verifies PNG file data using CRC-32.
///
/// PNG files store CRC-32 checksums for each chunk.
///
/// # Examples
///
/// ```
/// use abseil::absl_crc::file_formats::verify_png_chunk;
///
/// let chunk_data = b"IDAT\x00\x00\x00\x01";
/// let chunk_crc = 0x12345678u32;
/// let result = verify_png_chunk(chunk_data, chunk_crc);
/// ```
pub fn verify_png_chunk(chunk_data: &[u8], expected_crc: u32) -> bool {
    crc32::crc32(chunk_data) == expected_crc
}

/// Computes CRC for a PNG chunk (including chunk type).
///
/// # Examples
///
/// ```
/// use abseil::absl_crc::file_formats::compute_png_chunk_crc;
///
/// let chunk_type = b"IDAT";
/// let chunk_data = b"some data";
/// let crc = compute_png_chunk_crc(chunk_type, chunk_data);
/// ```
pub fn compute_png_chunk_crc(chunk_type: &[u8], chunk_data: &[u8]) -> u32 {
    let mut computer = crate::computer::CrcComputer::new(crate::variants::CrcAlgorithm::Crc32);
    computer.update(chunk_type);
    computer.update(chunk_data);
    computer.finalize() as u32
}

/// Verifies ZIP file data using CRC-32.
///
/// ZIP files store CRC-32 checksums for each file.
///
/// # Examples
///
/// ```
/// use abseil::absl_crc::file_formats::verify_zip_entry;
///
/// let file_data = b"File contents here";
/// let expected_crc = 0x12345678u32;
/// let result = verify_zip_entry(file_data, expected_crc);
/// ```
pub fn verify_zip_entry(file_data: &[u8], expected_crc: u32) -> bool {
    crc32::crc32(file_data) == expected_crc
}

/// Computes CRC-32 for a ZIP entry (using standard CRC-32).
///
/// # Examples
///
/// ```
/// use abseil::absl_crc::file_formats::compute_zip_crc;
///
/// let data = b"File data here";
/// let crc = compute_zip_crc(data);
/// ```
pub fn compute_zip_crc(data: &[u8]) -> u32 {
    crc32::crc32(data)
}

/// Verifies GZIP file data using CRC-32.
///
/// GZIP files store a CRC-32 of the uncompressed data.
///
/// # Examples
///
/// ```
/// use abseil::absl_crc::file_formats::verify_gzip;
///
/// let uncompressed_data = b"Hello, world!";
/// let expected_crc = 0x1c291ca3u32;
/// let result = verify_gzip(uncompressed_data, expected_crc);
/// ```
pub fn verify_gzip(uncompressed_data: &[u8], expected_crc: u32) -> bool {
    crc32::crc32(uncompressed_data) == expected_crc
}

/// Computes CRC-32 for GZIP (standard CRC-32).
///
/// # Examples
///
/// ```
/// use abseil::absl_crc::file_formats::compute_gzip_crc;
///
/// let data = b"Hello, world!";
/// let crc = compute_gzip_crc(data);
/// ```
pub fn compute_gzip_crc(data: &[u8]) -> u32 {
    crc32::crc32(data)
}

/// Verifies Ethernet frame using CRC-32.
///
/// Ethernet frames use CRC-32 for error detection.
///
/// # Examples
///
/// ```
/// use abseil::absl_crc::file_formats::verify_ethernet_frame;
///
/// let frame_data = b"Ethernet frame payload";
/// let expected_crc = 0x12345678u32;
/// let result = verify_ethernet_frame(frame_data, expected_crc);
/// ```
pub fn verify_ethernet_frame(frame_data: &[u8], expected_crc: u32) -> bool {
    crc32::crc32(frame_data) == expected_crc
}

/// Computes CRC-32 for an Ethernet frame.
///
/// # Examples
///
/// ```
/// use abseil::absl_crc::file_formats::compute_ethernet_crc;
///
/// let frame_data = b"Ethernet frame payload";
/// let crc = compute_ethernet_crc(frame_data);
/// ```
pub fn compute_ethernet_crc(frame_data: &[u8]) -> u32 {
    crc32::crc32(frame_data)
}

/// Verifies SCSI data using CRC-32C.
///
/// SCSI uses CRC-32C for data protection.
///
/// # Examples
///
/// ```
/// use abseil::absl_crc::file_formats::verify_scsi_data;
///
/// let scsi_data = b"SCSI data block";
/// let expected_crc = 0xe3069283u32;
/// let result = verify_scsi_data(scsi_data, expected_crc);
/// ```
pub fn verify_scsi_data(data: &[u8], expected_crc: u32) -> bool {
    crc32c_fast(data) == expected_crc
}

/// Computes CRC-32C for SCSI.
///
/// # Examples
///
/// ```
/// use abseil::absl_crc::file_formats::compute_scsi_crc;
///
/// let data = b"SCSI data block";
/// let crc = compute_scsi_crc(data);
/// ```
pub fn compute_scsi_crc(data: &[u8]) -> u32 {
    crc32c_fast(data)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_png_chunk_crc() {
        let chunk_type = b"IHDR";
        let chunk_data = b"test data";
        let crc = compute_png_chunk_crc(chunk_type, chunk_data);
        assert_ne!(crc, 0);
    }

    #[test]
    fn test_verify_png_chunk_valid() {
        let chunk_type = b"IHDR";
        let chunk_data = b"test data";
        let crc = compute_png_chunk_crc(chunk_type, chunk_data);
        assert!(verify_png_chunk(chunk_data, crc));
    }

    #[test]
    fn test_verify_png_chunk_invalid() {
        let chunk_data = b"test data";
        let fake_crc = 0x12345678;
        assert!(!verify_png_chunk(chunk_data, fake_crc));
    }

    #[test]
    fn test_verify_png_chunk_with_type() {
        let mut chunk = Vec::new();
        chunk.extend_from_slice(b"IHDR");
        chunk.extend_from_slice(b"test data");
        let crc = compute_png_chunk_crc(b"IHDR", b"test data");
        assert!(verify_png_chunk(&chunk[4..], crc));
    }

    #[test]
    fn test_verify_zip_entry_valid() {
        let file_data = b"Hello, world! This is test data.";
        let crc = compute_zip_crc(file_data);
        assert!(verify_zip_entry(file_data, crc));
    }

    #[test]
    fn test_verify_zip_entry_invalid() {
        let file_data = b"Hello, world!";
        let fake_crc = 0x12345678;
        assert!(!verify_zip_entry(file_data, fake_crc));
    }

    #[test]
    fn test_verify_gzip_valid() {
        let data = b"This is uncompressed data";
        let crc = compute_gzip_crc(data);
        assert!(verify_gzip(data, crc));
    }

    #[test]
    fn test_verify_gzip_invalid() {
        let data = b"Test data";
        let fake_crc = 0x12345678;
        assert!(!verify_gzip(data, fake_crc));
    }

    #[test]
    fn test_verify_ethernet_frame_valid() {
        let frame_data = b"This is the ethernet payload";
        let crc = compute_ethernet_crc(frame_data);
        assert!(verify_ethernet_frame(frame_data, crc));
    }

    #[test]
    fn test_verify_ethernet_frame_invalid() {
        let frame_data = b"Ethernet payload";
        let fake_crc = 0x12345678;
        assert!(!verify_ethernet_frame(frame_data, fake_crc));
    }

    #[test]
    fn test_compute_scsi_crc() {
        let data = b"SCSI protection information";
        let crc = compute_scsi_crc(data);
        assert_ne!(crc, 0);
    }

    #[test]
    fn test_verify_scsi_data_valid() {
        let data = b"SCSI data block";
        let crc = compute_scsi_crc(data);
        assert!(verify_scsi_data(data, crc));
    }

    #[test]
    fn test_verify_scsi_data_invalid() {
        let data = b"SCSI data";
        let fake_crc = 0x12345678;
        assert!(!verify_scsi_data(data, fake_crc));
    }
}
