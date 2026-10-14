//! IVF container parser
//!
//! IVF is a simple container format used for storing AV1 test vectors.
//! Structure:
//! - 32-byte header
//! - Frame entries: 4-byte size + 8-byte timestamp + frame data

use crate::bitreader::BitReader;
use crate::obu::parse_obu_header;
use bitvue_engine::BitvueError;

/// IVF file header size in bytes
pub const IVF_HEADER_SIZE: usize = 32;

/// IVF frame header size in bytes (size + timestamp)
pub const IVF_FRAME_HEADER_SIZE: usize = 12;

/// Maximum valid IVF frame size (100 MB)
pub const IVF_MAX_FRAME_SIZE: usize = 100 * 1024 * 1024;

/// Maximum number of frames to prevent DoS through millions of tiny frames
pub const IVF_MAX_FRAME_COUNT: usize = 1_000_000;

/// IVF file header (32 bytes)
#[derive(Debug, Clone)]
pub struct IvfHeader {
    /// Signature: "DKIF"
    pub signature: [u8; 4],
    /// Version (should be 0)
    pub version: u16,
    /// Header size (should be 32)
    pub header_size: u16,
    /// FourCC (e.g., "AV01")
    pub fourcc: [u8; 4],
    /// Video width
    pub width: u16,
    /// Video height
    pub height: u16,
    /// Frame rate denominator
    pub framerate_den: u32,
    /// Frame rate numerator
    pub framerate_num: u32,
    /// Number of frames
    pub frame_count: u32,
}

/// IVF frame entry
#[derive(Debug, Clone)]
pub struct IvfFrame {
    /// Frame size in bytes
    pub size: u32,
    /// Presentation timestamp
    pub timestamp: u64,
    /// Frame data (raw OBU bytes)
    pub data: Vec<u8>,
    /// Temporal layer ID (from frame header OBU)
    pub temporal_id: u8,
}

impl IvfFrame {
    /// Creates a new IvfFrameBuilder for constructing IvfFrame instances
    pub fn builder() -> IvfFrameBuilder {
        IvfFrameBuilder::default()
    }

    /// Get timestamp as TimestampPts for type-safe access
    ///
    /// Provides validation and type safety for timestamp operations.
    /// Returns an error if timestamp is negative (shouldn't happen with u64).
    #[inline]
    pub fn timestamp_pts(&self) -> Result<crate::TimestampPts, BitvueError> {
        // u64 should always be non-negative, but we validate anyway
        crate::TimestampPts::new(self.timestamp as i64)
    }
}

/// Builder for constructing IvfFrame instances
///
/// # Example
///
/// ```
/// use bitvue_av1_codec::ivf::IvfFrame;
///
/// let frame = IvfFrame::builder()
///     .size(1024)
///     .timestamp(0)
///     .data(vec![0x00, 0x00, 0x01])
///     .temporal_id(0)
///     .build()
///     .expect("required fields not set");
/// ```
#[derive(Debug, Default)]
pub struct IvfFrameBuilder {
    size: Option<u32>,
    timestamp: Option<u64>,
    data: Option<Vec<u8>>,
    temporal_id: Option<u8>,
}

impl IvfFrameBuilder {
    /// Set the frame size
    pub fn size(mut self, value: u32) -> Self {
        self.size = Some(value);
        self
    }

    /// Set the timestamp
    ///
    /// Note: Timestamps must be non-negative per media specification.
    /// For type-safe timestamp handling, consider using timestamp_pts() instead.
    pub fn timestamp(mut self, value: u64) -> Self {
        self.timestamp = Some(value);
        self
    }

    /// Set the timestamp from TimestampPts for type safety
    ///
    /// Provides validation that timestamp is non-negative.
    /// This is the preferred method for setting timestamps.
    pub fn timestamp_pts(mut self, pts: crate::TimestampPts) -> Self {
        self.timestamp = Some(pts.value() as u64);
        self
    }

    /// Set the frame data
    pub fn data(mut self, value: Vec<u8>) -> Self {
        self.data = Some(value);
        self
    }

    /// Set the temporal ID
    pub fn temporal_id(mut self, value: u8) -> Self {
        self.temporal_id = Some(value);
        self
    }

    /// Build the IvfFrame
    ///
    /// Returns an error if required fields (size, timestamp, temporal_id) are not set.
    pub fn build(self) -> Result<IvfFrame, BitvueError> {
        Ok(IvfFrame {
            size: self.size.ok_or_else(|| {
                BitvueError::InvalidData("IvfFrame: size is required".to_string())
            })?,
            timestamp: self.timestamp.ok_or_else(|| {
                BitvueError::InvalidData("IvfFrame: timestamp is required".to_string())
            })?,
            data: self.data.unwrap_or_default(),
            temporal_id: self.temporal_id.ok_or_else(|| {
                BitvueError::InvalidData("IvfFrame: temporal_id is required".to_string())
            })?,
        })
    }
}

/// Parse IVF header from data
pub fn parse_ivf_header(data: &[u8]) -> Result<IvfHeader, BitvueError> {
    // Validate header size upfront
    let header_bytes = data.get(0..32).ok_or(BitvueError::InsufficientData {
        needed: 32,
        available: data.len(),
    })?;

    // Use safe array accesses via get()
    let signature: [u8; 4] = header_bytes[0..4]
        .try_into()
        .map_err(|_| BitvueError::InvalidData("Invalid IVF signature length".to_string()))?;
    if &signature != b"DKIF" {
        return Err(BitvueError::InvalidData(format!(
            "Invalid IVF signature: {:?}",
            signature
        )));
    }

    let version_bytes: [u8; 2] = header_bytes
        .get(4..6)
        .ok_or_else(|| BitvueError::InvalidData("IVF header too short for version".to_string()))?
        .try_into()
        .map_err(|_| BitvueError::InvalidData("IVF version bytes invalid".to_string()))?;
    let version = u16::from_le_bytes(version_bytes);

    let header_size_bytes: [u8; 2] = header_bytes
        .get(6..8)
        .ok_or_else(|| {
            BitvueError::InvalidData("IVF header too short for header_size".to_string())
        })?
        .try_into()
        .map_err(|_| BitvueError::InvalidData("IVF header_size bytes invalid".to_string()))?;
    let header_size = u16::from_le_bytes(header_size_bytes);

    let fourcc: [u8; 4] = header_bytes
        .get(8..12)
        .and_then(|s| s.try_into().ok())
        .ok_or_else(|| BitvueError::InvalidData("Invalid IVF fourcc length".to_string()))?;

    let width_bytes: [u8; 2] = header_bytes
        .get(12..14)
        .ok_or_else(|| BitvueError::InvalidData("IVF header too short for width".to_string()))?
        .try_into()
        .map_err(|_| BitvueError::InvalidData("IVF width bytes invalid".to_string()))?;
    let width = u16::from_le_bytes(width_bytes);

    let height_bytes: [u8; 2] = header_bytes
        .get(14..16)
        .ok_or_else(|| BitvueError::InvalidData("IVF header too short for height".to_string()))?
        .try_into()
        .map_err(|_| BitvueError::InvalidData("IVF height bytes invalid".to_string()))?;
    let height = u16::from_le_bytes(height_bytes);

    let framerate_den_bytes: [u8; 4] = header_bytes
        .get(16..20)
        .ok_or_else(|| {
            BitvueError::InvalidData("IVF header too short for framerate_den".to_string())
        })?
        .try_into()
        .map_err(|_| BitvueError::InvalidData("IVF framerate_den bytes invalid".to_string()))?;
    let framerate_den = u32::from_le_bytes(framerate_den_bytes);

    let framerate_num_bytes: [u8; 4] = header_bytes
        .get(20..24)
        .ok_or_else(|| {
            BitvueError::InvalidData("IVF header too short for framerate_num".to_string())
        })?
        .try_into()
        .map_err(|_| BitvueError::InvalidData("IVF framerate_num bytes invalid".to_string()))?;
    let framerate_num = u32::from_le_bytes(framerate_num_bytes);

    let frame_count_bytes: [u8; 4] = header_bytes
        .get(24..28)
        .ok_or_else(|| {
            BitvueError::InvalidData("IVF header too short for frame_count".to_string())
        })?
        .try_into()
        .map_err(|_| BitvueError::InvalidData("IVF frame_count bytes invalid".to_string()))?;
    let frame_count = u32::from_le_bytes(frame_count_bytes);

    Ok(IvfHeader {
        signature,
        version,
        header_size,
        fourcc,
        width,
        height,
        framerate_den,
        framerate_num,
        frame_count,
    })
}

/// Parse all frames from IVF data
pub fn parse_ivf_frames(data: &[u8]) -> Result<(IvfHeader, Vec<IvfFrame>), BitvueError> {
    let header = parse_ivf_header(data)?;
    let mut frames = Vec::new();
    let mut offset = header.header_size as usize;

    while frames.len() < IVF_MAX_FRAME_COUNT {
        // Check for overflow in offset + 12 calculation
        let offset_end = offset.checked_add(12).ok_or_else(|| {
            BitvueError::InvalidData("IVF offset would cause integer overflow".to_string())
        })?;

        let frame_header = match data.get(offset..offset_end) {
            Some(h) => h,
            None => break, // End of data
        };
        let frame_size_bytes: [u8; 4] = frame_header
            .get(0..4)
            .ok_or_else(|| BitvueError::InvalidData("IVF frame header incomplete".to_string()))?
            .try_into()
            .map_err(|_| BitvueError::InvalidData("IVF frame size bytes invalid".to_string()))?;
        let frame_size = u32::from_le_bytes(frame_size_bytes);

        let timestamp_bytes: [u8; 8] = frame_header
            .get(4..12)
            .ok_or_else(|| BitvueError::InvalidData("IVF frame timestamp incomplete".to_string()))?
            .try_into()
            .map_err(|_| BitvueError::InvalidData("IVF timestamp bytes invalid".to_string()))?;
        let timestamp = u64::from_le_bytes(timestamp_bytes);

        offset += 12;

        // SECURITY: Validate frame size to prevent integer overflow and DoS
        // Maximum frame size is 100MB (IVF_MAX_FRAME_SIZE)
        if frame_size as usize > IVF_MAX_FRAME_SIZE {
            return Err(BitvueError::InvalidData(format!(
                "IVF frame size {} exceeds maximum allowed size of {}",
                frame_size, IVF_MAX_FRAME_SIZE
            )));
        }

        // Check for integer overflow in offset calculation
        let frame_end = offset.checked_add(frame_size as usize).ok_or_else(|| {
            BitvueError::InvalidData("IVF frame size would cause integer overflow".to_string())
        })?;

        // Check if frame data is available
        if frame_end > data.len() {
            break;
        }

        let frame_data = data
            .get(offset..frame_end)
            .ok_or_else(|| BitvueError::InvalidData("IVF frame data out of bounds".to_string()))?
            .to_vec();

        // Extract temporal_id from frame header OBU
        let temporal_id = extract_temporal_id_from_frame(&frame_data).unwrap_or(0);

        frames.push(IvfFrame {
            size: frame_size,
            timestamp,
            data: frame_data,
            temporal_id,
        });

        // Move offset to end of this frame (we already validated frame_end is valid)
        offset = frame_end;
    }

    Ok((header, frames))
}

/// Extract temporal_id from frame data (finds frame header OBU)
fn extract_temporal_id_from_frame(frame_data: &[u8]) -> Option<u8> {
    use crate::obu::ObuType;

    let mut pos = 0;

    while pos + 4 <= frame_data.len() {
        let obu_header = parse_obu_header(&mut BitReader::new(frame_data)).ok()?;

        // Look for frame header OBU (type 3) or frame OBU (type 6)
        if obu_header.obu_type == ObuType::FrameHeader || obu_header.obu_type == ObuType::Frame {
            return Some(obu_header.temporal_id);
        }

        pos += obu_header.header_size;

        // Skip size field if present
        if obu_header.has_size {
            if pos + 2 > frame_data.len() {
                break;
            }
            let obu_size = u16::from_le_bytes([frame_data[pos], frame_data[pos + 1]]) as usize;
            pos += 2 + obu_size;
        } else {
            // No size field, skip to end or find next OBU
            break;
        }
    }

    None
}

/// Extract raw OBU data from IVF file (concatenated frame data)
pub fn extract_obu_data(data: &[u8]) -> Result<Vec<u8>, BitvueError> {
    let (_, frames) = parse_ivf_frames(data)?;
    let mut obu_data = Vec::new();
    for frame in frames {
        obu_data.extend(frame.data);
    }
    Ok(obu_data)
}

/// Check if data is IVF format
pub fn is_ivf(data: &[u8]) -> bool {
    data.len() >= 4 && &data[0..4] == b"DKIF"
}

/// Check if data is AV1 in IVF
pub fn is_av1_ivf(data: &[u8]) -> bool {
    data.len() >= 12 && &data[0..4] == b"DKIF" && &data[8..12] == b"AV01"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_ivf() {
        let ivf_data = b"DKIF\x00\x00\x20\x00AV01";
        assert!(is_ivf(ivf_data));
        assert!(!is_ivf(b"notivf"));
    }

    #[test]
    fn test_is_av1_ivf() {
        let mut data = vec![0u8; 32];
        data[0..4].copy_from_slice(b"DKIF");
        data[8..12].copy_from_slice(b"AV01");
        assert!(is_av1_ivf(&data));

        data[8..12].copy_from_slice(b"VP90");
        assert!(!is_av1_ivf(&data));
    }

    #[test]
    fn test_parse_ivf_header() {
        let mut data = vec![0u8; 32];
        data[0..4].copy_from_slice(b"DKIF");
        data[4..6].copy_from_slice(&0u16.to_le_bytes()); // version
        data[6..8].copy_from_slice(&32u16.to_le_bytes()); // header size
        data[8..12].copy_from_slice(b"AV01");
        data[12..14].copy_from_slice(&1920u16.to_le_bytes()); // width
        data[14..16].copy_from_slice(&1080u16.to_le_bytes()); // height
        data[16..20].copy_from_slice(&1u32.to_le_bytes()); // framerate_den
        data[20..24].copy_from_slice(&30u32.to_le_bytes()); // framerate_num
        data[24..28].copy_from_slice(&100u32.to_le_bytes()); // frame_count

        let header = parse_ivf_header(&data).unwrap();
        assert_eq!(&header.signature, b"DKIF");
        assert_eq!(header.version, 0);
        assert_eq!(header.header_size, 32);
        assert_eq!(&header.fourcc, b"AV01");
        assert_eq!(header.width, 1920);
        assert_eq!(header.height, 1080);
        assert_eq!(header.framerate_den, 1);
        assert_eq!(header.framerate_num, 30);
        assert_eq!(header.frame_count, 100);
    }

    #[test]
    fn test_parse_ivf_frames() {
        let mut data = vec![0u8; 32];
        data[0..4].copy_from_slice(b"DKIF");
        data[6..8].copy_from_slice(&32u16.to_le_bytes()); // header size
        data[8..12].copy_from_slice(b"AV01");

        // Add a frame: 4-byte size + 8-byte timestamp + data
        let frame_data = vec![0x12, 0x00, 0x0a, 0x0b]; // OBU header bytes
        data.extend(&(frame_data.len() as u32).to_le_bytes());
        data.extend(&0u64.to_le_bytes());
        data.extend(&frame_data);

        let (_header, frames) = parse_ivf_frames(&data).unwrap();
        assert_eq!(frames.len(), 1);
        assert_eq!(frames[0].size, 4);
        assert_eq!(frames[0].data, frame_data);
    }

    fn header_bytes(header_size: u16) -> Vec<u8> {
        let mut data = vec![0u8; 32];
        data[0..4].copy_from_slice(b"DKIF");
        data[4..6].copy_from_slice(&1u16.to_le_bytes());
        data[6..8].copy_from_slice(&header_size.to_le_bytes());
        data[8..12].copy_from_slice(b"AV01");
        data[12..14].copy_from_slice(&640u16.to_le_bytes());
        data[14..16].copy_from_slice(&360u16.to_le_bytes());
        data[16..20].copy_from_slice(&1001u32.to_le_bytes());
        data[20..24].copy_from_slice(&30000u32.to_le_bytes());
        data[24..28].copy_from_slice(&7u32.to_le_bytes());
        data
    }

    fn push_frame(data: &mut Vec<u8>, size_field: u32, timestamp: u64, payload: &[u8]) {
        data.extend(size_field.to_le_bytes());
        data.extend(timestamp.to_le_bytes());
        data.extend(payload);
    }

    /// Header + three frames of 3, 0 and 5 bytes with timestamps 10, 20, 30.
    fn three_frames() -> Vec<u8> {
        let mut data = header_bytes(32);
        push_frame(&mut data, 3, 10, &[1, 2, 3]);
        push_frame(&mut data, 0, 20, &[]);
        push_frame(&mut data, 5, 30, &[4, 5, 6, 7, 8]);
        data
    }

    fn is_invalid_data<T: std::fmt::Debug>(result: Result<T, BitvueError>) -> bool {
        matches!(result, Err(BitvueError::InvalidData(_)))
    }

    #[test]
    fn every_header_field_is_read_from_its_own_offset() {
        let mut data = header_bytes(32);
        data.extend([0xee; 9]); // bytes after the 32-byte header are not part of it
        let header = parse_ivf_header(&data).unwrap();
        assert_eq!(&header.signature, b"DKIF");
        assert_eq!(header.version, 1);
        assert_eq!(header.header_size, 32);
        assert_eq!(&header.fourcc, b"AV01");
        assert_eq!((header.width, header.height), (640, 360));
        assert_eq!(header.framerate_den, 1001);
        assert_eq!(header.framerate_num, 30000);
        assert_eq!(header.frame_count, 7);
    }

    #[test]
    fn header_fields_keep_their_maximum_values() {
        let mut data = header_bytes(0xffff);
        data[12..16].copy_from_slice(&[0xff; 4]);
        data[16..28].copy_from_slice(&[0xff; 12]);
        let header = parse_ivf_header(&data).unwrap();
        assert_eq!(header.header_size, u16::MAX);
        assert_eq!((header.width, header.height), (u16::MAX, u16::MAX));
        assert_eq!(header.framerate_den, u32::MAX);
        assert_eq!(header.framerate_num, u32::MAX);
        assert_eq!(header.frame_count, u32::MAX);
    }

    #[test]
    fn every_truncation_of_the_header_reports_how_much_was_needed() {
        let data = header_bytes(32);
        for len in 0..32 {
            match parse_ivf_header(&data[..len]) {
                Err(BitvueError::InsufficientData { needed, available }) => {
                    assert_eq!((needed, available), (32, len), "prefix {len}");
                }
                other => panic!("prefix {len}: {other:?}"),
            }
        }
        assert!(parse_ivf_header(&data).is_ok());
    }

    #[test]
    fn a_flipped_bit_in_the_signature_is_rejected() {
        for byte in 0..4 {
            for bit in 0..8 {
                let mut data = header_bytes(32);
                data[byte] ^= 1 << bit;
                assert!(
                    is_invalid_data(parse_ivf_header(&data)),
                    "byte {byte} bit {bit}"
                );
                assert!(
                    is_invalid_data(parse_ivf_frames(&data)),
                    "frames {byte}/{bit}"
                );
            }
        }
        // Lowercase is not a signature either.
        let mut data = header_bytes(32);
        data[0..4].copy_from_slice(b"dkif");
        assert!(is_invalid_data(parse_ivf_header(&data)));
    }

    #[test]
    fn frames_are_returned_in_order_with_their_size_timestamp_and_data() {
        let (header, frames) = parse_ivf_frames(&three_frames()).unwrap();
        assert_eq!(header.frame_count, 7); // taken from the header, not checked against the frames
        assert_eq!(frames.len(), 3);
        assert_eq!((frames[0].size, frames[0].timestamp), (3, 10));
        assert_eq!(frames[0].data, vec![1, 2, 3]);
        assert_eq!((frames[1].size, frames[1].timestamp), (0, 20));
        assert!(frames[1].data.is_empty());
        assert_eq!((frames[2].size, frames[2].timestamp), (5, 30));
        assert_eq!(frames[2].data, vec![4, 5, 6, 7, 8]);
    }

    #[test]
    fn a_header_without_frames_gives_an_empty_list() {
        let (_, frames) = parse_ivf_frames(&header_bytes(32)).unwrap();
        assert!(frames.is_empty());
    }

    #[test]
    fn frames_start_where_the_header_size_field_says() {
        let mut data = header_bytes(40);
        data.extend([0xaa; 8]); // 8 extra header bytes
        push_frame(&mut data, 2, 99, &[9, 8]);
        let (header, frames) = parse_ivf_frames(&data).unwrap();
        assert_eq!(header.header_size, 40);
        assert_eq!(frames.len(), 1);
        assert_eq!(
            (frames[0].timestamp, &frames[0].data[..]),
            (99, &[9u8, 8][..])
        );
        // A header size that points past the end of the data leaves nothing to read.
        let (_, frames) = parse_ivf_frames(&header_bytes(1000)).unwrap();
        assert!(frames.is_empty());
        let (_, frames) = parse_ivf_frames(&header_bytes(u16::MAX)).unwrap();
        assert!(frames.is_empty());
    }

    #[test]
    fn a_partial_frame_header_at_the_end_is_ignored() {
        let complete = three_frames();
        for extra in 1..IVF_FRAME_HEADER_SIZE {
            let mut data = complete.clone();
            data.extend(vec![0x55; extra]);
            let (_, frames) = parse_ivf_frames(&data).unwrap();
            assert_eq!(frames.len(), 3, "{extra} trailing bytes");
        }
    }

    #[test]
    fn a_frame_whose_data_is_cut_short_ends_the_list_and_keeps_the_earlier_frames() {
        let mut data = header_bytes(32);
        push_frame(&mut data, 2, 1, &[1, 2]);
        push_frame(&mut data, 4, 2, &[1, 2, 3]); // one byte missing
        let (_, frames) = parse_ivf_frames(&data).unwrap();
        assert_eq!(frames.len(), 1);
        // With the byte present it is complete (boundary: data ends exactly at the frame end).
        data.push(4);
        let (_, frames) = parse_ivf_frames(&data).unwrap();
        assert_eq!(frames.len(), 2);
        assert_eq!(frames[1].data, vec![1, 2, 3, 4]);
    }

    #[test]
    fn every_prefix_of_a_stream_returns_exactly_the_frames_that_are_complete() {
        let data = three_frames();
        let ends = [32 + 12 + 3, 32 + 12 + 3 + 12, 32 + 12 + 3 + 12 + 12 + 5];
        for len in 32..=data.len() {
            let expected = ends.iter().filter(|&&end| end <= len).count();
            let (_, frames) = parse_ivf_frames(&data[..len]).unwrap();
            assert_eq!(frames.len(), expected, "prefix {len}");
        }
    }

    #[test]
    fn a_frame_size_above_the_limit_is_an_error_and_the_limit_itself_is_not() {
        for size in [IVF_MAX_FRAME_SIZE as u32 + 1, u32::MAX, 0x8000_0000] {
            let mut data = header_bytes(32);
            push_frame(&mut data, size, 0, &[]);
            assert!(is_invalid_data(parse_ivf_frames(&data)), "size {size}");
        }
        // Exactly the limit is allowed; the data is simply not there, so the list ends.
        let mut data = header_bytes(32);
        push_frame(&mut data, IVF_MAX_FRAME_SIZE as u32, 0, &[1, 2, 3]);
        let (_, frames) = parse_ivf_frames(&data).unwrap();
        assert!(frames.is_empty());
    }

    #[test]
    fn an_oversized_frame_after_good_ones_still_fails_the_whole_parse() {
        let mut data = three_frames();
        push_frame(&mut data, u32::MAX, 0, &[]);
        assert!(is_invalid_data(parse_ivf_frames(&data)));
        assert!(is_invalid_data(extract_obu_data(&data)));
    }

    #[test]
    fn a_header_size_of_zero_makes_the_signature_the_first_frame_size_and_fails() {
        // "DKIF" read as a little-endian frame size is far above the limit.
        let mut data = header_bytes(0);
        push_frame(&mut data, 1, 0, &[1]);
        assert!(is_invalid_data(parse_ivf_frames(&data)));
    }

    #[test]
    fn the_public_limits_have_their_documented_values() {
        assert_eq!(IVF_HEADER_SIZE, 32);
        assert_eq!(IVF_FRAME_HEADER_SIZE, 12);
        assert_eq!(IVF_MAX_FRAME_SIZE, 104_857_600);
        assert_eq!(IVF_MAX_FRAME_COUNT, 1_000_000);
        // The frame size limit as a literal, independent of the constant.
        let mut data = header_bytes(32);
        push_frame(&mut data, 104_857_601, 0, &[]);
        assert!(is_invalid_data(parse_ivf_frames(&data)));
        let mut data = header_bytes(32);
        push_frame(&mut data, 104_857_600, 0, &[]);
        assert!(parse_ivf_frames(&data).is_ok());
    }

    #[test]
    fn a_truncated_frame_ends_the_list_even_when_more_than_a_frame_header_of_bytes_remain() {
        // Declared 100 bytes, only 40 present: the leftovers must not be read as further frames.
        let mut data = header_bytes(32);
        push_frame(&mut data, 2, 1, &[1, 2]);
        push_frame(&mut data, 100, 2, &[0; 40]);
        let (_, frames) = parse_ivf_frames(&data).unwrap();
        assert_eq!(frames.len(), 1);
    }

    #[test]
    fn frames_without_a_frame_obu_report_temporal_id_0() {
        for frame in &parse_ivf_frames(&three_frames()).unwrap().1 {
            assert_eq!(frame.temporal_id, 0);
        }
        // A temporal delimiter followed by nothing that identifies a frame.
        let mut data = header_bytes(32);
        push_frame(&mut data, 2, 0, &[0x12, 0x00]);
        assert_eq!(parse_ivf_frames(&data).unwrap().1[0].temporal_id, 0);
    }

    #[test]
    fn the_frame_count_is_capped_at_the_maximum() {
        let mut data = header_bytes(32);
        data.reserve((IVF_MAX_FRAME_COUNT + 1) * IVF_FRAME_HEADER_SIZE);
        for ts in 0..=IVF_MAX_FRAME_COUNT as u64 {
            push_frame(&mut data, 0, ts, &[]);
        }
        let (_, frames) = parse_ivf_frames(&data).unwrap();
        assert_eq!(frames.len(), IVF_MAX_FRAME_COUNT);
        assert_eq!(
            frames[IVF_MAX_FRAME_COUNT - 1].timestamp,
            IVF_MAX_FRAME_COUNT as u64 - 1
        );
    }

    #[test]
    fn corrupting_any_byte_of_a_stream_never_panics() {
        let base = three_frames();
        for pos in 0..base.len() {
            for value in [0x00u8, 0x01, 0x7f, 0x80, 0xff] {
                let mut data = base.clone();
                data[pos] = value;
                let _ = parse_ivf_frames(&data);
                let _ = extract_obu_data(&data);
            }
        }
        let mut state = 0x9e37_79b9_7f4a_7c15u64;
        for _ in 0..3000 {
            let len = {
                state ^= state << 13;
                state ^= state >> 7;
                state ^= state << 17;
                (state % 200) as usize
            };
            let mut data = vec![0u8; len];
            for byte in &mut data {
                state ^= state << 13;
                state ^= state >> 7;
                state ^= state << 17;
                *byte = state as u8;
            }
            if len >= 4 && state.is_multiple_of(2) {
                data[0..4].copy_from_slice(b"DKIF");
            }
            let _ = parse_ivf_frames(&data);
        }
    }

    #[test]
    fn timestamps_keep_all_64_bits_and_only_non_negative_ones_convert_to_pts() {
        let mut data = header_bytes(32);
        push_frame(&mut data, 0, u64::MAX, &[]);
        push_frame(&mut data, 0, i64::MAX as u64, &[]);
        push_frame(&mut data, 0, 5, &[]);
        let (_, frames) = parse_ivf_frames(&data).unwrap();
        assert_eq!(frames[0].timestamp, u64::MAX);
        assert!(is_invalid_data(frames[0].timestamp_pts()));
        assert_eq!(frames[1].timestamp_pts().unwrap().value(), i64::MAX);
        assert_eq!(frames[2].timestamp_pts().unwrap().value(), 5);
    }

    /// An OBU header byte: type, extension flag, has_size flag.
    fn obu_header_byte(obu_type: u8, extension: bool, has_size: bool) -> u8 {
        (obu_type << 3) | ((extension as u8) << 2) | ((has_size as u8) << 1)
    }

    #[test]
    fn the_temporal_id_comes_from_a_leading_frame_or_frame_header_obu() {
        for obu_type in [3u8, 6] {
            let mut data = header_bytes(32);
            let payload = [obu_header_byte(obu_type, true, true), 3 << 5, 1, 0, 0, 0];
            push_frame(&mut data, payload.len() as u32, 0, &payload);
            let (_, frames) = parse_ivf_frames(&data).unwrap();
            assert_eq!(frames[0].temporal_id, 3, "obu type {obu_type}");
        }
        // Without the extension byte there is no temporal id: 0.
        let mut data = header_bytes(32);
        let payload = [obu_header_byte(6, false, true), 1, 0, 0, 0, 0];
        push_frame(&mut data, payload.len() as u32, 0, &payload);
        let (_, frames) = parse_ivf_frames(&data).unwrap();
        assert_eq!(frames[0].temporal_id, 0);
    }

    #[test]
    fn extracted_obu_data_is_the_frame_payloads_back_to_back() {
        assert_eq!(
            extract_obu_data(&three_frames()).unwrap(),
            vec![1, 2, 3, 4, 5, 6, 7, 8]
        );
        assert!(extract_obu_data(&header_bytes(32)).unwrap().is_empty());
        assert!(matches!(
            extract_obu_data(b"DKIF"),
            Err(BitvueError::InsufficientData { .. })
        ));
        let mut bad = header_bytes(32);
        bad[0] = b'X';
        assert!(is_invalid_data(extract_obu_data(&bad)));
    }

    #[test]
    fn the_container_checks_need_the_full_marker_and_the_exact_fourcc() {
        assert!(is_ivf(b"DKIF"));
        assert!(!is_ivf(b"DKI"));
        assert!(!is_ivf(b""));
        assert!(!is_ivf(b"dkif"));
        assert!(!is_ivf(b"XDKIF"));

        assert!(is_av1_ivf(b"DKIF\0\0\0\0AV01"));
        assert!(!is_av1_ivf(b"DKIF\0\0\0\0AV0")); // 11 bytes
        assert!(!is_av1_ivf(b"DKIF\0\0\0\0av01"));
        assert!(!is_av1_ivf(b"DKIF\0\0\0\0AV02"));
        assert!(!is_av1_ivf(b"DKIX\0\0\0\0AV01"));
        assert!(!is_av1_ivf(b"XKIF\0\0\0\0AV01"));
        assert!(!is_av1_ivf(b"DKIF"));
        // The FourCC must sit at offset 8.
        assert!(!is_av1_ivf(b"DKIFAV01\0\0\0\0"));
    }

    #[test]
    fn the_builder_requires_size_timestamp_and_temporal_id_and_defaults_the_data() {
        let all = || IvfFrame::builder().size(4).timestamp(9).temporal_id(2);
        let frame = all().build().unwrap();
        assert_eq!((frame.size, frame.timestamp, frame.temporal_id), (4, 9, 2));
        assert!(frame.data.is_empty());
        assert_eq!(all().data(vec![1, 2]).build().unwrap().data, vec![1, 2]);

        for (builder, field) in [
            (IvfFrame::builder().timestamp(9).temporal_id(2), "size"),
            (IvfFrame::builder().size(4).temporal_id(2), "timestamp"),
            (IvfFrame::builder().size(4).timestamp(9), "temporal_id"),
        ] {
            match builder.build() {
                Err(BitvueError::InvalidData(message)) => {
                    assert_eq!(message, format!("IvfFrame: {field} is required"));
                }
                other => panic!("{field}: {other:?}"),
            }
        }
    }

    #[test]
    fn the_builder_accepts_a_typed_timestamp() {
        let pts = crate::TimestampPts::new(1234).unwrap();
        let frame = IvfFrame::builder()
            .size(0)
            .timestamp_pts(pts)
            .temporal_id(0)
            .build()
            .unwrap();
        assert_eq!(frame.timestamp, 1234);
    }
}
