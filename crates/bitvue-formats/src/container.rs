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

/// Annex B NAL type range for H.264 NAL units
///
/// H.264 NAL types: slice (1), IDR (5), SPS (7), PPS (8), AUD (9)
/// Reference: ITU-T H.264 (02/2014) Table 7-1
pub const ANNEX_B_NAL_TYPE_MIN: u8 = 1;

/// Maximum H.264 NAL type in the common range
pub const ANNEX_B_NAL_TYPE_MAX: u8 = 9;

/// H.265 NAL type range for VPS, SPS, PPS
///
/// H.265 NAL types: VPS (32), SPS (33), PPS (34)
/// Reference: ITU-T H.265 (08/2021) Table 7-1
pub const HEVC_NAL_TYPE_VPS: u8 = 32;
pub const HEVC_NAL_TYPE_SPS: u8 = 33;
pub const HEVC_NAL_TYPE_PPS: u8 = 34;

/// Check if NAL type is in HEVC VPS/SPS/PPS range
#[inline]
pub fn is_hevc_parameter_set_nal_type(nal_type: u8) -> bool {
    (HEVC_NAL_TYPE_VPS..=HEVC_NAL_TYPE_PPS).contains(&nal_type)
}

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
        buffer.len() >= N && buffer[..N] == self.0
    }

    /// Get the byte slice
    #[inline]
    pub const fn as_bytes(&self) -> &[u8; N] {
        &self.0
    }

    /// Check if this magic bytes matches at a specific offset in buffer
    #[inline]
    pub fn matches_at(&self, buffer: &[u8], offset: usize) -> bool {
        // checked_add: a huge offset must not overflow (and wrap to a small, matching range)
        offset
            .checked_add(N)
            .and_then(|end| buffer.get(offset..end))
            .is_some_and(|window| window == self.0)
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
            // H.264 NAL type: bits 0-4 (5 bits)
            let h264_nal_type = buffer[4] & 0x1F;
            // H.265 NAL type: bits 1-6 (6 bits)
            let hevc_nal_type = (buffer[4] >> 1) & 0x3F;

            // Common H.264 NAL types: 1 (slice), 5 (IDR), 7 (SPS), 8 (PPS)
            // H.265 NAL types: 32 (VPS), 33 (SPS), 34 (PPS)
            if (ANNEX_B_NAL_TYPE_MIN..=ANNEX_B_NAL_TYPE_MAX).contains(&h264_nal_type)
                || h264_nal_type == ANNEX_B_NAL_TYPE_EXTENDED
                || is_hevc_parameter_set_nal_type(hevc_nal_type)
            {
                return Ok(ContainerFormat::AnnexB);
            }
        }
        // Check for 3-byte start code
        if MagicBytes::START_CODE_3.matches(buffer) {
            let h264_nal_type = buffer[3] & 0x1F;
            let hevc_nal_type = (buffer[3] >> 1) & 0x3F;

            if (ANNEX_B_NAL_TYPE_MIN..=ANNEX_B_NAL_TYPE_MAX).contains(&h264_nal_type)
                || h264_nal_type == ANNEX_B_NAL_TYPE_EXTENDED
                || is_hevc_parameter_set_nal_type(hevc_nal_type)
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

    // ==========================================================================
    // Comprehensive tests for public API: detect_container_format()
    // ==========================================================================

    #[test]
    fn test_detect_container_format_mp4() {
        // Test with real MP4 magic bytes
        let mut mp4_data = vec![0u8; 32];
        mp4_data[4..8].copy_from_slice(b"ftyp");
        mp4_data[8..12].copy_from_slice(b"avc1");

        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join("test_detect.mp4");
        std::fs::write(&test_file, &mp4_data).unwrap();

        let result = detect_container_format(&test_file);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), ContainerFormat::MP4);

        std::fs::remove_file(&test_file).ok();
    }

    #[test]
    fn test_detect_container_format_mkv() {
        // Test with MKV magic bytes
        let mut mkv_data = vec![0u8; 32];
        mkv_data[0..4].copy_from_slice(b"\x1a\x45\xdf\xa3");

        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join("test_detect.mkv");
        std::fs::write(&test_file, &mkv_data).unwrap();

        let result = detect_container_format(&test_file);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), ContainerFormat::Matroska);

        std::fs::remove_file(&test_file).ok();
    }

    #[test]
    fn test_detect_container_format_ivf() {
        // Test with IVF magic bytes
        let mut ivf_data = vec![0u8; 32];
        ivf_data[0..4].copy_from_slice(b"DKIF");
        ivf_data[4..8].copy_from_slice(&0u32.to_le_bytes());

        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join("test_detect.ivf");
        std::fs::write(&test_file, &ivf_data).unwrap();

        let result = detect_container_format(&test_file);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), ContainerFormat::IVF);

        std::fs::remove_file(&test_file).ok();
    }

    #[test]
    fn test_detect_container_format_annex_b() {
        // Test with Annex B magic bytes (NAL start code)
        let mut annex_b_data = vec![0u8; 32];
        annex_b_data[0..4].copy_from_slice(b"\x00\x00\x00\x01");

        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join("test_detect.h264");
        std::fs::write(&test_file, &annex_b_data).unwrap();

        let result = detect_container_format(&test_file);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), ContainerFormat::AnnexB);

        std::fs::remove_file(&test_file).ok();
    }

    #[test]
    fn test_detect_container_format_extension_fallback() {
        // Test extension fallback when magic bytes don't match
        let test_data = vec![0xFFu8; 32]; // No recognizable magic bytes

        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join("test_detect.webm");
        std::fs::write(&test_file, &test_data).unwrap();

        let result = detect_container_format(&test_file);
        assert!(result.is_ok());
        // Should fallback to extension detection (Matroska for .webm)
        assert_eq!(result.unwrap(), ContainerFormat::Matroska);

        std::fs::remove_file(&test_file).ok();
    }

    #[test]
    fn test_detect_container_format_unknown() {
        // Test with unknown format
        let test_data = vec![0xFFu8; 32];

        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join("test_detect.xyz");
        std::fs::write(&test_file, &test_data).unwrap();

        let result = detect_container_format(&test_file);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), ContainerFormat::Unknown);

        std::fs::remove_file(&test_file).ok();
    }

    #[test]
    fn test_detect_container_format_file_not_found() {
        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join("nonexistent_file.mp4");

        let result = detect_container_format(&test_file);
        assert!(result.is_err());
        assert!(result.unwrap_err().kind() == std::io::ErrorKind::NotFound);
    }

    #[test]
    fn test_detect_container_format_empty_file() {
        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join("empty.mp4");
        std::fs::write(&test_file, []).unwrap();

        let result = detect_container_format(&test_file);
        assert!(result.is_ok());
        // Empty file falls back to extension detection (.mp4 -> MP4)
        assert_eq!(result.unwrap(), ContainerFormat::MP4);

        std::fs::remove_file(&test_file).ok();
    }

    #[test]
    fn test_detect_container_format_partial_header() {
        // Test with partial header (smaller than magic bytes size)
        let test_data = vec![0x66u8; 2]; // "ft" but incomplete

        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join("partial.mp4");
        std::fs::write(&test_file, &test_data).unwrap();

        let result = detect_container_format(&test_file);
        assert!(result.is_ok());
        // Should fall back to extension detection
        assert_eq!(result.unwrap(), ContainerFormat::MP4);

        std::fs::remove_file(&test_file).ok();
    }

    // ==========================================================================
    // Tests for public API: is_supported_format()
    // ==========================================================================

    #[test]
    fn test_is_supported_format_mp4() {
        let mut mp4_data = vec![0u8; 32];
        mp4_data[4..8].copy_from_slice(b"ftyp");

        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join("test_supported.mp4");
        std::fs::write(&test_file, &mp4_data).unwrap();

        let result = is_supported_format(&test_file);
        assert!(result.is_ok());
        assert!(result.unwrap());

        std::fs::remove_file(&test_file).ok();
    }

    #[test]
    fn test_is_supported_format_mkv() {
        let mut mkv_data = vec![0u8; 32];
        mkv_data[0..4].copy_from_slice(b"\x1a\x45\xdf\xa3");

        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join("test_supported.mkv");
        std::fs::write(&test_file, &mkv_data).unwrap();

        let result = is_supported_format(&test_file);
        assert!(result.is_ok());
        assert!(result.unwrap());

        std::fs::remove_file(&test_file).ok();
    }

    #[test]
    fn test_is_supported_format_ivf() {
        let mut ivf_data = vec![0u8; 32];
        ivf_data[0..4].copy_from_slice(b"DKIF");
        ivf_data[4..8].copy_from_slice(&0u32.to_le_bytes());

        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join("test_supported.ivf");
        std::fs::write(&test_file, &ivf_data).unwrap();

        let result = is_supported_format(&test_file);
        assert!(result.is_ok());
        assert!(result.unwrap());

        std::fs::remove_file(&test_file).ok();
    }

    #[test]
    fn test_is_supported_format_annex_b() {
        let mut annex_b_data = vec![0u8; 32];
        annex_b_data[0..4].copy_from_slice(b"\x00\x00\x00\x01");

        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join("test_supported.h264");
        std::fs::write(&test_file, &annex_b_data).unwrap();

        let result = is_supported_format(&test_file);
        assert!(result.is_ok());
        assert!(result.unwrap());

        std::fs::remove_file(&test_file).ok();
    }

    #[test]
    fn test_is_supported_format_unsupported() {
        // Test with unsupported format (AVI)
        let mut test_data = vec![0u8; 32];
        test_data[0..4].copy_from_slice(b"RIFF");
        test_data[8..12].copy_from_slice(b"AVI ");

        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join("unsupported.avi");
        std::fs::write(&test_file, &test_data).unwrap();

        let result = is_supported_format(&test_file);
        assert!(result.is_ok());
        assert!(!result.unwrap());

        std::fs::remove_file(&test_file).ok();
    }

    #[test]
    fn test_is_supported_format_unknown() {
        let test_data = vec![0xFFu8; 32];

        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join("unknown.xyz");
        std::fs::write(&test_file, &test_data).unwrap();

        let result = is_supported_format(&test_file);
        assert!(result.is_ok());
        assert!(!result.unwrap());

        std::fs::remove_file(&test_file).ok();
    }
}

/// Detection tests on real files in a temp dir: every magic-byte rule, its boundaries, and the extension fallback.
#[cfg(test)]
mod detection_tests {
    use super::*;
    use std::path::PathBuf;

    fn write(name: &str, bytes: &[u8]) -> (tempfile::TempDir, PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join(name);
        std::fs::write(&p, bytes).unwrap();
        (dir, p)
    }

    fn magic(bytes: &[u8]) -> ContainerFormat {
        let (_d, p) = write("clip.bin", bytes);
        detect_from_magic_bytes(&p).unwrap()
    }

    fn padded(head: &[u8], total: usize) -> Vec<u8> {
        let mut v = head.to_vec();
        v.resize(total, 0xEE);
        v
    }

    #[test]
    fn container_signatures() {
        assert_eq!(
            magic(&padded(&[0, 0, 0, 0x18, b'f', b't', b'y', b'p'], 32)),
            ContainerFormat::MP4
        );
        assert_eq!(
            magic(&padded(&[0x1A, 0x45, 0xDF, 0xA3], 32)),
            ContainerFormat::Matroska
        );
        assert_eq!(magic(&padded(b"DKIF", 32)), ContainerFormat::IVF);
        let mut avi = padded(b"RIFF", 32);
        avi[8..12].copy_from_slice(b"AVI ");
        assert_eq!(magic(&avi), ContainerFormat::AVI);
        // RIFF but not AVI (WAVE)
        let mut wav = padded(b"RIFF", 32);
        wav[8..12].copy_from_slice(b"WAVE");
        assert_eq!(magic(&wav), ContainerFormat::Unknown);
        // 'ftyp' must be at offset 4, not 0 or 5
        assert_eq!(magic(&padded(b"ftyp", 32)), ContainerFormat::Unknown);
        let mut late = padded(&[0; 5], 32);
        late[5..9].copy_from_slice(b"ftyp");
        assert_eq!(magic(&late), ContainerFormat::Unknown);
    }

    #[test]
    fn files_shorter_than_eight_bytes_are_unknown() {
        assert_eq!(magic(b""), ContainerFormat::Unknown);
        assert_eq!(magic(b"DKIF123"), ContainerFormat::Unknown); // 7 bytes
        assert_eq!(magic(b"DKIF1234"), ContainerFormat::IVF); // 8 bytes
        assert_eq!(magic(&[0, 0, 0, 1, 0x67, 0, 0]), ContainerFormat::Unknown);
    }

    #[test]
    fn annex_b_four_byte_start_code_accepts_h264_and_hevc_types() {
        let nal = |b: u8| magic(&padded(&[0, 0, 0, 1, b], 12));
        // H.264: nal_unit_type = low 5 bits; 1..=9 and 20
        for t in [1u8, 5, 7, 8, 9, 20] {
            assert_eq!(nal(t), ContainerFormat::AnnexB, "h264 type {t}");
            assert_eq!(
                nal(0x60 | t),
                ContainerFormat::AnnexB,
                "h264 type {t} with nal_ref_idc"
            );
        }
        // H.265: nal_unit_type = bits 1..=6; 32..=34 (VPS/SPS/PPS)
        for t in [32u8, 33, 34] {
            assert_eq!(nal(t << 1), ContainerFormat::AnnexB, "hevc type {t}");
        }
        // neighbours of both ranges: 0, 10, 19, 21 (H.264) and 31, 35 (H.265, chosen so the low 5 bits fall outside H.264's set)
        assert_eq!(nal(0x00), ContainerFormat::Unknown);
        assert_eq!(nal(0x0A), ContainerFormat::Unknown);
        assert_eq!(nal(0x13), ContainerFormat::Unknown);
        assert_eq!(nal(0x15), ContainerFormat::Unknown);
        assert_eq!(nal(31 << 1), ContainerFormat::Unknown); // 0x3E: h264 type 30
                                                            // HEVC type 35 (AUD) is 0x46: its low 5 bits read as H.264 type 6, so it is still taken as Annex B
        assert_eq!(nal(35 << 1), ContainerFormat::AnnexB);
    }

    #[test]
    fn annex_b_three_byte_start_code_accepts_the_same_types() {
        let nal = |b: u8| magic(&padded(&[0, 0, 1, b], 12));
        for t in [1u8, 5, 7, 8, 9, 20] {
            assert_eq!(nal(t), ContainerFormat::AnnexB, "h264 type {t}");
            assert_eq!(
                nal(0x60 | t),
                ContainerFormat::AnnexB,
                "h264 type {t} with nal_ref_idc"
            );
        }
        for t in [32u8, 33, 34] {
            assert_eq!(nal(t << 1), ContainerFormat::AnnexB, "hevc type {t}");
        }
        assert_eq!(nal(0x00), ContainerFormat::Unknown);
        assert_eq!(nal(0x0A), ContainerFormat::Unknown);
        assert_eq!(nal(0x15), ContainerFormat::Unknown);
        assert_eq!(nal(31 << 1), ContainerFormat::Unknown);
    }

    #[test]
    fn annex_b_start_code_must_be_at_the_start() {
        assert_eq!(
            magic(&padded(&[0xFF, 0, 0, 1, 0x67], 12)),
            ContainerFormat::Unknown
        );
        assert_eq!(
            magic(&padded(&[0, 0, 2, 0x67], 12)),
            ContainerFormat::Unknown
        );
        // 00 00 00 00 01 is not a 4-byte start code at offset 0
        assert_eq!(
            magic(&padded(&[0, 0, 0, 0, 1, 0x67], 12)),
            ContainerFormat::Unknown
        );
    }

    #[test]
    fn four_byte_code_with_a_bad_type_is_not_reread_as_a_three_byte_code() {
        // bytes 0..3 are 00 00 00, so the 3-byte code cannot match; the result stays Unknown
        assert_eq!(
            magic(&padded(&[0, 0, 0, 1, 0x00], 12)),
            ContainerFormat::Unknown
        );
    }

    #[test]
    fn only_the_first_32_bytes_matter() {
        let mut d = padded(b"junk", 100);
        d[40..44].copy_from_slice(b"DKIF");
        assert_eq!(magic(&d), ContainerFormat::Unknown);
    }

    #[test]
    fn magic_wins_over_extension_and_extension_is_the_fallback() {
        let (_d, p) = write("clip.mp4", &padded(b"DKIF", 32));
        assert_eq!(detect_container_format(&p).unwrap(), ContainerFormat::IVF);
        let (_d, p) = write("clip.mkv", &padded(&[7; 4], 32));
        assert_eq!(
            detect_container_format(&p).unwrap(),
            ContainerFormat::Matroska
        );
        let (_d, p) = write("clip.xyz", &padded(&[7; 4], 32));
        assert_eq!(
            detect_container_format(&p).unwrap(),
            ContainerFormat::Unknown
        );
        let (_d, p) = write("noext", &padded(&[7; 4], 32));
        assert_eq!(
            detect_container_format(&p).unwrap(),
            ContainerFormat::Unknown
        );
        // too short for magic detection: the extension decides
        let (_d, p) = write("tiny.ivf", b"x");
        assert_eq!(detect_container_format(&p).unwrap(), ContainerFormat::IVF);
    }

    #[test]
    fn extension_table() {
        for (name, want) in [
            ("a.mp4", ContainerFormat::MP4),
            ("a.m4v", ContainerFormat::MP4),
            ("a.m4a", ContainerFormat::MP4),
            ("a.mov", ContainerFormat::MP4),
            ("a.MP4", ContainerFormat::MP4),
            ("a.MoV", ContainerFormat::MP4),
            ("a.mkv", ContainerFormat::Matroska),
            ("a.webm", ContainerFormat::Matroska),
            ("a.WEBM", ContainerFormat::Matroska),
            ("a.avi", ContainerFormat::AVI),
            ("a.ivf", ContainerFormat::IVF),
            ("a.h264", ContainerFormat::AnnexB),
            ("a.h265", ContainerFormat::AnnexB),
            ("a.hevc", ContainerFormat::AnnexB),
            ("a.265", ContainerFormat::AnnexB),
            ("a.264", ContainerFormat::Unknown),
            ("a.ts", ContainerFormat::Unknown),
            ("a", ContainerFormat::Unknown),
            (".mp4", ContainerFormat::Unknown),
            ("dir.mp4/file", ContainerFormat::Unknown),
            ("a.mp4.bak", ContainerFormat::Unknown),
        ] {
            assert_eq!(detect_from_extension(Path::new(name)), want, "{name}");
        }
    }

    #[test]
    fn support_matrix() {
        for (head, supported) in [
            (padded(&[0, 0, 0, 0x18, b'f', b't', b'y', b'p'], 32), true),
            (padded(&[0x1A, 0x45, 0xDF, 0xA3], 32), true),
            (padded(b"DKIF", 32), true),
            (padded(&[0, 0, 0, 1, 0x67], 32), true),
            (padded(&[0x47; 4], 32), false),
        ] {
            let (_d, p) = write("x.bin", &head);
            assert_eq!(is_supported_format(&p).unwrap(), supported, "{head:02x?}");
        }
        let mut avi = padded(b"RIFF", 32);
        avi[8..12].copy_from_slice(b"AVI ");
        let (_d, p) = write("x.avi", &avi);
        assert_eq!(detect_container_format(&p).unwrap(), ContainerFormat::AVI);
        assert!(!is_supported_format(&p).unwrap()); // detected, but no demuxer
    }

    #[test]
    fn io_errors_are_returned() {
        let dir = tempfile::tempdir().unwrap();
        let missing = dir.path().join("nope.mp4");
        let e = detect_from_magic_bytes(&missing).unwrap_err();
        assert_eq!(e.kind(), std::io::ErrorKind::NotFound);
        assert!(detect_container_format(&missing).is_err());
        assert!(is_supported_format(&missing).is_err());
        // a directory opens but cannot be read
        assert!(detect_from_magic_bytes(dir.path()).is_err());
    }

    #[test]
    fn likely_codec_for_every_format() {
        use ContainerFormat::*;
        for (f, codec) in [
            (MP4, Some("h264")),
            (Matroska, Some("vp9")),
            (IVF, Some("av1")),
            (AnnexB, Some("h264")),
            (AVI, Some("h264")),
            (Unknown, None),
        ] {
            assert_eq!(f.get_likely_codec(), codec, "{f:?}");
        }
    }

    #[test]
    fn hevc_parameter_set_range() {
        for t in 0..=u8::MAX {
            assert_eq!(
                is_hevc_parameter_set_nal_type(t),
                (32..=34).contains(&t),
                "{t}"
            );
        }
    }

    #[test]
    fn magic_bytes_helpers() {
        let m = MagicBytes::new(*b"ab");
        assert_eq!(m.as_bytes(), b"ab");
        assert!(m.matches_at(b"xxab", 2));
        assert!(!m.matches_at(b"xxab", 3));
        assert!(!m.matches_at(b"", 0));
        assert!(MagicBytes::<3>::START_CODE_3.matches(&[0, 0, 1, 9]));
        assert!(MagicBytes::<4>::START_CODE_4.matches(&[0, 0, 0, 1]));
        assert!(!MagicBytes::<4>::START_CODE_4.matches(&[0, 0, 1]));
    }

    #[test]
    fn matches_at_with_a_huge_offset_is_false_not_a_panic() {
        let m = MagicBytes::new(*b"ab");
        // offsets whose offset + N wraps to a small number (the release-build failure mode)
        for offset in [usize::MAX, usize::MAX - 1, usize::MAX - 2] {
            assert!(!m.matches_at(b"abab", offset), "{offset}");
        }
        // the last valid position still matches; one past it does not
        assert!(m.matches_at(b"xxab", 2));
        assert!(!m.matches_at(b"xxab", 3));
    }
}
