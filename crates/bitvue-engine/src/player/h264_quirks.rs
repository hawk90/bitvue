//! H264 Quirks - Codec-specific H264 handling
//!
//! Per FRAME_IDENTITY_CONTRACT:
//! - All frame references use display_idx (PRIMARY)
//! - decode_idx is internal only

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// H264 Picture Order Count (POC) type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PocType {
    /// Type 0: POC LSB
    Type0,
    /// Type 1: Cycle-based POC
    Type1,
    /// Type 2: Frame number based
    Type2,
}

/// H264 POC (Picture Order Count) mapping
///
/// Maps POC values to display_idx
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct H264PocMapping {
    /// POC type
    pub poc_type: PocType,

    /// POC to display_idx mapping
    pub poc_to_display: HashMap<i32, usize>,

    /// display_idx to POC mapping
    pub display_to_poc: HashMap<usize, i32>,

    /// Max POC LSB (for type 0)
    pub max_poc_lsb: u32,
}

impl H264PocMapping {
    /// Create new POC mapping
    pub fn new(poc_type: PocType, max_poc_lsb: u32) -> Self {
        Self {
            poc_type,
            poc_to_display: HashMap::new(),
            display_to_poc: HashMap::new(),
            max_poc_lsb,
        }
    }

    /// Register frame with POC
    pub fn register_frame(&mut self, poc: i32, display_idx: usize) {
        self.poc_to_display.insert(poc, display_idx);
        self.display_to_poc.insert(display_idx, poc);
    }

    /// Get display_idx from POC
    pub fn get_display_idx(&self, poc: i32) -> Option<usize> {
        self.poc_to_display.get(&poc).copied()
    }

    /// Get POC from display_idx
    pub fn get_poc(&self, display_idx: usize) -> Option<i32> {
        self.display_to_poc.get(&display_idx).copied()
    }
}

/// H264 frame type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum H264FrameType {
    /// IDR frame (Instantaneous Decoder Refresh)
    Idr,
    /// Regular I-frame (intra)
    I,
    /// P-frame (predictive)
    P,
    /// B-frame (bidirectional)
    B,
}

/// H264 reference frame info
///
/// FRAME_IDENTITY_CONTRACT: indexed by display_idx
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct H264ReferenceFrame {
    /// Display index - PRIMARY identifier
    pub display_idx: usize,

    /// Frame type
    pub frame_type: H264FrameType,

    /// Is long-term reference
    pub is_long_term: bool,

    /// Long-term reference index (if long-term)
    pub long_term_idx: Option<u8>,

    /// Reference marking
    pub marking: ReferenceMarking,
}

/// Reference frame marking
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReferenceMarking {
    /// Short-term reference
    ShortTerm,
    /// Long-term reference
    LongTerm,
    /// Unused for reference
    Unused,
}

impl H264ReferenceFrame {
    /// Create new reference frame
    pub fn new(display_idx: usize, frame_type: H264FrameType) -> Self {
        Self {
            display_idx,
            frame_type,
            is_long_term: false,
            long_term_idx: None,
            marking: ReferenceMarking::ShortTerm,
        }
    }

    /// Mark as long-term reference
    pub fn mark_long_term(&mut self, long_term_idx: u8) {
        self.is_long_term = true;
        self.long_term_idx = Some(long_term_idx);
        self.marking = ReferenceMarking::LongTerm;
    }

    /// Mark as unused
    pub fn mark_unused(&mut self) {
        self.marking = ReferenceMarking::Unused;
    }

    /// Get display index
    pub fn display_idx(&self) -> usize {
        self.display_idx
    }
}

/// H264 MMCO (Memory Management Control Operation)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MmcoOperation {
    /// End MMCO commands
    End,
    /// Mark short-term ref as unused
    MarkShortTermUnused { frame_num_diff: u32 },
    /// Mark long-term ref as unused
    MarkLongTermUnused { long_term_idx: u8 },
    /// Mark short-term as long-term
    MarkAsLongTerm { long_term_idx: u8 },
    /// Set max long-term index
    SetMaxLongTermIdx { max_idx: u8 },
    /// Mark all ref frames unused
    MarkAllUnused,
    /// Mark current as long-term
    MarkCurrentLongTerm { long_term_idx: u8 },
}

/// H264 slice group (FMO)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct H264SliceGroup {
    /// Slice group map type
    pub map_type: u8,

    /// Number of slice groups
    pub num_groups: u8,

    /// Run length (for map type 0)
    pub run_length: Option<Vec<u32>>,
}

impl H264SliceGroup {
    /// Create new slice group config
    pub fn new(map_type: u8, num_groups: u8) -> Self {
        Self {
            map_type,
            num_groups,
            run_length: None,
        }
    }
}

/// H264 SPS (Sequence Parameter Set) info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct H264SpsInfo {
    /// POC type
    pub poc_type: PocType,

    /// Max frame num
    pub max_frame_num: u32,

    /// Max POC LSB (for POC type 0)
    pub max_poc_lsb: u32,

    /// Number of reference frames
    pub num_ref_frames: u8,

    /// Frame mbs only flag (progressive vs interlaced)
    pub frame_mbs_only: bool,
}

impl H264SpsInfo {
    /// Create new SPS info
    pub fn new(
        poc_type: PocType,
        max_frame_num: u32,
        max_poc_lsb: u32,
        num_ref_frames: u8,
        frame_mbs_only: bool,
    ) -> Self {
        Self {
            poc_type,
            max_frame_num,
            max_poc_lsb,
            num_ref_frames,
            frame_mbs_only,
        }
    }
}

/// H264 quirks handler
///
/// Manages H264-specific features:
/// - POC (Picture Order Count) mapping
/// - Reference frame management (MMCO)
/// - IDR vs regular I-frames
/// - Slice groups (FMO)
/// - SVC temporal layers
///
/// FRAME_IDENTITY_CONTRACT:
/// - All frame references use display_idx
/// - POC mapping: POC → display_idx
/// - Reference frames indexed by display_idx
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct H264Quirks {
    /// SPS information
    pub sps_info: H264SpsInfo,

    /// POC mapping
    pub poc_mapping: H264PocMapping,

    /// Reference frame buffer (indexed by display_idx)
    pub reference_frames: HashMap<usize, H264ReferenceFrame>,

    /// IDR frame indices (display_idx)
    pub idr_frames: Vec<usize>,

    /// Recovery point indices (display_idx)
    pub recovery_points: Vec<usize>,

    /// Slice group configuration
    pub slice_groups: Option<H264SliceGroup>,

    /// SVC temporal layer mapping (display_idx → layer)
    pub temporal_layers: HashMap<usize, u8>,
}

impl H264Quirks {
    /// Create new H264 quirks handler
    pub fn new(sps_info: H264SpsInfo) -> Self {
        let poc_mapping = H264PocMapping::new(sps_info.poc_type, sps_info.max_poc_lsb);

        Self {
            sps_info,
            poc_mapping,
            reference_frames: HashMap::new(),
            idr_frames: Vec::new(),
            recovery_points: Vec::new(),
            slice_groups: None,
            temporal_layers: HashMap::new(),
        }
    }

    /// Register frame with POC
    pub fn register_frame(&mut self, display_idx: usize, poc: i32, frame_type: H264FrameType) {
        self.poc_mapping.register_frame(poc, display_idx);

        // Register as reference if I or P frame
        if matches!(
            frame_type,
            H264FrameType::Idr | H264FrameType::I | H264FrameType::P
        ) {
            let ref_frame = H264ReferenceFrame::new(display_idx, frame_type);
            self.reference_frames.insert(display_idx, ref_frame);
        }

        // Track IDR frames
        if frame_type == H264FrameType::Idr {
            self.idr_frames.push(display_idx);
        }
    }

    /// Get display_idx from POC
    pub fn get_display_idx_from_poc(&self, poc: i32) -> Option<usize> {
        self.poc_mapping.get_display_idx(poc)
    }

    /// Get POC from display_idx
    pub fn get_poc_from_display_idx(&self, display_idx: usize) -> Option<i32> {
        self.poc_mapping.get_poc(display_idx)
    }

    /// Get reference frame by display_idx
    pub fn get_reference_frame(&self, display_idx: usize) -> Option<&H264ReferenceFrame> {
        self.reference_frames.get(&display_idx)
    }

    /// Mark frame as long-term reference
    pub fn mark_long_term_reference(&mut self, display_idx: usize, long_term_idx: u8) {
        if let Some(ref_frame) = self.reference_frames.get_mut(&display_idx) {
            ref_frame.mark_long_term(long_term_idx);
        }
    }

    /// Mark frame as unused for reference
    pub fn mark_unused(&mut self, display_idx: usize) {
        if let Some(ref_frame) = self.reference_frames.get_mut(&display_idx) {
            ref_frame.mark_unused();
        }
    }

    /// Register IDR frame
    pub fn register_idr(&mut self, display_idx: usize) {
        if !self.idr_frames.contains(&display_idx) {
            self.idr_frames.push(display_idx);
        }
    }

    /// Check if frame is IDR
    pub fn is_idr_frame(&self, display_idx: usize) -> bool {
        self.idr_frames.contains(&display_idx)
    }

    /// Register recovery point
    pub fn register_recovery_point(&mut self, display_idx: usize) {
        if !self.recovery_points.contains(&display_idx) {
            self.recovery_points.push(display_idx);
        }
    }

    /// Check if frame is recovery point
    pub fn is_recovery_point(&self, display_idx: usize) -> bool {
        self.recovery_points.contains(&display_idx)
    }

    /// Set slice group configuration
    pub fn set_slice_groups(&mut self, slice_groups: H264SliceGroup) {
        self.slice_groups = Some(slice_groups);
    }

    /// Register SVC temporal layer
    pub fn register_temporal_layer(&mut self, display_idx: usize, layer: u8) {
        self.temporal_layers.insert(display_idx, layer);
    }

    /// Get temporal layer for frame
    pub fn get_temporal_layer(&self, display_idx: usize) -> Option<u8> {
        self.temporal_layers.get(&display_idx).copied()
    }

    /// Get all reference frames
    pub fn reference_frames(&self) -> Vec<&H264ReferenceFrame> {
        self.reference_frames.values().collect()
    }

    /// Get IDR frames
    pub fn idr_frames(&self) -> &[usize] {
        &self.idr_frames
    }

    /// Get recovery points
    pub fn recovery_points(&self) -> &[usize] {
        &self.recovery_points
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_poc_mapping_basic() {
        let mut poc_mapping = H264PocMapping::new(PocType::Type0, 256);

        poc_mapping.register_frame(0, 0);
        poc_mapping.register_frame(1, 1);
        poc_mapping.register_frame(3, 2); // display_idx=2, POC=3 (reordering)

        assert_eq!(poc_mapping.get_display_idx(0), Some(0));
        assert_eq!(poc_mapping.get_display_idx(1), Some(1));
        assert_eq!(poc_mapping.get_display_idx(3), Some(2));

        assert_eq!(poc_mapping.get_poc(0), Some(0));
        assert_eq!(poc_mapping.get_poc(1), Some(1));
        assert_eq!(poc_mapping.get_poc(2), Some(3));
    }

    #[test]
    fn test_h264_quirks_register_frame() {
        let sps_info = H264SpsInfo::new(PocType::Type0, 1024, 256, 4, true);
        let mut quirks = H264Quirks::new(sps_info);

        quirks.register_frame(0, 0, H264FrameType::Idr);
        quirks.register_frame(1, 2, H264FrameType::P);
        quirks.register_frame(2, 1, H264FrameType::B);

        // Verify POC mapping
        assert_eq!(quirks.get_display_idx_from_poc(0), Some(0));
        assert_eq!(quirks.get_display_idx_from_poc(2), Some(1));
        assert_eq!(quirks.get_display_idx_from_poc(1), Some(2));

        // Verify reference frames (IDR and P)
        assert!(quirks.get_reference_frame(0).is_some());
        assert!(quirks.get_reference_frame(1).is_some());
        assert!(quirks.get_reference_frame(2).is_none()); // B-frame not in ref buffer

        // Verify IDR tracking
        assert!(quirks.is_idr_frame(0));
        assert!(!quirks.is_idr_frame(1));
    }

    #[test]
    fn test_long_term_reference() {
        let sps_info = H264SpsInfo::new(PocType::Type0, 1024, 256, 4, true);
        let mut quirks = H264Quirks::new(sps_info);

        quirks.register_frame(0, 0, H264FrameType::Idr);
        quirks.mark_long_term_reference(0, 0);

        let ref_frame = quirks.get_reference_frame(0).unwrap();
        assert!(ref_frame.is_long_term);
        assert_eq!(ref_frame.long_term_idx, Some(0));
    }

    #[test]
    fn test_recovery_points() {
        let sps_info = H264SpsInfo::new(PocType::Type0, 1024, 256, 4, true);
        let mut quirks = H264Quirks::new(sps_info);

        quirks.register_recovery_point(5);
        quirks.register_recovery_point(10);

        assert!(!quirks.is_recovery_point(0));
        assert!(quirks.is_recovery_point(5));
        assert!(quirks.is_recovery_point(10));
    }

    #[test]
    fn test_temporal_layers() {
        let sps_info = H264SpsInfo::new(PocType::Type0, 1024, 256, 4, true);
        let mut quirks = H264Quirks::new(sps_info);

        quirks.register_temporal_layer(0, 0); // T0
        quirks.register_temporal_layer(1, 0); // T0
        quirks.register_temporal_layer(2, 1); // T1

        assert_eq!(quirks.get_temporal_layer(0), Some(0));
        assert_eq!(quirks.get_temporal_layer(1), Some(0));
        assert_eq!(quirks.get_temporal_layer(2), Some(1));
    }
}
