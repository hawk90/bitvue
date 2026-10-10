//! Overlay grid extraction: the public `extract_*_grid` entry points.

use crate::nal::{NalUnit, NalUnitType};
use crate::pps::{parse_pps, Pps};
use crate::sps::Sps;
use bitvue_engine::{
    mv_overlay::{BlockMode, MVGrid, MotionVector as CoreMV},
    partition_grid::{PartitionBlock, PartitionGrid, PartitionType},
    qp_heatmap::QPGrid,
    BitvueError,
};
use std::collections::HashMap;

use super::slice_mbs::*;
use super::types::*;

/// (grid_x, grid_y, block_w, block_h, values) for a single-plane u8 overlay grid.
pub(super) type U8GridResult = Result<(u32, u32, u32, u32, Vec<Option<u8>>), BitvueError>;

/// (grid_x, grid_y, block_w, block_h, l0_values, l1_values) for a dual-plane i8 overlay grid.
pub(super) type I8DualGridResult =
    Result<(u32, u32, u32, u32, Vec<Option<i8>>, Vec<Option<i8>>), BitvueError>;

/// Build SPS and PPS maps from NAL unit list.
pub(super) fn build_parameter_maps(nal_units: &[NalUnit]) -> (HashMap<u8, Sps>, HashMap<u8, Pps>) {
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
pub fn extract_prediction_mode_grid(nal_units: &[NalUnit], sps: &Sps) -> U8GridResult {
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
            if let Ok(mbs) = parse_slice_macroblocks(nal, &sps_map, &pps_map, sps, 26) {
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

/// Extension trait for NalUnitType
pub(super) trait NalUnitTypeExt {
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

/// Extract MB Type Grid from H.264 bitstream
///
/// Returns a 16×16 macroblock-resolution grid where each cell contains the
/// numeric index of the MbType enum value (or None for missing blocks):
///   I4x4=0, I16x16=1, IPCM=2, PLuma=3, P8x8=4, BDirect=5,
///   B16x16=6, B16x8=7, B8x16=8, B8x8=9, PSkip=10, BSkip=11
pub fn extract_mb_type_grid(nal_units: &[NalUnit], sps: &Sps) -> U8GridResult {
    let pic_width_in_mbs = sps.pic_width_in_mbs_minus1 + 1;
    let pic_height_in_mbs = sps.pic_height_in_map_units_minus1 + 1;
    let grid_w = pic_width_in_mbs;
    let grid_h = pic_height_in_mbs;

    let total_blocks = grid_w.checked_mul(grid_h).ok_or_else(|| {
        BitvueError::Decode(format!("Grid dimensions too large: {}x{}", grid_w, grid_h))
    })? as usize;

    let mut mb_types: Vec<Option<u8>> = Vec::with_capacity(total_blocks);
    let (sps_map, pps_map) = build_parameter_maps(nal_units);

    for nal in nal_units {
        if nal.header.nal_unit_type.is_slice() {
            if let Ok(mbs) = parse_slice_macroblocks(nal, &sps_map, &pps_map, sps, 26) {
                for mb in &mbs {
                    let type_idx = match mb.mb_type {
                        MbType::I4x4 => 0u8,
                        MbType::I16x16 => 1,
                        MbType::IPCM => 2,
                        MbType::PLuma => 3,
                        MbType::P8x8 => 4,
                        MbType::BDirect => 5,
                        MbType::B16x16 => 6,
                        MbType::B16x8 => 7,
                        MbType::B8x16 => 8,
                        MbType::B8x8 => 9,
                        MbType::PSkip => 10,
                        MbType::BSkip => 11,
                    };
                    mb_types.push(Some(type_idx));
                }
            }
        }
    }

    while mb_types.len() < total_blocks {
        mb_types.push(None);
    }
    mb_types.truncate(total_blocks);

    Ok((
        pic_width_in_mbs * 16,
        pic_height_in_mbs * 16,
        16,
        16,
        mb_types,
    ))
}

/// Extract Reference Index Grid from H.264 bitstream
///
/// Returns a 16×16 macroblock-resolution grid with L0 and L1 reference frame
/// indices per block.  None means the macroblock is intra or the list is
/// not used.
pub fn extract_ref_idx_grid(nal_units: &[NalUnit], sps: &Sps) -> I8DualGridResult {
    let pic_width_in_mbs = sps.pic_width_in_mbs_minus1 + 1;
    let pic_height_in_mbs = sps.pic_height_in_map_units_minus1 + 1;
    let grid_w = pic_width_in_mbs;
    let grid_h = pic_height_in_mbs;

    let total_blocks = grid_w.checked_mul(grid_h).ok_or_else(|| {
        BitvueError::Decode(format!("Grid dimensions too large: {}x{}", grid_w, grid_h))
    })? as usize;

    let mut l0: Vec<Option<i8>> = Vec::with_capacity(total_blocks);
    let mut l1: Vec<Option<i8>> = Vec::with_capacity(total_blocks);
    let (sps_map, pps_map) = build_parameter_maps(nal_units);

    for nal in nal_units {
        if nal.header.nal_unit_type.is_slice() {
            if let Ok(mbs) = parse_slice_macroblocks(nal, &sps_map, &pps_map, sps, 26) {
                for mb in &mbs {
                    l0.push(mb.ref_idx_l0);
                    l1.push(mb.ref_idx_l1);
                }
            }
        }
    }

    while l0.len() < total_blocks {
        l0.push(None);
        l1.push(None);
    }
    l0.truncate(total_blocks);
    l1.truncate(total_blocks);

    Ok((
        pic_width_in_mbs * 16,
        pic_height_in_mbs * 16,
        16,
        16,
        l0,
        l1,
    ))
}
