//! Frame Service - Frame information aggregation
//!
//! Collects and formats frame data from StreamState for frontend display.
//!
//! Refactored from god object (17 fields) to focused value objects.

use bitvue_core::UnitModel;
use serde::{Deserialize, Serialize};

/// Reference slot information for codec-specific reference naming
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct ReferenceSlotInfo {
    pub index: u8,
    pub name: String,
    pub frame_index: Option<usize>,
}

/// Frame position - temporal and position data
///
/// Groups frame_index, poc, pts, dts, temporal_id
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct FramePosition {
    /// Frame index in sequence
    pub frame_index: usize,
    /// Picture order count
    pub poc: i32,
    /// Presentation timestamp
    pub pts: Option<u64>,
    /// Decode timestamp
    pub dts: Option<u64>,
    /// Temporal layer ID
    pub temporal_id: Option<u8>,
}

#[allow(dead_code)]
impl FramePosition {
    /// Create from individual components
    pub fn new(
        frame_index: usize,
        poc: i32,
        pts: Option<u64>,
        dts: Option<u64>,
        temporal_id: Option<u8>,
    ) -> Self {
        Self {
            frame_index,
            poc,
            pts,
            dts,
            temporal_id,
        }
    }

    /// Create from UnitModel with defaults
    pub fn from_unit_model(unit: &bitvue_core::UnitModel) -> Self {
        // Get data from first frame unit
        let first_unit = unit.units.first();
        let frame_index = first_unit.and_then(|u| u.frame_index).unwrap_or(0);
        let pts = first_unit.and_then(|u| u.pts);
        let dts = first_unit.and_then(|u| u.dts);
        let temporal_id = first_unit.and_then(|u| u.temporal_id);

        Self {
            frame_index,
            poc: frame_index as i32,
            pts,
            dts,
            temporal_id,
        }
    }

    /// Create from UnitNode
    pub fn from_unit_node(unit: &bitvue_core::UnitNode) -> Self {
        let frame_index = unit.frame_index.unwrap_or(0);
        Self {
            frame_index,
            poc: frame_index as i32,
            pts: unit.pts,
            dts: unit.dts,
            temporal_id: unit.temporal_id,
        }
    }
}

/// Frame metadata - type and size data
///
/// Groups frame_type, nal_type, layer, offset, size
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct FrameMetadata {
    /// Frame type (I/P/B)
    pub frame_type: String,
    /// NAL unit type
    pub nal_type: String,
    /// Temporal layer
    pub layer: String,
    /// File offset
    pub offset: u64,
    /// Frame size in bytes
    pub size: usize,
}

#[allow(dead_code)]
impl FrameMetadata {
    /// Create from individual components
    pub fn new(frame_type: String, nal_type: String, layer: String, offset: u64, size: usize) -> Self {
        Self {
            frame_type,
            nal_type,
            layer,
            offset,
            size,
        }
    }

    /// Create from UnitModel
    pub fn from_unit_model(unit: &bitvue_core::UnitModel, display_type: &str) -> Self {
        // Get data from first frame unit
        let first_unit = unit.units.first();
        let unit_type = first_unit.map(|u| u.unit_type.as_str()).unwrap_or("FRAME");
        let offset = first_unit.map(|u| u.offset).unwrap_or(0);
        let size = first_unit.map(|u| u.size).unwrap_or(0);

        let nal_type = if unit_type == "FRAME" {
            display_type.to_string()
        } else {
            unit_type.to_string()
        };

        Self {
            frame_type: display_type.to_string(),
            nal_type,
            layer: "A".to_string(),
            offset,
            size,
        }
    }

    /// Create from UnitNode
    pub fn from_unit_node(unit: &bitvue_core::UnitNode, display_type: &str) -> Self {
        let nal_type = if unit.unit_type == "FRAME" {
            display_type.to_string()
        } else {
            unit.unit_type.clone()
        };

        Self {
            frame_type: display_type.to_string(),
            nal_type,
            layer: "A".to_string(), // Default temporal layer; most codecs don't use temporal scalability
            offset: unit.offset,
            size: unit.size,
        }
    }
}

/// Reference information - reference frame data
///
/// Groups ref_list, ref_frames, ref_slots, ref_slot_info
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct ReferenceInfo {
    /// Reference list as string
    pub ref_list: Option<String>,
    /// Referenced frame indices (for P/B frames)
    pub ref_frames: Option<Vec<usize>>,
    /// Reference slot indices (raw slot numbers from bitstream)
    pub ref_slots: Option<Vec<u8>>,
    /// Detailed reference slot information with names
    pub ref_slot_info: Option<Vec<ReferenceSlotInfo>>,
}

#[allow(dead_code)]
impl ReferenceInfo {
    /// Create from individual components
    pub fn new(
        ref_list: Option<String>,
        ref_frames: Option<Vec<usize>>,
        ref_slots: Option<Vec<u8>>,
        ref_slot_info: Option<Vec<ReferenceSlotInfo>>,
    ) -> Self {
        Self {
            ref_list,
            ref_frames,
            ref_slots,
            ref_slot_info,
        }
    }

    /// Create from UnitModel
    pub fn from_unit_model(unit: &bitvue_core::UnitModel) -> Self {
        // Extract ref_frames/ref_slots from the first frame unit
        let (ref_frames, ref_slots) = unit.units.first()
            .map(|u| (u.ref_frames.clone(), u.ref_slots.clone()))
            .unwrap_or((None, None));

        let ref_slot_info = generate_slot_info(ref_slots.clone(), ref_frames.clone());

        Self {
            ref_list: None, // Reference list string not yet extracted; ref_frames/ref_slots provide structured data
            ref_frames,
            ref_slots,
            ref_slot_info,
        }
    }

    /// Create from UnitNode
    pub fn from_unit_node(unit: &bitvue_core::UnitNode) -> Self {
        let ref_frames = unit.ref_frames.clone();
        let ref_slots = unit.ref_slots.clone();
        let ref_slot_info = generate_slot_info(ref_slots.clone(), ref_frames.clone());

        Self {
            ref_list: None, // Reference list string not yet extracted; ref_frames/ref_slots provide structured data
            ref_frames,
            ref_slots,
            ref_slot_info,
        }
    }

    /// Check if this frame has references
    pub fn has_references(&self) -> bool {
        self.ref_frames.as_ref().is_some_and(|v| !v.is_empty())
    }

    /// Get the number of reference frames
    pub fn ref_count(&self) -> usize {
        self.ref_frames.as_ref().map_or(0, |v| v.len())
    }
}

/// Frame information for filmstrip (VQAnalyzer parity)
///
/// Refactored from god object (17 fields) to focused composition.
/// Now composed of 3 focused value objects: FramePosition, FrameMetadata, ReferenceInfo.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct FrameDisplayData {
    /// Temporal and position data
    pub position: FramePosition,
    /// Type and size data
    pub metadata: FrameMetadata,
    /// Reference frame data
    pub references: ReferenceInfo,
}

#[allow(dead_code)]
impl FrameDisplayData {
    /// Create from individual components
    pub fn new(
        position: FramePosition,
        metadata: FrameMetadata,
        references: ReferenceInfo,
    ) -> Self {
        Self {
            position,
            metadata,
            references,
        }
    }

    /// Create from UnitModel
    pub fn from_unit_model(unit: &bitvue_core::UnitModel, display_type: &str) -> Self {
        Self {
            position: FramePosition::from_unit_model(unit),
            metadata: FrameMetadata::from_unit_model(unit, display_type),
            references: ReferenceInfo::from_unit_model(unit),
        }
    }

    /// Create from UnitNode
    pub fn from_unit_node(unit: &bitvue_core::UnitNode, display_type: &str) -> Self {
        Self {
            position: FramePosition::from_unit_node(unit),
            metadata: FrameMetadata::from_unit_node(unit, display_type),
            references: ReferenceInfo::from_unit_node(unit),
        }
    }

    /// Create from individual fields (legacy API compatibility)
    #[allow(clippy::too_many_arguments)]
    pub fn from_fields(
        frame_index: usize,
        frame_type: String,
        offset: u64,
        size: usize,
        poc: i32,
        nal_type: String,
        layer: String,
        pts: Option<u64>,
        dts: Option<u64>,
        ref_list: Option<String>,
        temporal_id: Option<u8>,
        ref_frames: Option<Vec<usize>>,
        ref_slots: Option<Vec<u8>>,
        ref_slot_info: Option<Vec<ReferenceSlotInfo>>,
    ) -> Self {
        Self {
            position: FramePosition {
                frame_index,
                poc,
                pts,
                dts,
                temporal_id,
            },
            metadata: FrameMetadata {
                frame_type,
                nal_type,
                layer,
                offset,
                size,
            },
            references: ReferenceInfo {
                ref_list,
                ref_frames,
                ref_slots,
                ref_slot_info,
            },
        }
    }

    // ========== Convenience accessors for field compatibility ==========

    pub fn frame_index(&self) -> usize { self.position.frame_index }
    pub fn frame_type(&self) -> &str { &self.metadata.frame_type }
    pub fn offset(&self) -> u64 { self.metadata.offset }
    pub fn size(&self) -> usize { self.metadata.size }
    pub fn poc(&self) -> i32 { self.position.poc }
    pub fn nal_type(&self) -> &str { &self.metadata.nal_type }
    pub fn layer(&self) -> &str { &self.metadata.layer }
    pub fn pts(&self) -> Option<u64> { self.position.pts }
    pub fn dts(&self) -> Option<u64> { self.position.dts }
    pub fn ref_list(&self) -> Option<&String> { self.references.ref_list.as_ref() }
    pub fn temporal_id(&self) -> Option<u8> { self.position.temporal_id }
    pub fn ref_frames(&self) -> Option<&Vec<usize>> { self.references.ref_frames.as_ref() }
    pub fn ref_slots(&self) -> Option<&Vec<u8>> { self.references.ref_slots.as_ref() }
    pub fn ref_slot_info(&self) -> Option<&Vec<ReferenceSlotInfo>> { self.references.ref_slot_info.as_ref() }
}

/// Codec-specific reference slot names
#[allow(dead_code)]
pub fn get_av1_slot_name(slot_idx: u8) -> &'static str {
    match slot_idx {
        0 => "LAST",
        1 => "LAST2",
        2 => "LAST3",
        3 => "GOLDEN",
        4 => "BWDREF",
        5 => "ALTREF2",
        6 => "ALTREF",
        _ => "SLOT7",
    }
}

#[allow(dead_code)]
pub fn get_hevc_slot_name(ref_idx: usize, list: u8) -> String {
    format!("L{}[{}]", list, ref_idx)
}

/// Frame service for aggregating frame information
#[allow(dead_code)]
pub struct FrameService;

#[allow(dead_code)]
impl FrameService {
    /// Create a new frame service
    pub fn new() -> Self {
        Self
    }

    /// Collect frame data from unit model
    pub fn collect_frames(unit_model: &UnitModel) -> Vec<FrameDisplayData> {
        let mut frames = Vec::new();

        for unit in &unit_model.units {
            if unit.frame_index.is_some() {
                let frame_type = unit.frame_type.clone().unwrap_or("UNKNOWN".to_string());

                // Map frame types to I/P/B for display
                let display_type = Self::normalize_frame_type(&frame_type);

                // Create from unit node directly
                frames.push(FrameDisplayData::from_unit_node(unit, display_type));
            }
        }

        frames
    }

    /// Normalize frame type to I/P/B
    fn normalize_frame_type(frame_type: &str) -> &str {
        match frame_type {
            "KEY" | "INTRA_ONLY" | "I" => "I",
            "INTER" | "P" => "P",
            "B" => "B",
            _ => {
                // Try to determine from frame_type string
                if frame_type.contains('I') {
                    "I"
                } else if frame_type.contains('P') {
                    "P"
                } else if frame_type.contains('B') {
                    "B"
                } else {
                    frame_type
                }
            }
        }
    }
}

/// Generate slot information with proper names based on slot indices
///
/// Public function for use by ReferenceInfo::from_unit_model
pub fn generate_slot_info(
    ref_slots: Option<Vec<u8>>,
    ref_frames: Option<Vec<usize>>,
) -> Option<Vec<ReferenceSlotInfo>> {
    match (ref_slots, ref_frames) {
        (Some(slots), Some(frames)) if !slots.is_empty() => {
            let mut slot_info = Vec::new();
            for (idx, slot_idx) in slots.iter().enumerate() {
                let frame_index = frames.get(idx).copied();
                slot_info.push(ReferenceSlotInfo {
                    index: *slot_idx,
                    name: get_av1_slot_name(*slot_idx).to_string(),
                    frame_index,
                });
            }
            Some(slot_info)
        }
        _ => None,
    }
}

impl Default for FrameService {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_frame_type() {
        assert_eq!(FrameService::normalize_frame_type("KEY"), "I");
        assert_eq!(FrameService::normalize_frame_type("INTRA_ONLY"), "I");
        assert_eq!(FrameService::normalize_frame_type("I"), "I");
        assert_eq!(FrameService::normalize_frame_type("INTER"), "P");
        assert_eq!(FrameService::normalize_frame_type("P"), "P");
        assert_eq!(FrameService::normalize_frame_type("B"), "B");
    }
}
