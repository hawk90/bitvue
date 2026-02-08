//! Matroska (MKV) container parser
//!
//! Implements minimal MKV parsing to extract AV1 video samples.
//! No external dependencies - pure Rust implementation.
//!
//! References:
//! - Matroska specification: <https://www.matroska.org/technical/elements.html>
//! - EBML specification: <https://github.com/ietf-wg-cellar/ebml-specification>

use bitvue_core::BitvueError;
use std::io::{Cursor, Read, Seek, SeekFrom};

/// EBML Element IDs (in hex)
mod element_id {
    pub const EBML: u32 = 0x1A45DFA3;
    pub const SEGMENT: u32 = 0x18538067;
    #[allow(dead_code)]
    pub const INFO: u32 = 0x1549A966;
    pub const TRACKS: u32 = 0x1654AE6B;
    pub const TRACK_ENTRY: u32 = 0xAE;
    pub const TRACK_NUMBER: u32 = 0xD7;
    pub const TRACK_TYPE: u32 = 0x83;
    pub const CODEC_ID: u32 = 0x86;
    pub const CLUSTER: u32 = 0x1F43B675;
    pub const TIMECODE: u32 = 0xE7;
    pub const SIMPLE_BLOCK: u32 = 0xA3;
    pub const BLOCK_GROUP: u32 = 0xA0;
    pub const BLOCK: u32 = 0xA1;
}

/// Read EBML variable-length integer (VINT)
///
/// Per EBML specification (https://github.com/matroska-org/ebml-specification):
/// - VINT format: [marker] [data bits]
/// - Marker: single '1' bit followed by zero or more '0' bits
/// - Valid marker patterns: 0x80 (1 byte), 0x40 (2 bytes), 0x20 (3 bytes), 0x10 (4 bytes),
///                        0x08 (5 bytes), 0x04 (6 bytes), 0x02 (7 bytes), 0x01 (8 bytes)
/// - Invalid: 0x00 (no marker bit)
/// - Data bits below the marker position can be any value
///
/// Examples:
/// - 0x81 = marker at bit 7, data = 0x01 → 1-byte VINT with value 1
/// - 0x40 = marker at bit 6, data = 0x00 → 2-byte VINT
/// - 0x01 = marker at bit 0, data = 0x00 → 8-byte VINT (valid per EBML spec)
///
/// Security: Most MKV elements use 1-4 byte VINTs; we limit to 4 bytes to prevent DoS
fn read_vint(cursor: &mut Cursor<&[u8]>) -> Result<u64, BitvueError> {
    // Most MKV elements use 1-4 byte VINTs; limit to prevent DoS attacks
    const MAX_VINT_LENGTH: usize = 4;

    let mut first_byte = [0u8; 1];
    cursor
        .read_exact(&mut first_byte)
        .map_err(|_| BitvueError::UnexpectedEof(cursor.position()))?;

    let first = first_byte[0];

    // Find the length marker (first 1 bit from MSB)
    let mut length = 0;
    for i in 0..8 {
        if (first & (0x80 >> i)) != 0 {
            length = i + 1;
            break;
        }
    }

    // Validate VINT length is within EBML specification (1-8 bytes)
    // The only invalid case is 0x00 (no marker bit found)
    // Note: 0x01 is VALID (8-byte VINT with marker at bit position 0)
    if length == 0 {
        return Err(BitvueError::InvalidData(
            "Invalid VINT: no marker bit found (all zeros)".to_string(),
        ));
    }

    // Enforce reasonable maximum to prevent DoS via malicious 8-byte VINTs
    if length > MAX_VINT_LENGTH {
        return Err(BitvueError::InvalidData(format!(
            "VINT length {} exceeds maximum allowed {}",
            length, MAX_VINT_LENGTH
        )));
    }

    // Extract value (remove length marker)
    // For length = 8, the first byte is all marker (0x01), so data bits = 0
    let mut value = if length < 8 {
        (first & (0xFF >> length)) as u64
    } else {
        // length = 8: marker is at bit 0, first byte has no data bits
        0
    };

    // Read remaining bytes
    for _ in 1..length {
        let mut byte = [0u8; 1];
        cursor
            .read_exact(&mut byte)
            .map_err(|_| BitvueError::UnexpectedEof(cursor.position()))?;
        value = (value << 8) | byte[0] as u64;
    }

    Ok(value)
}

/// Read EBML element ID (variable-length)
fn read_element_id(cursor: &mut Cursor<&[u8]>) -> Result<u32, BitvueError> {
    read_vint(cursor).map(|v| v as u32)
}

/// Read EBML element size
fn read_element_size(cursor: &mut Cursor<&[u8]>) -> Result<u64, BitvueError> {
    read_vint(cursor)
}

/// Read a string element
fn read_string(cursor: &mut Cursor<&[u8]>, size: usize) -> Result<String, BitvueError> {
    const MAX_STRING_SIZE: usize = 1_000_000; // 1MB max string to prevent DoS

    if size > MAX_STRING_SIZE {
        return Err(BitvueError::InvalidData(format!(
            "String size {} exceeds maximum allowed {}",
            size, MAX_STRING_SIZE
        )));
    }

    let mut buf = vec![0u8; size];
    cursor
        .read_exact(&mut buf)
        .map_err(|_| BitvueError::UnexpectedEof(cursor.position()))?;
    Ok(String::from_utf8_lossy(&buf)
        .trim_end_matches('\0')
        .to_string())
}

/// Read an unsigned integer element
fn read_uint(cursor: &mut Cursor<&[u8]>, size: usize) -> Result<u64, BitvueError> {
    // SECURITY: Validate size to prevent memory exhaustion
    // Unsigned integers in MKV are typically 1-8 bytes
    const MAX_UINT_SIZE: usize = 16; // Allow some margin for edge cases

    if size > MAX_UINT_SIZE {
        return Err(BitvueError::InvalidData(format!(
            "Uint size {} exceeds maximum allowed {}",
            size, MAX_UINT_SIZE
        )));
    }

    let mut buf = vec![0u8; size];
    cursor
        .read_exact(&mut buf)
        .map_err(|_| BitvueError::UnexpectedEof(cursor.position()))?;

    let mut value = 0u64;
    for byte in buf {
        value = (value << 8) | byte as u64;
    }
    Ok(value)
}

/// MKV container information
#[derive(Debug, Clone, Default)]
pub struct MkvInfo {
    /// Codec ID (e.g. "V_AV1")
    pub codec_id: Option<String>,
    /// Track number for video track
    pub video_track_number: Option<u64>,
    /// Total number of blocks/samples
    pub sample_count: usize,
    /// Sample data
    pub samples: Vec<Vec<u8>>,
    /// Timestamps (in nanoseconds)
    pub timestamps: Vec<u64>,
    /// Key frame indices (frames with keyframe flag set)
    pub key_frames: Vec<u32>,
}

/// Parse MKV file and extract AV1 samples
pub fn extract_av1_samples(data: &[u8]) -> Result<Vec<Vec<u8>>, BitvueError> {
    let info = parse_mkv(data)?;

    // Verify this is an AV1 file
    match &info.codec_id {
        Some(codec_id) if codec_id == "V_AV1" => {
            // AV1 codec confirmed
        }
        Some(codec_id) => {
            return Err(BitvueError::InvalidData(format!(
                "Not an AV1 file: found codec '{}'",
                codec_id
            )));
        }
        None => {
            return Err(BitvueError::InvalidData(
                "No codec information found in MKV".to_string(),
            ));
        }
    }

    Ok(info.samples)
}

/// Parse MKV file and extract H.264/AVC samples
///
/// Extracts NAL units from MKV container for H.264/AVC video streams.
/// Supports codec ID "V_MPEG4/ISO/AVC" (H.264/AVC in Matroska).
pub fn extract_avc_samples(data: &[u8]) -> Result<Vec<Vec<u8>>, BitvueError> {
    let info = parse_mkv(data)?;

    // Verify this is an H.264/AVC file
    match &info.codec_id {
        Some(codec_id) if codec_id == "V_MPEG4/ISO/AVC" => {
            // H.264/AVC codec confirmed
        }
        Some(codec_id) => {
            return Err(BitvueError::InvalidData(format!(
                "Not an H.264/AVC file: found codec '{}'",
                codec_id
            )));
        }
        None => {
            return Err(BitvueError::InvalidData(
                "No codec information found in MKV".to_string(),
            ));
        }
    }

    Ok(info.samples)
}

/// Parse MKV file and extract H.265/HEVC samples
///
/// Extracts NAL units from MKV container for H.265/HEVC video streams.
/// Supports codec ID "V_MPEGH/ISO/HEVC" (H.265/HEVC in Matroska).
pub fn extract_hevc_samples(data: &[u8]) -> Result<Vec<Vec<u8>>, BitvueError> {
    let info = parse_mkv(data)?;

    // Verify this is an H.265/HEVC file
    match &info.codec_id {
        Some(codec_id) if codec_id == "V_MPEGH/ISO/HEVC" => {
            // H.265/HEVC codec confirmed
        }
        Some(codec_id) => {
            return Err(BitvueError::InvalidData(format!(
                "Not an H.265/HEVC file: found codec '{}'",
                codec_id
            )));
        }
        None => {
            return Err(BitvueError::InvalidData(
                "No codec information found in MKV".to_string(),
            ));
        }
    }

    Ok(info.samples)
}

/// Parse MKV file structure
pub fn parse_mkv(data: &[u8]) -> Result<MkvInfo, BitvueError> {
    if data.len() < 4 {
        return Err(BitvueError::InvalidData(
            "File too small to be MKV".to_string(),
        ));
    }

    let mut cursor = Cursor::new(data);
    let mut info = MkvInfo::default();

    // Pre-allocate capacity for samples - most videos have at least a few hundred frames
    // This reduces reallocations during parsing
    info.samples.reserve(1000);
    info.timestamps.reserve(1000);
    info.key_frames.reserve(100); // Keyframes are typically less frequent

    // Parse EBML header
    let id = read_element_id(&mut cursor)?;
    if id != element_id::EBML {
        return Err(BitvueError::InvalidData(format!(
            "Not a valid EBML file: expected 0x{:X}, got 0x{:X}",
            element_id::EBML,
            id
        )));
    }

    let ebml_size = read_element_size(&mut cursor)?;
    cursor.seek(SeekFrom::Current(ebml_size as i64))?; // Skip EBML header

    // Parse Segment
    let segment_id = read_element_id(&mut cursor)?;
    if segment_id != element_id::SEGMENT {
        return Err(BitvueError::InvalidData(
            "Expected Segment element".to_string(),
        ));
    }

    let segment_size = read_element_size(&mut cursor)?;
    let segment_end = cursor.position() + segment_size;

    // SECURITY: Limit number of elements to prevent DoS via crafted files
    // with many small elements. Maximum 10K elements per level for defense in depth.
    const MAX_ELEMENTS_PER_LEVEL: usize = 10_000;
    let mut element_count = 0;

    // Parse segment children
    while cursor.position() < segment_end && cursor.position() < data.len() as u64 {
        if element_count >= MAX_ELEMENTS_PER_LEVEL {
            return Err(BitvueError::InvalidData(
                "Segment element count exceeded maximum".to_string()
            ));
        }
        element_count += 1;
        let element_id = read_element_id(&mut cursor)?;
        let element_size = read_element_size(&mut cursor)?;
        let element_end = cursor.position() + element_size;

        match element_id {
            element_id::TRACKS => {
                parse_tracks(&mut cursor, element_end, &mut info)?;
            }
            element_id::CLUSTER => {
                parse_cluster(&mut cursor, element_end, &mut info)?;
            }
            _ => {
                // Skip unknown elements
                cursor.seek(SeekFrom::Start(element_end))?;
            }
        }
    }

    Ok(info)
}

/// Parse Tracks element
fn parse_tracks(
    cursor: &mut Cursor<&[u8]>,
    tracks_end: u64,
    info: &mut MkvInfo,
) -> Result<(), BitvueError> {
    const MAX_ELEMENTS_PER_LEVEL: usize = 10_000;
    let mut element_count = 0;

    while cursor.position() < tracks_end {
        if element_count >= MAX_ELEMENTS_PER_LEVEL {
            return Err(BitvueError::InvalidData(
                "Tracks element count exceeded maximum".to_string()
            ));
        }
        element_count += 1;
        let id = read_element_id(cursor)?;
        let size = read_element_size(cursor)?;
        let element_end = cursor.position() + size;

        if id == element_id::TRACK_ENTRY {
            parse_track_entry(cursor, element_end, info)?;
        } else {
            cursor.seek(SeekFrom::Start(element_end))?;
        }
    }

    Ok(())
}

/// Parse TrackEntry element
fn parse_track_entry(
    cursor: &mut Cursor<&[u8]>,
    entry_end: u64,
    info: &mut MkvInfo,
) -> Result<(), BitvueError> {
    let mut track_number = None;
    let mut track_type = None;
    let mut codec_id = None;

    const MAX_ELEMENTS_PER_LEVEL: usize = 10_000;
    let mut element_count = 0;

    while cursor.position() < entry_end {
        if element_count >= MAX_ELEMENTS_PER_LEVEL {
            return Err(BitvueError::InvalidData(
                "TrackEntry element count exceeded maximum".to_string()
            ));
        }
        element_count += 1;
        let id = read_element_id(cursor)?;
        let size = read_element_size(cursor)?;
        let element_end = cursor.position() + size;

        match id {
            element_id::TRACK_NUMBER => {
                track_number = Some(read_uint(cursor, size as usize)?);
            }
            element_id::TRACK_TYPE => {
                track_type = Some(read_uint(cursor, size as usize)?);
            }
            element_id::CODEC_ID => {
                codec_id = Some(read_string(cursor, size as usize)?);
            }
            _ => {
                cursor.seek(SeekFrom::Start(element_end))?;
            }
        }
    }

    // Track type 1 = video
    if track_type == Some(1) {
        info.video_track_number = track_number;
        info.codec_id = codec_id;
    }

    Ok(())
}

/// Parse Cluster element
fn parse_cluster(
    cursor: &mut Cursor<&[u8]>,
    cluster_end: u64,
    info: &mut MkvInfo,
) -> Result<(), BitvueError> {
    let mut cluster_timecode = 0u64;

    const MAX_ELEMENTS_PER_LEVEL: usize = 10_000;
    let mut element_count = 0;

    while cursor.position() < cluster_end {
        if element_count >= MAX_ELEMENTS_PER_LEVEL {
            return Err(BitvueError::InvalidData(
                "Cluster element count exceeded maximum".to_string()
            ));
        }
        element_count += 1;
        let id = read_element_id(cursor)?;
        let size = read_element_size(cursor)?;
        let element_end = cursor.position() + size;

        match id {
            element_id::TIMECODE => {
                cluster_timecode = read_uint(cursor, size as usize)?;
            }
            element_id::SIMPLE_BLOCK => {
                parse_simple_block(cursor, size as usize, cluster_timecode, info)?;
            }
            element_id::BLOCK_GROUP => {
                parse_block_group(cursor, element_end, cluster_timecode, info)?;
            }
            _ => {
                cursor.seek(SeekFrom::Start(element_end))?;
            }
        }
    }

    Ok(())
}

/// Parse SimpleBlock element
fn parse_simple_block(
    cursor: &mut Cursor<&[u8]>,
    size: usize,
    cluster_timecode: u64,
    info: &mut MkvInfo,
) -> Result<(), BitvueError> {
    let start_pos = cursor.position();

    // Read track number (VINT)
    let track_number = read_vint(cursor)?;

    // Check if this is the video track
    if Some(track_number) != info.video_track_number {
        cursor.seek(SeekFrom::Start(start_pos + size as u64))?;
        return Ok(());
    }

    // Read timecode (16-bit signed integer)
    let mut timecode_bytes = [0u8; 2];
    cursor
        .read_exact(&mut timecode_bytes)
        .map_err(|_| BitvueError::UnexpectedEof(cursor.position()))?;
    let relative_timecode = i16::from_be_bytes(timecode_bytes) as i64;

    // Read flags (1 byte)
    let mut flags = [0u8; 1];
    cursor
        .read_exact(&mut flags)
        .map_err(|_| BitvueError::UnexpectedEof(cursor.position()))?;

    // Check if this is a key frame (bit 7 set)
    let is_keyframe = (flags[0] & 0x80) != 0;

    // Calculate frame data size
    let header_size = cursor.position() - start_pos;
    let frame_size = size as u64 - header_size;

    // Read frame data
    let mut frame_data = vec![0u8; frame_size as usize];
    cursor
        .read_exact(&mut frame_data)
        .map_err(|_| BitvueError::UnexpectedEof(cursor.position()))?;

    // Calculate absolute timestamp
    let timestamp = (cluster_timecode as i64 + relative_timecode) as u64;

    // Store frame number (1-indexed) as key frame if keyframe flag is set
    if is_keyframe {
        info.key_frames.push(info.sample_count as u32 + 1);
    }

    info.samples.push(frame_data);
    info.timestamps.push(timestamp);
    info.sample_count += 1;

    Ok(())
}

/// Parse BlockGroup element
fn parse_block_group(
    cursor: &mut Cursor<&[u8]>,
    group_end: u64,
    cluster_timecode: u64,
    info: &mut MkvInfo,
) -> Result<(), BitvueError> {
    const MAX_ELEMENTS_PER_LEVEL: usize = 10_000;
    let mut element_count = 0;

    while cursor.position() < group_end {
        if element_count >= MAX_ELEMENTS_PER_LEVEL {
            return Err(BitvueError::InvalidData(
                "BlockGroup element count exceeded maximum".to_string()
            ));
        }
        element_count += 1;
        let id = read_element_id(cursor)?;
        let size = read_element_size(cursor)?;

        if id == element_id::BLOCK {
            parse_simple_block(cursor, size as usize, cluster_timecode, info)?;
        } else {
            cursor.seek(SeekFrom::Current(size as i64))?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_read_vint() {
        // Single byte: 0x81 = 1000 0001 -> length 1, value 1
        let data = [0x81];
        let mut cursor = Cursor::new(&data[..]);
        assert_eq!(read_vint(&mut cursor).unwrap(), 1);

        // Two bytes: 0x4001 = 0100 0000 0000 0001 -> length 2, value 1
        let data = [0x40, 0x01];
        let mut cursor = Cursor::new(&data[..]);
        assert_eq!(read_vint(&mut cursor).unwrap(), 1);
    }

    #[test]
    fn test_invalid_mkv() {
        let result = parse_mkv(&[]);
        assert!(result.is_err());

        let result = parse_mkv(b"random data");
        assert!(result.is_err());
    }

    #[test]
    fn test_read_uint() {
        // 1-byte uint
        let data = [0x42];
        let mut cursor = Cursor::new(&data[..]);
        assert_eq!(read_uint(&mut cursor, 1).unwrap(), 0x42);

        // 2-byte uint
        let data = [0x12, 0x34];
        let mut cursor = Cursor::new(&data[..]);
        assert_eq!(read_uint(&mut cursor, 2).unwrap(), 0x1234);

        // 4-byte uint
        let data = [0x12, 0x34, 0x56, 0x78];
        let mut cursor = Cursor::new(&data[..]);
        assert_eq!(read_uint(&mut cursor, 4).unwrap(), 0x12345678);

        // 8-byte uint
        let data = [0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08];
        let mut cursor = Cursor::new(&data[..]);
        assert_eq!(read_uint(&mut cursor, 8).unwrap(), 0x0102030405060708u64);
    }

    #[test]
    fn test_read_string() {
        // Simple ASCII string
        let data = b"hello";
        let mut cursor = Cursor::new(&data[..]);
        assert_eq!(read_string(&mut cursor, 5).unwrap(), "hello");

        // String with null terminator
        let data = b"test\0";
        let mut cursor = Cursor::new(&data[..]);
        assert_eq!(read_string(&mut cursor, 5).unwrap(), "test");

        // String with multiple nulls
        let data = b"foo\0\0\0";
        let mut cursor = Cursor::new(&data[..]);
        assert_eq!(read_string(&mut cursor, 6).unwrap(), "foo");
    }

    #[test]
    fn test_vint_edge_cases() {
        // Maximum 1-byte VINT: 0xFF = 1111 1111 -> length 1, value 0x7F
        let data = [0xFF];
        let mut cursor = Cursor::new(&data[..]);
        assert_eq!(read_vint(&mut cursor).unwrap(), 0x7F);

        // 3-byte VINT: 0x20 0x00 0x01
        let data = [0x20, 0x00, 0x01];
        let mut cursor = Cursor::new(&data[..]);
        assert_eq!(read_vint(&mut cursor).unwrap(), 1);

        // 4-byte VINT
        let data = [0x10, 0x00, 0x00, 0xFF];
        let mut cursor = Cursor::new(&data[..]);
        assert_eq!(read_vint(&mut cursor).unwrap(), 0xFF);
    }

    #[test]
    fn test_invalid_vint() {
        // All zeros (no length marker)
        let data = [0x00];
        let mut cursor = Cursor::new(&data[..]);
        assert!(read_vint(&mut cursor).is_err());

        // Empty data
        let data: &[u8] = &[];
        let mut cursor = Cursor::new(data);
        assert!(read_vint(&mut cursor).is_err());

        // Valid VINT markers with complete data for each length (1-8 bytes)
        // 1-byte VINT: 0x81 = marker + value 1
        let data = [0x81];
        let mut cursor = Cursor::new(&data[..]);
        assert_eq!(read_vint(&mut cursor).unwrap(), 1);

        // 2-byte VINT: 0x40 0x01 = marker + value 1
        let data = [0x40, 0x01];
        let mut cursor = Cursor::new(&data[..]);
        assert_eq!(read_vint(&mut cursor).unwrap(), 1);

        // 3-byte VINT: 0x20 0x00 0x01 = marker + value 1
        let data = [0x20, 0x00, 0x01];
        let mut cursor = Cursor::new(&data[..]);
        assert_eq!(read_vint(&mut cursor).unwrap(), 1);

        // 4-byte VINT: 0x10 0x00 0x00 0x01 = marker + value 1
        let data = [0x10, 0x00, 0x00, 0x01];
        let mut cursor = Cursor::new(&data[..]);
        assert_eq!(read_vint(&mut cursor).unwrap(), 1);

        // 5-byte VINT: 0x08 0x00 0x00 0x00 0x01 = marker + value 1
        // SECURITY: VINTs longer than 4 bytes are rejected (DoS prevention)
        let data = [0x08, 0x00, 0x00, 0x00, 0x01];
        let mut cursor = Cursor::new(&data[..]);
        assert!(read_vint(&mut cursor).is_err());

        // 6-byte VINT: 0x04 0x00 0x00 0x00 0x00 0x01 = marker + value 1
        // SECURITY: VINTs longer than 4 bytes are rejected (DoS prevention)
        let data = [0x04, 0x00, 0x00, 0x00, 0x00, 0x01];
        let mut cursor = Cursor::new(&data[..]);
        assert!(read_vint(&mut cursor).is_err());

        // 7-byte VINT: 0x02 0x00 0x00 0x00 0x00 0x00 0x01 = marker + value 1
        // SECURITY: VINTs longer than 4 bytes are rejected (DoS prevention)
        let data = [0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01];
        let mut cursor = Cursor::new(&data[..]);
        assert!(read_vint(&mut cursor).is_err());

        // 8-byte VINT: 0x01 0x00 0x00 0x00 0x00 0x00 0x00 0x01 = marker + value 1
        // SECURITY: VINTs longer than 4 bytes are rejected (DoS prevention)
        let data = [0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01];
        let mut cursor = Cursor::new(&data[..]);
        assert!(read_vint(&mut cursor).is_err());
    }

    #[test]
    fn test_mkv_info_default() {
        let info = MkvInfo::default();
        assert_eq!(info.codec_id, None);
        assert_eq!(info.video_track_number, None);
        assert_eq!(info.sample_count, 0);
        assert_eq!(info.samples.len(), 0);
        assert_eq!(info.timestamps.len(), 0);
    }

    #[test]
    fn test_extract_av1_samples_non_av1() {
        // Create minimal MKV structure with non-AV1 codec
        // This will fail because we can't easily construct a valid MKV
        // But we can test the validation logic
        let result = extract_av1_samples(&[0x1A, 0x45, 0xDF, 0xA3]);
        assert!(result.is_err()); // Should fail on parsing, not codec check
    }
}
