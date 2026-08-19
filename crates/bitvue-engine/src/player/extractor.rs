//! Player Extractor API - viz_core.002
//!
//! Trait/API + codec adapters for extracting visualization data from bitstreams.
//!
//! Per FRAME_IDENTITY_CONTRACT:
//! - All extraction results indexed by display_idx (PTS order)
//! - decode_idx is internal implementation detail only
//! - Frame mapping joins display/decode order deterministically

use crate::BitvueError;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Codec identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Codec {
    AV1,
    H264,
    HEVC,
    VP9,
    VVC,
    AVS3,
}

impl Codec {
    /// Get codec name
    pub fn name(&self) -> &'static str {
        match self {
            Codec::AV1 => "AV1",
            Codec::H264 => "H.264",
            Codec::HEVC => "HEVC",
            Codec::VP9 => "VP9",
            Codec::VVC => "VVC",
            Codec::AVS3 => "AVS3",
        }
    }

    /// Check if codec is supported for extraction
    pub fn is_supported(&self) -> bool {
        matches!(self, Codec::AV1)
    }
}

/// Extracted frame metadata
///
/// FRAME_IDENTITY_CONTRACT: indexed by display_idx
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedFrame {
    /// Display index (PTS-sorted order) - PRIMARY identifier
    pub display_idx: usize,

    /// Presentation timestamp
    pub pts: u64,

    /// Decode timestamp (internal only, not exposed in UI)
    pub(crate) dts: u64,

    /// Frame type
    pub frame_type: String,

    /// Frame size in bytes
    pub size_bytes: usize,

    /// Bit offset in bitstream
    pub bit_offset: u64,

    /// QP values per block (if available)
    pub qp_map: Option<Vec<u8>>,

    /// Motion vectors per block (if available)
    pub motion_vectors: Option<Vec<MotionVector>>,

    /// Partition structure (if available)
    pub partitions: Option<Vec<Partition>>,
}

impl ExtractedFrame {
    /// Create new extracted frame
    pub fn new(display_idx: usize, pts: u64, dts: u64, frame_type: String) -> Self {
        Self {
            display_idx,
            pts,
            dts,
            frame_type,
            size_bytes: 0,
            bit_offset: 0,
            qp_map: None,
            motion_vectors: None,
            partitions: None,
        }
    }

    /// Set size in bytes (builder pattern)
    pub fn with_size(mut self, size_bytes: usize) -> Self {
        self.size_bytes = size_bytes;
        self
    }

    /// Set bit offset (builder pattern)
    pub fn with_bit_offset(mut self, bit_offset: u64) -> Self {
        self.bit_offset = bit_offset;
        self
    }

    /// Set QP map (builder pattern)
    pub fn with_qp_map(mut self, qp_map: Vec<u8>) -> Self {
        self.qp_map = Some(qp_map);
        self
    }

    /// Set motion vectors (builder pattern)
    pub fn with_motion_vectors(mut self, motion_vectors: Vec<MotionVector>) -> Self {
        self.motion_vectors = Some(motion_vectors);
        self
    }

    /// Set partitions (builder pattern)
    pub fn with_partitions(mut self, partitions: Vec<Partition>) -> Self {
        self.partitions = Some(partitions);
        self
    }

    /// Get display index (PRIMARY identifier)
    pub fn display_idx(&self) -> usize {
        self.display_idx
    }

    /// Get PTS
    pub fn pts(&self) -> u64 {
        self.pts
    }

    /// Get frame type
    pub fn frame_type(&self) -> &str {
        &self.frame_type
    }
}

/// Motion vector
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct MotionVector {
    /// Block X position
    pub x: u32,

    /// Block Y position
    pub y: u32,

    /// Horizontal motion
    pub mv_x: i16,

    /// Vertical motion
    pub mv_y: i16,

    /// Reference frame index
    pub ref_frame: u8,
}

/// Partition (block structure)
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Partition {
    /// X position
    pub x: u32,

    /// Y position
    pub y: u32,

    /// Width
    pub width: u32,

    /// Height
    pub height: u32,

    /// Partition type
    pub partition_type: PartitionType,
}

/// Partition type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PartitionType {
    /// Square partition
    Square,

    /// Horizontal split
    HorizontalSplit,

    /// Vertical split
    VerticalSplit,

    /// Recursive split
    RecursiveSplit,
}

/// Extraction result
pub type ExtractionResult = Result<Vec<ExtractedFrame>, BitvueError>;

/// Frame extractor trait
///
/// Implementers must ensure:
/// - Results indexed by display_idx (PTS order)
/// - No exposure of decode_idx to public API
/// - Deterministic extraction
pub trait FrameExtractor: Send + Sync {
    /// Get codec for this extractor
    fn codec(&self) -> Codec;

    /// Extract frames from bitstream
    ///
    /// Returns frames in display order (PTS-sorted), indexed by display_idx.
    ///
    /// FRAME_IDENTITY_CONTRACT: display_idx is PRIMARY
    fn extract(&self, data: &[u8]) -> ExtractionResult;

    /// Check if extractor supports a specific feature
    fn supports_qp_extraction(&self) -> bool {
        false
    }

    fn supports_mv_extraction(&self) -> bool {
        false
    }

    fn supports_partition_extraction(&self) -> bool {
        false
    }

    /// Get extractor capabilities
    fn capabilities(&self) -> ExtractorCapabilities {
        ExtractorCapabilities {
            qp_extraction: self.supports_qp_extraction(),
            mv_extraction: self.supports_mv_extraction(),
            partition_extraction: self.supports_partition_extraction(),
        }
    }
}

/// Extractor capabilities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractorCapabilities {
    pub qp_extraction: bool,
    pub mv_extraction: bool,
    pub partition_extraction: bool,
}

// ============================================================================
// Codec Adapters
// ============================================================================

/// AV1 frame extractor
pub struct Av1Extractor {
    /// Extract QP values
    extract_qp: bool,

    /// Extract motion vectors
    extract_mv: bool,

    /// Extract partitions
    extract_partitions: bool,
}

impl Av1Extractor {
    /// Create new AV1 extractor
    pub fn new() -> Self {
        Self {
            extract_qp: false,
            extract_mv: false,
            extract_partitions: false,
        }
    }

    /// Enable QP extraction
    pub fn with_qp(mut self) -> Self {
        self.extract_qp = true;
        self
    }

    /// Enable MV extraction
    pub fn with_mv(mut self) -> Self {
        self.extract_mv = true;
        self
    }

    /// Enable partition extraction
    pub fn with_partitions(mut self) -> Self {
        self.extract_partitions = true;
        self
    }
}

impl Default for Av1Extractor {
    fn default() -> Self {
        Self::new()
    }
}

impl FrameExtractor for Av1Extractor {
    fn codec(&self) -> Codec {
        Codec::AV1
    }

    fn extract(&self, data: &[u8]) -> ExtractionResult {
        // Stub implementation - full implementation in bitvue-av1 crate
        // For now, return empty results

        if data.is_empty() {
            return Err(BitvueError::InvalidData("Empty bitstream data".to_string()));
        }

        // TODO: Actual AV1 OBU parsing and frame extraction
        // This will be implemented using bitvue-av1 crate

        Ok(Vec::new())
    }

    fn supports_qp_extraction(&self) -> bool {
        self.extract_qp
    }

    fn supports_mv_extraction(&self) -> bool {
        self.extract_mv
    }

    fn supports_partition_extraction(&self) -> bool {
        self.extract_partitions
    }
}

/// H.264 frame extractor (stub)
pub struct H264Extractor;

impl H264Extractor {
    pub fn new() -> Self {
        Self
    }
}

impl Default for H264Extractor {
    fn default() -> Self {
        Self::new()
    }
}

impl FrameExtractor for H264Extractor {
    fn codec(&self) -> Codec {
        Codec::H264
    }

    fn extract(&self, _data: &[u8]) -> ExtractionResult {
        Err(BitvueError::UnsupportedCodec(
            "H.264 extraction not yet implemented".to_string(),
        ))
    }
}

/// HEVC frame extractor (stub)
pub struct HevcExtractor;

impl HevcExtractor {
    pub fn new() -> Self {
        Self
    }
}

impl Default for HevcExtractor {
    fn default() -> Self {
        Self::new()
    }
}

impl FrameExtractor for HevcExtractor {
    fn codec(&self) -> Codec {
        Codec::HEVC
    }

    fn extract(&self, _data: &[u8]) -> ExtractionResult {
        Err(BitvueError::UnsupportedCodec(
            "HEVC extraction not yet implemented".to_string(),
        ))
    }
}

/// VP9 frame extractor (stub)
pub struct Vp9Extractor;

impl Vp9Extractor {
    pub fn new() -> Self {
        Self
    }
}

impl Default for Vp9Extractor {
    fn default() -> Self {
        Self::new()
    }
}

impl FrameExtractor for Vp9Extractor {
    fn codec(&self) -> Codec {
        Codec::VP9
    }

    fn extract(&self, _data: &[u8]) -> ExtractionResult {
        Err(BitvueError::UnsupportedCodec(
            "VP9 extraction not yet implemented".to_string(),
        ))
    }
}

/// VVC frame extractor (stub)
pub struct VvcExtractor;

impl VvcExtractor {
    pub fn new() -> Self {
        Self
    }
}

impl Default for VvcExtractor {
    fn default() -> Self {
        Self::new()
    }
}

impl FrameExtractor for VvcExtractor {
    fn codec(&self) -> Codec {
        Codec::VVC
    }

    fn extract(&self, _data: &[u8]) -> ExtractionResult {
        Err(BitvueError::UnsupportedCodec(
            "VVC extraction not yet implemented".to_string(),
        ))
    }
}

/// AVS3 frame extractor (stub)
pub struct Avs3Extractor;

impl Avs3Extractor {
    pub fn new() -> Self {
        Self
    }
}

impl Default for Avs3Extractor {
    fn default() -> Self {
        Self::new()
    }
}

impl FrameExtractor for Avs3Extractor {
    fn codec(&self) -> Codec {
        Codec::AVS3
    }

    fn extract(&self, _data: &[u8]) -> ExtractionResult {
        Err(BitvueError::UnsupportedCodec(
            "AVS3 extraction not yet implemented".to_string(),
        ))
    }
}

// ============================================================================
// Extractor Registry
// ============================================================================

/// Extractor registry for managing codec extractors
pub struct ExtractorRegistry {
    extractors: HashMap<Codec, Box<dyn FrameExtractor>>,
}

impl ExtractorRegistry {
    /// Create new registry
    pub fn new() -> Self {
        Self {
            extractors: HashMap::new(),
        }
    }

    /// Register an extractor
    pub fn register(&mut self, codec: Codec, extractor: Box<dyn FrameExtractor>) {
        self.extractors.insert(codec, extractor);
    }

    /// Get extractor for codec
    pub fn get(&self, codec: Codec) -> Option<&dyn FrameExtractor> {
        self.extractors.get(&codec).map(|e| e.as_ref())
    }

    /// Check if codec is supported
    pub fn supports(&self, codec: Codec) -> bool {
        self.extractors.contains_key(&codec)
    }

    /// Get all supported codecs
    pub fn supported_codecs(&self) -> Vec<Codec> {
        self.extractors.keys().copied().collect()
    }

    /// Create default registry with all codecs
    pub fn with_defaults() -> Self {
        let mut registry = Self::new();

        registry.register(Codec::AV1, Box::new(Av1Extractor::new()));
        registry.register(Codec::H264, Box::new(H264Extractor::new()));
        registry.register(Codec::HEVC, Box::new(HevcExtractor::new()));
        registry.register(Codec::VP9, Box::new(Vp9Extractor::new()));
        registry.register(Codec::VVC, Box::new(VvcExtractor::new()));
        registry.register(Codec::AVS3, Box::new(Avs3Extractor::new()));

        registry
    }
}

impl Default for ExtractorRegistry {
    fn default() -> Self {
        Self::with_defaults()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_codec_names() {
        assert_eq!(Codec::AV1.name(), "AV1");
        assert_eq!(Codec::H264.name(), "H.264");
        assert_eq!(Codec::HEVC.name(), "HEVC");
        assert_eq!(Codec::VP9.name(), "VP9");
        assert_eq!(Codec::VVC.name(), "VVC");
        assert_eq!(Codec::AVS3.name(), "AVS3");
    }

    #[test]
    fn test_codec_support() {
        assert!(Codec::AV1.is_supported());
        assert!(!Codec::H264.is_supported());
        assert!(!Codec::HEVC.is_supported());
    }

    #[test]
    fn test_extracted_frame_creation() {
        let frame = ExtractedFrame::new(5, 100, 100, "I".to_string());

        assert_eq!(frame.display_idx(), 5);
        assert_eq!(frame.pts(), 100);
        assert_eq!(frame.frame_type(), "I");
    }

    #[test]
    fn test_av1_extractor_capabilities() {
        let extractor = Av1Extractor::new();
        assert!(!extractor.supports_qp_extraction());
        assert!(!extractor.supports_mv_extraction());

        let extractor = Av1Extractor::new().with_qp().with_mv();
        assert!(extractor.supports_qp_extraction());
        assert!(extractor.supports_mv_extraction());
    }

    #[test]
    fn test_av1_extractor_empty_data() {
        let extractor = Av1Extractor::new();
        let result = extractor.extract(&[]);
        assert!(result.is_err());
    }

    #[test]
    fn test_unsupported_codec_extractors() {
        let h264 = H264Extractor::new();
        assert!(h264.extract(&[0u8; 100]).is_err());

        let hevc = HevcExtractor::new();
        assert!(hevc.extract(&[0u8; 100]).is_err());

        let vp9 = Vp9Extractor::new();
        assert!(vp9.extract(&[0u8; 100]).is_err());

        let vvc = VvcExtractor::new();
        assert!(vvc.extract(&[0u8; 100]).is_err());

        let avs3 = Avs3Extractor::new();
        assert!(avs3.extract(&[0u8; 100]).is_err());
    }

    #[test]
    fn test_registry_creation() {
        let registry = ExtractorRegistry::new();
        assert!(!registry.supports(Codec::AV1));
    }

    #[test]
    fn test_registry_with_defaults() {
        let registry = ExtractorRegistry::with_defaults();
        assert!(registry.supports(Codec::AV1));
        assert!(registry.supports(Codec::H264));
        assert!(registry.supports(Codec::HEVC));
    }

    #[test]
    fn test_registry_register_get() {
        let mut registry = ExtractorRegistry::new();
        registry.register(Codec::AV1, Box::new(Av1Extractor::new()));

        assert!(registry.supports(Codec::AV1));
        assert!(registry.get(Codec::AV1).is_some());
        assert!(registry.get(Codec::H264).is_none());
    }

    #[test]
    fn test_registry_supported_codecs() {
        let registry = ExtractorRegistry::with_defaults();
        let codecs = registry.supported_codecs();

        assert!(codecs.contains(&Codec::AV1));
        assert!(codecs.contains(&Codec::H264));
        assert_eq!(codecs.len(), 6);
    }

    #[test]
    fn test_frame_identity_contract_display_idx_primary() {
        // Verify that ExtractedFrame uses display_idx as PRIMARY identifier
        let frame = ExtractedFrame::new(3, 99, 33, "B".to_string());

        // display_idx is publicly accessible
        assert_eq!(frame.display_idx, 3);
        assert_eq!(frame.display_idx(), 3);

        // dts is internal only (not in public API)
        // This is enforced by pub(crate) visibility
    }

    #[test]
    fn test_motion_vector_creation() {
        let mv = MotionVector {
            x: 4,
            y: 8,
            mv_x: -16,
            mv_y: 8,
            ref_frame: 1,
        };

        assert_eq!(mv.x, 4);
        assert_eq!(mv.mv_x, -16);
    }

    #[test]
    fn test_partition_types() {
        let partition = Partition {
            x: 0,
            y: 0,
            width: 64,
            height: 64,
            partition_type: PartitionType::Square,
        };

        assert_eq!(partition.partition_type, PartitionType::Square);
    }

    #[test]
    fn test_extractor_capabilities() {
        let extractor = Av1Extractor::new().with_qp().with_mv();
        let caps = extractor.capabilities();

        assert!(caps.qp_extraction);
        assert!(caps.mv_extraction);
        assert!(!caps.partition_extraction);
    }
}
