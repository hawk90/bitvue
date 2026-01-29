//! IVF container parser
//!
//! IVF is a simple container format used for storing AV1 test vectors.
//! Structure:
//! - 32-byte header
//! - Frame entries: 4-byte size + 8-byte timestamp + frame data

use crate::bitreader::BitReader;
use crate::obu::parse_obu_header;
use bitvue_core::BitvueError;

/// IVF file header size in bytes
pub const IVF_HEADER_SIZE: usize = 32;

/// IVF frame header size in bytes (size + timestamp)
pub const IVF_FRAME_HEADER_SIZE: usize = 12;

/// Maximum valid IVF frame size (100 MB)
pub const IVF_MAX_FRAME_SIZE: usize = 100 * 1024 * 1024;

/// Default block size for QP/MV overlay grids
pub const OVERLAY_BLOCK_SIZE: u32 = 64;

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
}

/// Builder for constructing IvfFrame instances
///
/// # Example
///
/// ```
/// use bitvue_av1::ivf::IvfFrame;
///
/// let frame = IvfFrame::builder()
///     .size(1024)
///     .timestamp(0)
///     .data(vec![0x00, 0x00, 0x01])
///     .temporal_id(0)
///     .build();
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
    pub fn timestamp(mut self, value: u64) -> Self {
        self.timestamp = Some(value);
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
    /// # Panics
    ///
    /// Panics if required fields (size, timestamp, temporal_id) are not set.
    pub fn build(self) -> IvfFrame {
        IvfFrame {
            size: self.size.expect("size is required"),
            timestamp: self.timestamp.expect("timestamp is required"),
            data: self.data.unwrap_or_default(),
            temporal_id: self.temporal_id.expect("temporal_id is required"),
        }
    }
}

/// Parse IVF header from data
pub fn parse_ivf_header(data: &[u8]) -> Result<IvfHeader, BitvueError> {
    if data.len() < 32 {
        return Err(BitvueError::InsufficientData {
            needed: 32,
            available: data.len(),
        });
    }

    let signature: [u8; 4] = data[0..4].try_into().unwrap();
    if &signature != b"DKIF" {
        return Err(BitvueError::InvalidData(format!(
            "Invalid IVF signature: {:?}",
            signature
        )));
    }

    let version = u16::from_le_bytes([data[4], data[5]]);
    let header_size = u16::from_le_bytes([data[6], data[7]]);
    let fourcc: [u8; 4] = data[8..12].try_into().unwrap();
    let width = u16::from_le_bytes([data[12], data[13]]);
    let height = u16::from_le_bytes([data[14], data[15]]);
    let framerate_den = u32::from_le_bytes([data[16], data[17], data[18], data[19]]);
    let framerate_num = u32::from_le_bytes([data[20], data[21], data[22], data[23]]);
    let frame_count = u32::from_le_bytes([data[24], data[25], data[26], data[27]]);

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

    while offset + 12 <= data.len() {
        let frame_size = u32::from_le_bytes([
            data[offset],
            data[offset + 1],
            data[offset + 2],
            data[offset + 3],
        ]);
        let timestamp = u64::from_le_bytes([
            data[offset + 4],
            data[offset + 5],
            data[offset + 6],
            data[offset + 7],
            data[offset + 8],
            data[offset + 9],
            data[offset + 10],
            data[offset + 11],
        ]);

        offset += 12;

        if offset + frame_size as usize > data.len() {
            break;
        }

        let frame_data = data[offset..offset + frame_size as usize].to_vec();

        // Extract temporal_id from frame header OBU
        let temporal_id = extract_temporal_id_from_frame(&frame_data).unwrap_or(0);

        frames.push(IvfFrame {
            size: frame_size,
            timestamp,
            data: frame_data,
            temporal_id,
        });

        offset += frame_size as usize;
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
}
