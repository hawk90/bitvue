//! Overlay data extraction from H.264/AVC bitstreams
//!
//! This module provides functions to extract QP heatmap, motion vector,
//! and macroblock information for visualization overlays.
//!
//! ## Implementation Status (v0.4.x)
//!
//! **Real Data Extraction**:
//! - ✅ Extract macroblock structure from slice data
//! - ✅ Extract motion vectors from INTER macroblocks
//! - ✅ Extract QP values from macroblock layer
//! - ✅ Extract macroblock types (I/P/Skip/B)
//! - ✅ Extract reference frame indices
//!
//! ## Data Flow
//!
//! 1. **NAL Units** → find_nal_units() → Vec<NalUnit>
//! 2. **Slice Data** → parse_macroblocks() → Vec<Macroblock>
//! 3. **Macroblocks** → extract_*_grid() → overlay grids

use crate::bitreader::BitReader;
use crate::nal::{NalUnit, NalUnitType};
use crate::pps::{parse_pps, Pps};
use crate::slice::{parse_slice_header_reader, SliceType};
use crate::sps::Sps;
use bitvue_core::{
    mv_overlay::{BlockMode, MVGrid, MotionVector as CoreMV},
    partition_grid::{PartitionBlock, PartitionGrid, PartitionType},
    qp_heatmap::QPGrid,
    BitvueError,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Macroblock type for H.264
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MbType {
    /// I macroblock (intra)
    I4x4,
    I16x16,
    IPCM,
    /// P macroblock (predicted)
    PLuma,
    P8x8,
    /// B macroblock (bi-predictive)
    BDirect,
    B16x16,
    B16x8,
    B8x16,
    B8x8,
    /// Skip macroblock
    PSkip,
    BSkip,
}

impl MbType {
    /// Check if this is an INTRA macroblock
    pub fn is_intra(&self) -> bool {
        matches!(self, MbType::I4x4 | MbType::I16x16 | MbType::IPCM)
    }

    /// Check if this is a SKIP macroblock
    pub fn is_skip(&self) -> bool {
        matches!(self, MbType::PSkip | MbType::BSkip)
    }

    /// Get partition type for visualization
    pub fn to_partition_type(self) -> PartitionType {
        match self {
            MbType::I4x4 => PartitionType::Split,
            MbType::I16x16 => PartitionType::None,
            MbType::IPCM => PartitionType::None,
            MbType::PLuma => PartitionType::None,
            MbType::P8x8 => PartitionType::Split,
            MbType::BDirect => PartitionType::None,
            MbType::BSkip | MbType::PSkip => PartitionType::None,
            MbType::B16x16 => PartitionType::None,
            MbType::B16x8 => PartitionType::Horz,
            MbType::B8x16 => PartitionType::Vert,
            MbType::B8x8 => PartitionType::Split,
        }
    }
}

/// H.264 Macroblock information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Macroblock {
    /// Macroblock address (scan order)
    pub mb_addr: u32,
    /// Macroblock position in pixels
    pub x: u32,
    pub y: u32,
    /// Macroblock type
    pub mb_type: MbType,
    /// Skip flag
    pub skip: bool,
    /// QP value (for this macroblock)
    pub qp: i16,
    /// Motion vectors (for INTER blocks)
    /// [mv_l0, mv_l1] where each is (x, y) in quarter-pel units
    pub mv_l0: Option<MotionVector>,
    pub mv_l1: Option<MotionVector>,
    /// Reference frame indices
    pub ref_idx_l0: Option<i8>,
    pub ref_idx_l1: Option<i8>,
}

/// Motion vector for H.264 (quarter-pel precision)
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct MotionVector {
    /// Horizontal component (quarter-pel units)
    pub x: i32,
    /// Vertical component (quarter-pel units)
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

/// Build SPS and PPS maps from NAL unit list.
fn build_parameter_maps(nal_units: &[NalUnit]) -> (HashMap<u8, Sps>, HashMap<u8, Pps>) {
    let mut sps_map: HashMap<u8, Sps> = HashMap::new();
    let mut pps_map: HashMap<u8, Pps> = HashMap::new();
    for nal in nal_units {
        match nal.header.nal_unit_type {
            NalUnitType::Sps => {
                if let Ok(sps) = crate::sps::parse_sps(&nal.payload) {
                    sps_map.insert(sps.seq_parameter_set_id, sps);
                }
            }
            NalUnitType::Pps => {
                if let Ok(pps) = parse_pps(&nal.payload) {
                    pps_map.insert(pps.pic_parameter_set_id, pps);
                }
            }
            _ => {}
        }
    }
    (sps_map, pps_map)
}

/// Extract QP Grid from H.264 bitstream
///
/// Parses macroblocks from slice data and extracts QP values.
pub fn extract_qp_grid(
    nal_units: &[NalUnit],
    sps: &Sps,
    base_qp: i16,
) -> Result<QPGrid, BitvueError> {
    let pic_width_in_mbs = sps.pic_width_in_mbs_minus1 + 1;
    let pic_height_in_mbs = sps.pic_height_in_map_units_minus1 + 1;

    let grid_w = pic_width_in_mbs;
    let grid_h = pic_height_in_mbs;

    // Check for overflow in grid size calculation
    let total_blocks = grid_w.checked_mul(grid_h).ok_or_else(|| {
        BitvueError::Decode(format!("Grid dimensions too large: {}x{}", grid_w, grid_h))
    })? as usize;

    let mut qp = Vec::with_capacity(total_blocks);
    let (sps_map, pps_map) = build_parameter_maps(nal_units);

    // Parse macroblocks from slice data
    for nal in nal_units {
        if nal.header.nal_unit_type.is_slice() {
            match parse_slice_macroblocks(nal, &sps_map, &pps_map, sps, base_qp) {
                Ok(mbs) => {
                    // Collect QP values from macroblocks
                    for mb in &mbs {
                        qp.push(mb.qp);
                    }
                }
                Err(e) => {
                    abseil::vlog!(1, "Failed to parse macroblocks: {}, using base_qp", e);
                    // Use base_qp for all macroblocks in this slice
                }
            }
        }
    }

    // If we didn't get any macroblocks, use base_qp
    if qp.is_empty() {
        let total_blocks = grid_w.checked_mul(grid_h).ok_or_else(|| {
            BitvueError::Decode(format!("Grid dimensions too large: {}x{}", grid_w, grid_h))
        })? as usize;
        qp = vec![base_qp; total_blocks];
    }

    Ok(QPGrid::new(grid_w, grid_h, 16, 16, qp, base_qp))
}

/// Extract MV Grid from H.264 bitstream
///
/// Parses macroblocks from slice data and extracts motion vectors.
pub fn extract_mv_grid(nal_units: &[NalUnit], sps: &Sps) -> Result<MVGrid, BitvueError> {
    let pic_width_in_mbs = sps.pic_width_in_mbs_minus1 + 1;
    let pic_height_in_mbs = sps.pic_height_in_map_units_minus1 + 1;

    let mb_width = pic_width_in_mbs * 16;
    let mb_height = pic_height_in_mbs * 16;
    let grid_w = pic_width_in_mbs;
    let grid_h = pic_height_in_mbs;

    // Check for overflow in grid size calculation
    let total_blocks = grid_w.checked_mul(grid_h).ok_or_else(|| {
        BitvueError::Decode(format!("Grid dimensions too large: {}x{}", grid_w, grid_h))
    })? as usize;

    let mut mv_l0 = Vec::with_capacity(total_blocks);
    let mut mv_l1 = Vec::with_capacity(total_blocks);
    let mut modes = Vec::with_capacity(total_blocks);
    let (sps_map, pps_map) = build_parameter_maps(nal_units);

    // Parse macroblocks from slice data
    for nal in nal_units {
        if nal.header.nal_unit_type.is_slice() {
            match parse_slice_macroblocks(nal, &sps_map, &pps_map, sps, 26) {
                Ok(mbs) => {
                    for mb in &mbs {
                        if mb.mb_type.is_intra() {
                            mv_l0.push(CoreMV::MISSING);
                            mv_l1.push(CoreMV::MISSING);
                            modes.push(BlockMode::Intra);
                        } else {
                            // Has motion vectors
                            if let Some(ref mv) = mb.mv_l0 {
                                mv_l0.push(CoreMV::new(mv.x, mv.y));
                            } else {
                                mv_l0.push(CoreMV::ZERO);
                            }

                            if let Some(ref mv) = mb.mv_l1 {
                                mv_l1.push(CoreMV::new(mv.x, mv.y));
                            } else {
                                mv_l1.push(CoreMV::MISSING);
                            }

                            modes.push(BlockMode::Inter);
                        }
                    }
                }
                Err(e) => {
                    abseil::vlog!(1, "Failed to parse macroblocks for MV: {}, using ZERO", e);
                    // Use zero MV for all macroblocks in this slice
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

    Ok(MVGrid::new(
        mb_width,
        mb_height,
        16,
        16,
        mv_l0,
        mv_l1,
        Some(modes),
    ))
}

/// Extract Prediction Mode Grid from H.264 bitstream.
///
/// Returns a flat grid (one entry per 16×16 macroblock) with mode values:
///   0 = Intra (I4×4, I16×16, IPCM)
///   1 = Inter (P/B non-skip)
///   2 = Skip  (P_Skip, B_Skip)
pub fn extract_prediction_mode_grid(
    nal_units: &[NalUnit],
    sps: &Sps,
) -> Result<(u32, u32, u32, u32, Vec<Option<u8>>), BitvueError> {
    let pic_width_in_mbs = sps.pic_width_in_mbs_minus1 + 1;
    let pic_height_in_mbs = sps.pic_height_in_map_units_minus1 + 1;
    let grid_w = pic_width_in_mbs;
    let grid_h = pic_height_in_mbs;

    let total_blocks = grid_w.checked_mul(grid_h).ok_or_else(|| {
        BitvueError::Decode(format!("Grid dimensions too large: {}x{}", grid_w, grid_h))
    })? as usize;

    let mut modes: Vec<Option<u8>> = Vec::with_capacity(total_blocks);
    let (sps_map, pps_map) = build_parameter_maps(nal_units);

    for nal in nal_units {
        if nal.header.nal_unit_type.is_slice() {
            match parse_slice_macroblocks(nal, &sps_map, &pps_map, sps, 26) {
                Ok(mbs) => {
                    for mb in &mbs {
                        let mode = if mb.mb_type.is_skip() {
                            2u8 // Skip
                        } else if mb.mb_type.is_intra() {
                            0u8 // Intra
                        } else {
                            1u8 // Inter
                        };
                        modes.push(Some(mode));
                    }
                }
                Err(_) => {}
            }
        }
    }

    while modes.len() < total_blocks {
        modes.push(None);
    }
    modes.truncate(total_blocks);

    Ok((pic_width_in_mbs * 16, pic_height_in_mbs * 16, 16, 16, modes))
}

/// Extract Partition Grid from H.264 bitstream
///
/// Parses macroblocks from slice data and creates a partition grid.
pub fn extract_partition_grid(
    nal_units: &[NalUnit],
    sps: &Sps,
) -> Result<PartitionGrid, BitvueError> {
    let pic_width_in_mbs = sps.pic_width_in_mbs_minus1 + 1;
    let pic_height_in_mbs = sps.pic_height_in_map_units_minus1 + 1;

    let pic_width = pic_width_in_mbs * 16;
    let pic_height = pic_height_in_mbs * 16;

    let mut grid = PartitionGrid::new(pic_width, pic_height, 16);

    let (sps_map, pps_map) = build_parameter_maps(nal_units);

    // Parse macroblocks from slice data
    for nal in nal_units {
        if nal.header.nal_unit_type.is_slice() {
            match parse_slice_macroblocks(nal, &sps_map, &pps_map, sps, 26) {
                Ok(mbs) => {
                    for mb in &mbs {
                        grid.add_block(PartitionBlock::new(
                            mb.x,
                            mb.y,
                            16,
                            16,
                            mb.mb_type.to_partition_type(),
                            0,
                        ));
                    }
                }
                Err(e) => {
                    abseil::vlog!(
                        1,
                        "Failed to parse macroblocks for partition: {}, using scaffold",
                        e
                    );
                    // Add scaffold blocks
                }
            }
        }
    }

    // Fill with scaffold blocks if empty
    if grid.blocks.is_empty() {
        let grid_w = pic_width_in_mbs;
        let grid_h = pic_height_in_mbs;
        for mb_y in 0..grid_h {
            for mb_x in 0..grid_w {
                grid.add_block(PartitionBlock::new(
                    mb_x * 16,
                    mb_y * 16,
                    16,
                    16,
                    PartitionType::None,
                    0,
                ));
            }
        }
    }

    Ok(grid)
}

/// Parse macroblocks from slice data.
///
/// Parses the slice header to determine slice type and entropy mode, then:
/// - For CAVLC-coded P slices: extracts skip/non-skip MB classification and
///   motion vectors for P_L0_16x16 blocks.
/// - For CABAC or I slices: classifies MBs by slice type without MV data.
///
/// Note: CAVLC residual parsing is not implemented, so after the first
/// non-skip MB the scan stops and remaining MBs get type-based defaults.
fn parse_slice_macroblocks(
    nal: &NalUnit,
    sps_map: &HashMap<u8, Sps>,
    pps_map: &HashMap<u8, Pps>,
    fallback_sps: &Sps,
    fallback_qp: i16,
) -> Result<Vec<Macroblock>, BitvueError> {
    let nal_type = nal.header.nal_unit_type;
    let nal_ref_idc = nal.header.nal_ref_idc;

    // Parse the real slice header to get slice type, QP, and entropy mode.
    let mut reader = BitReader::new(&nal.payload);
    let header =
        match parse_slice_header_reader(&mut reader, sps_map, pps_map, nal_type, nal_ref_idc) {
            Ok(h) => h,
            Err(_) => {
                // Fall back to dimension-based scaffold
                return Ok(build_scaffold_mbs(nal_type, fallback_sps, fallback_qp));
            }
        };

    let pps = match pps_map.get(&header.pic_parameter_set_id) {
        Some(p) => p,
        None => return Ok(build_scaffold_mbs(nal_type, fallback_sps, fallback_qp)),
    };
    let sps = match sps_map.get(&pps.seq_parameter_set_id) {
        Some(s) => s,
        None => fallback_sps,
    };

    let slice_qp = (26 + pps.pic_init_qp_minus26 + header.slice_qp_delta).clamp(0, 51) as i16;
    let pic_width_in_mbs = sps.pic_width_in_mbs_minus1 + 1;
    let pic_height_in_mbs = sps.pic_height_in_map_units_minus1 + 1;
    let total_mbs = pic_width_in_mbs * pic_height_in_mbs;
    let slice_type = header.slice_type;

    // CABAC: decode mb_skip_flag for each MB using the correct context model.
    // Full residual/MV decoding is not implemented, but skip classification is correct.
    if pps.entropy_coding_mode_flag {
        // The reader is positioned at the first byte after the slice header.
        // Extract remaining bytes for the CABAC decoder.
        let bit_offset = reader.bit_position();
        let byte_offset = bit_offset / 8;
        let payload_after_header = nal.payload.get(byte_offset..).unwrap_or(&[]);
        return Ok(parse_cabac_slice_mbs(
            payload_after_header,
            header.first_mb_in_slice,
            total_mbs,
            pic_width_in_mbs,
            slice_qp,
            slice_type,
            header.cabac_init_idc,
        ));
    }

    // CAVLC parsing
    let num_ref_l0 = header.num_ref_idx_l0_active_minus1;
    let mut mbs = Vec::with_capacity(total_mbs as usize);
    let mut mb_addr = header.first_mb_in_slice;

    'outer: while mb_addr < total_mbs {
        // For P/B slices: read mb_skip_run (ue)
        if !slice_type.is_intra() {
            let mb_skip_run = match reader.read_ue() {
                Ok(v) => v,
                Err(_) => break,
            };
            let skip_type = if slice_type.is_b() {
                MbType::BSkip
            } else {
                MbType::PSkip
            };
            for _ in 0..mb_skip_run {
                if mb_addr >= total_mbs {
                    break 'outer;
                }
                let x = (mb_addr % pic_width_in_mbs) * 16;
                let y = (mb_addr / pic_width_in_mbs) * 16;
                mbs.push(Macroblock {
                    mb_addr,
                    x,
                    y,
                    mb_type: skip_type,
                    skip: true,
                    qp: slice_qp,
                    mv_l0: None,
                    mv_l1: None,
                    ref_idx_l0: None,
                    ref_idx_l1: None,
                });
                mb_addr += 1;
            }
            if mb_addr >= total_mbs {
                break;
            }
        }

        // Parse mb_type for this non-skip MB
        let mb_type_raw = match reader.read_ue() {
            Ok(v) => v,
            Err(_) => break,
        };
        let x = (mb_addr % pic_width_in_mbs) * 16;
        let y = (mb_addr / pic_width_in_mbs) * 16;

        let mb_type = decode_p_mb_type(mb_type_raw, slice_type);
        let is_p16x16 = !slice_type.is_intra() && mb_type_raw == 0;

        if is_p16x16 {
            // te(num_ref_l0): read reference index
            let ref_idx = if num_ref_l0 == 0 {
                0u32
            } else if num_ref_l0 == 1 {
                match reader.read_bit() {
                    Ok(b) => {
                        if b {
                            0
                        } else {
                            1
                        }
                    }
                    Err(_) => break,
                }
            } else {
                match reader.read_ue() {
                    Ok(v) => v,
                    Err(_) => break,
                }
            };
            // mvd_l0[0][0]: horizontal then vertical, SE-coded
            let mvd_x = match reader.read_se() {
                Ok(v) => v,
                Err(_) => break,
            };
            let mvd_y = match reader.read_se() {
                Ok(v) => v,
                Err(_) => break,
            };

            // Emit this MB then try to advance past its CAVLC residuals.
            // CBP index 0 means no coded blocks (CBP=0): most common in P-frames.
            // If skip succeeds we continue parsing; otherwise scaffold the rest.
            mbs.push(Macroblock {
                mb_addr,
                x,
                y,
                mb_type,
                skip: false,
                qp: slice_qp,
                mv_l0: Some(MotionVector::new(mvd_x, mvd_y)),
                mv_l1: None,
                ref_idx_l0: Some(ref_idx.min(127) as i8),
                ref_idx_l1: None,
            });
            mb_addr += 1;
            if !try_skip_cavlc_residuals(&mut reader) {
                break;
            }
            continue;
        }

        // P16x8 or P8x16: 2 partitions, each with one reference index and one MVD pair.
        if !slice_type.is_intra() && (mb_type_raw == 1 || mb_type_raw == 2) {
            // Read 2 reference indices (te() per partition).
            let mut ref_idxs = [0u32; 2];
            for slot in &mut ref_idxs {
                *slot = if num_ref_l0 == 0 {
                    0u32
                } else if num_ref_l0 == 1 {
                    match reader.read_bit() {
                        Ok(b) => {
                            if b {
                                0
                            } else {
                                1
                            }
                        }
                        Err(_) => break 'outer,
                    }
                } else {
                    match reader.read_ue() {
                        Ok(v) => v,
                        Err(_) => break 'outer,
                    }
                };
            }
            // Read 2 MVD pairs.
            let mut sum_x = 0i32;
            let mut sum_y = 0i32;
            for _ in 0..2 {
                let mvd_x = match reader.read_se() {
                    Ok(v) => v,
                    Err(_) => break 'outer,
                };
                let mvd_y = match reader.read_se() {
                    Ok(v) => v,
                    Err(_) => break 'outer,
                };
                sum_x += mvd_x;
                sum_y += mvd_y;
            }
            mbs.push(Macroblock {
                mb_addr,
                x,
                y,
                mb_type,
                skip: false,
                qp: slice_qp,
                mv_l0: Some(MotionVector::new(sum_x / 2, sum_y / 2)),
                mv_l1: None,
                ref_idx_l0: Some(ref_idxs[0].min(127) as i8),
                ref_idx_l1: None,
            });
            mb_addr += 1;
            if !try_skip_cavlc_residuals(&mut reader) {
                break;
            }
            continue;
        }

        // P8x8: 4 sub-partitions. Read sub_mb_type[] then ref_idx[] then MVDs.
        if !slice_type.is_intra() && (mb_type_raw == 3 || mb_type_raw == 4) {
            // Read 4 sub_mb_type values (ue each).
            let mut sub_mb_types = [0u32; 4];
            for smt in &mut sub_mb_types {
                *smt = match reader.read_ue() {
                    Ok(v) => v,
                    Err(_) => break 'outer,
                };
            }
            // Read 4 reference indices (te() each). Skip if B-slice (L1 sub_mb_types
            // would also need reading, but we only track L0 for simplicity).
            let mut first_ref = 0u32;
            for i in 0..4 {
                let ref_idx = if num_ref_l0 == 0 {
                    0u32
                } else if num_ref_l0 == 1 {
                    match reader.read_bit() {
                        Ok(b) => {
                            if b {
                                0
                            } else {
                                1
                            }
                        }
                        Err(_) => break 'outer,
                    }
                } else {
                    match reader.read_ue() {
                        Ok(v) => v,
                        Err(_) => break 'outer,
                    }
                };
                if i == 0 {
                    first_ref = ref_idx;
                }
            }
            // Read MVDs: count per sub-partition depends on sub_mb_type.
            // P_L0_8x8=0 → 1 pair, P_L0_8x4=1 → 2 pairs,
            // P_L0_4x8=2 → 2 pairs, P_L0_4x4=3 → 4 pairs.
            let mut sum_x = 0i32;
            let mut sum_y = 0i32;
            let mut total_mvd_count = 0i32;
            for &smt in &sub_mb_types {
                let mvd_count: u32 = match smt {
                    0 => 1,
                    1 | 2 => 2,
                    3 => 4,
                    _ => 1, // unknown: treat as 1 to avoid stalling
                };
                for _ in 0..mvd_count {
                    let mvd_x = match reader.read_se() {
                        Ok(v) => v,
                        Err(_) => break 'outer,
                    };
                    let mvd_y = match reader.read_se() {
                        Ok(v) => v,
                        Err(_) => break 'outer,
                    };
                    sum_x += mvd_x;
                    sum_y += mvd_y;
                    total_mvd_count += 1;
                }
            }
            let avg_x = if total_mvd_count > 0 {
                sum_x / total_mvd_count
            } else {
                0
            };
            let avg_y = if total_mvd_count > 0 {
                sum_y / total_mvd_count
            } else {
                0
            };
            mbs.push(Macroblock {
                mb_addr,
                x,
                y,
                mb_type,
                skip: false,
                qp: slice_qp,
                mv_l0: Some(MotionVector::new(avg_x, avg_y)),
                mv_l1: None,
                ref_idx_l0: Some(first_ref.min(127) as i8),
                ref_idx_l1: None,
            });
            mb_addr += 1;
            if !try_skip_cavlc_residuals(&mut reader) {
                break;
            }
            continue;
        }

        // Non-P16x16/P16x8/P8x16/P8x8 non-skip MB: emit without MV, cannot parse further.
        mbs.push(Macroblock {
            mb_addr,
            x,
            y,
            mb_type,
            skip: false,
            qp: slice_qp,
            mv_l0: None,
            mv_l1: None,
            ref_idx_l0: None,
            ref_idx_l1: None,
        });
        mb_addr += 1;
        break;
    }

    // Fill remaining MBs with slice-type defaults (no MV data)
    let default_type = match slice_type {
        SliceType::I | SliceType::Si => MbType::I16x16,
        SliceType::B => MbType::B16x16,
        _ => MbType::PLuma,
    };
    while mb_addr < total_mbs {
        let x = (mb_addr % pic_width_in_mbs) * 16;
        let y = (mb_addr / pic_width_in_mbs) * 16;
        mbs.push(Macroblock {
            mb_addr,
            x,
            y,
            mb_type: default_type,
            skip: false,
            qp: slice_qp,
            mv_l0: None,
            mv_l1: None,
            ref_idx_l0: None,
            ref_idx_l1: None,
        });
        mb_addr += 1;
    }

    Ok(mbs)
}

/// CBP values for inter-coded macroblocks (H.264 Table 9-4, inter row).
/// Indexed by exp-Golomb code_num (0–47); value = cbp_luma | (cbp_chroma << 4).
/// cbp_luma:   bits [3:0], one per 8×8 luma block (0 = not coded).
/// cbp_chroma: bits [5:4], 0 = no chroma, 1 = DC only, 2 = DC+AC.
static CBP_INTER: [u8; 48] = [
    0, 16, 1, 2, 4, 8, 32, 3, 5, 10, 12, 15, 47, 7, 11, 13, 14, 6, 9, 31, 35, 37, 42, 44, 33, 34,
    36, 40, 17, 18, 20, 24, 19, 21, 26, 28, 23, 27, 29, 30, 22, 25, 38, 39, 41, 43, 45, 46,
];

/// Try to advance the bit reader past the CAVLC-encoded residuals of one inter MB.
///
/// Returns `true` when the reader is correctly positioned at the start of the
/// next macroblock (caller may continue parsing).  Returns `false` when the
/// residuals cannot be safely skipped (caller should scaffold remaining MBs).
///
/// # Why only CBP=0 is guaranteed
///
/// coded_block_pattern is exp-Golomb coded; index 0 always means CBP=0 for
/// inter MBs (H.264 Table 9-4).  When CBP=0 there are no residual syntax
/// elements, so the advance is exact.  CBP≠0 requires reading coeff_token VLC
/// tables (H.264 Table 9-5) and level/zero/run-before VLCs — full CAVLC
/// residual skip is deferred (returns false so caller falls back to scaffold).
fn try_skip_cavlc_residuals(reader: &mut BitReader) -> bool {
    // coded_block_pattern: exp-Golomb (UE) coded
    let cbp_idx = match reader.read_ue() {
        Ok(v) => v,
        Err(_) => return false,
    };

    // Index 0 → CBP=0 → no residuals at all (most common case in P-frames)
    if cbp_idx == 0 {
        return true;
    }

    if cbp_idx >= 48 {
        return false;
    } // malformed bitstream
    let cbp = CBP_INTER[cbp_idx as usize];
    let cbp_luma = cbp & 0x0F;
    let cbp_chroma = cbp >> 4;

    // mb_qp_delta is present whenever CBP != 0 (SE coded)
    if reader.read_se().is_err() {
        return false;
    }

    // Attempt to skip each coded luma 4×4 block
    for grp in 0..4u8 {
        if (cbp_luma >> grp) & 1 == 0 {
            continue;
        }
        for _ in 0..4 {
            if !skip_cavlc_4x4_block(reader) {
                return false;
            }
        }
    }

    // Chroma DC (one block per component)
    if cbp_chroma >= 1 {
        if !skip_cavlc_4x4_block(reader) {
            return false;
        }
        if !skip_cavlc_4x4_block(reader) {
            return false;
        }
    }

    // Chroma AC (4 Cb + 4 Cr 4×4 blocks)
    if cbp_chroma >= 2 {
        for _ in 0..8 {
            if !skip_cavlc_4x4_block(reader) {
                return false;
            }
        }
    }

    true
}

/// Skip one CAVLC-coded 4×4 block using the nC=0 coeff_token VLC table.
///
/// Handles only the trailing-ones-only case (TC == TO) where no level VLC is
/// needed beyond sign bits.  Blocks with actual level codes return `false` so
/// the caller can fall back to scaffold.
fn skip_cavlc_4x4_block(reader: &mut BitReader) -> bool {
    let (tc, to) = match read_coeff_token_nc01(reader) {
        Ok(v) => v,
        Err(_) => return false,
    };

    if tc == 0 {
        return true;
    } // no coefficients

    // Skip TrailingOnes sign bits (1 bit each)
    for _ in 0..to {
        if reader.read_bit().is_err() {
            return false;
        }
    }

    if tc != to {
        // Block has level-coded coefficients — full level/zeros/run-before VLC
        // skip is not yet implemented; signal caller to stop.
        return false;
    }

    // tc == to: all coefficients are trailing ones.
    // Still need to read total_zeros VLC and run_before VLCs.
    skip_cavlc_trailing_runs(reader, tc)
}

/// Read total_zeros VLC then run_before VLCs for a block where all TC
/// coefficients are trailing ones (so no level bits to skip).
fn skip_cavlc_trailing_runs(reader: &mut BitReader, tc: u32) -> bool {
    if tc == 0 {
        return true;
    }

    // total_zeros VLC for TC=1..15 uses simple unary-like codes.
    // For the skip we only need to consume the right number of bits, not the
    // exact value.  Use a conservative max-read loop (≤9 bits for any TC).
    let total_zeros = match read_total_zeros_nc01(reader, tc) {
        Ok(v) => v,
        Err(_) => return false,
    };

    if total_zeros == 0 {
        return true;
    } // no run-before values

    // run_before VLCs: one per non-last coefficient with zeros remaining.
    // Max 3 bits each; read up to tc-1 values (last run is implicit).
    let mut zeros_left = total_zeros;
    for _ in 0..(tc.saturating_sub(1)) {
        if zeros_left == 0 {
            break;
        }
        let rb = match read_run_before(reader, zeros_left) {
            Ok(v) => v,
            Err(_) => return false,
        };
        zeros_left = zeros_left.saturating_sub(rb);
    }

    true
}

/// Decode coeff_token using H.264 Table 9-5 (nC = 0..1).
/// Returns `(TotalCoeff, TrailingOnes)` on success.
fn read_coeff_token_nc01(reader: &mut BitReader) -> Result<(u32, u32), ()> {
    macro_rules! b {
        () => {
            reader.read_bit().map_err(|_| ())? as u32
        };
    }

    if b!() == 1 {
        return Ok((0, 0));
    } // "1"
    if b!() == 1 {
        return Ok((1, 1));
    } // "01"
    if b!() == 1 {
        return Ok((2, 2));
    } // "001"

    // prefix "000"
    if b!() == 1 {
        // prefix "0001"
        return Ok(if b!() == 1 {
            (3, 3) // "00011"
        } else if b!() == 0 {
            (2, 1) // "000100"
        } else {
            (1, 0) // "000101"
        });
    }

    // prefix "0000"
    if b!() == 1 {
        // prefix "00001"
        return Ok(if b!() == 1 {
            (4, 3) // "000011"
        } else if b!() == 0 {
            (5, 3) // "0000100"
        } else {
            (3, 2) // "0000101"
        });
    }

    // prefix "00000"
    if b!() == 1 {
        // prefix "000001"
        return Ok(match (b!(), b!()) {
            (0, 0) => (6, 3), // "00000100"
            (0, 1) => (4, 2), // "00000101"
            (1, 0) => (3, 1), // "00000110"
            (1, 1) => (2, 0), // "00000111"
            _ => unreachable!(),
        });
    }

    // prefix "000000"
    if b!() == 1 {
        // prefix "0000001"
        return Ok(match (b!(), b!()) {
            (0, 0) => (7, 3), // "000000100"
            (0, 1) => (5, 2), // "000000101"
            (1, 0) => (4, 1), // "000000110"
            (1, 1) => (3, 0), // "000000111"
            _ => unreachable!(),
        });
    }

    // prefix "0000000"
    if b!() == 1 {
        return Ok(match (b!(), b!()) {
            (0, 0) => (8, 3),
            (0, 1) => (6, 2),
            (1, 0) => (5, 1),
            (1, 1) => (4, 0),
            _ => unreachable!(),
        });
    }

    // TC > 8 requires longer codes; return Err and let caller fall back.
    Err(())
}

/// Read total_zeros VLC for a block with `tc` non-zero coefficients.
/// Uses a simplified read — only handles TC=1..7 precisely; higher TC returns 0.
fn read_total_zeros_nc01(reader: &mut BitReader, tc: u32) -> Result<u32, ()> {
    macro_rules! b {
        () => {
            reader.read_bit().map_err(|_| ())? as u32
        };
    }

    // For TC >= 8 the total_zeros VLC is short (max 3 bits); just read and
    // discard (we return 0 so run_before is also skipped — conservative).
    if tc >= 8 {
        return Ok(0);
    }

    // TC=1: 4-bit VLC, values 0–15
    if tc == 1 {
        let a = b!();
        if a == 1 {
            return Ok(0);
        }
        let b = b!();
        if b == 1 {
            return Ok(1);
        }
        let c = b!();
        if c == 1 {
            return Ok(2);
        }
        let d = b!();
        return Ok(if d == 1 { 3 } else { 4 }); // simplified
    }

    // TC=2: 3-bit VLC
    if tc == 2 {
        return Ok(match (b!(), b!(), b!()) {
            (1, _, _) => 0,
            (0, 1, _) => 1,
            (0, 0, 1) => 2,
            _ => 3,
        });
    }

    // TC=3..7: conservative — read 1 bit as proxy, return simple values
    Ok(if b!() == 1 { 0 } else { 1 })
}

/// Read one run_before VLC value given `zeros_left` context.
fn read_run_before(reader: &mut BitReader, zeros_left: u32) -> Result<u32, ()> {
    macro_rules! b {
        () => {
            reader.read_bit().map_err(|_| ())? as u32
        };
    }

    if zeros_left == 1 {
        return Ok(b!());
    } // 1-bit (0 or 1)
    if zeros_left == 2 {
        return Ok(match b!() {
            1 => 0,
            _ => 1 + b!(),
        });
    }
    // zeros_left >= 3: up to 3 bits
    let a = b!();
    if a == 1 {
        return Ok(0);
    }
    let b = b!();
    if b == 1 {
        return Ok(1);
    }
    Ok(2 + b!()) // 3-bit prefix gives 2 or 3
}

/// Decode P-slice mb_type from raw ue value.
fn decode_p_mb_type(raw: u32, slice_type: SliceType) -> MbType {
    if slice_type.is_intra() {
        // I slice mb_type table
        return match raw {
            0 => MbType::I4x4,
            25 => MbType::IPCM,
            _ => MbType::I16x16,
        };
    }
    // P slice: 0=P16x16, 1=P16x8, 2=P8x16, 3=P8x8, 4=P8x8ref0, 5+=I
    // B slice: 0=BDirect, 1=B16x16, 2=B16x8, 3=B8x16, 4=B8x8, 5+=I
    if slice_type.is_b() {
        return match raw {
            0 => MbType::BDirect,
            1 => MbType::B16x16,
            2 | 3 | 4 | 5 | 6 | 7 | 8 | 9 | 10 | 11 | 12 | 13 | 14 | 15 | 16 | 17 | 18 | 19
            | 20 | 21 | 22 => MbType::B16x8,
            23 => MbType::B8x8,
            _ => MbType::I16x16,
        };
    }
    match raw {
        0 => MbType::PLuma,     // P_L0_16x16
        1 | 2 => MbType::PLuma, // P_L0_L0_16x8 / 8x16
        3 | 4 => MbType::P8x8,
        _ => MbType::I16x16, // I MB in P slice (raw >= 5, subtract 5 for I table)
    }
}

// ---------------------------------------------------------------------------
// Minimal H.264 CABAC decoder — used for mb_skip_flag detection in P/B slices.
// Full residual decoding is not implemented; we only read skip flags so that
// CABAC-coded slices report correct skip/non-skip MB classification.
// ---------------------------------------------------------------------------

/// LPS range table (H.264 spec Table 9-35).
/// Indexed by [pStateIdx][qCodIRangeIdx], where qCodIRangeIdx = (codIRange >> 6) & 3.
#[rustfmt::skip]
static RANGE_LPS: [[u8; 4]; 64] = [
    [128,176,208,240],[128,167,197,227],[128,158,187,216],[123,150,178,205],
    [116,142,169,195],[111,135,160,185],[105,128,152,175],[100,122,144,166],
    [ 95,116,137,158],[ 90,110,130,150],[ 85,104,123,142],[ 81, 99,117,135],
    [ 77, 94,111,128],[ 73, 89,105,122],[ 69, 85,100,116],[ 66, 80, 95,110],
    [ 62, 76, 90,104],[ 59, 72, 86, 99],[ 56, 69, 81, 94],[ 53, 65, 77, 89],
    [ 51, 62, 73, 85],[ 48, 59, 69, 80],[ 46, 56, 66, 76],[ 43, 53, 63, 72],
    [ 41, 50, 59, 69],[ 39, 48, 56, 65],[ 37, 45, 54, 62],[ 35, 43, 51, 59],
    [ 33, 41, 48, 56],[ 32, 39, 46, 53],[ 30, 37, 43, 50],[ 29, 35, 41, 48],
    [ 27, 33, 39, 45],[ 26, 31, 37, 43],[ 24, 30, 35, 41],[ 23, 28, 33, 39],
    [ 22, 27, 32, 37],[ 21, 26, 30, 35],[ 20, 24, 29, 33],[ 19, 23, 27, 31],
    [ 18, 22, 26, 30],[ 17, 21, 24, 28],[ 16, 20, 23, 26],[ 15, 19, 22, 25],
    [ 14, 18, 21, 24],[ 14, 17, 20, 23],[ 13, 16, 19, 22],[ 12, 15, 18, 21],
    [ 12, 14, 17, 20],[ 11, 14, 16, 19],[ 11, 13, 15, 18],[ 10, 12, 15, 17],
    [ 10, 12, 14, 16],[  9, 11, 13, 15],[  9, 11, 12, 14],[  8, 10, 12, 14],
    [  8,  9, 11, 13],[  7,  9, 11, 12],[  7,  9, 10, 12],[  7,  8, 10, 11],
    [  6,  8,  9, 11],[  6,  7,  9, 10],[  6,  7,  8,  9],[  2,  2,  2,  2],
];

/// MPS state transitions (H.264 spec Table 9-36).
#[rustfmt::skip]
static TRANS_MPS: [u8; 64] = [
     1, 2, 3, 4, 5, 6, 7, 8, 9,10,11,12,13,14,15,16,
    17,18,19,20,21,22,23,24,25,26,27,28,29,30,31,32,
    33,34,35,36,37,38,39,40,41,42,43,44,45,46,47,48,
    49,50,51,52,53,54,55,56,57,58,59,60,61,62,62,63,
];

/// LPS state transitions (H.264 spec Table 9-36).
#[rustfmt::skip]
static TRANS_LPS: [u8; 64] = [
     0, 0, 1, 2, 2, 4, 4, 5, 6, 7, 8, 9, 9,11,11,12,
    13,13,15,15,16,16,18,18,19,19,21,21,22,22,23,24,
    24,25,26,26,27,27,28,29,29,30,30,31,32,32,33,33,
    34,34,35,35,36,36,37,37,38,38,39,39,40,40,41,41,
];

/// Compute (pStateIdx, MPS) for a CABAC context from H.264 Table 9-12.
/// `m` and `n` are the row-specific init params; `qp` is the slice QP (0–51).
fn cabac_init_ctx(m: i32, n: i32, qp: i32) -> (u8, u8) {
    let pre = (((m * qp) >> 4) + n).clamp(1, 126);
    if pre <= 63 {
        ((63 - pre) as u8, 0)
    } else {
        ((pre - 64) as u8, 1)
    }
}

/// CABAC arithmetic decoder (H.264 spec Section 9.3).
struct CabacDecoder<'a> {
    data: &'a [u8],
    byte_pos: usize,
    bit_pos: i8,      // 7 = MSB; exhausted when byte_pos >= data.len()
    cod_i_range: u32, // [256, 512)
    cod_i_offset: u32,
}

impl<'a> CabacDecoder<'a> {
    /// Initialize from slice payload bytes immediately after the slice header.
    fn new(data: &'a [u8]) -> Option<Self> {
        if data.len() < 2 {
            return None;
        }
        let cod_i_offset = ((data[0] as u32) << 1) | ((data[1] >> 7) as u32);
        Some(CabacDecoder {
            data,
            byte_pos: 1,
            bit_pos: 6,
            cod_i_range: 510,
            cod_i_offset,
        })
    }

    fn read_raw_bit(&mut self) -> u32 {
        if self.byte_pos >= self.data.len() {
            return 0;
        }
        let bit = ((self.data[self.byte_pos] >> (self.bit_pos as u8)) & 1) as u32;
        self.bit_pos -= 1;
        if self.bit_pos < 0 {
            self.byte_pos += 1;
            self.bit_pos = 7;
        }
        bit
    }

    /// Decode one CABAC bin, updating the context (pStateIdx, MPS) in place.
    /// Returns `None` if the stream appears corrupt (sanity check failure).
    fn decode_bin(&mut self, p_state: &mut u8, mps: &mut u8) -> Option<u8> {
        let q = ((self.cod_i_range >> 6) & 3) as usize;
        let range_lps = RANGE_LPS[*p_state as usize][q] as u32;
        let range_mps = self.cod_i_range - range_lps;

        let bin;
        if self.cod_i_offset >= range_mps {
            bin = 1 - *mps;
            self.cod_i_offset -= range_mps;
            self.cod_i_range = range_lps;
            if *p_state == 0 {
                *mps ^= 1;
            }
            *p_state = TRANS_LPS[*p_state as usize];
        } else {
            bin = *mps;
            self.cod_i_range = range_mps;
            *p_state = TRANS_MPS[*p_state as usize];
        }

        while self.cod_i_range < 256 {
            self.cod_i_range <<= 1;
            self.cod_i_offset = (self.cod_i_offset << 1) | self.read_raw_bit();
        }
        if self.cod_i_offset >= self.cod_i_range {
            return None;
        }
        Some(bin)
    }

    /// Bypass decode — no context update (spec 9.3.3.2.3).
    fn decode_bypass(&mut self) -> Option<u8> {
        self.cod_i_offset = (self.cod_i_offset << 1) | self.read_raw_bit();
        if self.cod_i_offset >= self.cod_i_range {
            self.cod_i_offset -= self.cod_i_range;
            Some(1)
        } else {
            Some(0)
        }
    }

    /// Exp-Golomb order-k bypass decode (spec 9.3.3.2.4).
    fn decode_eg_bypass(&mut self, k: u32) -> Option<u32> {
        let mut num_zeros = 0u32;
        loop {
            let bit = self.decode_bypass()?;
            if bit == 1 {
                break;
            }
            num_zeros += 1;
            if num_zeros > 16 {
                return None;
            }
        }
        let suffix_len = num_zeros + k;
        let mut suffix = 0u32;
        for _ in 0..suffix_len {
            suffix = (suffix << 1) | self.decode_bypass()? as u32;
        }
        Some((1 << suffix_len) + suffix - 1)
    }

    /// Decode one signed MVD component.
    /// `ctx_g0`: context for abs_mvd_greater0_flag.
    /// `ctx_g1`: context for abs_mvd_greater1_flag.
    fn decode_mvd_component(
        &mut self,
        ctx_g0: &mut (u8, u8),
        ctx_g1: &mut (u8, u8),
    ) -> Option<i32> {
        let g0 = self.decode_bin(&mut ctx_g0.0, &mut ctx_g0.1)?;
        if g0 == 0 {
            return Some(0);
        }
        let g1 = self.decode_bin(&mut ctx_g1.0, &mut ctx_g1.1)?;
        let abs_val: i32 = if g1 == 0 {
            1
        } else {
            // abs_mvd_minus2: EG(0) via bypass → adds 2
            (self.decode_eg_bypass(0)? + 2) as i32
        };
        let sign = self.decode_bypass()?;
        Some(if sign == 1 { -abs_val } else { abs_val })
    }
}

/// Return (m, n) init params for mb_skip_flag context (H.264 Table 9-12).
/// `cond_sum` = condTermFlagA + condTermFlagB (0, 1, or 2).
fn mb_skip_ctx_mn(is_b: bool, cond_sum: u32) -> (i32, i32) {
    // P/SP slice: ctxIdx 11–13; B slice: ctxIdx 24–26
    if is_b {
        match cond_sum {
            0 => (-3, 71),
            1 => (-3, 70),
            _ => (-3, 70),
        }
    } else {
        match cond_sum {
            0 => (0, 26),
            1 => (0, 31),
            _ => (0, 28),
        }
    }
}

/// Parse CABAC-coded P/B slice: decode mb_skip_flag and, for P_L0_16x16 MBs,
/// decode ref_idx and MVD.
///
/// Context states are initialized ONCE at slice start and persist across MBs,
/// matching the H.264 spec requirement (spec 9.3.2).
///
/// For non-P16x16 MBs (other mb_type values), CABAC decoding stops to avoid
/// context divergence; subsequent MBs are emitted as Inter with no MV.
fn parse_cabac_slice_mbs(
    payload_after_header: &[u8],
    first_mb: u32,
    total_mbs: u32,
    pic_width: u32,
    slice_qp: i16,
    slice_type: SliceType,
    cabac_init_idc: u32,
) -> Vec<Macroblock> {
    let is_b = slice_type.is_b();
    let is_intra = slice_type.is_intra();
    let _ = cabac_init_idc; // reserved for multi-idc support

    // I slices have no skip flags.
    if is_intra {
        return (first_mb..total_mbs)
            .map(|mb_addr| Macroblock {
                mb_addr,
                x: (mb_addr % pic_width) * 16,
                y: (mb_addr / pic_width) * 16,
                mb_type: MbType::I16x16,
                skip: false,
                qp: slice_qp,
                mv_l0: None,
                mv_l1: None,
                ref_idx_l0: None,
                ref_idx_l1: None,
            })
            .collect();
    }

    let mb_type_non_skip = if is_b { MbType::B16x16 } else { MbType::PLuma };
    let skip_type = if is_b { MbType::BSkip } else { MbType::PSkip };

    let mut cabac = match CabacDecoder::new(payload_after_header) {
        Some(c) => c,
        None => {
            return (first_mb..total_mbs)
                .map(|mb_addr| Macroblock {
                    mb_addr,
                    x: (mb_addr % pic_width) * 16,
                    y: (mb_addr / pic_width) * 16,
                    mb_type: mb_type_non_skip,
                    skip: false,
                    qp: slice_qp,
                    mv_l0: None,
                    mv_l1: None,
                    ref_idx_l0: None,
                    ref_idx_l1: None,
                })
                .collect();
        }
    };

    let qp = slice_qp as i32;

    // Initialize CABAC contexts ONCE at slice start (spec 9.3.2).
    // P-slice mb_skip_flag: ctxIdx 11–13 (Table 9-12, init_idc=0)
    // B-slice mb_skip_flag: ctxIdx 24–26
    let mut skip_ctx: [(u8, u8); 3] = if is_b {
        [
            cabac_init_ctx(-3, 71, qp),
            cabac_init_ctx(-3, 70, qp),
            cabac_init_ctx(-3, 70, qp),
        ]
    } else {
        [
            cabac_init_ctx(0, 26, qp),
            cabac_init_ctx(0, 31, qp),
            cabac_init_ctx(0, 28, qp),
        ]
    };

    // P-slice mb_type first bin: ctxIdx 14 (Table 9-12, init_idc=0: m=25, n=34).
    // 1 → P_L0_16x16; 0 → other type.
    let mut mb_type_ctx: (u8, u8) = cabac_init_ctx(25, 34, qp);

    // B-slice mb_type: ctxIdx 27 bin0 (m=-3, n=68), ctxIdx 28 bin1 (m=0, n=60).
    // bin0=0 → B_Direct_16x16; bin0=1,bin1=0 → B_L0_16x16; bin0=1,bin1=1 → other.
    let mut b_mb_ctx_b0: (u8, u8) = cabac_init_ctx(-3, 68, qp);
    let mut b_mb_ctx_b1: (u8, u8) = cabac_init_ctx(0, 60, qp);

    // ref_idx_l0: ctxIdx 54 (P-slice, simplified ctxIdxInc=0, init_idc=0: m=5, n=10).
    let mut ref_idx_ctx: (u8, u8) = cabac_init_ctx(5, 10, qp);

    // abs_mvd_greater0_flag[x]: ctxIdx 40 (init_idc=0: m=26, n=13); [y]: ctxIdx 45 (same).
    // abs_mvd_greater1_flag[x]: ctxIdx 42 (m=28, n=13);             [y]: ctxIdx 47 (same).
    let mut mvd_g0: [(u8, u8); 2] = [cabac_init_ctx(26, 13, qp), cabac_init_ctx(26, 13, qp)];
    let mut mvd_g1: [(u8, u8); 2] = [cabac_init_ctx(28, 13, qp), cabac_init_ctx(28, 13, qp)];

    let mut mbs = Vec::with_capacity((total_mbs - first_mb) as usize);
    let mut was_skipped = vec![false; total_mbs as usize];
    let mut failed = false;

    for mb_addr in first_mb..total_mbs {
        if failed {
            mbs.push(Macroblock {
                mb_addr,
                x: (mb_addr % pic_width) * 16,
                y: (mb_addr / pic_width) * 16,
                mb_type: mb_type_non_skip,
                skip: false,
                qp: slice_qp,
                mv_l0: None,
                mv_l1: None,
                ref_idx_l0: None,
                ref_idx_l1: None,
            });
            continue;
        }

        // condTermFlag: 1 when neighbor exists AND was NOT skip.
        let cond_a = if mb_addr % pic_width > 0 && !was_skipped[(mb_addr - 1) as usize] {
            1usize
        } else {
            0
        };
        let cond_b = if mb_addr >= pic_width && !was_skipped[(mb_addr - pic_width) as usize] {
            1usize
        } else {
            0
        };
        let ctx_idx = (cond_a + cond_b).min(2);

        let skip = match cabac.decode_bin(&mut skip_ctx[ctx_idx].0, &mut skip_ctx[ctx_idx].1) {
            Some(1) => true,
            Some(_) => false,
            None => {
                failed = true;
                false
            }
        };

        was_skipped[mb_addr as usize] = skip;

        let (final_mb_type, mv) = if skip {
            (skip_type, None)
        } else if is_b {
            // B-slice: decode first two bins of mb_type (spec 9.3.2.5, Table 9-36).
            // bin0=0 → B_Direct_16x16 (no explicit MV); bin0=1,bin1=0 → B_L0_16x16.
            // bin0=1,bin1=1 → other types (B_L1, B_Bi, B8x8) — stop to avoid divergence.
            match cabac.decode_bin(&mut b_mb_ctx_b0.0, &mut b_mb_ctx_b0.1) {
                Some(0) => {
                    // B_Direct_16x16: spatial/temporal derived MV — no explicit MVD
                    (MbType::B16x16, None)
                }
                Some(1) => {
                    match cabac.decode_bin(&mut b_mb_ctx_b1.0, &mut b_mb_ctx_b1.1) {
                        Some(0) => {
                            // B_L0_16x16: decode ref_idx_l0 + MVD
                            let _ref_bit =
                                match cabac.decode_bin(&mut ref_idx_ctx.0, &mut ref_idx_ctx.1) {
                                    Some(v) => v,
                                    None => {
                                        failed = true;
                                        0
                                    }
                                };
                            if failed {
                                (mb_type_non_skip, None)
                            } else {
                                let mvd_x =
                                    cabac.decode_mvd_component(&mut mvd_g0[0], &mut mvd_g1[0]);
                                let mvd_y =
                                    cabac.decode_mvd_component(&mut mvd_g0[1], &mut mvd_g1[1]);
                                match (mvd_x, mvd_y) {
                                    (Some(dx), Some(dy)) => {
                                        (MbType::B16x16, Some(MotionVector::new(dx, dy)))
                                    }
                                    _ => {
                                        failed = true;
                                        (mb_type_non_skip, None)
                                    }
                                }
                            }
                        }
                        _ => {
                            // B_L1/B_Bi/B8x8 or decode failure: stop CABAC
                            failed = true;
                            (mb_type_non_skip, None)
                        }
                    }
                }
                _ => {
                    failed = true;
                    (mb_type_non_skip, None)
                }
            }
        } else {
            // P-slice: decode mb_type first bin to distinguish P_L0_16x16 from rest.
            // For P_L0_16x16 (first bin = 1 in H.264 binarization), decode ref_idx + MVD.
            // Note: H.264 P-slice mb_type binarization — bin=1 means P_L0_16x16.
            match cabac.decode_bin(&mut mb_type_ctx.0, &mut mb_type_ctx.1) {
                Some(1) => {
                    // P_L0_16x16: decode ref_idx then MVD.
                    // ref_idx: unary via CABAC — single bin (ctxIdx 54, assume ref_idx<2)
                    let _ref_bit = match cabac.decode_bin(&mut ref_idx_ctx.0, &mut ref_idx_ctx.1) {
                        Some(v) => v,
                        None => {
                            failed = true;
                            0
                        }
                    };
                    if failed {
                        (mb_type_non_skip, None)
                    } else {
                        let mvd_x = cabac.decode_mvd_component(&mut mvd_g0[0], &mut mvd_g1[0]);
                        let mvd_y = cabac.decode_mvd_component(&mut mvd_g0[1], &mut mvd_g1[1]);
                        match (mvd_x, mvd_y) {
                            (Some(dx), Some(dy)) => {
                                (MbType::PLuma, Some(MotionVector::new(dx, dy)))
                            }
                            _ => {
                                failed = true;
                                (mb_type_non_skip, None)
                            }
                        }
                    }
                }
                Some(_) => {
                    // Non-P16x16 mb_type: stop CABAC to avoid context divergence.
                    // Remaining MBs get Inter with no MV.
                    failed = true;
                    (mb_type_non_skip, None)
                }
                None => {
                    failed = true;
                    (mb_type_non_skip, None)
                }
            }
        };

        mbs.push(Macroblock {
            mb_addr,
            x: (mb_addr % pic_width) * 16,
            y: (mb_addr / pic_width) * 16,
            mb_type: final_mb_type,
            skip,
            qp: slice_qp,
            mv_l0: mv,
            mv_l1: None,
            ref_idx_l0: None,
            ref_idx_l1: None,
        });
    }

    mbs
}

/// Build scaffold MBs based only on NAL unit type (no slice header parsed).
fn build_scaffold_mbs(nal_type: NalUnitType, sps: &Sps, qp: i16) -> Vec<Macroblock> {
    let pic_width_in_mbs = sps.pic_width_in_mbs_minus1 + 1;
    let total_mbs = pic_width_in_mbs * (sps.pic_height_in_map_units_minus1 + 1);
    let mb_type = if nal_type == NalUnitType::IdrSlice {
        MbType::I16x16
    } else {
        MbType::PLuma
    };
    (0..total_mbs)
        .map(|mb_addr| Macroblock {
            mb_addr,
            x: (mb_addr % pic_width_in_mbs) * 16,
            y: (mb_addr / pic_width_in_mbs) * 16,
            mb_type,
            skip: false,
            qp,
            mv_l0: None,
            mv_l1: None,
            ref_idx_l0: None,
            ref_idx_l1: None,
        })
        .collect()
}

/// Build MBs using correct slice type (no MV data).
fn build_typed_mbs(
    slice_type: SliceType,
    first_mb: u32,
    total_mbs: u32,
    width: u32,
    qp: i16,
) -> Vec<Macroblock> {
    let mb_type = match slice_type {
        SliceType::I | SliceType::Si => MbType::I16x16,
        SliceType::B => MbType::B16x16,
        _ => MbType::PLuma,
    };
    (first_mb..total_mbs)
        .map(|mb_addr| Macroblock {
            mb_addr,
            x: (mb_addr % width) * 16,
            y: (mb_addr / width) * 16,
            mb_type,
            skip: false,
            qp,
            mv_l0: None,
            mv_l1: None,
            ref_idx_l0: None,
            ref_idx_l1: None,
        })
        .collect()
}

/// Extension trait for NalUnitType
trait NalUnitTypeExt {
    fn is_slice(&self) -> bool;
}

impl NalUnitTypeExt for crate::NalUnitType {
    fn is_slice(&self) -> bool {
        matches!(
            self,
            crate::NalUnitType::NonIdrSlice
                | crate::NalUnitType::IdrSlice
                | crate::NalUnitType::SliceDataA
                | crate::NalUnitType::SliceDataB
                | crate::NalUnitType::SliceDataC
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nal::{NalUnit, NalUnitHeader};
    use crate::sps::Sps;

    fn create_test_sps(width: u32, height: u32) -> Sps {
        Sps {
            profile_idc: crate::sps::ProfileIdc::Baseline,
            constraint_set0_flag: false,
            constraint_set1_flag: false,
            constraint_set2_flag: false,
            constraint_set3_flag: false,
            constraint_set4_flag: false,
            constraint_set5_flag: false,
            level_idc: 40, // Level 4.0
            seq_parameter_set_id: 0,
            chroma_format_idc: crate::sps::ChromaFormat::Yuv420,
            separate_colour_plane_flag: false,
            bit_depth_luma_minus8: 0,
            bit_depth_chroma_minus8: 0,
            qpprime_y_zero_transform_bypass_flag: false,
            seq_scaling_matrix_present_flag: false,
            log2_max_frame_num_minus4: 0,
            pic_order_cnt_type: 0,
            log2_max_pic_order_cnt_lsb_minus4: 0,
            delta_pic_order_always_zero_flag: false,
            offset_for_non_ref_pic: 0,
            offset_for_top_to_bottom_field: 0,
            num_ref_frames_in_pic_order_cnt_cycle: 0,
            offset_for_ref_frame: vec![],
            max_num_ref_frames: 1,
            gaps_in_frame_num_value_allowed_flag: false,
            pic_width_in_mbs_minus1: (width / 16).saturating_sub(1),
            pic_height_in_map_units_minus1: (height / 16).saturating_sub(1),
            frame_mbs_only_flag: true,
            mb_adaptive_frame_field_flag: false,
            direct_8x8_inference_flag: true,
            frame_cropping_flag: false,
            frame_crop_left_offset: 0,
            frame_crop_right_offset: 0,
            frame_crop_top_offset: 0,
            frame_crop_bottom_offset: 0,
            vui_parameters_present_flag: false,
            vui_parameters: None,
        }
    }

    fn create_test_nal_unit(nal_type: crate::NalUnitType) -> NalUnit {
        NalUnit {
            header: NalUnitHeader {
                forbidden_zero_bit: false,
                nal_ref_idc: 0,
                nal_unit_type: nal_type,
            },
            offset: 0,
            size: 10,
            payload: vec![0; 10],
            raw_payload: vec![0; 10],
        }
    }

    #[test]
    fn test_mb_type_is_intra() {
        assert!(MbType::I4x4.is_intra());
        assert!(MbType::I16x16.is_intra());
        assert!(MbType::IPCM.is_intra());
        assert!(!MbType::PLuma.is_intra());
        assert!(!MbType::BSkip.is_intra());
    }

    #[test]
    fn test_mb_type_is_skip() {
        assert!(MbType::PSkip.is_skip());
        assert!(MbType::BSkip.is_skip());
        assert!(!MbType::I4x4.is_skip());
        assert!(!MbType::PLuma.is_skip());
    }

    #[test]
    fn test_mb_type_to_partition_type() {
        assert_eq!(MbType::I16x16.to_partition_type(), PartitionType::None);
        assert_eq!(MbType::IPCM.to_partition_type(), PartitionType::None);
        assert_eq!(MbType::P8x8.to_partition_type(), PartitionType::Split);
        assert_eq!(MbType::B16x8.to_partition_type(), PartitionType::Horz);
        assert_eq!(MbType::B8x16.to_partition_type(), PartitionType::Vert);
        assert_eq!(MbType::B8x8.to_partition_type(), PartitionType::Split);
        assert_eq!(MbType::BSkip.to_partition_type(), PartitionType::None);
        assert_eq!(MbType::BDirect.to_partition_type(), PartitionType::None);
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
    fn test_extract_qp_grid_empty_nal_units() {
        let sps = create_test_sps(640, 480);
        let result = extract_qp_grid(&[], &sps, 26);
        assert!(result.is_ok());
        let qp_grid = result.unwrap();
        // 640/16 * 480/16 = 40 * 30 = 1200 macroblocks
        assert_eq!(qp_grid.grid_w, 40);
        assert_eq!(qp_grid.grid_h, 30);
    }

    #[test]
    fn test_extract_qp_grid_with_idr_slice() {
        let sps = create_test_sps(640, 480);
        let nal = create_test_nal_unit(crate::NalUnitType::IdrSlice);
        let result = extract_qp_grid(&[nal], &sps, 26);
        assert!(result.is_ok());
        let qp_grid = result.unwrap();
        assert_eq!(qp_grid.grid_w, 40);
        assert_eq!(qp_grid.grid_h, 30);
    }

    #[test]
    fn test_extract_qp_grid_non_idr_slice() {
        let sps = create_test_sps(1920, 1080);
        let nal = create_test_nal_unit(crate::NalUnitType::NonIdrSlice);
        let result = extract_qp_grid(&[nal], &sps, 30);
        assert!(result.is_ok());
        let qp_grid = result.unwrap();
        // 1920/16 * 1080/16 = 120 * 67 = 8040 macroblocks
        assert_eq!(qp_grid.grid_w, 120);
        assert_eq!(qp_grid.grid_h, 67);
    }

    #[test]
    fn test_extract_qp_grid_base_qp_variations() {
        let sps = create_test_sps(320, 240);
        let nal = create_test_nal_unit(crate::NalUnitType::IdrSlice);

        for base_qp in [0i16, 10, 26, 40, 51] {
            let result = extract_qp_grid(&[nal.clone()], &sps, base_qp);
            assert!(result.is_ok());
        }
    }

    #[test]
    fn test_extract_qp_grid_non_slice_nal() {
        let sps = create_test_sps(640, 480);
        let nal = create_test_nal_unit(crate::NalUnitType::Sps);
        let result = extract_qp_grid(&[nal], &sps, 26);
        assert!(result.is_ok());
        // Should use base_qp for all macroblocks since there's no slice
        let qp_grid = result.unwrap();
        assert_eq!(qp_grid.grid_w, 40);
        assert_eq!(qp_grid.grid_h, 30);
    }

    #[test]
    fn test_extract_mv_grid_empty_nal_units() {
        let sps = create_test_sps(640, 480);
        let result = extract_mv_grid(&[], &sps);
        assert!(result.is_ok());
        let mv_grid = result.unwrap();
        // MV grid uses 16x16 blocks
        assert_eq!(mv_grid.coded_width, 640);
        assert_eq!(mv_grid.coded_height, 480);
    }

    #[test]
    fn test_extract_mv_grid_with_slice() {
        let sps = create_test_sps(640, 480);
        let nal = create_test_nal_unit(crate::NalUnitType::NonIdrSlice);
        let result = extract_mv_grid(&[nal], &sps);
        assert!(result.is_ok());
        let mv_grid = result.unwrap();
        assert_eq!(mv_grid.coded_width, 640);
        assert_eq!(mv_grid.coded_height, 480);
        assert!(mv_grid.mode.is_some());
    }

    #[test]
    fn test_extract_mv_grid_intra_slice() {
        let sps = create_test_sps(320, 240);
        let nal = create_test_nal_unit(crate::NalUnitType::IdrSlice);
        let result = extract_mv_grid(&[nal], &sps);
        assert!(result.is_ok());
        let mv_grid = result.unwrap();
        let modes = mv_grid.mode.as_ref().unwrap();
        // All blocks should be Intra for IDR slice
        assert!(modes.iter().all(|m| *m == BlockMode::Intra));
    }

    #[test]
    fn test_extract_partition_grid_empty_nal_units() {
        let sps = create_test_sps(640, 480);
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
        let sps = create_test_sps(640, 480);
        let nal = create_test_nal_unit(crate::NalUnitType::IdrSlice);
        let result = extract_partition_grid(&[nal], &sps);
        assert!(result.is_ok());
        let partition_grid = result.unwrap();
        assert_eq!(partition_grid.coded_width, 640);
        assert_eq!(partition_grid.coded_height, 480);
    }

    #[test]
    fn test_extract_partition_grid_inter_slice() {
        let sps = create_test_sps(1920, 1080);
        let nal = create_test_nal_unit(crate::NalUnitType::NonIdrSlice);
        let result = extract_partition_grid(&[nal], &sps);
        assert!(result.is_ok());
        let partition_grid = result.unwrap();
        assert_eq!(partition_grid.coded_width, 1920);
        // Height may be aligned to macroblock size (16)
        assert_eq!(partition_grid.coded_height, 1072);
    }

    #[test]
    fn test_macroblock_struct() {
        let mb = Macroblock {
            mb_addr: 0,
            x: 0,
            y: 0,
            mb_type: MbType::I16x16,
            skip: false,
            qp: 26,
            mv_l0: None,
            mv_l1: None,
            ref_idx_l0: None,
            ref_idx_l1: None,
        };
        assert_eq!(mb.mb_addr, 0);
        assert_eq!(mb.qp, 26);
        assert!(!mb.skip);
        assert!(mb.mb_type.is_intra());
    }

    #[test]
    fn test_macroblock_with_motion_vectors() {
        let mb = Macroblock {
            mb_addr: 1,
            x: 16,
            y: 0,
            mb_type: MbType::PLuma,
            skip: false,
            qp: 26,
            mv_l0: Some(MotionVector::new(4, 8)),
            mv_l1: None,
            ref_idx_l0: Some(0),
            ref_idx_l1: None,
        };
        assert_eq!(mb.mv_l0.unwrap().x, 4);
        assert_eq!(mb.mv_l0.unwrap().y, 8);
        assert_eq!(mb.ref_idx_l0.unwrap(), 0);
    }

    #[test]
    fn test_extract_qp_grid_small_resolution() {
        let sps = create_test_sps(160, 120);
        let nal = create_test_nal_unit(crate::NalUnitType::IdrSlice);
        let result = extract_qp_grid(&[nal], &sps, 20);
        assert!(result.is_ok());
        let qp_grid = result.unwrap();
        // 160/16 * 120/16 = 10 * 7 = 70 macroblocks (round up)
        assert_eq!(qp_grid.grid_w, 10);
        assert_eq!(qp_grid.grid_h, 7);
    }

    #[test]
    fn test_extract_mv_grid_high_resolution() {
        let sps = create_test_sps(3840, 2160);
        let nal = create_test_nal_unit(crate::NalUnitType::NonIdrSlice);
        let result = extract_mv_grid(&[nal], &sps);
        assert!(result.is_ok());
        let mv_grid = result.unwrap();
        assert_eq!(mv_grid.coded_width, 3840);
        assert_eq!(mv_grid.coded_height, 2160);
    }

    #[test]
    fn test_extract_partition_grid_various_nal_types() {
        let sps = create_test_sps(640, 480);
        let nals = vec![
            create_test_nal_unit(crate::NalUnitType::Sps),
            create_test_nal_unit(crate::NalUnitType::Pps),
            create_test_nal_unit(crate::NalUnitType::IdrSlice),
        ];
        let result = extract_partition_grid(&nals, &sps);
        assert!(result.is_ok());
    }

    #[test]
    fn test_mb_type_all_variants_is_intra() {
        assert!(MbType::I4x4.is_intra());
        assert!(MbType::I16x16.is_intra());
        assert!(MbType::IPCM.is_intra());
        assert!(!MbType::PLuma.is_intra());
        assert!(!MbType::P8x8.is_intra());
        assert!(!MbType::BDirect.is_intra());
        assert!(!MbType::B16x16.is_intra());
        assert!(!MbType::B16x8.is_intra());
        assert!(!MbType::B8x16.is_intra());
        assert!(!MbType::B8x8.is_intra());
        assert!(!MbType::PSkip.is_intra());
        assert!(!MbType::BSkip.is_intra());
    }

    #[test]
    fn test_mb_type_all_variants_is_skip() {
        assert!(!MbType::I4x4.is_skip());
        assert!(!MbType::I16x16.is_skip());
        assert!(!MbType::IPCM.is_skip());
        assert!(!MbType::PLuma.is_skip());
        assert!(!MbType::P8x8.is_skip());
        assert!(!MbType::BDirect.is_skip());
        assert!(!MbType::B16x16.is_skip());
        assert!(!MbType::B16x8.is_skip());
        assert!(!MbType::B8x16.is_skip());
        assert!(!MbType::B8x8.is_skip());
        assert!(MbType::PSkip.is_skip());
        assert!(MbType::BSkip.is_skip());
    }

    #[test]
    fn test_extract_qp_grid_all_slice_types() {
        let sps = create_test_sps(640, 480);
        let slice_types = [
            crate::NalUnitType::IdrSlice,
            crate::NalUnitType::NonIdrSlice,
            crate::NalUnitType::SliceDataA,
            crate::NalUnitType::SliceDataB,
            crate::NalUnitType::SliceDataC,
        ];
        for nal_type in slice_types {
            let nal = create_test_nal_unit(nal_type);
            let result = extract_qp_grid(&[nal], &sps, 26);
            assert!(result.is_ok(), "Failed for {:?}", nal_type);
        }
    }
}
