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
use std::io::Read;
use std::path::Path;

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
    match path.extension()
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
pub fn detect_from_magic_bytes(path: &Path) -> Result<ContainerFormat, std::io::Error> {
    let mut file = File::open(path)?;
    let mut buffer = [0u8; 32];

    // Read first 32 bytes for magic detection
    let n = file.read(&mut buffer)?;
    if n < 8 {
        return Ok(ContainerFormat::Unknown);
    }

    // Check for known magic bytes/signatures

    // MP4/ISO BMFF: ftyp box at start
    // Format: 4 bytes size, 4 bytes "ftyp", then brand
    if n >= 12 && &buffer[4..8] == b"ftyp" {
        return Ok(ContainerFormat::MP4);
    }

    // Matroska/WebM: EBML header
    // Starts with 0x1A45DFA3 (EBML ID)
    if n >= 4 && buffer[0] == 0x1A && buffer[1] == 0x45 && buffer[2] == 0xDF && buffer[3] == 0xA3 {
        return Ok(ContainerFormat::Matroska);
    }

    // AVI: RIFF...AVI
    if n >= 12 && &buffer[0..4] == b"RIFF" && &buffer[8..12] == b"AVI " {
        return Ok(ContainerFormat::AVI);
    }

    // IVF: DKIF (VP9/AV1)
    if n >= 4 && &buffer[0..4] == b"DKIF" {
        return Ok(ContainerFormat::IVF);
    }

    // Annex B H.264/H.265: Start code 0x00 0x00 0x00 0x01 or 0x00 0x00 0x01
    if n >= 5 {
        // Check for 4-byte start code
        if buffer[0] == 0x00 && buffer[1] == 0x00 && buffer[2] == 0x00 && buffer[3] == 0x01 {
            // Verify NAL unit type (5th byte)
            let nal_type = buffer[4] & 0x1F;
            // Common H.264 NAL types: 1 (slice), 5 (IDR), 7 (SPS), 8 (PPS)
            // H.265 NAL types are different but similar
            if (1..=9).contains(&nal_type) || nal_type == 20 {
                return Ok(ContainerFormat::AnnexB);
            }
        }
        // Check for 3-byte start code
        if buffer[0] == 0x00 && buffer[1] == 0x00 && buffer[2] == 0x01 {
            let nal_type = buffer[3] & 0x1F;
            if (1..=9).contains(&nal_type) || nal_type == 20 {
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
        ContainerFormat::MP4 | ContainerFormat::Matroska | ContainerFormat::IVF | ContainerFormat::AnnexB
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_from_extension() {
        assert_eq!(detect_from_extension(Path::new("test.mp4")), ContainerFormat::MP4);
        assert_eq!(detect_from_extension(Path::new("test.mkv")), ContainerFormat::Matroska);
        assert_eq!(detect_from_extension(Path::new("test.webm")), ContainerFormat::Matroska);
        assert_eq!(detect_from_extension(Path::new("test.avi")), ContainerFormat::AVI);
        assert_eq!(detect_from_extension(Path::new("test.ivf")), ContainerFormat::IVF);
        assert_eq!(detect_from_extension(Path::new("test.h264")), ContainerFormat::AnnexB);
        assert_eq!(detect_from_extension(Path::new("test.xyz")), ContainerFormat::Unknown);
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
