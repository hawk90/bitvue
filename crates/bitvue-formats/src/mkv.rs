//! Matroska (MKV) container parser
//!
//! Implements minimal MKV parsing to extract AV1 video samples.
//! No external dependencies - pure Rust implementation.
//!
//! References:
//! - Matroska specification: <https://www.matroska.org/technical/elements.html>
//! - EBML specification: <https://github.com/ietf-wg-cellar/ebml-specification>

use bitvue_engine::BitvueError;
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
/// Element sizes may be 1-8 bytes (muxers write the Segment size, and unknown sizes, as 8-byte
/// VINTs). Element IDs are at most 4 bytes and keep their marker bit; see `read_element_id`.
fn read_vint(cursor: &mut Cursor<&[u8]>) -> Result<u64, BitvueError> {
    read_vint_with_len(cursor, 8).map(|(value, _)| value)
}

/// `read_vint` with a caller-chosen maximum length; also returns the VINT's length in bytes.
fn read_vint_with_len(
    cursor: &mut Cursor<&[u8]>,
    max_length: usize,
) -> Result<(u64, usize), BitvueError> {
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

    if length > max_length {
        return Err(BitvueError::InvalidData(format!(
            "VINT length {} exceeds maximum allowed {}",
            length, max_length
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

    Ok((value, length))
}

/// Read an EBML element ID (1-4 bytes). Unlike a size, an ID keeps its length-marker bit:
/// `0x1A45DFA3` is the four bytes `1A 45 DF A3`, so the marker bit that `read_vint` strips is put
/// back (`0xAE` is the one-byte ID `0xAE`, not `0x2E`).
fn read_element_id(cursor: &mut Cursor<&[u8]>) -> Result<u32, BitvueError> {
    let (value, length) = read_vint_with_len(cursor, 4)?;
    Ok((value | (1u64 << (7 * length))) as u32)
}

/// Read an EBML element size; `None` is the reserved "unknown size" (every data bit set), which
/// live/streaming muxers write for the Segment and for Clusters.
fn read_element_size_or_unknown(cursor: &mut Cursor<&[u8]>) -> Result<Option<u64>, BitvueError> {
    let (value, length) = read_vint_with_len(cursor, 8)?;
    let all_ones = (1u64 << (7 * length)) - 1;
    Ok((value != all_ones).then_some(value))
}

/// Read an EBML element size for an element that must have a known one.
fn read_element_size(cursor: &mut Cursor<&[u8]>) -> Result<u64, BitvueError> {
    read_element_size_or_unknown(cursor)?.ok_or_else(|| {
        BitvueError::InvalidData(
            "element has an unknown size, which is only allowed for Segment and Cluster"
                .to_string(),
        )
    })
}

/// Level-1 elements of a Segment. An unknown-size Cluster ends where the next one of these starts.
fn is_segment_child(id: u32) -> bool {
    matches!(
        id,
        element_id::CLUSTER
            | element_id::INFO
            | element_id::TRACKS
            | 0x114D_9B74 // SeekHead
            | 0x1C53_BB6B // Cues
            | 0x1043_A770 // Chapters
            | 0x1254_C367 // Tags
            | 0x1941_A469 // Attachments
    )
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

    // PERFORMANCE: Optimize string allocation
    // Try valid UTF-8 first (zero-copy), fall back to lossy conversion
    let result = if let Ok(valid_str) = std::str::from_utf8(&buf) {
        // Valid UTF-8 - trim nulls and convert to String
        valid_str.trim_end_matches('\0').to_string()
    } else {
        // Invalid UTF-8 - use lossy conversion and trim nulls
        String::from_utf8_lossy(&buf)
            .trim_end_matches('\0')
            .to_string()
    };

    Ok(result)
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

    // An unknown Segment size means "to the end of the file".
    let segment_end = match read_element_size_or_unknown(&mut cursor)? {
        Some(size) => cursor.position().saturating_add(size),
        None => data.len() as u64,
    };

    // SECURITY: Limit number of elements to prevent DoS via crafted files
    // with many small elements. Maximum 10K elements per level for defense in depth.
    const MAX_ELEMENTS_PER_LEVEL: usize = 10_000;
    let mut element_count = 0;

    // Parse segment children
    while cursor.position() < segment_end && cursor.position() < data.len() as u64 {
        if element_count >= MAX_ELEMENTS_PER_LEVEL {
            return Err(BitvueError::InvalidData(
                "Segment element count exceeded maximum".to_string(),
            ));
        }
        element_count += 1;
        let element_id = read_element_id(&mut cursor)?;
        let element_size = read_element_size_or_unknown(&mut cursor)?;

        match (element_id, element_size) {
            // An unknown-size Cluster runs until the next Segment child.
            (element_id::CLUSTER, None) => {
                parse_cluster(&mut cursor, data.len() as u64, true, &mut info)?;
            }
            (_, None) => {
                return Err(BitvueError::InvalidData(
                    "element has an unknown size, which is only allowed for Segment and Cluster"
                        .to_string(),
                ));
            }
            (element_id::TRACKS, Some(size)) => {
                let element_end = cursor.position() + size;
                parse_tracks(&mut cursor, element_end, &mut info)?;
            }
            (element_id::CLUSTER, Some(size)) => {
                let element_end = cursor.position() + size;
                parse_cluster(&mut cursor, element_end, false, &mut info)?;
            }
            (_, Some(size)) => {
                // Skip elements we do not read
                let element_end = cursor.position() + size;
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
                "Tracks element count exceeded maximum".to_string(),
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
                "TrackEntry element count exceeded maximum".to_string(),
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

/// Parse Cluster element. With `unknown_size` the cluster has no end offset: it stops (leaving the
/// cursor on it) at the next Segment child such as the following Cluster or the Cues.
fn parse_cluster(
    cursor: &mut Cursor<&[u8]>,
    cluster_end: u64,
    unknown_size: bool,
    info: &mut MkvInfo,
) -> Result<(), BitvueError> {
    let mut cluster_timecode = 0u64;

    const MAX_ELEMENTS_PER_LEVEL: usize = 10_000;
    let mut element_count = 0;

    while cursor.position() < cluster_end {
        if element_count >= MAX_ELEMENTS_PER_LEVEL {
            return Err(BitvueError::InvalidData(
                "Cluster element count exceeded maximum".to_string(),
            ));
        }
        element_count += 1;
        let id_start = cursor.position();
        let id = read_element_id(cursor)?;
        if unknown_size && is_segment_child(id) {
            cursor.seek(SeekFrom::Start(id_start))?;
            return Ok(());
        }
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
    let frame_size = (size as u64).checked_sub(header_size).ok_or_else(|| {
        BitvueError::InvalidData(format!(
            "MKV block declares {size} bytes, less than its {header_size}-byte header"
        ))
    })?;
    // Do not allocate what the file cannot hold: `size` can be up to 2^28 - 1 (#269)
    let remaining = (cursor.get_ref().len() as u64).saturating_sub(cursor.position());
    if frame_size > remaining {
        return Err(BitvueError::UnexpectedEof(cursor.position()));
    }

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
                "BlockGroup element count exceeded maximum".to_string(),
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
        let data = [0x08, 0x00, 0x00, 0x00, 0x01];
        let mut cursor = Cursor::new(&data[..]);
        assert_eq!(read_vint(&mut cursor).unwrap(), 1);

        // 6-byte VINT: 0x04 0x00 0x00 0x00 0x00 0x01 = marker + value 1
        let data = [0x04, 0x00, 0x00, 0x00, 0x00, 0x01];
        let mut cursor = Cursor::new(&data[..]);
        assert_eq!(read_vint(&mut cursor).unwrap(), 1);

        // 7-byte VINT: 0x02 0x00 0x00 0x00 0x00 0x00 0x01 = marker + value 1
        let data = [0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01];
        let mut cursor = Cursor::new(&data[..]);
        assert_eq!(read_vint(&mut cursor).unwrap(), 1);

        // 8-byte VINT: 0x01 0x00 0x00 0x00 0x00 0x00 0x00 0x01 = marker + value 1
        let data = [0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01];
        let mut cursor = Cursor::new(&data[..]);
        assert_eq!(read_vint(&mut cursor).unwrap(), 1);
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

/// Tests that do not depend on how element IDs are decoded (#248), plus spec-correct whole-file tests
/// that are `#[ignore]`d until the ID/size decoding is fixed.
#[cfg(test)]
mod element_tests {
    use super::*;

    fn cur(d: &[u8]) -> Cursor<&[u8]> {
        Cursor::new(d)
    }

    fn vint(d: &[u8]) -> Result<u64, BitvueError> {
        read_vint(&mut cur(d))
    }

    fn invalid(r: Result<impl std::fmt::Debug, BitvueError>, needle: &str) {
        let e = r.unwrap_err().to_string();
        assert!(e.contains(needle), "{e}");
    }

    // ---- read_vint ----

    #[test]
    fn vint_widths_one_to_four_bytes() {
        assert_eq!(vint(&[0x80]).unwrap(), 0);
        assert_eq!(vint(&[0xFF]).unwrap(), 0x7F);
        assert_eq!(vint(&[0x40, 0x00]).unwrap(), 0);
        assert_eq!(vint(&[0x7F, 0xFF]).unwrap(), 0x3FFF);
        assert_eq!(vint(&[0x20, 0x00, 0x00]).unwrap(), 0);
        assert_eq!(vint(&[0x3F, 0xFF, 0xFF]).unwrap(), 0x1F_FFFF);
        assert_eq!(vint(&[0x10, 0x00, 0x00, 0x00]).unwrap(), 0);
        assert_eq!(vint(&[0x1F, 0xFF, 0xFF, 0xFF]).unwrap(), 0x0FFF_FFFF);
        // the marker is the first 1 bit; lower data bits of the first byte are value bits
        assert_eq!(vint(&[0x5A, 0xBC]).unwrap(), 0x1ABC);
        assert_eq!(vint(&[0x12, 0x34, 0x56, 0x78]).unwrap(), 0x0234_5678);
    }

    #[test]
    fn vint_reads_only_its_own_bytes() {
        let d = [0x82, 0xAA, 0xBB];
        let mut c = cur(&d);
        assert_eq!(read_vint(&mut c).unwrap(), 2);
        assert_eq!(c.position(), 1);
        let d = [0x41, 0x02, 0xAA];
        let mut c = cur(&d);
        assert_eq!(read_vint(&mut c).unwrap(), 0x102);
        assert_eq!(c.position(), 2);
        assert_eq!(read_element_size(&mut cur(&[0x85])).unwrap(), 5);
    }

    #[test]
    fn vint_rejects_zero_first_byte_and_truncation() {
        invalid(vint(&[0x00, 0x01]), "no marker bit");
        for d in [&[][..], &[0x40], &[0x20, 0x00], &[0x10, 0x00, 0x00]] {
            assert!(
                matches!(vint(d), Err(BitvueError::UnexpectedEof(_))),
                "{d:02x?}"
            );
        }
    }

    #[test]
    fn vint_seven_byte_form_keeps_its_data_bit() {
        // marker at bit 1 of the first byte, bit 0 is the top data bit
        assert_eq!(
            vint(&[0x03, 0, 0, 0, 0, 0, 0x2A]).unwrap(),
            (1 << 48) | 0x2A
        );
        assert_eq!(vint(&[0x02, 0, 0, 0, 0, 0, 0x2A]).unwrap(), 0x2A);
    }

    #[test]
    fn vint_eight_byte_sizes_are_valid() {
        assert_eq!(vint(&[0x01, 0, 0, 0, 0, 0, 0, 0x2A]).unwrap(), 0x2A);
        assert_eq!(vint(&[0x08, 0, 0, 0, 0x2A]).unwrap(), 0x2A);
        // an unknown size is all ones in the data bits
        assert_eq!(
            vint(&[0x01, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF]).unwrap(),
            (1 << 56) - 1
        );
    }

    // ---- read_uint / read_string ----

    #[test]
    fn uint_sizes() {
        assert_eq!(read_uint(&mut cur(&[]), 0).unwrap(), 0);
        assert_eq!(read_uint(&mut cur(&[0xFF; 16]), 16).unwrap(), u64::MAX); // high bytes shift out
        invalid(read_uint(&mut cur(&[0; 17]), 17), "Uint size 17");
        assert!(matches!(
            read_uint(&mut cur(&[1, 2]), 3),
            Err(BitvueError::UnexpectedEof(_))
        ));
        let d = [0, 0, 0, 0, 0, 0, 0, 0x2A, 0xEE];
        let mut c = cur(&d);
        assert_eq!(read_uint(&mut c, 8).unwrap(), 0x2A);
        assert_eq!(c.position(), 8);
    }

    #[test]
    fn string_sizes_and_encodings() {
        assert_eq!(read_string(&mut cur(&[]), 0).unwrap(), "");
        assert_eq!(read_string(&mut cur(b"\0\0"), 2).unwrap(), "");
        // trailing NULs are trimmed, inner ones are kept
        assert_eq!(read_string(&mut cur(b"a\0b\0"), 4).unwrap(), "a\0b");
        // invalid UTF-8 falls back to lossy conversion and still trims NULs
        assert_eq!(
            read_string(&mut cur(&[b'V', 0xFF, b'1', 0]), 4).unwrap(),
            "V\u{FFFD}1"
        );
        assert_eq!(
            read_string(&mut cur("V_AV1".as_bytes()), 5).unwrap(),
            "V_AV1"
        );
        assert!(matches!(
            read_string(&mut cur(b"ab"), 3),
            Err(BitvueError::UnexpectedEof(_))
        ));
        invalid(read_string(&mut cur(&[]), 1_000_001), "exceeds maximum");
        // exactly the limit passes the size check (then runs out of data)
        assert!(matches!(
            read_string(&mut cur(&[]), 1_000_000),
            Err(BitvueError::UnexpectedEof(_))
        ));
    }

    // ---- parse_simple_block ----

    fn video_info() -> MkvInfo {
        MkvInfo {
            video_track_number: Some(1),
            ..Default::default()
        }
    }

    /// SimpleBlock payload: track (1-byte vint), signed 16-bit relative timecode, flags, frame
    fn block(track: u8, rel: i16, flags: u8, frame: &[u8]) -> Vec<u8> {
        let mut v = vec![0x80 | track];
        v.extend(rel.to_be_bytes());
        v.push(flags);
        v.extend_from_slice(frame);
        v
    }

    #[test]
    fn simple_block_stores_frame_timestamp_and_keyframe_flag() {
        let mut info = video_info();
        let b1 = block(1, 10, 0x80, &[1, 2, 3]);
        let b2 = block(1, -4, 0x00, &[4]);
        let mut d = b1.clone();
        d.extend(&b2);
        let mut c = cur(&d);
        parse_simple_block(&mut c, b1.len(), 1000, &mut info).unwrap();
        assert_eq!(c.position(), b1.len() as u64);
        parse_simple_block(&mut c, b2.len(), 1000, &mut info).unwrap();
        assert_eq!(info.samples, vec![vec![1, 2, 3], vec![4]]);
        assert_eq!(info.timestamps, vec![1010, 996]);
        assert_eq!(info.key_frames, vec![1]); // 1-based index of the key frame
        assert_eq!(info.sample_count, 2);
    }

    #[test]
    fn simple_block_key_frame_numbers_follow_the_sample_count() {
        let mut info = video_info();
        for (i, flags) in [0x80u8, 0x00, 0x00, 0x80].into_iter().enumerate() {
            let b = block(1, i as i16, flags, &[i as u8]);
            parse_simple_block(&mut cur(&b), b.len(), 0, &mut info).unwrap();
        }
        assert_eq!(info.key_frames, vec![1, 4]);
        // any other flag bit (invisible, lacing, discardable) is not a key frame
        let b = block(1, 0, 0x7F, &[9]);
        parse_simple_block(&mut cur(&b), b.len(), 0, &mut info).unwrap();
        assert_eq!(info.key_frames, vec![1, 4]);
    }

    #[test]
    fn simple_block_skips_other_tracks_and_leaves_the_cursor_after_them() {
        let mut info = video_info();
        let other = block(2, 0, 0x80, &[7, 7, 7, 7]);
        let mut d = other.clone();
        d.push(0xEE);
        let mut c = cur(&d);
        parse_simple_block(&mut c, other.len(), 0, &mut info).unwrap();
        assert_eq!(c.position(), other.len() as u64);
        assert!(info.samples.is_empty() && info.key_frames.is_empty());
        // no video track known at all: nothing is stored
        let mut none = MkvInfo::default();
        let b = block(1, 0, 0x80, &[1]);
        parse_simple_block(&mut cur(&b), b.len(), 0, &mut none).unwrap();
        assert!(none.samples.is_empty());
    }

    #[test]
    fn simple_block_with_a_multi_byte_track_number() {
        let mut info = MkvInfo {
            video_track_number: Some(300),
            ..Default::default()
        };
        let mut b = vec![0x41, 0x2C]; // 2-byte vint = 300
        b.extend([0, 5, 0x80, 0xAA]);
        parse_simple_block(&mut cur(&b), b.len(), 100, &mut info).unwrap();
        assert_eq!(info.timestamps, vec![105]);
        assert_eq!(info.samples, vec![vec![0xAA]]);
    }

    #[test]
    fn simple_block_with_no_frame_data_is_an_empty_sample() {
        let mut info = video_info();
        let b = block(1, 0, 0x00, &[]);
        parse_simple_block(&mut cur(&b), b.len(), 0, &mut info).unwrap();
        assert_eq!(info.samples, vec![Vec::<u8>::new()]);
    }

    #[test]
    fn simple_block_truncation_is_an_error() {
        for cut in 0..4 {
            let full = block(1, 5, 0x80, &[1, 2, 3]);
            let b = &full[..cut];
            assert!(
                parse_simple_block(&mut cur(b), full.len(), 0, &mut video_info()).is_err(),
                "cut {cut}"
            );
        }
        // frame data shorter than the declared size
        let full = block(1, 5, 0x80, &[1, 2, 3]);
        let r = parse_simple_block(
            &mut cur(&full[..full.len() - 1]),
            full.len(),
            0,
            &mut video_info(),
        );
        assert!(matches!(r, Err(BitvueError::UnexpectedEof(_))));
    }

    #[test]
    fn simple_block_timestamp_arithmetic_wraps_below_zero_like_the_cast_says() {
        // cluster timecode 0 with a negative relative timecode: (0 + -4) as u64
        let mut info = video_info();
        let b = block(1, -4, 0x00, &[1]);
        parse_simple_block(&mut cur(&b), b.len(), 0, &mut info).unwrap();
        assert_eq!(info.timestamps, vec![(-4i64) as u64]);
    }

    #[test]
    fn simple_block_with_a_declared_size_below_its_header_is_an_error_not_a_panic() {
        // #269: size 2 for a 4-byte header (track vint + 2 timecode bytes + flags) underflowed
        let b = block(1, 0, 0x80, &[1, 2, 3]);
        for size in [0, 1, 2, 3] {
            let mut info = video_info();
            let e = parse_simple_block(&mut cur(&b), size, 0, &mut info)
                .unwrap_err()
                .to_string();
            assert!(e.contains("less than its"), "{size}: {e}");
            assert_eq!(info.sample_count, 0);
        }
        // exactly the header: an empty frame, not an error
        let mut info = video_info();
        parse_simple_block(&mut cur(&b), 4, 0, &mut info).unwrap();
        assert_eq!(info.samples, vec![Vec::<u8>::new()]);
    }

    #[test]
    fn simple_block_declaring_more_than_the_file_holds_fails_before_allocating() {
        // 2^28 - 1 is the largest size a 4-byte vint can declare
        let b = block(1, 0, 0x80, &[1, 2, 3]);
        let e = parse_simple_block(&mut cur(&b), (1 << 28) - 1, 0, &mut video_info());
        assert!(matches!(e, Err(BitvueError::UnexpectedEof(_))), "{e:?}");
    }

    // ---- top-level errors that hold regardless of ID decoding ----

    #[test]
    fn parse_mkv_rejects_short_and_foreign_data() {
        invalid(parse_mkv(&[0x1A, 0x45, 0xDF]), "too small");
        // exactly 4 bytes is long enough to look at, just not valid
        let e = parse_mkv(&[0x1A, 0x45, 0xDF, 0xA3])
            .unwrap_err()
            .to_string();
        assert!(!e.contains("too small"), "{e}");
        assert!(parse_mkv(&[0x1A, 0x45, 0xDF, 0xA3]).is_err()); // header id only
        assert!(parse_mkv(b"RIFF....AVI ").is_err());
        assert!(parse_mkv(&[0u8; 64]).is_err());
        assert!(extract_av1_samples(b"not matroska").is_err());
        assert!(extract_avc_samples(b"not matroska").is_err());
        assert!(extract_hevc_samples(b"not matroska").is_err());
    }

    // ---- spec-correct whole files: fail until #248 is fixed ----

    fn size(n: usize) -> Vec<u8> {
        if n < 0x7F {
            vec![0x80 | n as u8]
        } else if n < 0x3FFF {
            vec![0x40 | (n >> 8) as u8, n as u8]
        } else {
            vec![0x20 | (n >> 16) as u8, (n >> 8) as u8, n as u8]
        }
    }

    fn el(id: &[u8], payload: &[u8]) -> Vec<u8> {
        let mut v = id.to_vec();
        v.extend(size(payload.len()));
        v.extend_from_slice(payload);
        v
    }

    fn uint_el(id: &[u8], v: u8) -> Vec<u8> {
        el(id, &[v])
    }

    fn track(number: u8, kind: u8, codec: &str) -> Vec<u8> {
        let mut e = uint_el(&[0xD7], number);
        e.extend(uint_el(&[0x83], kind));
        e.extend(el(&[0x86], codec.as_bytes()));
        el(&[0xAE], &e)
    }

    fn mkv_file(codec: &str, clusters: &[Vec<u8>]) -> Vec<u8> {
        let mut f = el(&[0x1A, 0x45, 0xDF, 0xA3], &el(&[0x42, 0x82], b"matroska"));
        let mut seg = el(&[0xEC], &[0; 3]); // Void
        let mut tracks = track(2, 2, "A_OPUS");
        tracks.extend(track(1, 1, codec));
        seg.extend(el(&[0x16, 0x54, 0xAE, 0x6B], &tracks));
        for c in clusters {
            seg.extend(c);
        }
        f.extend(el(&[0x18, 0x53, 0x80, 0x67], &seg));
        f
    }

    fn cluster(timecode: u8, blocks: &[Vec<u8>]) -> Vec<u8> {
        let mut c = uint_el(&[0xE7], timecode);
        for b in blocks {
            c.extend(b);
        }
        el(&[0x1F, 0x43, 0xB6, 0x75], &c)
    }

    fn simple(track: u8, rel: i16, flags: u8, frame: &[u8]) -> Vec<u8> {
        el(&[0xA3], &block(track, rel, flags, frame))
    }

    fn group(track: u8, rel: i16, frame: &[u8]) -> Vec<u8> {
        let mut g = el(&[0xA1], &block(track, rel, 0x00, frame));
        g.extend(uint_el(&[0x9B], 1)); // BlockDuration, skipped
        el(&[0xA0], &g)
    }

    /// #248: ffmpeg/libwebm write the Segment size as an 8-byte VINT (`01 00 00 00 ..`), which the
    /// parser used to refuse; a muxer-style file must parse the same as the small-size form.
    #[test]
    fn segment_size_written_as_an_eight_byte_vint_parses() {
        let small = mkv_file("V_AV1", &[cluster(0, &[simple(1, 0, 0x80, &[7, 7])])]);
        // Re-encode the Segment size field (id at `pos`, 1-byte size) as 8 bytes.
        let pos = small
            .windows(4)
            .position(|w| w == [0x18, 0x53, 0x80, 0x67])
            .unwrap();
        assert!(small[pos + 4] & 0x80 != 0, "fixture uses a 1-byte size");
        let len = (small[pos + 4] & 0x7F) as u64;
        let mut wide = small[..pos + 4].to_vec();
        wide.push(0x01);
        wide.extend_from_slice(&len.to_be_bytes()[1..]);
        wide.extend_from_slice(&small[pos + 5..]);

        let info = parse_mkv(&wide).unwrap();
        assert_eq!(info.samples, vec![vec![7, 7]]);
        assert_eq!(info.codec_id.as_deref(), Some("V_AV1"));
    }

    /// A Cluster written with the reserved "unknown size" (live muxers): `1F43B675 FF <children>`.
    fn unknown_size_cluster(timecode: u8, blocks: &[Vec<u8>]) -> Vec<u8> {
        let mut c = vec![0x1F, 0x43, 0xB6, 0x75, 0xFF];
        c.extend(uint_el(&[0xE7], timecode));
        for b in blocks {
            c.extend(b);
        }
        c
    }

    /// #352: an unknown-size Cluster ends at the next Segment child; before, it swallowed the rest
    /// of the file and every following Cluster's blocks were silently dropped.
    #[test]
    fn unknown_size_clusters_end_at_the_next_segment_child() {
        let f = mkv_file(
            "V_AV1",
            &[
                unknown_size_cluster(10, &[simple(1, 0, 0x80, &[1]), simple(1, 1, 0x00, &[2])]),
                unknown_size_cluster(20, &[simple(1, 0, 0x00, &[3])]),
                // known-size cluster after them, then a Cues element
                cluster(30, &[simple(1, 0, 0x00, &[4])]),
                el(&[0x1C, 0x53, 0xBB, 0x6B], &[0xEC, 0x80]),
            ],
        );
        let i = parse_mkv(&f).unwrap();
        assert_eq!(i.samples, vec![vec![1], vec![2], vec![3], vec![4]]);
        assert_eq!(i.timestamps, vec![10, 11, 20, 30]);
        assert_eq!(i.key_frames, vec![1]); // 1-based, like the other tests
    }

    #[test]
    fn unknown_size_cluster_followed_by_cues_stops_at_the_cues() {
        let cues = el(&[0x1C, 0x53, 0xBB, 0x6B], &[0xEC, 0x80]);
        let f = mkv_file(
            "V_AV1",
            &[unknown_size_cluster(5, &[simple(1, 0, 0x80, &[9])]), cues],
        );
        assert_eq!(parse_mkv(&f).unwrap().samples, vec![vec![9]]);
    }

    #[test]
    fn unknown_size_on_any_other_element_is_an_error_not_a_guess() {
        // A Tracks element with an unknown size.
        let mut f = el(&[0x1A, 0x45, 0xDF, 0xA3], &el(&[0x42, 0x82], b"matroska"));
        let mut seg = vec![0x16, 0x54, 0xAE, 0x6B, 0xFF];
        seg.extend(track(1, 1, "V_AV1"));
        f.extend(el(&[0x18, 0x53, 0x80, 0x67], &seg));
        invalid(parse_mkv(&f), "unknown size");
    }

    #[test]
    fn unknown_size_segment_runs_to_the_end_of_the_file() {
        let mut f = mkv_file("V_AV1", &[cluster(0, &[simple(1, 0, 0x80, &[7])])]);
        let pos = f
            .windows(4)
            .position(|w| w == [0x18, 0x53, 0x80, 0x67])
            .unwrap();
        f[pos + 4] = 0xFF; // 1-byte size field -> unknown
        assert_eq!(parse_mkv(&f).unwrap().samples, vec![vec![7]]);
    }

    #[test]
    fn whole_file_tracks_clusters_and_blocks() {
        let f = mkv_file(
            "V_AV1",
            &[
                cluster(
                    100,
                    &[
                        simple(1, 0, 0x80, &[1, 1]),
                        simple(2, 0, 0x80, &[9]),
                        simple(1, 33, 0x00, &[2]),
                    ],
                ),
                cluster(200, &[group(1, 5, &[3, 3, 3]), group(2, 5, &[8])]),
            ],
        );
        let i = parse_mkv(&f).unwrap();
        assert_eq!(i.codec_id.as_deref(), Some("V_AV1"));
        assert_eq!(i.video_track_number, Some(1));
        assert_eq!(i.samples, vec![vec![1, 1], vec![2], vec![3, 3, 3]]);
        assert_eq!(i.timestamps, vec![100, 133, 205]);
        assert_eq!(i.key_frames, vec![1]);
        assert_eq!(i.sample_count, 3);
        assert_eq!(extract_av1_samples(&f).unwrap().len(), 3);
        assert!(extract_avc_samples(&f)
            .unwrap_err()
            .to_string()
            .contains("Not an H.264/AVC file"));
        assert!(extract_hevc_samples(&f)
            .unwrap_err()
            .to_string()
            .contains("Not an H.265/HEVC file"));
    }

    #[test]
    fn whole_file_codec_ids_select_the_extractor() {
        let c = cluster(0, &[simple(1, 0, 0x80, &[5])]);
        let avc = mkv_file("V_MPEG4/ISO/AVC", std::slice::from_ref(&c));
        assert_eq!(extract_avc_samples(&avc).unwrap(), vec![vec![5]]);
        assert!(extract_av1_samples(&avc)
            .unwrap_err()
            .to_string()
            .contains("V_MPEG4/ISO/AVC"));
        let hevc = mkv_file("V_MPEGH/ISO/HEVC", std::slice::from_ref(&c));
        assert_eq!(extract_hevc_samples(&hevc).unwrap(), vec![vec![5]]);
    }

    #[test]
    fn whole_file_without_a_video_track_has_no_codec() {
        let mut f = el(&[0x1A, 0x45, 0xDF, 0xA3], &[]);
        let tracks = el(&[0x16, 0x54, 0xAE, 0x6B], &track(1, 2, "A_OPUS"));
        f.extend(el(&[0x18, 0x53, 0x80, 0x67], &tracks));
        assert!(parse_mkv(&f).unwrap().codec_id.is_none());
        assert!(extract_av1_samples(&f)
            .unwrap_err()
            .to_string()
            .contains("No codec information"));
    }

    fn file_with(segment_children: &[u8]) -> Vec<u8> {
        let mut f = el(&[0x1A, 0x45, 0xDF, 0xA3], &[]);
        f.extend(el(&[0x18, 0x53, 0x80, 0x67], segment_children));
        f
    }

    #[test]
    fn whole_file_bytes_after_the_segment_are_not_parsed() {
        let mut f = mkv_file("V_AV1", &[cluster(0, &[simple(1, 0, 0x80, &[1])])]);
        let ok = parse_mkv(&f).unwrap();
        f.extend([0x00, 0x00, 0x00]); // not a valid element: would be an error if read as one
        let with_tail = parse_mkv(&f).unwrap();
        assert_eq!(with_tail.samples, ok.samples);
    }

    #[test]
    fn whole_file_element_count_limit_is_10000_per_level() {
        let voids = |n: usize| [0xECu8, 0x80].repeat(n);
        type Wrap = fn(&[u8]) -> Vec<u8>;
        let levels: [(&str, Wrap); 5] = [
            ("Segment", |v| v.to_vec()),
            ("Tracks", |v| el(&[0x16, 0x54, 0xAE, 0x6B], v)),
            ("TrackEntry", |v| {
                el(&[0x16, 0x54, 0xAE, 0x6B], &el(&[0xAE], v))
            }),
            ("Cluster", |v| el(&[0x1F, 0x43, 0xB6, 0x75], v)),
            ("BlockGroup", |v| {
                el(&[0x1F, 0x43, 0xB6, 0x75], &el(&[0xA0], v))
            }),
        ];
        for (name, wrap) in levels {
            assert!(
                parse_mkv(&file_with(&wrap(&voids(10_000)))).is_ok(),
                "{name}: 10000 elements"
            );
            let e = parse_mkv(&file_with(&wrap(&voids(10_001))))
                .unwrap_err()
                .to_string();
            assert!(
                e.contains(&format!("{name} element count exceeded")),
                "{name}: {e}"
            );
        }
    }

    #[test]
    fn whole_file_wrong_top_level_elements() {
        // EBML header ok, but the next element is not a Segment
        let mut f = el(&[0x1A, 0x45, 0xDF, 0xA3], &[]);
        f.extend(el(&[0xEC], &[0; 4]));
        assert!(parse_mkv(&f)
            .unwrap_err()
            .to_string()
            .contains("Expected Segment"));
        // truncated mid-element
        let ok = mkv_file("V_AV1", &[cluster(0, &[simple(1, 0, 0x80, &[1, 2, 3])])]);
        assert!(parse_mkv(&ok[..ok.len() - 2]).is_err());
    }
}
