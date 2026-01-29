//! Container Format Detection
//!
//! Supports detecting video file formats from magic bytes and file extensions:
//! - MP4 (ISO Base Media File Format)
//! - MKV (Matroska)
//! - WebM (Matroska variant)
//! - AVI (Audio Video Interleave)
//! - IVF (VP9/AV1 raw)
//! - Annex B (H.264/H.265 raw byte stream)

use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;

// ============================================================================
// Constants
// ============================================================================

/// Annex B NAL type range for common H.264/H.265 NAL units
///
/// Includes: slice (1), IDR (5), SPS (7), PPS (8), AUD (9)
/// Reference: ITU-T H.264 (02/2014) Table 7-1
pub const ANNEX_B_NAL_TYPE_MIN: u8 = 1;

/// Maximum NAL type in the common range
///
/// Excludes reserved and higher-level NAL types
pub const ANNEX_B_NAL_TYPE_MAX: u8 = 9;

/// Special NAL type for extended NAL units
///
/// Used for NAL units with payload greater than 1 byte
/// Reference: ITU-T H.264 (02/2014) Section 7.3.1
pub const ANNEX_B_NAL_TYPE_EXTENDED: u8 = 20;

/// Type-safe wrapper for magic byte patterns using const generics
///
/// Provides compile-time size guarantees for magic byte sequences,
/// preventing buffer overruns and type errors in format detection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MagicBytes<const N: usize>(pub [u8; N]);

impl<const N: usize> MagicBytes<N> {
    /// Create a new MagicBytes wrapper
    ///
    /// # Example
    /// ```
    /// use bitvue_formats::container::MagicBytes;
    /// const DKIF: MagicBytes<4> = MagicBytes(*b"DKIF");
    /// const FTYP: MagicBytes<4> = MagicBytes(*b"ftyp");
    /// ```
    pub const fn new(bytes: [u8; N]) -> Self {
        Self(bytes)
    }

    /// Check if the provided buffer matches this magic byte pattern
    ///
    /// Returns `false` if the buffer is smaller than N bytes
    #[inline]
    pub fn matches(&self, buffer: &[u8]) -> bool {
        buffer.len() >= N && &buffer[..N] == &self.0
    }

    /// Get the byte slice
    #[inline]
    pub const fn as_bytes(&self) -> &[u8; N] {
        &self.0
    }

    /// Check if this magic bytes matches at a specific offset in buffer
    #[inline]
    pub fn matches_at(&self, buffer: &[u8], offset: usize) -> bool {
        buffer.len() >= offset + N && &buffer[offset..offset + N] == &self.0
    }
}

// Common magic byte patterns for video formats
impl MagicBytes<4> {
    /// MP4/ISO BMFF ftyp box marker
    pub const FTYP: Self = Self(*b"ftyp");

    /// Matroska EBML header (first 4 bytes)
    pub const EBML: Self = Self([0x1A, 0x45, 0xDF, 0xA3]);

    /// AVI RIFF header
    pub const RIFF: Self = Self(*b"RIFF");

    /// AVI list type
    pub const AVI: Self = Self(*b"AVI ");

    /// IVF header marker
    pub const DKIF: Self = Self(*b"DKIF");
}

// Magic byte patterns with start codes
impl MagicBytes<3> {
    /// Annex B 3-byte start code
    pub const START_CODE_3: Self = Self([0x00, 0x00, 0x01]);
}

impl MagicBytes<4> {
    /// Annex B 4-byte start code
    pub const START_CODE_4: Self = Self([0x00, 0x00, 0x00, 0x01]);
}

/// Container format types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContainerFormat {
    /// MP4 container (ISO BMFF)
    MP4,
    /// Matroska (MKV/WebM)
    Matroska,
    /// AVI container
    AVI,
    /// IVF raw container (VP9/AV1)
    IVF,
    /// Annex B byte stream (H.264/H.265)
    AnnexB,
    /// Unknown format
    Unknown,
}

impl ContainerFormat {
    /// Get codec hint from container format
    pub fn get_likely_codec(&self) -> Option<&'static str> {
        match self {
            ContainerFormat::MP4 => Some("h264"), // Most common, could also be hevc, av1
            ContainerFormat::Matroska => Some("vp9"), // WebM typically VP9, could also be av1, vp8
            ContainerFormat::IVF => Some("av1"),  // IVF is VP9 or AV1
            ContainerFormat::AnnexB => Some("h264"), // Annex B is H.264 or H.265
            ContainerFormat::AVI => Some("h264"), // AVI typically H.264
            ContainerFormat::Unknown => None,
        }
    }
}

/// Detect container format from file extension
pub fn detect_from_extension(path: &Path) -> ContainerFormat {
    match path
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|s| s.to_lowercase())
        .as_deref()
    {
        Some("mp4") | Some("m4v") | Some("m4a") | Some("mov") => ContainerFormat::MP4,
        Some("mkv") => ContainerFormat::Matroska,
        Some("webm") => ContainerFormat::Matroska,
        Some("avi") => ContainerFormat::AVI,
        Some("ivf") => ContainerFormat::IVF,
        Some("h264") | Some("h265") | Some("hevc") | Some("265") => ContainerFormat::AnnexB,
        _ => ContainerFormat::Unknown,
    }
}

/// Detect container format from magic bytes
///
/// Uses buffered I/O (BufReader) for improved performance on large files.
pub fn detect_from_magic_bytes(path: &Path) -> Result<ContainerFormat, std::io::Error> {
    let file = File::open(path)?;
    let mut reader = BufReader::new(file);
    let mut buffer = [0u8; 32];

    // Read first 32 bytes for magic detection (buffered for performance)
    let n = reader.read(&mut buffer)?;
    if n < 8 {
        return Ok(ContainerFormat::Unknown);
    }

    // Use type-safe magic byte patterns
    let buffer = &buffer[..n];

    // MP4/ISO BMFF: ftyp box at offset 4
    // Format: 4 bytes size, 4 bytes "ftyp", then brand
    if MagicBytes::FTYP.matches_at(buffer, 4) {
        return Ok(ContainerFormat::MP4);
    }

    // Matroska/WebM: EBML header at start
    if MagicBytes::EBML.matches(buffer) {
        return Ok(ContainerFormat::Matroska);
    }

    // AVI: RIFF header at start, AVI list type at offset 8
    if MagicBytes::RIFF.matches(buffer) && MagicBytes::AVI.matches_at(buffer, 8) {
        return Ok(ContainerFormat::AVI);
    }

    // IVF: DKIF marker at start
    if MagicBytes::DKIF.matches(buffer) {
        return Ok(ContainerFormat::IVF);
    }

    // Annex B H.264/H.265: Start code detection
    if buffer.len() >= 5 {
        // Check for 4-byte start code
        if MagicBytes::START_CODE_4.matches(buffer) {
            // Verify NAL unit type (5th byte)
            let nal_type = buffer[4] & 0x1F;
            // Common H.264 NAL types: 1 (slice), 5 (IDR), 7 (SPS), 8 (PPS)
            // H.265 NAL types are different but similar
            if (ANNEX_B_NAL_TYPE_MIN..=ANNEX_B_NAL_TYPE_MAX).contains(&nal_type)
                || nal_type == ANNEX_B_NAL_TYPE_EXTENDED
            {
                return Ok(ContainerFormat::AnnexB);
            }
        }
        // Check for 3-byte start code
        if MagicBytes::START_CODE_3.matches(buffer) {
            let nal_type = buffer[3] & 0x1F;
            if (ANNEX_B_NAL_TYPE_MIN..=ANNEX_B_NAL_TYPE_MAX).contains(&nal_type)
                || nal_type == ANNEX_B_NAL_TYPE_EXTENDED
            {
                return Ok(ContainerFormat::AnnexB);
            }
        }
    }

    Ok(ContainerFormat::Unknown)
}

/// Detect container format using both extension and magic bytes
pub fn detect_container_format(path: &Path) -> Result<ContainerFormat, std::io::Error> {
    // First try magic bytes (more reliable)
    let format = detect_from_magic_bytes(path)?;

    // If magic bytes detection failed, fall back to extension
    if format == ContainerFormat::Unknown {
        Ok(detect_from_extension(path))
    } else {
        Ok(format)
    }
}

/// Check if file format is supported
pub fn is_supported_format(path: &Path) -> Result<bool, std::io::Error> {
    Ok(matches!(
        detect_container_format(path)?,
        ContainerFormat::MP4
            | ContainerFormat::Matroska
            | ContainerFormat::IVF
            | ContainerFormat::AnnexB
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_magic_bytes_creation() {
        // Test creating magic bytes from byte strings
        const DKIF: MagicBytes<4> = MagicBytes(*b"DKIF");
        const FTYP: MagicBytes<4> = MagicBytes(*b"ftyp");

        assert_eq!(DKIF.as_bytes(), b"DKIF");
        assert_eq!(FTYP.as_bytes(), b"ftyp");
    }

    #[test]
    fn test_magic_bytes_matches() {
        const MAGIC: MagicBytes<4> = MagicBytes(*b"TEST");

        // Exact match
        assert!(MAGIC.matches(b"TESTDATA"));
        assert!(MAGIC.matches(b"TEST"));

        // Not enough bytes
        assert!(!MAGIC.matches(b"TES"));

        // Wrong bytes
        assert!(!MAGIC.matches(b"TOAST"));
    }

    #[test]
    fn test_magic_bytes_matches_at() {
        const MAGIC: MagicBytes<4> = MagicBytes(*b"HEAD");

        // Match at offset 0
        assert!(MAGIC.matches_at(b"HEADER", 0));

        // Match at offset 4
        assert!(MAGIC.matches_at(b"xxxxHEAD", 4));

        // Not enough bytes at offset
        assert!(!MAGIC.matches_at(b"HEAD", 1));

        // Wrong bytes at offset
        assert!(!MAGIC.matches_at(b"xxxxTOLD", 4));
    }

    #[test]
    fn test_magic_bytes_equality() {
        const MAGIC1: MagicBytes<4> = MagicBytes(*b"TEST");
        const MAGIC2: MagicBytes<4> = MagicBytes(*b"TEST");
        const MAGIC3: MagicBytes<4> = MagicBytes(*b"OTHR"); // 4 bytes

        assert_eq!(MAGIC1, MAGIC2);
        assert_ne!(MAGIC1, MAGIC3);
    }

    #[test]
    fn test_magic_bytes_predefined_constants() {
        // Test that all predefined constants work
        assert!(MagicBytes::FTYP.matches(b"ftypxxxx"));
        assert!(MagicBytes::DKIF.matches(b"DKIFxxxx"));
        assert!(MagicBytes::RIFF.matches(b"RIFFxxxx"));
        assert!(MagicBytes::AVI.matches(b"AVI xxxx"));

        // EBML header is binary
        let ebml_header = [0x1A, 0x45, 0xDF, 0xA3, 0x00];
        assert!(MagicBytes::EBML.matches(&ebml_header));
    }

    #[test]
    fn test_magic_bytes_start_codes() {
        // 3-byte start code
        assert!(MagicBytes::START_CODE_3.matches(b"\x00\x00\x01\xFF"));
        assert!(!MagicBytes::START_CODE_3.matches(b"\x00\x00\x02\xFF"));

        // 4-byte start code
        assert!(MagicBytes::START_CODE_4.matches(b"\x00\x00\x00\x01\xFF"));
        assert!(!MagicBytes::START_CODE_4.matches(b"\x00\x00\x00\x02\xFF"));
    }

    #[test]
    fn test_magic_bytes_different_sizes() {
        const MAGIC_3: MagicBytes<3> = MagicBytes(*b"ABC");
        const MAGIC_4: MagicBytes<4> = MagicBytes(*b"ABCD");
        const MAGIC_8: MagicBytes<8> = MagicBytes(*b"ABCDEFGH");

        assert!(MAGIC_3.matches(b"ABC"));
        assert!(!MAGIC_3.matches(b"AB")); // Too short

        assert!(MAGIC_4.matches(b"ABCD"));
        assert!(!MAGIC_4.matches(b"ABC")); // Too short

        assert!(MAGIC_8.matches(b"ABCDEFGH"));
        assert!(!MAGIC_8.matches(b"ABCDEFG")); // Too short
    }

    #[test]
    fn test_detect_from_extension() {
        assert_eq!(
            detect_from_extension(Path::new("test.mp4")),
            ContainerFormat::MP4
        );
        assert_eq!(
            detect_from_extension(Path::new("test.mkv")),
            ContainerFormat::Matroska
        );
        assert_eq!(
            detect_from_extension(Path::new("test.webm")),
            ContainerFormat::Matroska
        );
        assert_eq!(
            detect_from_extension(Path::new("test.avi")),
            ContainerFormat::AVI
        );
        assert_eq!(
            detect_from_extension(Path::new("test.ivf")),
            ContainerFormat::IVF
        );
        assert_eq!(
            detect_from_extension(Path::new("test.h264")),
            ContainerFormat::AnnexB
        );
        assert_eq!(
            detect_from_extension(Path::new("test.xyz")),
            ContainerFormat::Unknown
        );
    }

    #[test]
    fn test_container_format_get_likely_codec() {
        assert_eq!(ContainerFormat::MP4.get_likely_codec(), Some("h264"));
        assert_eq!(ContainerFormat::Matroska.get_likely_codec(), Some("vp9"));
        assert_eq!(ContainerFormat::IVF.get_likely_codec(), Some("av1"));
        assert_eq!(ContainerFormat::AnnexB.get_likely_codec(), Some("h264"));
        assert_eq!(ContainerFormat::Unknown.get_likely_codec(), None);
    }
}
