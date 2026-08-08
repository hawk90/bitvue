//! Spatial Hierarchy Evidence Layer - Extended Evidence Chain Layer 6
//!
//! Tracks hierarchical spatial relationships: Frame → Tile → CTU → Block
//! for drill-down exploration and "Explain This Pixel" functionality.
//!
//! Per PERFECT_VISUALIZATION_SPEC: Layer 6 enables hierarchical drill-down
//! navigation from frame level to individual blocks.

use crate::evidence::EvidenceId;
use serde::{Deserialize, Serialize};

// ═══════════════════════════════════════════════════════════════════════════
// Coordinate Types
// ═══════════════════════════════════════════════════════════════════════════

/// Rectangle in coded coordinates (picture space)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CodedRect {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

impl CodedRect {
    pub fn new(x: u32, y: u32, width: u32, height: u32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    pub fn contains(&self, px: u32, py: u32) -> bool {
        px >= self.x && px < self.x + self.width && py >= self.y && py < self.y + self.height
    }

    pub fn area(&self) -> u64 {
        self.width as u64 * self.height as u64
    }

    pub fn center(&self) -> (u32, u32) {
        (self.x + self.width / 2, self.y + self.height / 2)
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Prediction Modes
// ═══════════════════════════════════════════════════════════════════════════

/// Block prediction mode (codec-agnostic)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PredictionMode {
    // Intra modes
    IntraDc,
    IntraHorizontal,
    IntraVertical,
    IntraDiagonal,
    IntraPaeth,
    IntraSmooth,
    IntraOther(u8),

    // Inter modes
    InterSingle,   // Single reference
    InterCompound, // Multiple references (compound)
    InterGlobalMv, // Global motion
    InterLocalMv,  // Local motion

    // Special modes
    Skip,
    Copy,
    Palette,
    IntraBlockCopy,

    // Unknown
    Unknown,
}

impl PredictionMode {
    pub fn is_intra(&self) -> bool {
        matches!(
            self,
            Self::IntraDc
                | Self::IntraHorizontal
                | Self::IntraVertical
                | Self::IntraDiagonal
                | Self::IntraPaeth
                | Self::IntraSmooth
                | Self::IntraOther(_)
                | Self::IntraBlockCopy
                | Self::Palette
        )
    }

    pub fn is_inter(&self) -> bool {
        matches!(
            self,
            Self::InterSingle | Self::InterCompound | Self::InterGlobalMv | Self::InterLocalMv
        )
    }

    pub fn is_skip(&self) -> bool {
        matches!(self, Self::Skip | Self::Copy)
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            Self::IntraDc => "INTRA_DC",
            Self::IntraHorizontal => "INTRA_H",
            Self::IntraVertical => "INTRA_V",
            Self::IntraDiagonal => "INTRA_D",
            Self::IntraPaeth => "PAETH",
            Self::IntraSmooth => "SMOOTH",
            Self::IntraOther(_) => "INTRA",
            Self::InterSingle => "INTER",
            Self::InterCompound => "COMPOUND",
            Self::InterGlobalMv => "GLOBALMV",
            Self::InterLocalMv => "LOCALMV",
            Self::Skip => "SKIP",
            Self::Copy => "COPY",
            Self::Palette => "PALETTE",
            Self::IntraBlockCopy => "IBC",
            Self::Unknown => "UNKNOWN",
        }
    }
}

/// Transform type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TransformType {
    Dct,
    Adst,
    FlipAdst,
    Identity,
    Wht,
    Mixed(u8, u8), // (row_type, col_type)
    Unknown,
}

// ═══════════════════════════════════════════════════════════════════════════
// Motion Vector
// ═══════════════════════════════════════════════════════════════════════════

/// Motion vector with sub-pixel precision
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct MotionVector {
    /// X component in quarter-pel units
    pub x: i16,
    /// Y component in quarter-pel units
    pub y: i16,
    /// Reference frame index
    pub ref_frame: u8,
}

impl MotionVector {
    pub fn new(x: i16, y: i16, ref_frame: u8) -> Self {
        Self { x, y, ref_frame }
    }

    /// Get pixel-space coordinates (divide by 4 for quarter-pel)
    pub fn to_pixels(&self) -> (f32, f32) {
        (self.x as f32 / 4.0, self.y as f32 / 4.0)
    }

    /// Get magnitude in quarter-pel units
    pub fn magnitude(&self) -> f32 {
        ((self.x as f32).powi(2) + (self.y as f32).powi(2)).sqrt()
    }

    /// Get magnitude in pixels
    pub fn magnitude_pixels(&self) -> f32 {
        self.magnitude() / 4.0
    }

    /// Get angle in radians
    pub fn angle(&self) -> f32 {
        (self.y as f32).atan2(self.x as f32)
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Spatial Hierarchy Elements
// ═══════════════════════════════════════════════════════════════════════════

/// Block-level evidence (leaf node in hierarchy)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockEvidence {
    /// Unique block ID
    pub id: EvidenceId,

    /// Block index within parent
    pub block_idx: u32,

    /// Position and size in coded coordinates
    pub coded_rect: CodedRect,

    /// Prediction mode
    pub mode: PredictionMode,

    /// Quantization parameter
    pub qp: i8,

    /// Motion vector for L0 (forward)
    pub mv_l0: Option<MotionVector>,

    /// Motion vector for L1 (backward)
    pub mv_l1: Option<MotionVector>,

    /// Reference frame indices used
    pub ref_frames: Vec<u8>,

    /// Transform type
    pub transform_type: TransformType,

    /// Transform size (may differ from block size)
    pub transform_size: Option<(u8, u8)>,

    /// Bits consumed for this block
    pub bits_consumed: u32,

    /// Link to parent CTU
    pub ctu_link: EvidenceId,

    /// Link to syntax evidence
    pub syntax_link: EvidenceId,
}

impl BlockEvidence {
    pub fn new(
        id: EvidenceId,
        block_idx: u32,
        coded_rect: CodedRect,
        mode: PredictionMode,
        ctu_link: EvidenceId,
        syntax_link: EvidenceId,
    ) -> Self {
        Self {
            id,
            block_idx,
            coded_rect,
            mode,
            qp: 0,
            mv_l0: None,
            mv_l1: None,
            ref_frames: Vec::new(),
            transform_type: TransformType::Unknown,
            transform_size: None,
            bits_consumed: 0,
            ctu_link,
            syntax_link,
        }
    }

    pub fn contains_point(&self, x: u32, y: u32) -> bool {
        self.coded_rect.contains(x, y)
    }
}

/// CTU (Coding Tree Unit) / Superblock evidence
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CtuEvidence {
    /// Unique CTU ID
    pub id: EvidenceId,

    /// CTU index within tile/frame
    pub ctu_idx: u32,

    /// Position and size in coded coordinates
    pub coded_rect: CodedRect,

    /// Maximum split depth reached
    pub max_split_depth: u8,

    /// Total bits consumed
    pub total_bits: u32,

    /// Child blocks
    pub blocks: Vec<BlockEvidence>,

    /// Link to parent tile
    pub tile_link: EvidenceId,

    /// Link to syntax evidence
    pub syntax_link: EvidenceId,

    /// Statistics
    pub stats: CtuStats,
}

/// CTU statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CtuStats {
    pub intra_block_count: u32,
    pub inter_block_count: u32,
    pub skip_block_count: u32,
    pub avg_qp: f32,
    pub min_qp: i8,
    pub max_qp: i8,
}

impl CtuEvidence {
    pub fn new(
        id: EvidenceId,
        ctu_idx: u32,
        coded_rect: CodedRect,
        tile_link: EvidenceId,
        syntax_link: EvidenceId,
    ) -> Self {
        Self {
            id,
            ctu_idx,
            coded_rect,
            max_split_depth: 0,
            total_bits: 0,
            blocks: Vec::new(),
            tile_link,
            syntax_link,
            stats: CtuStats::default(),
        }
    }

    pub fn add_block(&mut self, block: BlockEvidence) {
        self.blocks.push(block);
        self.recompute_stats();
    }

    pub fn recompute_stats(&mut self) {
        let mut stats = CtuStats::default();
        let mut qp_sum = 0i32;
        let mut qp_count = 0u32;

        for block in &self.blocks {
            if block.mode.is_intra() {
                stats.intra_block_count += 1;
            } else if block.mode.is_inter() {
                stats.inter_block_count += 1;
            } else if block.mode.is_skip() {
                stats.skip_block_count += 1;
            }

            qp_sum += block.qp as i32;
            qp_count += 1;

            if qp_count == 1 || block.qp < stats.min_qp {
                stats.min_qp = block.qp;
            }
            if qp_count == 1 || block.qp > stats.max_qp {
                stats.max_qp = block.qp;
            }
        }

        if qp_count > 0 {
            stats.avg_qp = qp_sum as f32 / qp_count as f32;
        }

        self.stats = stats;
    }

    pub fn find_block_at(&self, x: u32, y: u32) -> Option<&BlockEvidence> {
        self.blocks.iter().find(|b| b.contains_point(x, y))
    }

    pub fn contains_point(&self, x: u32, y: u32) -> bool {
        self.coded_rect.contains(x, y)
    }
}

/// Tile evidence
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TileEvidence {
    /// Unique tile ID
    pub id: EvidenceId,

    /// Tile index within frame
    pub tile_idx: u32,

    /// Tile row and column
    pub tile_row: u32,
    pub tile_col: u32,

    /// Position and size in coded coordinates
    pub coded_rect: CodedRect,

    /// Total bits consumed
    pub total_bits: u32,

    /// CTU count
    pub ctu_count: u32,

    /// Child CTUs
    pub ctus: Vec<CtuEvidence>,

    /// Link to parent frame
    pub frame_link: EvidenceId,

    /// Link to syntax evidence
    pub syntax_link: EvidenceId,

    /// Statistics
    pub stats: TileStats,
}

/// Tile statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TileStats {
    pub block_count: u32,
    pub intra_ratio: f32,
    pub inter_ratio: f32,
    pub skip_ratio: f32,
    pub avg_qp: f32,
    pub bits_per_pixel: f32,
}

impl TileEvidence {
    pub fn new(
        id: EvidenceId,
        tile_idx: u32,
        tile_row: u32,
        tile_col: u32,
        coded_rect: CodedRect,
        frame_link: EvidenceId,
        syntax_link: EvidenceId,
    ) -> Self {
        Self {
            id,
            tile_idx,
            tile_row,
            tile_col,
            coded_rect,
            total_bits: 0,
            ctu_count: 0,
            ctus: Vec::new(),
            frame_link,
            syntax_link,
            stats: TileStats::default(),
        }
    }

    pub fn add_ctu(&mut self, ctu: CtuEvidence) {
        self.ctus.push(ctu);
        self.ctu_count = self.ctus.len() as u32;
        self.recompute_stats();
    }

    pub fn recompute_stats(&mut self) {
        let mut stats = TileStats::default();
        let mut total_intra = 0u32;
        let mut total_inter = 0u32;
        let mut total_skip = 0u32;
        let mut qp_sum = 0f32;
        let mut total_bits = 0u32;

        for ctu in &self.ctus {
            stats.block_count += ctu.blocks.len() as u32;
            total_intra += ctu.stats.intra_block_count;
            total_inter += ctu.stats.inter_block_count;
            total_skip += ctu.stats.skip_block_count;
            qp_sum += ctu.stats.avg_qp * ctu.blocks.len() as f32;
            total_bits += ctu.total_bits;
        }

        if stats.block_count > 0 {
            stats.intra_ratio = total_intra as f32 / stats.block_count as f32;
            stats.inter_ratio = total_inter as f32 / stats.block_count as f32;
            stats.skip_ratio = total_skip as f32 / stats.block_count as f32;
            stats.avg_qp = qp_sum / stats.block_count as f32;
        }

        let pixels = self.coded_rect.area();
        if pixels > 0 {
            stats.bits_per_pixel = total_bits as f32 / pixels as f32;
        }

        self.total_bits = total_bits;
        self.stats = stats;
    }

    pub fn find_ctu_at(&self, x: u32, y: u32) -> Option<&CtuEvidence> {
        self.ctus.iter().find(|c| c.contains_point(x, y))
    }

    pub fn find_block_at(&self, x: u32, y: u32) -> Option<(&CtuEvidence, &BlockEvidence)> {
        for ctu in &self.ctus {
            if let Some(block) = ctu.find_block_at(x, y) {
                return Some((ctu, block));
            }
        }
        None
    }

    pub fn contains_point(&self, x: u32, y: u32) -> bool {
        self.coded_rect.contains(x, y)
    }
}

/// Frame-level spatial hierarchy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrameSpatialHierarchy {
    /// Unique frame hierarchy ID
    pub id: EvidenceId,

    /// Frame display index
    pub display_idx: u64,

    /// Frame decode index
    pub decode_idx: u64,

    /// Frame dimensions
    pub width: u32,
    pub height: u32,

    /// Tile grid dimensions
    pub tile_rows: u32,
    pub tile_cols: u32,

    /// Child tiles
    pub tiles: Vec<TileEvidence>,

    /// Link to decode evidence
    pub decode_link: EvidenceId,

    /// Total bits for frame
    pub total_bits: u32,

    /// Statistics
    pub stats: FrameSpatialStats,
}

/// Frame spatial statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FrameSpatialStats {
    pub tile_count: u32,
    pub ctu_count: u32,
    pub block_count: u32,
    pub intra_ratio: f32,
    pub inter_ratio: f32,
    pub skip_ratio: f32,
    pub avg_qp: f32,
    pub bits_per_pixel: f32,
}

impl FrameSpatialHierarchy {
    pub fn new(
        id: EvidenceId,
        display_idx: u64,
        decode_idx: u64,
        width: u32,
        height: u32,
        decode_link: EvidenceId,
    ) -> Self {
        Self {
            id,
            display_idx,
            decode_idx,
            width,
            height,
            tile_rows: 1,
            tile_cols: 1,
            tiles: Vec::new(),
            decode_link,
            total_bits: 0,
            stats: FrameSpatialStats::default(),
        }
    }

    pub fn add_tile(&mut self, tile: TileEvidence) {
        self.tiles.push(tile);
        self.recompute_stats();
    }

    pub fn recompute_stats(&mut self) {
        let mut stats = FrameSpatialStats::default();
        let mut total_bits = 0u32;
        let mut qp_sum = 0f32;

        for tile in &self.tiles {
            stats.tile_count += 1;
            stats.ctu_count += tile.ctu_count;
            stats.block_count += tile.stats.block_count;
            qp_sum += tile.stats.avg_qp * tile.stats.block_count as f32;
            total_bits += tile.total_bits;
        }

        if stats.block_count > 0 {
            let mut total_intra = 0u32;
            let mut total_inter = 0u32;
            let mut total_skip = 0u32;

            for tile in &self.tiles {
                for ctu in &tile.ctus {
                    total_intra += ctu.stats.intra_block_count;
                    total_inter += ctu.stats.inter_block_count;
                    total_skip += ctu.stats.skip_block_count;
                }
            }

            stats.intra_ratio = total_intra as f32 / stats.block_count as f32;
            stats.inter_ratio = total_inter as f32 / stats.block_count as f32;
            stats.skip_ratio = total_skip as f32 / stats.block_count as f32;
            stats.avg_qp = qp_sum / stats.block_count as f32;
        }

        let pixels = self.width as u64 * self.height as u64;
        if pixels > 0 {
            stats.bits_per_pixel = total_bits as f32 / pixels as f32;
        }

        self.total_bits = total_bits;
        self.stats = stats;
    }

    /// Find tile at coded coordinates
    pub fn find_tile_at(&self, x: u32, y: u32) -> Option<&TileEvidence> {
        self.tiles.iter().find(|t| t.contains_point(x, y))
    }

    /// Find CTU at coded coordinates
    pub fn find_ctu_at(&self, x: u32, y: u32) -> Option<(&TileEvidence, &CtuEvidence)> {
        for tile in &self.tiles {
            if let Some(ctu) = tile.find_ctu_at(x, y) {
                return Some((tile, ctu));
            }
        }
        None
    }

    /// Find block at coded coordinates
    pub fn find_block_at(&self, x: u32, y: u32) -> Option<SpatialHit<'_>> {
        for tile in &self.tiles {
            if let Some((ctu, block)) = tile.find_block_at(x, y) {
                return Some(SpatialHit { tile, ctu, block });
            }
        }
        None
    }
}

/// Result of spatial lookup
#[derive(Debug)]
pub struct SpatialHit<'a> {
    pub tile: &'a TileEvidence,
    pub ctu: &'a CtuEvidence,
    pub block: &'a BlockEvidence,
}

// ═══════════════════════════════════════════════════════════════════════════
// Spatial Hierarchy Index
// ═══════════════════════════════════════════════════════════════════════════

/// Index for spatial hierarchy evidence
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SpatialHierarchyIndex {
    frames: Vec<FrameSpatialHierarchy>,
}

impl SpatialHierarchyIndex {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add(&mut self, frame: FrameSpatialHierarchy) {
        self.frames.push(frame);
        self.frames.sort_by_key(|f| f.display_idx);
    }

    pub fn find_by_id(&self, id: &str) -> Option<&FrameSpatialHierarchy> {
        self.frames.iter().find(|f| f.id == id)
    }

    pub fn find_by_display_idx(&self, display_idx: u64) -> Option<&FrameSpatialHierarchy> {
        self.frames.iter().find(|f| f.display_idx == display_idx)
    }

    pub fn find_by_decode_link(&self, decode_id: &str) -> Option<&FrameSpatialHierarchy> {
        self.frames.iter().find(|f| f.decode_link == decode_id)
    }

    pub fn len(&self) -> usize {
        self.frames.len()
    }

    pub fn is_empty(&self) -> bool {
        self.frames.is_empty()
    }

    pub fn all(&self) -> &[FrameSpatialHierarchy] {
        &self.frames
    }
}

// ============================================================================
// Tests
// ============================================================================
#[cfg(test)]
include!("spatial_hierarchy_test.rs");
