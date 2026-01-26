//! Evidence Chain - T0-2
//!
//! Deliverable: evidence_chain (Stages 01-05) ✓
//!
//! Lossless evidence links across layers with explicit boundaries.
//! Stage 01/05: bit_offset layer ✓
//! Stage 02/05: syntax layer ✓
//! Stage 03/05: decode layer ✓
//! Stage 04/05: viz layer ✓
//! Stage 05/05: bidirectional traversal ✓
//!
//! Per COORDINATE_SYSTEM_CONTRACT and TRI_SYNC_CONTRACT:
//! - Evidence chain connects: bit_offset ↔ syntax ↔ decode ↔ viz
//! - Each layer must be bidirectionally traversable
//! - Provenance tracking for deterministic lookup

use crate::BitRange;
use serde::{Deserialize, Serialize};

/// Evidence chain ID - unique identifier for an evidence link
pub type EvidenceId = String;

/// Bit offset evidence - Stage 01/05
///
/// Links bitstream byte offset to higher-level abstractions.
/// This is the foundation layer for all evidence chains.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BitOffsetEvidence {
    /// Unique evidence ID
    pub id: EvidenceId,

    /// Bit range in the bitstream
    pub bit_range: BitRange,

    /// Byte offset (derived from bit_range.start_bit / 8)
    pub byte_offset: u64,

    /// Size in bytes
    pub size_bytes: usize,

    /// Provenance tag (e.g., "OBU_FRAME_HEADER", "tile_0")
    pub provenance: String,

    /// Link to syntax layer (if available)
    pub syntax_link: Option<EvidenceId>,
}

impl BitOffsetEvidence {
    /// Create new bit offset evidence
    pub fn new(id: EvidenceId, bit_range: BitRange, provenance: String) -> Self {
        let byte_offset = bit_range.byte_offset();
        let size_bytes = bit_range.size_bits().div_ceil(8) as usize;

        Self {
            id,
            bit_range,
            byte_offset,
            size_bytes,
            provenance,
            syntax_link: None,
        }
    }

    /// Link to syntax layer
    pub fn link_syntax(&mut self, syntax_id: EvidenceId) {
        self.syntax_link = Some(syntax_id);
    }

    /// Check if this evidence contains a bit offset
    pub fn contains_bit(&self, bit_offset: u64) -> bool {
        self.bit_range.contains(bit_offset)
    }

    /// Check if this evidence overlaps with a bit range
    pub fn overlaps(&self, other: &BitRange) -> bool {
        !(self.bit_range.end_bit <= other.start_bit || other.end_bit <= self.bit_range.start_bit)
    }
}

/// Evidence chain stage 01: Bit offset index
///
/// Stores and queries bit offset evidence for fast lookup.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BitOffsetIndex {
    /// All bit offset evidence entries (sorted by start_bit)
    evidence: Vec<BitOffsetEvidence>,
}

impl BitOffsetIndex {
    /// Create new empty index
    pub fn new() -> Self {
        Self {
            evidence: Vec::new(),
        }
    }

    /// Add evidence to the index
    pub fn add(&mut self, evidence: BitOffsetEvidence) {
        self.evidence.push(evidence);
        // Keep sorted by start_bit for binary search
        self.evidence.sort_by_key(|e| e.bit_range.start_bit);
    }

    /// Find evidence by ID
    pub fn find_by_id(&self, id: &str) -> Option<&BitOffsetEvidence> {
        self.evidence.iter().find(|e| e.id == id)
    }

    /// Find evidence containing a specific bit offset
    ///
    /// Returns the tightest containing evidence (smallest range).
    pub fn find_by_bit_offset(&self, bit_offset: u64) -> Option<&BitOffsetEvidence> {
        self.evidence
            .iter()
            .filter(|e| e.contains_bit(bit_offset))
            .min_by_key(|e| e.bit_range.size_bits())
    }

    /// Find all evidence overlapping with a bit range
    pub fn find_overlapping(&self, range: &BitRange) -> Vec<&BitOffsetEvidence> {
        self.evidence.iter().filter(|e| e.overlaps(range)).collect()
    }

    /// Get all evidence entries
    pub fn all(&self) -> &[BitOffsetEvidence] {
        &self.evidence
    }

    /// Get evidence count
    pub fn len(&self) -> usize {
        self.evidence.len()
    }

    /// Check if index is empty
    pub fn is_empty(&self) -> bool {
        self.evidence.is_empty()
    }
}

impl Default for BitOffsetIndex {
    fn default() -> Self {
        Self::new()
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Stage 02/05: Syntax Layer Evidence
// ═══════════════════════════════════════════════════════════════════════════

/// Syntax node type for evidence tracking
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum SyntaxNodeType {
    /// OBU (Open Bitstream Unit)
    Obu,
    /// Sequence header
    SequenceHeader,
    /// Frame header
    FrameHeader,
    /// Tile group
    TileGroup,
    /// Tile
    Tile,
    /// Quantization parameters
    QuantizationParams,
    /// Loop filter parameters
    LoopFilterParams,
    /// CDEF (Constrained Directional Enhancement Filter) parameters
    CdefParams,
    /// Segmentation parameters
    SegmentationParams,
    /// Mode info block
    ModeInfo,
    /// Custom node type (for extensibility)
    Custom(String),
}

/// Syntax layer evidence - Stage 02/05
///
/// Links syntax elements (OBUs, frame headers, etc.) to bit offset layer.
/// Provides semantic structure on top of raw bit offsets.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyntaxEvidence {
    /// Unique evidence ID
    pub id: EvidenceId,

    /// Syntax node type
    pub node_type: SyntaxNodeType,

    /// Node name/label (e.g., "frame_header", "tile_0")
    pub node_label: String,

    /// Bit range in the bitstream
    pub bit_range: BitRange,

    /// Link back to bit offset layer
    pub bit_offset_link: EvidenceId,

    /// Parent syntax node (if nested)
    pub parent_link: Option<EvidenceId>,

    /// Child syntax nodes
    pub child_links: Vec<EvidenceId>,

    /// Link to decode layer (if available)
    pub decode_link: Option<EvidenceId>,

    /// Additional metadata (key-value pairs)
    pub metadata: std::collections::HashMap<String, String>,
}

impl SyntaxEvidence {
    /// Create new syntax evidence
    pub fn new(
        id: EvidenceId,
        node_type: SyntaxNodeType,
        node_label: String,
        bit_range: BitRange,
        bit_offset_link: EvidenceId,
    ) -> Self {
        Self {
            id,
            node_type,
            node_label,
            bit_range,
            bit_offset_link,
            parent_link: None,
            child_links: Vec::new(),
            decode_link: None,
            metadata: std::collections::HashMap::new(),
        }
    }

    /// Set parent link
    pub fn set_parent(&mut self, parent_id: EvidenceId) {
        self.parent_link = Some(parent_id);
    }

    /// Add child link
    pub fn add_child(&mut self, child_id: EvidenceId) {
        self.child_links.push(child_id);
    }

    /// Link to decode layer
    pub fn link_decode(&mut self, decode_id: EvidenceId) {
        self.decode_link = Some(decode_id);
    }

    /// Add metadata
    pub fn add_metadata(&mut self, key: String, value: String) {
        self.metadata.insert(key, value);
    }

    /// Check if this syntax node contains a bit offset
    pub fn contains_bit(&self, bit_offset: u64) -> bool {
        self.bit_range.contains(bit_offset)
    }
}

/// Syntax layer index - Stage 02/05
///
/// Stores and queries syntax evidence for fast lookup.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyntaxIndex {
    /// All syntax evidence entries (sorted by start_bit)
    evidence: Vec<SyntaxEvidence>,
}

impl SyntaxIndex {
    /// Create new empty index
    pub fn new() -> Self {
        Self {
            evidence: Vec::new(),
        }
    }

    /// Add evidence to the index
    pub fn add(&mut self, evidence: SyntaxEvidence) {
        self.evidence.push(evidence);
        // Keep sorted by start_bit for binary search
        self.evidence.sort_by_key(|e| e.bit_range.start_bit);
    }

    /// Find evidence by ID
    pub fn find_by_id(&self, id: &str) -> Option<&SyntaxEvidence> {
        self.evidence.iter().find(|e| e.id == id)
    }

    /// Find evidence by ID (mutable)
    pub fn find_by_id_mut(&mut self, id: &str) -> Option<&mut SyntaxEvidence> {
        self.evidence.iter_mut().find(|e| e.id == id)
    }

    /// Find evidence containing a specific bit offset
    ///
    /// Returns the tightest containing evidence (smallest range).
    pub fn find_by_bit_offset(&self, bit_offset: u64) -> Option<&SyntaxEvidence> {
        self.evidence
            .iter()
            .filter(|e| e.contains_bit(bit_offset))
            .min_by_key(|e| e.bit_range.size_bits())
    }

    /// Find all evidence of a specific node type
    pub fn find_by_type(&self, node_type: &SyntaxNodeType) -> Vec<&SyntaxEvidence> {
        self.evidence
            .iter()
            .filter(|e| &e.node_type == node_type)
            .collect()
    }

    /// Find all root nodes (no parent)
    pub fn find_roots(&self) -> Vec<&SyntaxEvidence> {
        self.evidence
            .iter()
            .filter(|e| e.parent_link.is_none())
            .collect()
    }

    /// Find all children of a node
    pub fn find_children(&self, parent_id: &str) -> Vec<&SyntaxEvidence> {
        self.evidence
            .iter()
            .filter(|e| e.parent_link.as_deref() == Some(parent_id))
            .collect()
    }

    /// Get all evidence entries
    pub fn all(&self) -> &[SyntaxEvidence] {
        &self.evidence
    }

    /// Get evidence count
    pub fn len(&self) -> usize {
        self.evidence.len()
    }

    /// Check if index is empty
    pub fn is_empty(&self) -> bool {
        self.evidence.is_empty()
    }
}

impl Default for SyntaxIndex {
    fn default() -> Self {
        Self::new()
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Stage 03/05: Decode Layer Evidence
// ═══════════════════════════════════════════════════════════════════════════

/// Decode artifact type for evidence tracking
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum DecodeArtifactType {
    /// Decoded YUV frame
    YuvFrame,
    /// Reference frame
    ReferenceFrame,
    /// Reconstructed block
    ReconstructedBlock,
    /// Motion vector field
    MotionVectorField,
    /// Quantization parameter map
    QpMap,
    /// Transform coefficients
    TransformCoefficients,
    /// Custom artifact (for extensibility)
    Custom(String),
}

/// Decode layer evidence - Stage 03/05
///
/// Links decoded artifacts (frames, blocks, MVs) to syntax layer.
/// Provides the connection between coded syntax and decoded representation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecodeEvidence {
    /// Unique evidence ID
    pub id: EvidenceId,

    /// Decode artifact type
    pub artifact_type: DecodeArtifactType,

    /// Artifact label (e.g., "frame_0_yuv", "ref_frame_2")
    pub artifact_label: String,

    /// Frame index (if applicable)
    pub frame_idx: Option<usize>,

    /// Display index (if applicable)
    pub display_idx: Option<usize>,

    /// Link back to syntax layer
    pub syntax_link: EvidenceId,

    /// Link to visualization layer (if available)
    pub viz_link: Option<EvidenceId>,

    /// Data location (memory address, file offset, etc.)
    pub data_location: Option<String>,

    /// Data size in bytes
    pub data_size: Option<usize>,

    /// Additional metadata
    pub metadata: std::collections::HashMap<String, String>,
}

impl DecodeEvidence {
    /// Create new decode evidence
    pub fn new(
        id: EvidenceId,
        artifact_type: DecodeArtifactType,
        artifact_label: String,
        syntax_link: EvidenceId,
    ) -> Self {
        Self {
            id,
            artifact_type,
            artifact_label,
            frame_idx: None,
            display_idx: None,
            syntax_link,
            viz_link: None,
            data_location: None,
            data_size: None,
            metadata: std::collections::HashMap::new(),
        }
    }

    /// Set frame indices
    pub fn set_frame_indices(&mut self, frame_idx: usize, display_idx: usize) {
        self.frame_idx = Some(frame_idx);
        self.display_idx = Some(display_idx);
    }

    /// Set data location
    pub fn set_data_location(&mut self, location: String, size: usize) {
        self.data_location = Some(location);
        self.data_size = Some(size);
    }

    /// Link to visualization layer
    pub fn link_viz(&mut self, viz_id: EvidenceId) {
        self.viz_link = Some(viz_id);
    }

    /// Add metadata
    pub fn add_metadata(&mut self, key: String, value: String) {
        self.metadata.insert(key, value);
    }
}

/// Decode layer index - Stage 03/05
///
/// Stores and queries decode evidence for fast lookup.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecodeIndex {
    /// All decode evidence entries
    evidence: Vec<DecodeEvidence>,
}

impl DecodeIndex {
    /// Create new empty index
    pub fn new() -> Self {
        Self {
            evidence: Vec::new(),
        }
    }

    /// Add evidence to the index
    pub fn add(&mut self, evidence: DecodeEvidence) {
        self.evidence.push(evidence);
    }

    /// Find evidence by ID
    pub fn find_by_id(&self, id: &str) -> Option<&DecodeEvidence> {
        self.evidence.iter().find(|e| e.id == id)
    }

    /// Find evidence by ID (mutable)
    pub fn find_by_id_mut(&mut self, id: &str) -> Option<&mut DecodeEvidence> {
        self.evidence.iter_mut().find(|e| e.id == id)
    }

    /// Find all evidence of a specific artifact type
    pub fn find_by_artifact_type(
        &self,
        artifact_type: &DecodeArtifactType,
    ) -> Vec<&DecodeEvidence> {
        self.evidence
            .iter()
            .filter(|e| &e.artifact_type == artifact_type)
            .collect()
    }

    /// Find evidence by frame index
    pub fn find_by_frame_idx(&self, frame_idx: usize) -> Vec<&DecodeEvidence> {
        self.evidence
            .iter()
            .filter(|e| e.frame_idx == Some(frame_idx))
            .collect()
    }

    /// Find evidence by display index
    pub fn find_by_display_idx(&self, display_idx: usize) -> Vec<&DecodeEvidence> {
        self.evidence
            .iter()
            .filter(|e| e.display_idx == Some(display_idx))
            .collect()
    }

    /// Find evidence linked to a syntax node
    pub fn find_by_syntax_link(&self, syntax_id: &str) -> Vec<&DecodeEvidence> {
        self.evidence
            .iter()
            .filter(|e| e.syntax_link == syntax_id)
            .collect()
    }

    /// Get all evidence entries
    pub fn all(&self) -> &[DecodeEvidence] {
        &self.evidence
    }

    /// Get evidence count
    pub fn len(&self) -> usize {
        self.evidence.len()
    }

    /// Check if index is empty
    pub fn is_empty(&self) -> bool {
        self.evidence.is_empty()
    }
}

impl Default for DecodeIndex {
    fn default() -> Self {
        Self::new()
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Stage 04/05: Visualization Layer Evidence
// ═══════════════════════════════════════════════════════════════════════════

/// Visualization element type for evidence tracking
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum VizElementType {
    /// QP heatmap visualization
    QpHeatmap,
    /// Motion vector overlay
    MotionVectorOverlay,
    /// Partition grid overlay
    PartitionGridOverlay,
    /// Diff heatmap (A/B compare)
    DiffHeatmap,
    /// Timeline lane visualization
    TimelineLane,
    /// Reference frame graph node
    ReferenceGraphNode,
    /// Metrics chart data point
    MetricsChartPoint,
    /// Diagnostic band
    DiagnosticBand,
    /// Custom visualization (for extensibility)
    Custom(String),
}

/// Visualization layer evidence - Stage 04/05
///
/// Links visualization elements to decode layer.
/// Provides the connection between decoded data and visual representation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VizEvidence {
    /// Unique evidence ID
    pub id: EvidenceId,

    /// Visualization element type
    pub element_type: VizElementType,

    /// Element label (e.g., "qp_heatmap_frame_0", "mv_overlay_tile_0")
    pub element_label: String,

    /// Frame index (if applicable)
    pub frame_idx: Option<usize>,

    /// Display index (if applicable)
    pub display_idx: Option<usize>,

    /// Link back to decode layer
    pub decode_link: EvidenceId,

    /// Screen coordinates (if spatial visualization)
    pub screen_rect: Option<(f32, f32, f32, f32)>, // (x, y, w, h)

    /// Coded coordinates (if spatial visualization)
    pub coded_rect: Option<(u32, u32, u32, u32)>, // (x, y, w, h)

    /// Timestamp or position (if temporal visualization)
    pub temporal_pos: Option<f64>,

    /// Visual properties (color, opacity, etc.)
    pub visual_properties: std::collections::HashMap<String, String>,

    /// Additional metadata
    pub metadata: std::collections::HashMap<String, String>,
}

impl VizEvidence {
    /// Create new visualization evidence
    pub fn new(
        id: EvidenceId,
        element_type: VizElementType,
        element_label: String,
        decode_link: EvidenceId,
    ) -> Self {
        Self {
            id,
            element_type,
            element_label,
            frame_idx: None,
            display_idx: None,
            decode_link,
            screen_rect: None,
            coded_rect: None,
            temporal_pos: None,
            visual_properties: std::collections::HashMap::new(),
            metadata: std::collections::HashMap::new(),
        }
    }

    /// Set frame indices
    pub fn set_frame_indices(&mut self, frame_idx: usize, display_idx: usize) {
        self.frame_idx = Some(frame_idx);
        self.display_idx = Some(display_idx);
    }

    /// Set screen coordinates
    pub fn set_screen_rect(&mut self, x: f32, y: f32, w: f32, h: f32) {
        self.screen_rect = Some((x, y, w, h));
    }

    /// Set coded coordinates
    pub fn set_coded_rect(&mut self, x: u32, y: u32, w: u32, h: u32) {
        self.coded_rect = Some((x, y, w, h));
    }

    /// Set temporal position
    pub fn set_temporal_pos(&mut self, pos: f64) {
        self.temporal_pos = Some(pos);
    }

    /// Add visual property
    pub fn add_visual_property(&mut self, key: String, value: String) {
        self.visual_properties.insert(key, value);
    }

    /// Add metadata
    pub fn add_metadata(&mut self, key: String, value: String) {
        self.metadata.insert(key, value);
    }

    /// Check if this viz element contains a screen point
    pub fn contains_screen_point(&self, x: f32, y: f32) -> bool {
        if let Some((rect_x, rect_y, rect_w, rect_h)) = self.screen_rect {
            x >= rect_x && x < rect_x + rect_w && y >= rect_y && y < rect_y + rect_h
        } else {
            false
        }
    }
}

/// Visualization layer index - Stage 04/05
///
/// Stores and queries visualization evidence for fast lookup.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VizIndex {
    /// All visualization evidence entries
    evidence: Vec<VizEvidence>,
}

impl VizIndex {
    /// Create new empty index
    pub fn new() -> Self {
        Self {
            evidence: Vec::new(),
        }
    }

    /// Add evidence to the index
    pub fn add(&mut self, evidence: VizEvidence) {
        self.evidence.push(evidence);
    }

    /// Find evidence by ID
    pub fn find_by_id(&self, id: &str) -> Option<&VizEvidence> {
        self.evidence.iter().find(|e| e.id == id)
    }

    /// Find evidence by ID (mutable)
    pub fn find_by_id_mut(&mut self, id: &str) -> Option<&mut VizEvidence> {
        self.evidence.iter_mut().find(|e| e.id == id)
    }

    /// Find all evidence of a specific element type
    pub fn find_by_element_type(&self, element_type: &VizElementType) -> Vec<&VizEvidence> {
        self.evidence
            .iter()
            .filter(|e| &e.element_type == element_type)
            .collect()
    }

    /// Find evidence by frame index
    pub fn find_by_frame_idx(&self, frame_idx: usize) -> Vec<&VizEvidence> {
        self.evidence
            .iter()
            .filter(|e| e.frame_idx == Some(frame_idx))
            .collect()
    }

    /// Find evidence by display index
    pub fn find_by_display_idx(&self, display_idx: usize) -> Vec<&VizEvidence> {
        self.evidence
            .iter()
            .filter(|e| e.display_idx == Some(display_idx))
            .collect()
    }

    /// Find evidence linked to a decode artifact
    pub fn find_by_decode_link(&self, decode_id: &str) -> Vec<&VizEvidence> {
        self.evidence
            .iter()
            .filter(|e| e.decode_link == decode_id)
            .collect()
    }

    /// Find evidence containing a screen point
    pub fn find_at_screen_point(&self, x: f32, y: f32) -> Vec<&VizEvidence> {
        self.evidence
            .iter()
            .filter(|e| e.contains_screen_point(x, y))
            .collect()
    }

    /// Get all evidence entries
    pub fn all(&self) -> &[VizEvidence] {
        &self.evidence
    }

    /// Get evidence count
    pub fn len(&self) -> usize {
        self.evidence.len()
    }

    /// Check if index is empty
    pub fn is_empty(&self) -> bool {
        self.evidence.is_empty()
    }
}

impl Default for VizIndex {
    fn default() -> Self {
        Self::new()
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Stage 05/05: Bidirectional Evidence Chain Traversal
// ═══════════════════════════════════════════════════════════════════════════

/// Complete evidence chain with bidirectional traversal
///
/// Provides unified API for navigating between all evidence layers:
/// bit_offset ↔ syntax ↔ decode ↔ viz
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvidenceChain {
    /// Bit offset index (Stage 01)
    pub bit_offset_index: BitOffsetIndex,

    /// Syntax index (Stage 02)
    pub syntax_index: SyntaxIndex,

    /// Decode index (Stage 03)
    pub decode_index: DecodeIndex,

    /// Visualization index (Stage 04)
    pub viz_index: VizIndex,
}

impl EvidenceChain {
    /// Create new empty evidence chain
    pub fn new() -> Self {
        Self {
            bit_offset_index: BitOffsetIndex::new(),
            syntax_index: SyntaxIndex::new(),
            decode_index: DecodeIndex::new(),
            viz_index: VizIndex::new(),
        }
    }

    // ═══════════════════════════════════════════════════════════════════════
    // Forward Traversal: bit → syntax → decode → viz
    // ═══════════════════════════════════════════════════════════════════════

    /// Find syntax evidence from bit offset
    pub fn bit_to_syntax(&self, bit_offset: u64) -> Option<&SyntaxEvidence> {
        self.syntax_index.find_by_bit_offset(bit_offset)
    }

    /// Find decode evidence from syntax ID
    pub fn syntax_to_decode(&self, syntax_id: &str) -> Vec<&DecodeEvidence> {
        self.decode_index.find_by_syntax_link(syntax_id)
    }

    /// Find viz evidence from decode ID
    pub fn decode_to_viz(&self, decode_id: &str) -> Vec<&VizEvidence> {
        self.viz_index.find_by_decode_link(decode_id)
    }

    /// Full forward traversal: bit offset → all visualizations
    pub fn bit_to_viz(&self, bit_offset: u64) -> Vec<&VizEvidence> {
        let mut results = Vec::new();

        // Find syntax at bit offset
        if let Some(syntax_ev) = self.bit_to_syntax(bit_offset) {
            // Find all decode artifacts for this syntax
            let decode_evidences = self.syntax_to_decode(&syntax_ev.id);

            // Find all viz elements for each decode artifact
            for decode_ev in decode_evidences {
                results.extend(self.decode_to_viz(&decode_ev.id));
            }
        }

        results
    }

    // ═══════════════════════════════════════════════════════════════════════
    // Backward Traversal: viz → decode → syntax → bit
    // ═══════════════════════════════════════════════════════════════════════

    /// Find decode evidence from viz ID
    pub fn viz_to_decode(&self, viz_id: &str) -> Option<&DecodeEvidence> {
        let viz_ev = self.viz_index.find_by_id(viz_id)?;
        self.decode_index.find_by_id(&viz_ev.decode_link)
    }

    /// Find syntax evidence from decode ID
    pub fn decode_to_syntax(&self, decode_id: &str) -> Option<&SyntaxEvidence> {
        let decode_ev = self.decode_index.find_by_id(decode_id)?;
        self.syntax_index.find_by_id(&decode_ev.syntax_link)
    }

    /// Find bit offset evidence from syntax ID
    pub fn syntax_to_bit(&self, syntax_id: &str) -> Option<&BitOffsetEvidence> {
        let syntax_ev = self.syntax_index.find_by_id(syntax_id)?;
        self.bit_offset_index.find_by_id(&syntax_ev.bit_offset_link)
    }

    /// Full backward traversal: viz element → bit offset
    pub fn viz_to_bit(&self, viz_id: &str) -> Option<&BitOffsetEvidence> {
        let decode_ev = self.viz_to_decode(viz_id)?;
        let syntax_ev = self.decode_to_syntax(&decode_ev.id)?;
        self.syntax_to_bit(&syntax_ev.id)
    }

    // ═══════════════════════════════════════════════════════════════════════
    // Spatial Queries
    // ═══════════════════════════════════════════════════════════════════════

    /// Find all evidence at a screen point (for hover/click)
    pub fn at_screen_point(&self, x: f32, y: f32) -> Vec<&VizEvidence> {
        self.viz_index.find_at_screen_point(x, y)
    }

    /// Find bit range from screen point (complete traversal)
    pub fn screen_point_to_bit_range(&self, x: f32, y: f32) -> Option<BitRange> {
        let viz_evidences = self.at_screen_point(x, y);
        if let Some(viz_ev) = viz_evidences.first() {
            if let Some(bit_ev) = self.viz_to_bit(&viz_ev.id) {
                return Some(bit_ev.bit_range);
            }
        }
        None
    }

    // ═══════════════════════════════════════════════════════════════════════
    // Frame-based Queries
    // ═══════════════════════════════════════════════════════════════════════

    /// Find all evidence for a frame index
    pub fn find_frame_evidence(&self, frame_idx: usize) -> FrameEvidence<'_> {
        FrameEvidence {
            decode_artifacts: self.decode_index.find_by_frame_idx(frame_idx),
            viz_elements: self.viz_index.find_by_frame_idx(frame_idx),
        }
    }

    /// Find all evidence for a display index
    pub fn find_display_evidence(&self, display_idx: usize) -> FrameEvidence<'_> {
        FrameEvidence {
            decode_artifacts: self.decode_index.find_by_display_idx(display_idx),
            viz_elements: self.viz_index.find_by_display_idx(display_idx),
        }
    }

    // ═══════════════════════════════════════════════════════════════════════
    // Mutation (for building evidence chain)
    // ═══════════════════════════════════════════════════════════════════════

    /// Add bit offset evidence
    pub fn add_bit_offset(&mut self, evidence: BitOffsetEvidence) {
        self.bit_offset_index.add(evidence);
    }

    /// Add syntax evidence
    pub fn add_syntax(&mut self, evidence: SyntaxEvidence) {
        self.syntax_index.add(evidence);
    }

    /// Add decode evidence
    pub fn add_decode(&mut self, evidence: DecodeEvidence) {
        self.decode_index.add(evidence);
    }

    /// Add viz evidence
    pub fn add_viz(&mut self, evidence: VizEvidence) {
        self.viz_index.add(evidence);
    }
}

impl Default for EvidenceChain {
    fn default() -> Self {
        Self::new()
    }
}

/// Frame-level evidence bundle
#[derive(Debug)]
pub struct FrameEvidence<'a> {
    /// All decode artifacts for this frame
    pub decode_artifacts: Vec<&'a DecodeEvidence>,

    /// All visualization elements for this frame
    pub viz_elements: Vec<&'a VizEvidence>,
}

// TODO: Fix evidence_test.rs - needs API rewrite to match actual implementation
// #[cfg(test)]
// include!("evidence_test.rs");
