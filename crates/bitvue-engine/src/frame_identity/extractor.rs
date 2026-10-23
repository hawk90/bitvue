//! Frame identity extractor API and codec implementations (viz_core.002).
//!
//! Each codec implements [`FrameIdentityExtractor`] to pull PTS/DTS metadata out of its
//! bitstream; [`FrameMapper`] combines an extractor with the resulting [`super::FrameIndexMap`]
//! for a one-call build pipeline.

use super::{FrameIndexMap, FrameMetadata};

/// Frame identity extractor trait for codec-specific frame metadata extraction
///
/// Deliverable: extract_api:FrameIdentity:Core:AV1:viz_core
///
/// Each codec implements this trait to extract PTS/DTS from its bitstream.
/// The extractor provides codec-specific logic while maintaining the
/// FRAME_IDENTITY_CONTRACT (display_idx vs decode_idx).
pub trait FrameIdentityExtractor {
    /// Extract frame metadata from raw bitstream data
    ///
    /// Returns frame metadata in decode order.
    /// The FrameIndexMap will sort these into display order.
    fn extract_frames(&self, data: &[u8]) -> Result<Vec<FrameMetadata>, ExtractionError>;

    /// Get codec name for this extractor
    fn codec_name(&self) -> &'static str;

    /// Check if this codec supports PTS/DTS extraction
    fn supports_timestamps(&self) -> bool {
        true
    }
}

/// Error type for frame identity extraction
#[derive(Debug, Clone)]
pub enum ExtractionError {
    /// Invalid bitstream format
    InvalidFormat(String),
    /// Codec parsing failed
    ParseError(String),
    /// Unsupported codec feature
    Unsupported(String),
    /// I/O error
    IoError(String),
}

impl std::fmt::Display for ExtractionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExtractionError::InvalidFormat(msg) => write!(f, "Invalid format: {}", msg),
            ExtractionError::ParseError(msg) => write!(f, "Parse error: {}", msg),
            ExtractionError::Unsupported(msg) => write!(f, "Unsupported: {}", msg),
            ExtractionError::IoError(msg) => write!(f, "I/O error: {}", msg),
        }
    }
}

impl std::error::Error for ExtractionError {}

// ============================================================================
// AV1 Frame Identity Extractor
// ============================================================================

/// AV1 frame identity extractor
///
/// Extracts frame metadata (PTS/DTS) from AV1 bitstream.
/// Supports:
/// - IVF container (with PTS)
/// - Raw OBU stream (no container timestamps)
/// - ISOBMFF/Matroska (future)
pub struct Av1FrameIdentityExtractor {
    /// Whether to expect container timestamps
    pub has_container: bool,
}

impl Av1FrameIdentityExtractor {
    /// Create new AV1 extractor
    pub fn new() -> Self {
        Self {
            has_container: false,
        }
    }

    /// Create AV1 extractor expecting container timestamps
    pub fn with_container() -> Self {
        Self {
            has_container: true,
        }
    }
}

impl Default for Av1FrameIdentityExtractor {
    fn default() -> Self {
        Self::new()
    }
}

impl FrameIdentityExtractor for Av1FrameIdentityExtractor {
    /// Extract frame metadata from a raw AV1 OBU stream.
    ///
    /// Scans OBUs to count displayable frames.  Because a raw AV1 stream
    /// carries no external PTS/DTS (those come from a container), each
    /// discovered frame is given `pts = None` and `dts = None`.
    ///
    /// When `has_container = true` the caller is expected to merge container
    /// timestamps into the returned metadata after the fact; this method still
    /// returns `None` timestamps since the raw OBU bytes contain none.
    ///
    /// A "frame" is counted when an OBU of type Frame (6) or FrameHeader (3)
    /// is encountered and the parsed `show_frame` flag is set, **or** when
    /// a `show_existing_frame` OBU is found (also displayable).
    ///
    /// If the bitstream cannot be parsed at all an empty vec is returned
    /// rather than an error, per the resilient-parsing contract.
    fn extract_frames(&self, data: &[u8]) -> Result<Vec<FrameMetadata>, ExtractionError> {
        // Minimal inline OBU scanner — avoids a circular dependency on
        // bitvue-av1-codec by duplicating only the OBU framing parse.
        let mut frames: Vec<FrameMetadata> = Vec::new();
        let mut offset = 0usize;

        while offset < data.len() {
            let slice = &data[offset..];
            let obu = match read_obu(slice) {
                ObuRead::Obu(obu) => obu,
                // Skip one byte and try to resync
                ObuRead::Resync => {
                    offset += 1;
                    continue;
                }
                ObuRead::Stop => break,
            };

            // ── Detect displayable frames ──────────────────────────────────
            // OBU types: 3 = FRAME_HEADER, 6 = FRAME
            if obu.obu_type == 3 || obu.obu_type == 6 {
                let payload = &slice[obu.payload_start..obu.payload_start + obu.payload_size];
                if let Some(true) = parse_av1_show_frame_flag(payload) {
                    frames.push(FrameMetadata {
                        pts: None,
                        dts: None,
                    });
                }
            }

            offset += obu.total_bytes();
        }

        Ok(frames)
    }

    fn codec_name(&self) -> &'static str {
        "AV1"
    }
}

/// Where one OBU lies in a byte slice that starts at its header.
struct ObuSpan {
    obu_type: u8,
    payload_start: usize,
    payload_size: usize,
}

impl ObuSpan {
    fn total_bytes(&self) -> usize {
        self.payload_start + self.payload_size
    }
}

/// What the OBU at the start of a slice is.
enum ObuRead {
    Obu(ObuSpan),
    /// The header is invalid (forbidden bit set): drop one byte and look again.
    Resync,
    /// The data ends inside the OBU: nothing more can be read.
    Stop,
}

/// Reads the OBU header and size field at the start of `slice` (spec 5.3): the type, the optional
/// extension byte, and a LEB128 size when the header has one (otherwise the OBU takes the rest of
/// the slice).
fn read_obu(slice: &[u8]) -> ObuRead {
    let Some(&byte0) = slice.first() else {
        return ObuRead::Stop;
    };
    // forbidden_bit (bit 7) must be 0
    if byte0 & 0x80 != 0 {
        return ObuRead::Resync;
    }
    let obu_type = (byte0 >> 3) & 0x0F; // bits 6-3
    let has_extension = (byte0 >> 2) & 0x01 != 0;
    let has_size = (byte0 >> 1) & 0x01 != 0;

    let header_bytes = if has_extension { 2usize } else { 1usize };
    if slice.len() < header_bytes {
        return ObuRead::Stop;
    }

    let (payload_start, payload_size) = if has_size {
        let (size, end) = read_leb128(slice, header_bytes);
        (end, size as usize)
    } else {
        (header_bytes, slice.len().saturating_sub(header_bytes))
    };

    if payload_start + payload_size > slice.len() {
        // Truncated OBU — stop
        return ObuRead::Stop;
    }
    ObuRead::Obu(ObuSpan {
        obu_type,
        payload_start,
        payload_size,
    })
}

/// The LEB128 value that starts at `start` and the offset just past it. Reads at most 8 bytes and
/// stops early at the end of `slice`; an unterminated or cut-off value is returned as read so far.
fn read_leb128(slice: &[u8], start: usize) -> (u64, usize) {
    let mut value = 0u64;
    let mut shift = 0u32;
    let mut pos = start;
    while pos < slice.len() && pos - start < 8 {
        let b = slice[pos];
        pos += 1;
        value |= u64::from(b & 0x7F) << shift;
        shift += 7;
        if b & 0x80 == 0 {
            break;
        }
    }
    (value, pos)
}

/// Minimal AV1 frame-header parser that extracts only the `show_frame` flag.
///
/// Returns `Some(true)` when the frame is displayable (show_frame=1 or
/// show_existing_frame=1), `Some(false)` when it is not, and `None` when
/// the payload is too short to parse.
fn parse_av1_show_frame_flag(payload: &[u8]) -> Option<bool> {
    if payload.is_empty() {
        return None;
    }
    let byte0 = payload[0];

    // show_existing_frame is bit 7 of the first byte
    let show_existing = (byte0 & 0x80) != 0;
    if show_existing {
        return Some(true);
    }

    // frame_type occupies bits 6-5, show_frame is bit 4
    let show_frame = (byte0 & 0x10) != 0;
    Some(show_frame)
}

// ============================================================================
// Codec Extractor Stubs (unsupported codecs)
// ============================================================================

/// H.264 frame identity extractor (stub)
pub struct H264FrameIdentityExtractor;

impl FrameIdentityExtractor for H264FrameIdentityExtractor {
    fn extract_frames(&self, _data: &[u8]) -> Result<Vec<FrameMetadata>, ExtractionError> {
        Err(ExtractionError::Unsupported(
            "H.264 frame extraction not yet implemented".to_string(),
        ))
    }

    fn codec_name(&self) -> &'static str {
        "H.264"
    }

    fn supports_timestamps(&self) -> bool {
        false
    }
}

/// HEVC frame identity extractor (stub)
pub struct HevcFrameIdentityExtractor;

impl FrameIdentityExtractor for HevcFrameIdentityExtractor {
    fn extract_frames(&self, _data: &[u8]) -> Result<Vec<FrameMetadata>, ExtractionError> {
        Err(ExtractionError::Unsupported(
            "HEVC frame extraction not yet implemented".to_string(),
        ))
    }

    fn codec_name(&self) -> &'static str {
        "HEVC"
    }

    fn supports_timestamps(&self) -> bool {
        false
    }
}

/// VP9 frame identity extractor (stub)
pub struct Vp9FrameIdentityExtractor;

impl FrameIdentityExtractor for Vp9FrameIdentityExtractor {
    fn extract_frames(&self, _data: &[u8]) -> Result<Vec<FrameMetadata>, ExtractionError> {
        Err(ExtractionError::Unsupported(
            "VP9 frame extraction not yet implemented".to_string(),
        ))
    }

    fn codec_name(&self) -> &'static str {
        "VP9"
    }

    fn supports_timestamps(&self) -> bool {
        false
    }
}

// ============================================================================
// T0-1: Frame Mapping Join (viz_core.003)
// ============================================================================

/// Build FrameIndexMap from extractor and raw data
///
/// Deliverable: frame_map:FrameIdentity:Core:AV1:viz_core
///
/// This function binds extracted units to display_idx/decode_idx and PTS/DTS mapping.
/// Per FRAME_IDENTITY_CONTRACT:
/// - Primary timeline index = display_idx (PTS order)
/// - decode_idx is internal only
pub fn build_frame_map_from_extractor(
    extractor: &dyn FrameIdentityExtractor,
    data: &[u8],
) -> Result<FrameIndexMap, ExtractionError> {
    // Extract frame metadata in decode order
    let frames = extractor.extract_frames(data)?;

    // Build FrameIndexMap (sorts into display order)
    Ok(FrameIndexMap::new(&frames))
}

/// Frame mapper - combines extractor and index map
///
/// High-level API for frame identity resolution.
/// Handles codec detection, extraction, and mapping in one step.
pub struct FrameMapper {
    extractor: Box<dyn FrameIdentityExtractor>,
    index_map: Option<FrameIndexMap>,
}

impl FrameMapper {
    /// Create new mapper with specific extractor
    pub fn new(extractor: Box<dyn FrameIdentityExtractor>) -> Self {
        Self {
            extractor,
            index_map: None,
        }
    }

    /// Create AV1 mapper
    pub fn for_av1() -> Self {
        Self::new(Box::new(Av1FrameIdentityExtractor::new()))
    }

    /// Create H.264 mapper
    pub fn for_h264() -> Self {
        Self::new(Box::new(H264FrameIdentityExtractor))
    }

    /// Create HEVC mapper
    pub fn for_hevc() -> Self {
        Self::new(Box::new(HevcFrameIdentityExtractor))
    }

    /// Create VP9 mapper
    pub fn for_vp9() -> Self {
        Self::new(Box::new(Vp9FrameIdentityExtractor))
    }

    /// Build frame index map from data
    pub fn build(&mut self, data: &[u8]) -> Result<&FrameIndexMap, ExtractionError> {
        let map = build_frame_map_from_extractor(self.extractor.as_ref(), data)?;
        self.index_map = Some(map);
        Ok(self.index_map.as_ref().unwrap())
    }

    /// Get the built index map
    pub fn index_map(&self) -> Option<&FrameIndexMap> {
        self.index_map.as_ref()
    }

    /// Get codec name
    pub fn codec_name(&self) -> &'static str {
        self.extractor.codec_name()
    }
}

#[cfg(test)]
#[path = "av1_extractor_test.rs"]
mod av1_extractor_test;
