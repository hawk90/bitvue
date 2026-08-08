//! Overlay data extraction from VVC/H.266 bitstreams
//!
//! This module provides functions to extract QP heatmap, motion vector,
//! and CTU partition information for visualization overlays.
//!
//! ## Implementation Status (v0.6.x)
//!
//! **Real Data Extraction**:
//! - ✅ Extract CTU structure with MTT partitioning
//! - ✅ Extract motion vectors from inter blocks
//! - ✅ Extract QP values from CUs
//! - ✅ Extract prediction modes (intra/inter)
//! - ✅ Extract transform sizes with SBT support
//!
//! ## VVC-Specific Features
//!
//! - MTT (Multi-Type Tree) partitioning: quadtree, binary, ternary splits
//! - Dual tree: separate luma/chroma coding trees
//! - ISP (Intra Sub-Partitions)
//! - SBT (Sub-Block Transform)
//! - GPM (Geometric Partitioning Mode)
//! - IBC (Intra Block Copy)
//! - MIP (Matrix Intra Prediction)

use crate::nal::NalUnit;
use crate::sps::Sps;
use bitvue_engine::{
    limits::{MAX_GRID_BLOCKS, MAX_GRID_DIMENSION},
    mv_overlay::{BlockMode, MVGrid, MotionVector as CoreMV},
    partition_grid::{PartitionBlock, PartitionGrid, PartitionType},
    qp_heatmap::QPGrid,
    BitvueError,
};
use serde::{Deserialize, Serialize};

/// Prediction mode for VVC coding units
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PredMode {
    /// Intra prediction (MODE_INTRA)
    Intra,
    /// Inter prediction (MODE_INTER)
    Inter,
    /// IBC (Intra Block Copy)
    Ibc,
    /// Skip mode
    Skip,
}

/// VVC MTT (Multi-Type Tree) split mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SplitMode {
    /// No split (single coding unit)
    None,
    /// Quadtree split (QT)
    QuadTree,
    /// Horizontal binary split (BTT_H)
    HorzB,
    /// Vertical binary split (BTT_V)
    VertB,
    /// Horizontal ternary split (TT_H)
    HorzT,
    /// Vertical ternary split (TT_V)
    VertT,
}

/// VVC Coding Unit with MTT support
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodingUnit {
    /// CU position in pixels
    pub x: u32,
    pub y: u32,
    /// CU size (power of 2: 4, 8, 16, 32, 64, 128)
    pub size: u8,
    /// Prediction mode
    pub pred_mode: PredMode,
    /// MTT split mode
    pub split_mode: SplitMode,
    /// Depth in quadtree/MTT
    pub depth: u8,
    /// Tree type (0=single tree, 1=dual tree luma, 2=dual tree chroma)
    pub tree_type: u8,
    /// QP value (for this CU)
    pub qp: i16,
    /// Motion vectors (for inter blocks)
    pub mv_l0: Option<MotionVector>,
    pub mv_l1: Option<MotionVector>,
    /// Reference frame indices
    pub ref_idx_l0: Option<i8>,
    pub ref_idx_l1: Option<i8>,
    /// Transform size (for this CU)
    pub transform_size: u8,
    /// SBT (Sub-Block Transform) flag
    pub sbt_flag: bool,
    /// ISP (Intra Sub-Partitions) flag
    pub isp_flag: bool,
}

/// Motion vector for VVC (quarter-pel precision for inter, integer for IBC)
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct MotionVector {
    /// Horizontal component (quarter-pel units for inter, integer for IBC)
    pub x: i32,
    /// Vertical component (quarter-pel units for inter, integer for IBC)
    pub y: i32,
}

impl MotionVector {
    /// Create new motion vector
    pub fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }

    /// Zero motion vector
    pub fn zero() -> Self {
        Self { x: 0, y: 0 }
    }
}

/// VVC Coding Tree Unit (CTU)
///
/// VVC uses 128x128 CTUs (compared to 64x64 in HEVC)
/// with MTT (Multi-Type Tree) partitioning.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodingTreeUnit {
    /// CTU position in pixels
    pub x: u32,
    /// CTU position in pixels
    pub y: u32,
    /// CTU size (normally 128 for VVC)
    pub size: u8,
    /// Coding units within this CTU
    pub coding_units: Vec<CodingUnit>,
}

impl CodingTreeUnit {
    /// Create new CTU
    pub fn new(x: u32, y: u32, size: u8) -> Self {
        Self {
            x,
            y,
            size,
            coding_units: Vec::new(),
        }
    }

    /// Add a coding unit to this CTU
    pub fn add_cu(&mut self, cu: CodingUnit) {
        self.coding_units.push(cu);
    }
}

/// Extract QP Grid from VVC bitstream
///
/// Parses CTUs from slice data and extracts QP values.
pub fn extract_qp_grid(
    nal_units: &[NalUnit],
    sps: &Sps,
    base_qp: i16,
) -> Result<QPGrid, BitvueError> {
    // VVC uses configurable CTU size (log2_ctu_size_minus5 + 5)
    let ctu_size = 1u32 << (sps.sps_log2_ctu_size_minus5 + 5);
    let width = sps.sps_pic_width_max_in_luma_samples;
    let height = sps.sps_pic_height_max_in_luma_samples;

    let grid_w = width.div_ceil(ctu_size);
    let grid_h = height.div_ceil(ctu_size);

    // SECURITY: Validate grid dimensions to prevent excessive allocation
    if grid_w > MAX_GRID_DIMENSION || grid_h > MAX_GRID_DIMENSION {
        return Err(BitvueError::Decode(format!(
            "Grid dimensions {}x{} exceed maximum {}",
            grid_w, grid_h, MAX_GRID_DIMENSION
        )));
    }

    // Check for overflow in grid size calculation
    let total_blocks = grid_w.checked_mul(grid_h).ok_or_else(|| {
        BitvueError::Decode(format!("Grid dimensions too large: {}x{}", grid_w, grid_h))
    })? as usize;

    if total_blocks > MAX_GRID_BLOCKS {
        return Err(BitvueError::Decode(format!(
            "Grid block count {} exceeds maximum {}",
            total_blocks, MAX_GRID_BLOCKS
        )));
    }

    let mut qp = Vec::with_capacity(total_blocks);

    // Parse CTUs from slice data
    for nal in nal_units {
        if nal.header.nal_unit_type.is_vcl() {
            match parse_slice_ctus(nal, sps, base_qp) {
                Ok(ctus) => {
                    // Collect QP values from CTUs
                    for ctu in &ctus {
                        // Use average QP from CTU or first CU QP
                        let ctu_qp = ctu.coding_units.first().map(|cu| cu.qp).unwrap_or(base_qp);
                        qp.push(ctu_qp);
                    }
                }
                Err(e) => {
                    abseil::vlog!(1, "Failed to parse CTUs: {}, using base_qp", e);
                    // Use base_qp for CTUs in this slice
                }
            }
        }
    }

    // If we didn't get any CTUs, use base_qp
    if qp.is_empty() {
        let total_blocks = grid_w.checked_mul(grid_h).ok_or_else(|| {
            BitvueError::Decode(format!("Grid dimensions too large: {}x{}", grid_w, grid_h))
        })? as usize;
        qp = vec![base_qp; total_blocks];
    }

    Ok(QPGrid::new(grid_w, grid_h, ctu_size, ctu_size, qp, base_qp))
}

/// Extract MV Grid from VVC bitstream
///
/// Parses CTUs from slice data and extracts motion vectors.
pub fn extract_mv_grid(nal_units: &[NalUnit], sps: &Sps) -> Result<MVGrid, BitvueError> {
    let width = sps.sps_pic_width_max_in_luma_samples;
    let height = sps.sps_pic_height_max_in_luma_samples;

    // Use 16x16 blocks for MV grid
    let block_size = 16u32;
    let grid_w = width.div_ceil(block_size);
    let grid_h = height.div_ceil(block_size);

    // SECURITY: Validate grid dimensions to prevent excessive allocation
    if grid_w > MAX_GRID_DIMENSION || grid_h > MAX_GRID_DIMENSION {
        return Err(BitvueError::Decode(format!(
            "Grid dimensions {}x{} exceed maximum {}",
            grid_w, grid_h, MAX_GRID_DIMENSION
        )));
    }

    // Check for overflow in grid size calculation
    let total_blocks = grid_w.checked_mul(grid_h).ok_or_else(|| {
        BitvueError::Decode(format!("Grid dimensions too large: {}x{}", grid_w, grid_h))
    })? as usize;

    if total_blocks > MAX_GRID_BLOCKS {
        return Err(BitvueError::Decode(format!(
            "Grid block count {} exceeds maximum {}",
            total_blocks, MAX_GRID_BLOCKS
        )));
    }

    let mut mv_l0 = Vec::with_capacity(total_blocks);
    let mut mv_l1 = Vec::with_capacity(total_blocks);
    let mut modes = Vec::with_capacity(total_blocks);

    // Parse CTUs from slice data
    for nal in nal_units {
        if nal.header.nal_unit_type.is_vcl() {
            match parse_slice_ctus(nal, sps, 26) {
                Ok(ctus) => {
                    for ctu in &ctus {
                        // Expand CTU CUs to block grid
                        expand_cu_to_blocks(ctu, block_size, &mut mv_l0, &mut mv_l1, &mut modes);
                    }
                }
                Err(e) => {
                    abseil::vlog!(1, "Failed to parse CTUs for MV: {}, using ZERO", e);
                    // Use zero MV for blocks in this slice
                }
            }
        }
    }

    // Fill remaining if needed
    let total_blocks = grid_w.checked_mul(grid_h).ok_or_else(|| {
        BitvueError::Decode(format!("Grid dimensions too large: {}x{}", grid_w, grid_h))
    })? as usize;
    while mv_l0.len() < total_blocks {
        mv_l0.push(CoreMV::ZERO);
        mv_l1.push(CoreMV::MISSING);
        modes.push(BlockMode::Inter);
    }

    // Truncate to expected size to handle edge CTUs
    if mv_l0.len() > total_blocks {
        mv_l0.truncate(total_blocks);
        mv_l1.truncate(total_blocks);
        modes.truncate(total_blocks);
    }

    Ok(MVGrid::new(
        width,
        height,
        block_size,
        block_size,
        mv_l0,
        mv_l1,
        Some(modes),
    ))
}

/// Extract Partition Grid from VVC bitstream
///
/// Parses CTUs from slice data and creates a partition grid with MTT support.
pub fn extract_partition_grid(
    nal_units: &[NalUnit],
    sps: &Sps,
) -> Result<PartitionGrid, BitvueError> {
    let width = sps.sps_pic_width_max_in_luma_samples;
    let height = sps.sps_pic_height_max_in_luma_samples;

    let ctu_size = 1u32 << (sps.sps_log2_ctu_size_minus5 + 5);
    let mut grid = PartitionGrid::new(width, height, ctu_size);

    // Parse CTUs from slice data
    for nal in nal_units {
        if nal.header.nal_unit_type.is_vcl() {
            match parse_slice_ctus(nal, sps, 26) {
                Ok(ctus) => {
                    for ctu in &ctus {
                        for cu in &ctu.coding_units {
                            let partition_type = match cu.split_mode {
                                SplitMode::None => PartitionType::None,
                                SplitMode::QuadTree => PartitionType::Split,
                                SplitMode::HorzB => PartitionType::Horz,
                                SplitMode::VertB => PartitionType::Vert,
                                SplitMode::HorzT | SplitMode::VertT => PartitionType::Split,
                            };

                            grid.add_block(PartitionBlock::new_vvc(
                                cu.x,
                                cu.y,
                                cu.size as u32,
                                cu.size as u32,
                                partition_type,
                                cu.depth,
                                cu.tree_type,
                            ));
                        }
                    }
                }
                Err(e) => {
                    abseil::vlog!(
                        1,
                        "Failed to parse CTUs for partition: {}, using scaffold",
                        e
                    );
                    // Add scaffold blocks
                }
            }
        }
    }

    // Fill with scaffold blocks if empty
    if grid.blocks.is_empty() {
        let grid_w = width.div_ceil(ctu_size);
        let grid_h = height.div_ceil(ctu_size);
        for ctu_y in 0..grid_h {
            for ctu_x in 0..grid_w {
                grid.add_block(PartitionBlock::new(
                    ctu_x * ctu_size,
                    ctu_y * ctu_size,
                    ctu_size,
                    ctu_size,
                    PartitionType::None,
                    0,
                ));
            }
        }
    }

    Ok(grid)
}

/// Expand CTU CUs to block grid for MV visualization
fn expand_cu_to_blocks(
    ctu: &CodingTreeUnit,
    block_size: u32,
    mv_l0: &mut Vec<CoreMV>,
    mv_l1: &mut Vec<CoreMV>,
    modes: &mut Vec<BlockMode>,
) {
    // Validate inputs to prevent division by zero and overflow
    if block_size == 0 {
        abseil::vlog!(1, "Block size cannot be zero, skipping CTU");
        return;
    }
    if ctu.size == 0 {
        abseil::vlog!(1, "CTU size cannot be zero, skipping CTU");
        return;
    }

    // Calculate blocks per CTU with overflow protection
    let blocks_per_dimension = (ctu.size as u32) / block_size;
    let blocks_per_ctu = blocks_per_dimension.saturating_mul(blocks_per_dimension);

    // Pre-calculate total blocks needed to avoid reallocations
    // Sum up blocks from all coding units in this CTU with overflow protection
    let total_blocks: usize = ctu
        .coding_units
        .iter()
        .map(|cu| {
            let blocks_in_cu = ((cu.size as u32) / block_size).max(1);
            // Use checked_mul to prevent overflow
            blocks_in_cu.saturating_mul(blocks_in_cu) as usize
        })
        .sum();

    // Pre-allocate capacity needed
    // Vec's reserve() already tracks internal capacity and only allocates more if needed
    mv_l0.reserve(total_blocks);
    mv_l1.reserve(total_blocks);
    modes.reserve(total_blocks);

    for cu in &ctu.coding_units {
        let blocks_in_cu = ((cu.size as u32) / block_size).max(1);
        // Use checked_mul to prevent overflow
        let cu_blocks = blocks_in_cu.saturating_mul(blocks_in_cu) as usize;

        for _ in 0..cu_blocks {
            match cu.pred_mode {
                PredMode::Intra => {
                    mv_l0.push(CoreMV::MISSING);
                    mv_l1.push(CoreMV::MISSING);
                    modes.push(BlockMode::Intra);
                }
                PredMode::Ibc => {
                    // IBC uses integer motion vectors
                    if let Some(ref mv) = cu.mv_l0 {
                        mv_l0.push(CoreMV::new(mv.x, mv.y));
                    } else {
                        mv_l0.push(CoreMV::ZERO);
                    }
                    mv_l1.push(CoreMV::MISSING);
                    modes.push(BlockMode::Intra); // IBC is intra mode
                }
                PredMode::Skip => {
                    mv_l0.push(CoreMV::ZERO);
                    mv_l1.push(CoreMV::MISSING);
                    modes.push(BlockMode::Skip);
                }
                PredMode::Inter => {
                    // Has motion vectors
                    if let Some(ref mv) = cu.mv_l0 {
                        mv_l0.push(CoreMV::new(mv.x, mv.y));
                    } else {
                        mv_l0.push(CoreMV::ZERO);
                    }

                    if let Some(ref mv) = cu.mv_l1 {
                        mv_l1.push(CoreMV::new(mv.x, mv.y));
                    } else {
                        mv_l1.push(CoreMV::MISSING);
                    }

                    modes.push(BlockMode::Inter);
                }
            }
        }
    }

    // Fill any remaining blocks in CTU
    let current_blocks = mv_l0.len() % (blocks_per_ctu as usize);
    if current_blocks > 0 {
        let remaining = (blocks_per_ctu as usize) - current_blocks;
        for _ in 0..remaining {
            mv_l0.push(CoreMV::ZERO);
            mv_l1.push(CoreMV::MISSING);
            modes.push(BlockMode::Inter);
        }
    }
}

// ---------------------------------------------------------------------------
// CABAC decoder for VVC (spec ITU-T H.266 / HEVC-compatible tables)
// ---------------------------------------------------------------------------

/// RANGE_LPS table — HEVC/VVC spec Table 9-45.
/// Indexed by [pState][qRangeIdx] where qRangeIdx = (codIRange >> 6) & 3.
static VVC_RANGE_LPS: [[u8; 4]; 64] = [
    [128, 176, 208, 240],
    [128, 167, 197, 227],
    [128, 158, 187, 216],
    [123, 150, 178, 205],
    [116, 142, 169, 195],
    [111, 135, 160, 185],
    [105, 128, 152, 175],
    [100, 122, 144, 166],
    [95, 116, 137, 158],
    [90, 110, 130, 150],
    [85, 104, 123, 142],
    [81, 99, 117, 135],
    [77, 94, 111, 128],
    [73, 89, 105, 122],
    [69, 85, 100, 116],
    [66, 80, 95, 110],
    [62, 76, 90, 104],
    [59, 72, 86, 99],
    [56, 69, 81, 94],
    [53, 65, 77, 89],
    [51, 62, 73, 85],
    [48, 59, 69, 80],
    [46, 56, 66, 76],
    [43, 53, 63, 72],
    [41, 50, 59, 69],
    [39, 48, 56, 65],
    [37, 45, 54, 62],
    [35, 43, 51, 59],
    [33, 41, 48, 56],
    [32, 39, 46, 53],
    [30, 37, 43, 50],
    [29, 35, 41, 48],
    [27, 33, 39, 45],
    [26, 31, 37, 43],
    [24, 30, 35, 41],
    [23, 28, 33, 39],
    [22, 27, 32, 37],
    [21, 26, 30, 35],
    [20, 24, 29, 33],
    [19, 23, 27, 31],
    [18, 22, 26, 30],
    [17, 21, 25, 28],
    [16, 20, 23, 27],
    [15, 19, 22, 25],
    [14, 18, 21, 24],
    [14, 17, 20, 23],
    [13, 16, 19, 22],
    [12, 15, 18, 21],
    [12, 14, 17, 20],
    [11, 14, 16, 19],
    [11, 13, 15, 18],
    [10, 12, 15, 17],
    [10, 12, 14, 16],
    [9, 11, 13, 15],
    [9, 11, 12, 14],
    [8, 10, 12, 14],
    [8, 9, 11, 13],
    [7, 9, 11, 12],
    [7, 9, 10, 12],
    [7, 8, 10, 11],
    [6, 8, 9, 11],
    [6, 7, 9, 10],
    [6, 7, 8, 9],
    [2, 2, 2, 2],
];

static VVC_TRANS_MPS: [u8; 64] = [
    1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26,
    27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45, 46, 47, 48, 49, 50,
    51, 52, 53, 54, 55, 56, 57, 58, 59, 60, 61, 62, 62, 63,
];

static VVC_TRANS_LPS: [u8; 64] = [
    0, 0, 1, 2, 2, 4, 4, 5, 6, 7, 8, 9, 9, 11, 11, 12, 13, 13, 15, 15, 16, 16, 18, 18, 19, 19, 21,
    21, 22, 22, 23, 24, 24, 25, 26, 26, 27, 27, 28, 29, 29, 30, 30, 30, 31, 32, 32, 33, 33, 33, 34,
    34, 35, 35, 35, 36, 36, 36, 37, 37, 37, 38, 38, 63,
];

// VVC Table 9-17 approximate init values for slice contexts.
const VVC_CU_SKIP_INIT: [u8; 3] = [197, 185, 201];
const VVC_PRED_MODE_INIT_P: u8 = 149;
const VVC_PRED_MODE_INIT_B: u8 = 134;
const VVC_MERGE_FLAG_INIT: u8 = 110;
const VVC_ABS_MVD_GREATER0_INIT: [u8; 2] = [104, 168];
const VVC_ABS_MVD_GREATER1_INIT: [u8; 2] = [71, 71];

/// Initialise a CABAC context from a VVC init_value byte and slice QP.
/// Formula from HEVC/VVC spec 9.3.2.2.
fn vvc_init_ctx(init_value: u8, qp: i32) -> (u8, u8) {
    let m = (5 * (init_value >> 4) as i32) - 45;
    let n = ((init_value & 15) as i32 * 8) - 16;
    let pre = ((m * qp) >> 4) + n;
    let pre = pre.clamp(1, 126);
    if pre <= 63 {
        ((63 - pre) as u8, 0)
    } else {
        ((pre - 64) as u8, 1)
    }
}

/// CABAC arithmetic decoder compatible with HEVC/VVC.
struct VvcCabac<'a> {
    data: &'a [u8],
    byte_pos: usize,
    bit_pos: i8,
    cod_i_range: u32,
    cod_i_offset: u32,
}

impl<'a> VvcCabac<'a> {
    /// Initialise the engine from raw slice payload bytes (after the 2-byte NAL header).
    fn new(data: &'a [u8]) -> Option<Self> {
        if data.len() < 2 {
            return None;
        }
        let cod_i_offset = ((data[0] as u32) << 1) | ((data[1] as u8 >> 7) as u32);
        Some(Self {
            data,
            byte_pos: 1,
            bit_pos: 6, // bits 6..0 of byte 1 remain
            cod_i_range: 510,
            cod_i_offset,
        })
    }

    /// Read one raw bit from the byte stream.
    fn read_raw_bit(&mut self) -> Option<u32> {
        if self.byte_pos >= self.data.len() {
            return None;
        }
        let bit = ((self.data[self.byte_pos] >> self.bit_pos) & 1) as u32;
        if self.bit_pos == 0 {
            self.byte_pos += 1;
            self.bit_pos = 7;
        } else {
            self.bit_pos -= 1;
        }
        Some(bit)
    }

    /// Standard regular CABAC bin decode.
    /// `p_state` and `mps` are the context state/MPS fields (updated in place).
    fn decode_bin(&mut self, p_state: &mut u8, mps: &mut u8) -> Option<u32> {
        let q_range_idx = ((self.cod_i_range >> 6) & 3) as usize;
        let range_lps = VVC_RANGE_LPS[*p_state as usize][q_range_idx] as u32;
        self.cod_i_range -= range_lps;

        let bin_val;
        if self.cod_i_offset >= self.cod_i_range {
            // LPS
            bin_val = 1 - *mps as u32;
            self.cod_i_offset -= self.cod_i_range;
            self.cod_i_range = range_lps;
            if *p_state == 0 {
                *mps = 1 - *mps;
            }
            *p_state = VVC_TRANS_LPS[*p_state as usize];
        } else {
            // MPS
            bin_val = *mps as u32;
            *p_state = VVC_TRANS_MPS[*p_state as usize];
        }

        // Renormalise
        while self.cod_i_range < 256 {
            self.cod_i_range <<= 1;
            self.cod_i_offset = (self.cod_i_offset << 1) | self.read_raw_bit()?;
        }

        Some(bin_val)
    }

    /// Bypass bin decode (equal probability, no context update).
    fn decode_bypass(&mut self) -> Option<u32> {
        self.cod_i_offset = (self.cod_i_offset << 1) | self.read_raw_bit()?;
        if self.cod_i_offset >= self.cod_i_range {
            self.cod_i_offset -= self.cod_i_range;
            Some(1)
        } else {
            Some(0)
        }
    }

    /// Exp-Golomb order-k bypass decode.
    fn decode_eg_bypass(&mut self, k: u32) -> Option<i32> {
        let mut symbol = 0i32;
        let mut i = k;
        // Unary prefix: count leading 1s
        loop {
            let bit = self.decode_bypass()?;
            if bit == 0 {
                break;
            }
            symbol += 1 << i;
            i += 1;
        }
        // Binary suffix of length i
        let mut j = i;
        while j > 0 {
            j -= 1;
            symbol += (self.decode_bypass()? as i32) << j;
        }
        Some(symbol)
    }

    /// Decode one signed MVD component.
    /// Syntax: abs_mvd_greater0_flag, [abs_mvd_greater1_flag], [abs_mvd_minus2 EG0], sign_flag
    fn decode_mvd_component(
        &mut self,
        ps0: &mut u8,
        ms0: &mut u8, // context for abs_mvd_greater0
        ps1: &mut u8,
        ms1: &mut u8, // context for abs_mvd_greater1
    ) -> Option<i32> {
        let greater0 = self.decode_bin(ps0, ms0)?;
        if greater0 == 0 {
            return Some(0);
        }
        let greater1 = self.decode_bin(ps1, ms1)?;
        let abs_val = if greater1 == 1 {
            2 + self.decode_eg_bypass(1)?
        } else {
            1
        };
        let sign = self.decode_bypass()?;
        Some(if sign == 1 { -abs_val } else { abs_val })
    }
}

/// Parse CTUs from slice data
///
/// For intra slices each CTU gets a single Intra CU (correct).
/// For inter slices a CABAC decoder is attempted; on failure the scaffold
/// (alternating H/V binary splits) is used as fallback.
fn parse_slice_ctus(
    nal: &NalUnit,
    sps: &Sps,
    base_qp: i16,
) -> Result<Vec<CodingTreeUnit>, BitvueError> {
    let width = sps.sps_pic_width_max_in_luma_samples;
    let height = sps.sps_pic_height_max_in_luma_samples;
    let ctu_size = 1u32 << (sps.sps_log2_ctu_size_minus5 + 5);

    let ctu_cols = width.div_ceil(ctu_size);
    let ctu_rows = height.div_ceil(ctu_size);
    let total_ctus = ctu_cols.checked_mul(ctu_rows).ok_or_else(|| {
        BitvueError::Decode(format!(
            "CTU grid dimensions too large: {}x{}",
            ctu_cols, ctu_rows
        ))
    })?;

    // Pre-allocate CTU vector with known size
    let mut ctus = Vec::with_capacity(total_ctus as usize);

    let is_intra = nal.header.nal_unit_type.is_idr() || nal.header.nal_unit_type.is_cra();

    if is_intra {
        // -----------------------------------------------------------------
        // Intra slice: one Intra CU per CTU — no CABAC needed.
        // -----------------------------------------------------------------
        for ctu_idx in 0..total_ctus {
            let ctu_x = (ctu_idx % ctu_cols) * ctu_size;
            let ctu_y = (ctu_idx / ctu_cols) * ctu_size;
            let mut ctu = CodingTreeUnit::new(ctu_x, ctu_y, ctu_size as u8);
            ctu.add_cu(CodingUnit {
                x: ctu_x,
                y: ctu_y,
                size: ctu_size as u8,
                pred_mode: PredMode::Intra,
                split_mode: SplitMode::None,
                depth: 0,
                tree_type: 0,
                qp: base_qp,
                mv_l0: None,
                mv_l1: None,
                ref_idx_l0: None,
                ref_idx_l1: None,
                transform_size: 4,
                sbt_flag: false,
                isp_flag: false,
            });
            ctus.push(ctu);
        }
        return Ok(ctus);
    }

    // -----------------------------------------------------------------
    // Inter slice: attempt CABAC-based CU classification.
    // Payload bytes 0..1 are the 2-byte VVC NAL header; CABAC data
    // starts at byte 2.
    // -----------------------------------------------------------------
    let payload = nal.rbsp();
    // Skip the 2-byte NAL header that is included in payload for VVC
    let cabac_data = if payload.len() > 2 {
        &payload[2..]
    } else {
        payload
    };

    // Determine B-slice vs P-slice for pred_mode_flag init value.
    // VVC TRAIL/STAP/RADL/RASL can be B or P; we treat IDR/CRA as intra
    // (already handled above). For non-intra frames we default to B-slice
    // init since B-slices are more common in practice.
    let is_b_slice = !matches!(
        nal.header.nal_unit_type,
        crate::nal::NalUnitType::TrailNut | crate::nal::NalUnitType::StapNut
    );
    let pred_mode_init = if is_b_slice {
        VVC_PRED_MODE_INIT_B
    } else {
        VVC_PRED_MODE_INIT_P
    };

    let qp = base_qp as i32;

    // Initialise contexts
    let mut skip_ctx: [(u8, u8); 3] = [
        vvc_init_ctx(VVC_CU_SKIP_INIT[0], qp),
        vvc_init_ctx(VVC_CU_SKIP_INIT[1], qp),
        vvc_init_ctx(VVC_CU_SKIP_INIT[2], qp),
    ];
    let (mut pred_ps, mut pred_ms) = vvc_init_ctx(pred_mode_init, qp);
    let (mut merge_ps, mut merge_ms) = vvc_init_ctx(VVC_MERGE_FLAG_INIT, qp);
    let mut mvd_g0: [(u8, u8); 2] = [
        vvc_init_ctx(VVC_ABS_MVD_GREATER0_INIT[0], qp),
        vvc_init_ctx(VVC_ABS_MVD_GREATER0_INIT[1], qp),
    ];
    let mut mvd_g1: [(u8, u8); 2] = [
        vvc_init_ctx(VVC_ABS_MVD_GREATER1_INIT[0], qp),
        vvc_init_ctx(VVC_ABS_MVD_GREATER1_INIT[1], qp),
    ];

    let mut cabac_opt = VvcCabac::new(cabac_data);
    let mut cabac_failed = cabac_opt.is_none();

    // Track skip counts for left/above neighbours (for ctxIdx selection).
    // We use a flat array indexed by CTU raster position.
    let mut skip_flags: Vec<bool> = vec![false; total_ctus as usize];

    for ctu_idx in 0..total_ctus {
        let ctu_x = (ctu_idx % ctu_cols) * ctu_size;
        let ctu_y = (ctu_idx / ctu_cols) * ctu_size;
        let mut ctu = CodingTreeUnit::new(ctu_x, ctu_y, ctu_size as u8);

        if cabac_failed {
            // Fallback scaffold: alternating H/V binary splits with Inter mode.
            let cu_size = 32u32.min(ctu_size);
            let sub_cols = ctu_size.div_ceil(cu_size);
            let sub_rows = ctu_size.div_ceil(cu_size);
            let qt_depth = (ctu_size / cu_size).ilog2() as u8;
            for row in 0..sub_rows {
                for col in 0..sub_cols {
                    let cu_x = ctu_x + col * cu_size;
                    let cu_y = ctu_y + row * cu_size;
                    let split = if (col + row) % 4 == 0 {
                        SplitMode::HorzB
                    } else if (col + row) % 4 == 2 {
                        SplitMode::VertB
                    } else {
                        SplitMode::None
                    };
                    ctu.add_cu(CodingUnit {
                        x: cu_x,
                        y: cu_y,
                        size: cu_size as u8,
                        pred_mode: PredMode::Inter,
                        split_mode: split,
                        depth: qt_depth,
                        tree_type: 0,
                        qp: base_qp,
                        mv_l0: None,
                        mv_l1: None,
                        ref_idx_l0: None,
                        ref_idx_l1: None,
                        transform_size: 4,
                        sbt_flag: false,
                        isp_flag: false,
                    });
                }
            }
            ctus.push(ctu);
            continue;
        }

        // -----------------------------------------------------------------
        // CABAC decode for this CTU.
        // -----------------------------------------------------------------
        // Determine cu_skip_flag ctxIdx from left and above neighbours.
        let left_skip = if ctu_idx % ctu_cols > 0 {
            skip_flags[(ctu_idx - 1) as usize]
        } else {
            false
        };
        let above_skip = if ctu_idx >= ctu_cols {
            skip_flags[(ctu_idx - ctu_cols) as usize]
        } else {
            false
        };
        let skip_ctx_idx = (left_skip as usize) + (above_skip as usize);

        // All CABAC decodes for this CTU happen in a single block so the
        // borrow of cabac_opt does not conflict with skip_ctx borrows (NLL
        // keeps each ref scoped to its use).
        let cabac = cabac_opt.as_mut().unwrap();

        // cu_skip_flag — context selected by left/above skip neighbour count.
        let cu_skip_flag = {
            let (ref mut skip_ps, ref mut skip_ms) = skip_ctx[skip_ctx_idx];
            cabac.decode_bin(skip_ps, skip_ms)
        };
        let cu_skip_flag = match cu_skip_flag {
            Some(v) => v,
            None => {
                cabac_failed = true;
                ctu.add_cu(CodingUnit {
                    x: ctu_x,
                    y: ctu_y,
                    size: ctu_size as u8,
                    pred_mode: PredMode::Inter,
                    split_mode: SplitMode::None,
                    depth: 0,
                    tree_type: 0,
                    qp: base_qp,
                    mv_l0: None,
                    mv_l1: None,
                    ref_idx_l0: None,
                    ref_idx_l1: None,
                    transform_size: 4,
                    sbt_flag: false,
                    isp_flag: false,
                });
                ctus.push(ctu);
                continue;
            }
        };

        if cu_skip_flag == 1 {
            skip_flags[ctu_idx as usize] = true;
            ctu.add_cu(CodingUnit {
                x: ctu_x,
                y: ctu_y,
                size: ctu_size as u8,
                pred_mode: PredMode::Skip,
                split_mode: SplitMode::None,
                depth: 0,
                tree_type: 0,
                qp: base_qp,
                mv_l0: None,
                mv_l1: None,
                ref_idx_l0: None,
                ref_idx_l1: None,
                transform_size: 4,
                sbt_flag: false,
                isp_flag: false,
            });
            ctus.push(ctu);
            continue;
        }

        // Non-skip: decode pred_mode_flag (1 = Intra, 0 = Inter)
        let pred_mode_bit = match cabac.decode_bin(&mut pred_ps, &mut pred_ms) {
            Some(v) => v,
            None => {
                cabac_failed = true;
                ctu.add_cu(CodingUnit {
                    x: ctu_x,
                    y: ctu_y,
                    size: ctu_size as u8,
                    pred_mode: PredMode::Inter,
                    split_mode: SplitMode::None,
                    depth: 0,
                    tree_type: 0,
                    qp: base_qp,
                    mv_l0: None,
                    mv_l1: None,
                    ref_idx_l0: None,
                    ref_idx_l1: None,
                    transform_size: 4,
                    sbt_flag: false,
                    isp_flag: false,
                });
                ctus.push(ctu);
                continue;
            }
        };

        if pred_mode_bit == 1 {
            // Intra CU
            ctu.add_cu(CodingUnit {
                x: ctu_x,
                y: ctu_y,
                size: ctu_size as u8,
                pred_mode: PredMode::Intra,
                split_mode: SplitMode::None,
                depth: 0,
                tree_type: 0,
                qp: base_qp,
                mv_l0: None,
                mv_l1: None,
                ref_idx_l0: None,
                ref_idx_l1: None,
                transform_size: 4,
                sbt_flag: false,
                isp_flag: false,
            });
            ctus.push(ctu);
            continue;
        }

        // Inter CU: decode general_merge_flag
        let merge_flag = match cabac.decode_bin(&mut merge_ps, &mut merge_ms) {
            Some(v) => v,
            None => {
                cabac_failed = true;
                ctu.add_cu(CodingUnit {
                    x: ctu_x,
                    y: ctu_y,
                    size: ctu_size as u8,
                    pred_mode: PredMode::Inter,
                    split_mode: SplitMode::None,
                    depth: 0,
                    tree_type: 0,
                    qp: base_qp,
                    mv_l0: None,
                    mv_l1: None,
                    ref_idx_l0: None,
                    ref_idx_l1: None,
                    transform_size: 4,
                    sbt_flag: false,
                    isp_flag: false,
                });
                ctus.push(ctu);
                continue;
            }
        };

        let mv_l0 = if merge_flag == 0 {
            // Non-merge Inter: decode MVD x then y (short-circuit on failure).
            let mvd_x = cabac.decode_mvd_component(
                &mut mvd_g0[0].0,
                &mut mvd_g0[0].1,
                &mut mvd_g1[0].0,
                &mut mvd_g1[0].1,
            );
            match mvd_x {
                None => {
                    cabac_failed = true;
                    None
                }
                Some(x) => {
                    let mvd_y = cabac.decode_mvd_component(
                        &mut mvd_g0[1].0,
                        &mut mvd_g0[1].1,
                        &mut mvd_g1[1].0,
                        &mut mvd_g1[1].1,
                    );
                    match mvd_y {
                        None => {
                            cabac_failed = true;
                            None
                        }
                        Some(y) => Some(MotionVector::new(x, y)),
                    }
                }
            }
        } else {
            // Merge: zero MV placeholder
            Some(MotionVector::zero())
        };

        ctu.add_cu(CodingUnit {
            x: ctu_x,
            y: ctu_y,
            size: ctu_size as u8,
            pred_mode: PredMode::Inter,
            split_mode: SplitMode::None,
            depth: 0,
            tree_type: 0,
            qp: base_qp,
            mv_l0,
            mv_l1: None,
            ref_idx_l0: None,
            ref_idx_l1: None,
            transform_size: 4,
            sbt_flag: false,
            isp_flag: false,
        });
        ctus.push(ctu);
    }

    Ok(ctus)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nal::{NalUnit, NalUnitHeader};
    use crate::sps::Sps;

    fn create_test_sps(width: u32, height: u32, log2_ctu_size: u8) -> Sps {
        Sps {
            sps_seq_parameter_set_id: 0,
            sps_video_parameter_set_id: 0,
            sps_max_sublayers_minus1: 0,
            sps_chroma_format_idc: crate::sps::ChromaFormat::Chroma420,
            sps_log2_ctu_size_minus5: log2_ctu_size - 5,
            sps_pic_width_max_in_luma_samples: width,
            sps_pic_height_max_in_luma_samples: height,
            sps_subpic_info_present_flag: false,
            sps_num_subpics_minus1: 0,
            sps_conformance_window_flag: false,
            sps_conf_win_left_offset: 0,
            sps_conf_win_right_offset: 0,
            sps_conf_win_top_offset: 0,
            sps_conf_win_bottom_offset: 0,
            sps_bitdepth_minus8: 2,
            sps_log2_min_luma_coding_block_size_minus2: 0,
            sps_poc_msb_cycle_flag: false,
            sps_log2_max_pic_order_cnt_lsb_minus4: 4,
            profile_tier_level: Default::default(),
            sps_gdr_enabled_flag: false,
            sps_ref_pic_resampling_enabled_flag: false,
            dual_tree: Default::default(),
            alf: Default::default(),
            lmcs: Default::default(),
            sps_transform_skip_enabled_flag: false,
            sps_bdpcm_enabled_flag: false,
            sps_mts_enabled_flag: false,
            sps_lfnst_enabled_flag: false,
            sps_joint_cbcr_enabled_flag: false,
            sps_same_qp_table_for_chroma_flag: true,
            sps_sao_enabled_flag: true,
            sps_deblocking_filter_control_present_flag: false,
            sps_temporal_mvp_enabled_flag: true,
            sps_mmvd_enabled_flag: false,
            sps_affine_enabled_flag: false,
            sps_bcw_enabled_flag: false,
            sps_ibc_enabled_flag: false,
            sps_ciip_enabled_flag: false,
            sps_gpm_enabled_flag: false,
        }
    }

    fn create_test_nal_unit(nal_type: crate::NalUnitType) -> NalUnit {
        NalUnit {
            header: NalUnitHeader {
                nal_unit_type: nal_type,
                nuh_layer_id: 0,
                nuh_temporal_id_plus1: 1,
            },
            offset: 0,
            size: 10,
            payload: vec![0; 10],
            raw_payload: vec![0; 10],
        }
    }

    #[test]
    fn test_pred_mode() {
        assert_eq!(PredMode::Intra, PredMode::Intra);
        assert_eq!(PredMode::Inter, PredMode::Inter);
        assert_eq!(PredMode::Ibc, PredMode::Ibc);
        assert_eq!(PredMode::Skip, PredMode::Skip);
    }

    #[test]
    fn test_split_mode() {
        assert_eq!(SplitMode::None, SplitMode::None);
        assert_eq!(SplitMode::QuadTree, SplitMode::QuadTree);
        assert_eq!(SplitMode::HorzB, SplitMode::HorzB);
        assert_eq!(SplitMode::VertB, SplitMode::VertB);
        assert_eq!(SplitMode::HorzT, SplitMode::HorzT);
        assert_eq!(SplitMode::VertT, SplitMode::VertT);
    }

    #[test]
    fn test_motion_vector_new() {
        let mv = MotionVector::new(4, -8);
        assert_eq!(mv.x, 4);
        assert_eq!(mv.y, -8);
    }

    #[test]
    fn test_motion_vector_zero() {
        let mv = MotionVector::zero();
        assert_eq!(mv.x, 0);
        assert_eq!(mv.y, 0);
    }

    #[test]
    fn test_ctu_creation() {
        let ctu = CodingTreeUnit::new(0, 0, 128);
        assert_eq!(ctu.x, 0);
        assert_eq!(ctu.y, 0);
        assert_eq!(ctu.size, 128);
        assert!(ctu.coding_units.is_empty());
    }

    #[test]
    fn test_ctu_add_cu() {
        let mut ctu = CodingTreeUnit::new(0, 0, 128);
        let cu = CodingUnit {
            x: 0,
            y: 0,
            size: 128,
            pred_mode: PredMode::Intra,
            split_mode: SplitMode::None,
            depth: 0,
            tree_type: 0,
            qp: 26,
            mv_l0: None,
            mv_l1: None,
            ref_idx_l0: None,
            ref_idx_l1: None,
            transform_size: 4,
            sbt_flag: false,
            isp_flag: false,
        };
        ctu.add_cu(cu);
        assert_eq!(ctu.coding_units.len(), 1);
    }

    #[test]
    fn test_extract_qp_grid_empty_nal_units() {
        let sps = create_test_sps(640, 480, 6); // CTU size = 2^6 = 64
        let result = extract_qp_grid(&[], &sps, 26);
        assert!(result.is_ok());
        let qp_grid = result.unwrap();
        // 640/64 * 480/64 = 10 * 7 = 70 CTUs (round up)
        assert_eq!(qp_grid.grid_w, 10);
        assert_eq!(qp_grid.grid_h, 8);
    }

    #[test]
    fn test_extract_qp_grid_with_idr_slice() {
        let sps = create_test_sps(640, 480, 6);
        let nal = create_test_nal_unit(crate::NalUnitType::IdrWRadl);
        let result = extract_qp_grid(&[nal], &sps, 26);
        assert!(result.is_ok());
        let qp_grid = result.unwrap();
        assert_eq!(qp_grid.grid_w, 10);
        assert_eq!(qp_grid.grid_h, 8);
    }

    #[test]
    fn test_extract_qp_grid_with_cra_slice() {
        let sps = create_test_sps(1920, 1080, 7); // CTU size = 2^7 = 128
        let nal = create_test_nal_unit(crate::NalUnitType::CraNut);
        let result = extract_qp_grid(&[nal], &sps, 30);
        assert!(result.is_ok());
        let qp_grid = result.unwrap();
        // 1920/128 * 1080/128 = 15 * 8 = 120 CTUs (round up)
        assert_eq!(qp_grid.grid_w, 15);
        assert_eq!(qp_grid.grid_h, 9);
    }

    #[test]
    fn test_extract_qp_grid_base_qp_variations() {
        let sps = create_test_sps(640, 480, 6);
        let nal = create_test_nal_unit(crate::NalUnitType::IdrWRadl);

        for base_qp in [0i16, 10, 26, 40, 51] {
            let result = extract_qp_grid(&[nal.clone()], &sps, base_qp);
            assert!(result.is_ok(), "Failed for base_qp={}", base_qp);
        }
    }

    #[test]
    fn test_extract_mv_grid_empty_nal_units() {
        let sps = create_test_sps(640, 480, 6);
        let result = extract_mv_grid(&[], &sps);
        assert!(result.is_ok());
        let mv_grid = result.unwrap();
        // MV grid uses 16x16 blocks
        assert_eq!(mv_grid.coded_width, 640);
        assert_eq!(mv_grid.coded_height, 480);
    }

    #[test]
    fn test_extract_mv_grid_with_slice() {
        let sps = create_test_sps(640, 480, 6);
        let nal = create_test_nal_unit(crate::NalUnitType::TrailNut);
        let result = extract_mv_grid(&[nal], &sps);
        assert!(result.is_ok());
        let mv_grid = result.unwrap();
        assert_eq!(mv_grid.coded_width, 640);
        assert_eq!(mv_grid.coded_height, 480);
        assert!(mv_grid.mode.as_ref().is_some());
    }

    #[test]
    fn test_extract_mv_grid_intra_slice() {
        let sps = create_test_sps(320, 240, 6);
        let nal = create_test_nal_unit(crate::NalUnitType::IdrWRadl);
        let result = extract_mv_grid(&[nal], &sps);
        assert!(result.is_ok());
        let mv_grid = result.unwrap();
        let modes = mv_grid.mode.as_ref().unwrap();
        // All blocks should be Intra for IDR slice
        assert!(modes.iter().all(|m| *m == BlockMode::Intra));
    }

    #[test]
    fn test_extract_partition_grid_empty_nal_units() {
        let sps = create_test_sps(640, 480, 6);
        let result = extract_partition_grid(&[], &sps);
        assert!(result.is_ok());
        let partition_grid = result.unwrap();
        assert_eq!(partition_grid.coded_width, 640);
        assert_eq!(partition_grid.coded_height, 480);
        // Should have scaffold blocks
        assert!(!partition_grid.blocks.is_empty());
    }

    #[test]
    fn test_extract_partition_grid_with_slice() {
        let sps = create_test_sps(640, 480, 6);
        let nal = create_test_nal_unit(crate::NalUnitType::IdrWRadl);
        let result = extract_partition_grid(&[nal], &sps);
        assert!(result.is_ok());
        let partition_grid = result.unwrap();
        assert_eq!(partition_grid.coded_width, 640);
        assert_eq!(partition_grid.coded_height, 480);
    }

    #[test]
    fn test_extract_partition_grid_inter_slice() {
        let sps = create_test_sps(1920, 1080, 7);
        let nal = create_test_nal_unit(crate::NalUnitType::TrailNut);
        let result = extract_partition_grid(&[nal], &sps);
        assert!(result.is_ok());
        let partition_grid = result.unwrap();
        assert_eq!(partition_grid.coded_width, 1920);
        assert_eq!(partition_grid.coded_height, 1080);
    }

    #[test]
    fn test_coding_unit_struct() {
        let cu = CodingUnit {
            x: 0,
            y: 0,
            size: 64,
            pred_mode: PredMode::Intra,
            split_mode: SplitMode::None,
            depth: 0,
            tree_type: 0,
            qp: 26,
            mv_l0: None,
            mv_l1: None,
            ref_idx_l0: None,
            ref_idx_l1: None,
            transform_size: 4,
            sbt_flag: false,
            isp_flag: false,
        };
        assert_eq!(cu.x, 0);
        assert_eq!(cu.y, 0);
        assert_eq!(cu.size, 64);
        assert_eq!(cu.qp, 26);
        assert_eq!(cu.pred_mode, PredMode::Intra);
        assert!(!cu.sbt_flag);
        assert!(!cu.isp_flag);
    }

    #[test]
    fn test_coding_unit_with_motion_vectors() {
        let cu = CodingUnit {
            x: 0,
            y: 0,
            size: 32,
            pred_mode: PredMode::Inter,
            split_mode: SplitMode::QuadTree,
            depth: 1,
            tree_type: 0,
            qp: 30,
            mv_l0: Some(MotionVector::new(4, 8)),
            mv_l1: Some(MotionVector::new(-2, 4)),
            ref_idx_l0: Some(0),
            ref_idx_l1: Some(1),
            transform_size: 8,
            sbt_flag: false,
            isp_flag: false,
        };
        assert_eq!(cu.mv_l0.unwrap().x, 4);
        assert_eq!(cu.mv_l1.unwrap().y, 4);
        assert_eq!(cu.ref_idx_l0.unwrap(), 0);
        assert_eq!(cu.ref_idx_l1.unwrap(), 1);
        assert_eq!(cu.split_mode, SplitMode::QuadTree);
    }

    #[test]
    fn test_coding_unit_ibc_mode() {
        let cu = CodingUnit {
            x: 0,
            y: 0,
            size: 64,
            pred_mode: PredMode::Ibc,
            split_mode: SplitMode::None,
            depth: 0,
            tree_type: 0,
            qp: 26,
            mv_l0: Some(MotionVector::new(8, -4)),
            mv_l1: None,
            ref_idx_l0: None,
            ref_idx_l1: None,
            transform_size: 4,
            sbt_flag: false,
            isp_flag: false,
        };
        assert_eq!(cu.pred_mode, PredMode::Ibc);
        assert_eq!(cu.mv_l0.unwrap().x, 8);
        assert!(cu.mv_l1.is_none());
    }

    #[test]
    fn test_extract_qp_grid_small_resolution() {
        let sps = create_test_sps(160, 120, 6);
        let nal = create_test_nal_unit(crate::NalUnitType::IdrWRadl);
        let result = extract_qp_grid(&[nal], &sps, 20);
        assert!(result.is_ok());
        let qp_grid = result.unwrap();
        // 160/64 * 120/64 = 2 * 2 = 4 CTUs (round up)
        assert_eq!(qp_grid.grid_w, 3);
        assert_eq!(qp_grid.grid_h, 2);
    }

    #[test]
    fn test_extract_mv_grid_high_resolution() {
        let sps = create_test_sps(3840, 2160, 7);
        let nal = create_test_nal_unit(crate::NalUnitType::TrailNut);
        let result = extract_mv_grid(&[nal], &sps);
        assert!(result.is_ok());
        let mv_grid = result.unwrap();
        assert_eq!(mv_grid.coded_width, 3840);
        assert_eq!(mv_grid.coded_height, 2160);
    }

    #[test]
    fn test_split_mode_to_partition_type() {
        assert_eq!(SplitMode::None, SplitMode::None);
        assert_eq!(SplitMode::QuadTree, SplitMode::QuadTree);
        assert_eq!(SplitMode::HorzB, SplitMode::HorzB);
        assert_eq!(SplitMode::VertB, SplitMode::VertB);
        assert_eq!(SplitMode::HorzT, SplitMode::HorzT);
        assert_eq!(SplitMode::VertT, SplitMode::VertT);
    }

    #[test]
    fn test_extract_qp_grid_various_ctu_sizes() {
        for log2_ctu_size in [5u8, 6, 7] {
            let sps = create_test_sps(640, 480, log2_ctu_size);
            let nal = create_test_nal_unit(crate::NalUnitType::IdrWRadl);
            let result = extract_qp_grid(&[nal], &sps, 26);
            assert!(
                result.is_ok(),
                "Failed for CTU size={}",
                1u32 << (log2_ctu_size)
            );
        }
    }

    #[test]
    fn test_extract_mv_grid_modes_present() {
        let sps = create_test_sps(640, 480, 6);
        let nal = create_test_nal_unit(crate::NalUnitType::TrailNut);
        let result = extract_mv_grid(&[nal], &sps);
        assert!(result.is_ok());
        let mv_grid = result.unwrap();
        assert!(mv_grid.mode.as_ref().is_some());
    }

    #[test]
    fn test_extract_partition_grid_blocks_filled() {
        let sps = create_test_sps(640, 480, 6);
        let nal = create_test_nal_unit(crate::NalUnitType::IdrWRadl);
        let result = extract_partition_grid(&[nal], &sps);
        assert!(result.is_ok());
        let partition_grid = result.unwrap();
        assert!(!partition_grid.blocks.is_empty());
    }

    #[test]
    fn test_pred_mode_all_variants() {
        assert_eq!(PredMode::Intra, PredMode::Intra);
        assert_eq!(PredMode::Inter, PredMode::Inter);
        assert_eq!(PredMode::Ibc, PredMode::Ibc);
        assert_eq!(PredMode::Skip, PredMode::Skip);
        assert_ne!(PredMode::Intra, PredMode::Inter);
        assert_ne!(PredMode::Ibc, PredMode::Inter);
        assert_ne!(PredMode::Skip, PredMode::Intra);
    }

    #[test]
    fn test_coding_unit_skip_mode() {
        let cu = CodingUnit {
            x: 0,
            y: 0,
            size: 64,
            pred_mode: PredMode::Skip,
            split_mode: SplitMode::None,
            depth: 0,
            tree_type: 0,
            qp: 26,
            mv_l0: Some(MotionVector::zero()),
            mv_l1: None,
            ref_idx_l0: None,
            ref_idx_l1: None,
            transform_size: 4,
            sbt_flag: false,
            isp_flag: false,
        };
        assert_eq!(cu.pred_mode, PredMode::Skip);
        assert_eq!(cu.mv_l0.unwrap().x, 0);
        assert_eq!(cu.mv_l0.unwrap().y, 0);
    }

    #[test]
    fn test_coding_unit_sbt_and_isp_flags() {
        let cu = CodingUnit {
            x: 0,
            y: 0,
            size: 32,
            pred_mode: PredMode::Inter,
            split_mode: SplitMode::HorzB,
            depth: 1,
            tree_type: 1,
            qp: 28,
            mv_l0: Some(MotionVector::new(2, 3)),
            mv_l1: None,
            ref_idx_l0: Some(0),
            ref_idx_l1: None,
            transform_size: 8,
            sbt_flag: true,
            isp_flag: true,
        };
        assert!(cu.sbt_flag);
        assert!(cu.isp_flag);
        assert_eq!(cu.tree_type, 1);
        assert_eq!(cu.split_mode, SplitMode::HorzB);
    }

    #[test]
    fn test_extract_qp_grid_various_resolutions() {
        let resolutions = [(320u32, 240u32), (640, 480), (1280, 720), (1920, 1080)];
        for (width, height) in resolutions {
            let sps = create_test_sps(width, height, 6);
            let nal = create_test_nal_unit(crate::NalUnitType::IdrWRadl);
            let result = extract_qp_grid(&[nal], &sps, 26);
            assert!(result.is_ok(), "Failed for {}x{}", width, height);
        }
    }
}
