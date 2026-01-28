//! Core types for bitstream analysis

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Information about a parsed bitstream
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BitstreamInfo {
    /// Codec type (e.g., "AV1", "H.264", "HEVC")
    pub codec: String,
    /// File path or identifier
    pub source: String,
    /// Total size in bytes
    pub size: u64,
    /// Sequence/stream level parameters
    pub sequence: Option<SequenceInfo>,
    /// List of frames
    pub frames: Vec<FrameInfo>,
}

/// Sequence-level information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SequenceInfo {
    /// Profile (e.g., Main, High)
    pub profile: u8,
    /// Level
    pub level: u8,
    /// Video width in pixels
    pub width: u32,
    /// Video height in pixels
    pub height: u32,
    /// Bit depth (8, 10, 12)
    pub bit_depth: u8,
    /// Chroma subsampling (e.g., "4:2:0", "4:4:4")
    pub chroma_subsampling: String,
}

/// Frame-level information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrameInfo {
    /// Frame index (0-based)
    pub index: usize,
    /// Frame type
    pub frame_type: FrameType,
    /// Byte offset in the bitstream
    pub offset: u64,
    /// Size in bytes
    pub size: u64,
    /// Presentation timestamp (PTS) in container units
    pub pts: Option<u64>,
    /// Decode timestamp (DTS) in container units
    pub dts: Option<u64>,
    /// Whether this frame is shown
    pub show_frame: bool,
    /// List of blocks in this frame
    pub blocks: Vec<BlockInfo>,
}

/// Frame type enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FrameType {
    /// Key frame (I-frame)
    Key,
    /// Inter frame (P-frame)
    Inter,
    /// Bidirectional frame (B-frame, H.264/HEVC)
    BFrame,
    /// Intra-only frame
    IntraOnly,
    /// Switch frame
    Switch,
}

impl FrameType {
    /// Returns true if this is a key frame (I-frame)
    pub fn is_key(self) -> bool {
        matches!(self, FrameType::Key)
    }

    /// Returns true if this is an inter frame (P-frame)
    pub fn is_inter(self) -> bool {
        matches!(self, FrameType::Inter)
    }

    /// Returns true if this is a bidirectional frame (B-frame)
    pub fn is_b_frame(self) -> bool {
        matches!(self, FrameType::BFrame)
    }

    /// Returns true if this is an intra-only frame
    pub fn is_intra_only(self) -> bool {
        matches!(self, FrameType::IntraOnly)
    }

    /// Returns true if this is a switch frame
    pub fn is_switch(self) -> bool {
        matches!(self, FrameType::Switch)
    }

    /// Returns true if this is any intra frame (Key or IntraOnly)
    pub fn is_intra(self) -> bool {
        matches!(self, FrameType::Key | FrameType::IntraOnly)
    }

    /// Returns true if this frame can be used as a reference
    pub fn is_reference(self) -> bool {
        // Key frames are always reference frames
        // Inter frames may be reference frames (not strictly determined by type)
        // B-frames are typically not reference frames
        !matches!(self, FrameType::BFrame)
    }

    /// Returns the short name (single character)
    pub fn short_name(self) -> &'static str {
        match self {
            FrameType::Key => "I",
            FrameType::Inter => "P",
            FrameType::BFrame => "B",
            FrameType::IntraOnly => "I",
            FrameType::Switch => "S",
        }
    }

    /// Parses a frame type from a string
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_uppercase().as_str() {
            "KEY" | "I" | "I-FRAME" => Some(FrameType::Key),
            "INTER" | "P" | "P-FRAME" => Some(FrameType::Inter),
            "B" | "B-FRAME" => Some(FrameType::BFrame),
            "INTRA_ONLY" | "INTRAONLY" => Some(FrameType::IntraOnly),
            "SWITCH" | "S" => Some(FrameType::Switch),
            _ => None,
        }
    }

    /// Returns all frame type variants
    pub fn all() -> &'static [FrameType] {
        &[
            FrameType::Key,
            FrameType::Inter,
            FrameType::BFrame,
            FrameType::IntraOnly,
            FrameType::Switch,
        ]
    }

    /// Returns a description of the frame type
    pub fn description(self) -> &'static str {
        match self {
            FrameType::Key => {
                "Key frame (I-frame) - can be decoded without reference to other frames"
            }
            FrameType::Inter => "Inter frame (P-frame) - uses references to previous frames",
            FrameType::BFrame => {
                "Bidirectional frame (B-frame) - uses references to both past and future frames"
            }
            FrameType::IntraOnly => "Intra-only frame - all blocks use intra prediction",
            FrameType::Switch => "Switch frame - used for transitioning between different streams",
        }
    }
}

impl std::fmt::Display for FrameType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FrameType::Key => write!(f, "KEY"),
            FrameType::Inter => write!(f, "INTER"),
            FrameType::BFrame => write!(f, "B"),
            FrameType::IntraOnly => write!(f, "INTRA_ONLY"),
            FrameType::Switch => write!(f, "SWITCH"),
        }
    }
}

/// Block-level information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockInfo {
    /// Block position (x, y) in pixels
    pub x: u32,
    pub y: u32,
    /// Block size (width, height)
    pub width: u32,
    pub height: u32,
    /// Prediction mode
    pub prediction_mode: PredictionMode,
    /// Quantization parameter
    pub qp: Option<u8>,
    /// Bits used for this block
    pub bits: Option<u32>,
    /// Motion vector (for inter blocks)
    pub motion_vector: Option<MotionVector>,
}

/// Prediction mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PredictionMode {
    Intra(IntraMode),
    Inter,
    Skip,
}

/// Intra prediction mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum IntraMode {
    DC,
    Vertical,
    Horizontal,
    Diagonal(u8), // angle
    Smooth,
    Paeth,
}

/// Motion vector
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct MotionVector {
    /// Horizontal component (in 1/8 pel units)
    pub x: i16,
    /// Vertical component (in 1/8 pel units)
    pub y: i16,
    /// Reference frame index
    pub ref_frame: i8,
}

/// Overlay layer type for visualization
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum OverlayLayer {
    Grid,
    BlockPartition,
    PredictionMode,
    IntraDirection,
    MotionVectors,
    ReferenceFrames,
    QpMap,
    BitsPerBlock,
    TransformType,
    Skip,
    SegmentId,
    FilmGrain,
}

impl OverlayLayer {
    pub fn all() -> &'static [OverlayLayer] {
        &[
            OverlayLayer::Grid,
            OverlayLayer::BlockPartition,
            OverlayLayer::PredictionMode,
            OverlayLayer::IntraDirection,
            OverlayLayer::MotionVectors,
            OverlayLayer::ReferenceFrames,
            OverlayLayer::QpMap,
            OverlayLayer::BitsPerBlock,
            OverlayLayer::TransformType,
            OverlayLayer::Skip,
            OverlayLayer::SegmentId,
            OverlayLayer::FilmGrain,
        ]
    }

    pub fn name(&self) -> &'static str {
        match self {
            OverlayLayer::Grid => "Grid",
            OverlayLayer::BlockPartition => "Block Partition",
            OverlayLayer::PredictionMode => "Prediction Mode",
            OverlayLayer::IntraDirection => "Intra Direction",
            OverlayLayer::MotionVectors => "Motion Vectors",
            OverlayLayer::ReferenceFrames => "Reference Frames",
            OverlayLayer::QpMap => "QP Map",
            OverlayLayer::BitsPerBlock => "Bits/Block",
            OverlayLayer::TransformType => "Transform Type",
            OverlayLayer::Skip => "Skip",
            OverlayLayer::SegmentId => "Segment ID",
            OverlayLayer::FilmGrain => "Film Grain",
        }
    }
}

// ============================================================================
// Syntax Tree Types (Phase 0 - Tri-sync)
// ============================================================================

/// Bit range in the bitstream (start_bit, end_bit)
///
/// Bit offsets are absolute from the start of the file.
/// Range is [start, end) - i.e., start is inclusive, end is exclusive.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BitRange {
    pub start_bit: u64,
    pub end_bit: u64,
}

impl BitRange {
    pub fn new(start_bit: u64, end_bit: u64) -> Self {
        Self { start_bit, end_bit }
    }

    /// Returns true if this range contains the given bit offset
    pub fn contains(&self, bit_offset: u64) -> bool {
        self.start_bit <= bit_offset && bit_offset < self.end_bit
    }

    /// Returns true if this range fully contains another range
    pub fn contains_range(&self, other: &BitRange) -> bool {
        self.start_bit <= other.start_bit && other.end_bit <= self.end_bit
    }

    /// Returns the size in bits
    pub fn size_bits(&self) -> u64 {
        self.end_bit - self.start_bit
    }

    /// Returns the byte offset (start_bit / 8)
    pub fn byte_offset(&self) -> u64 {
        self.start_bit / 8
    }
}

/// Syntax node identifier (unique within a SyntaxModel)
pub type SyntaxNodeId = String;

/// Syntax node representing a parsed field in the bitstream
///
/// Per TRI_SYNC_CONTRACT.md:
/// - Each node has a bit_range indicating its position in the bitstream
/// - Nodes form a tree structure via parent/children relationships
/// - The tightest containing node is used for reverse mapping (Hex → Syntax)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyntaxNode {
    /// Unique identifier for this node
    pub node_id: SyntaxNodeId,

    /// Bit range this node occupies [start, end)
    pub bit_range: BitRange,

    /// Field name (e.g., "obu_type", "frame_width_minus_1")
    pub field_name: String,

    /// Parsed value as string (e.g., "3", "0x1F", "FRAME_HEADER")
    /// None for container nodes that don't have a direct value
    pub value: Option<String>,

    /// Parent node ID (None for root)
    pub parent: Option<SyntaxNodeId>,

    /// Child node IDs
    pub children: Vec<SyntaxNodeId>,

    /// Tree depth (0 for root)
    pub depth: usize,
}

impl SyntaxNode {
    /// Create a new syntax node
    pub fn new(
        node_id: SyntaxNodeId,
        bit_range: BitRange,
        field_name: String,
        value: Option<String>,
        parent: Option<SyntaxNodeId>,
        depth: usize,
    ) -> Self {
        Self {
            node_id,
            bit_range,
            field_name,
            value,
            parent,
            children: Vec::new(),
            depth,
        }
    }

    /// Add a child to this node
    pub fn add_child(&mut self, child_id: SyntaxNodeId) {
        self.children.push(child_id);
    }
}

/// Syntax model for a parsed unit (e.g., one OBU)
///
/// Contains the complete syntax tree with bit-level positioning.
/// Used for Syntax Tree panel and Tri-sync.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyntaxModel {
    /// All nodes indexed by node_id
    pub nodes: HashMap<SyntaxNodeId, SyntaxNode>,

    /// Root node ID
    pub root_id: SyntaxNodeId,

    /// Unit key this syntax belongs to
    pub unit_key: String,
}

impl SyntaxModel {
    /// Create a new empty syntax model
    pub fn new(root_id: SyntaxNodeId, unit_key: String) -> Self {
        Self {
            nodes: HashMap::new(),
            root_id,
            unit_key,
        }
    }

    /// Add a node to the model
    pub fn add_node(&mut self, node: SyntaxNode) {
        // If this node has a parent, add it to the parent's children list
        if let Some(parent_id) = &node.parent {
            if let Some(parent_node) = self.nodes.get_mut(parent_id) {
                parent_node.add_child(node.node_id.clone());
            }
        }

        self.nodes.insert(node.node_id.clone(), node);
    }

    /// Get a node by ID
    pub fn get_node(&self, node_id: &str) -> Option<&SyntaxNode> {
        self.nodes.get(node_id)
    }

    /// Find the tightest node containing a bit range
    ///
    /// Per TRI_SYNC_CONTRACT.md Section 3:
    /// 1. Find all nodes whose bit_range fully contains `bit_range`
    /// 2. Choose the smallest containing node (tightest range)
    /// 3. If multiple nodes have identical range, choose deepest
    /// 4. If still tied, choose lexicographically smallest node_id
    pub fn find_nearest_node(&self, bit_range: &BitRange) -> Option<&SyntaxNode> {
        let mut candidates: Vec<&SyntaxNode> = self
            .nodes
            .values()
            .filter(|node| node.bit_range.contains_range(bit_range))
            .collect();

        if candidates.is_empty() {
            // No containing node - find nearest by distance to start
            return self.find_nearest_by_distance(bit_range);
        }

        // Sort by:
        // 1. Range size (ascending - smallest first)
        // 2. Depth (descending - deepest first)
        // 3. node_id (lexicographic - stable tie-breaker)
        candidates.sort_by(|a, b| {
            let size_a = a.bit_range.size_bits();
            let size_b = b.bit_range.size_bits();

            size_a
                .cmp(&size_b)
                .then_with(|| b.depth.cmp(&a.depth))
                .then_with(|| a.node_id.cmp(&b.node_id))
        });

        candidates.first().copied()
    }

    /// Find nearest node by minimal distance to range start
    fn find_nearest_by_distance(&self, bit_range: &BitRange) -> Option<&SyntaxNode> {
        self.nodes.values().min_by_key(|node| {
            // Distance from bit_range.start to node's range
            if node.bit_range.end_bit <= bit_range.start_bit {
                // Node is before target
                bit_range.start_bit.saturating_sub(node.bit_range.end_bit)
            } else if node.bit_range.start_bit >= bit_range.end_bit {
                // Node is after target
                node.bit_range.start_bit.saturating_sub(bit_range.end_bit)
            } else {
                // Overlap (shouldn't happen if contains_range returned false)
                0
            }
        })
    }

    /// Get all leaf nodes (nodes with no children)
    pub fn leaf_nodes(&self) -> Vec<&SyntaxNode> {
        self.nodes
            .values()
            .filter(|node| node.children.is_empty())
            .collect()
    }

    /// Get the root node
    pub fn root_node(&self) -> Option<&SyntaxNode> {
        self.get_node(&self.root_id)
    }
}

/// Comprehensive test suite with Arrange-Act-Assert pattern
#[cfg(test)]
mod tests {
    include!("types_test.rs");
}
