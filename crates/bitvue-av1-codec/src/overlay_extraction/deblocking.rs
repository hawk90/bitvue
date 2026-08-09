//! Deblocking (loop filter) boundary-strength extraction, per AV1 spec Section 7.14.2.
//!
//! No existing implementation of boundary-strength (BS) derivation existed anywhere in this
//! workspace before this module -- unlike CDEF/loop-restoration/film-grain (`advanced_features`),
//! which only ever needed frame-header-level values, BS genuinely depends on comparing real
//! per-coding-unit data (`skip`/`mode`/`ref_frames`/`mv`) across each block edge. That data
//! already exists here via `cu_parser::parse_all_coding_units` (real tile-data parsing, same
//! source `frame_analysis`'s partition/prediction/transform grids use) -- this module is new
//! *algorithm* work on top of already-parsed data, not new bitstream parsing.
//!
//! # `skip` as an exact (not approximate) residual proxy
//!
//! The spec's real BS=1 condition checks whether either side of an edge has non-zero coded
//! transform coefficients -- which would normally require entropy-decoding residuals (not
//! implemented anywhere in this codebase, see `crate::tile`'s module doc). But AV1's `skip` flag
//! is defined as "no residual is coded for this block" -- so `!skip` is the *exact* condition,
//! not a proxy requiring residual decode.
//!
//! # Known simplification: block-edge granularity, not exact 4x4 transform-edge granularity
//!
//! The real spec derives BS per 4-pixel edge, including internal edges where a coding block's
//! transform tree subdivides below the block's own size. This module reports one BS value per
//! real coding-unit boundary (using each CU's actual `x`/`y`/`width`/`height`, not a fixed grid)
//! -- exact for the common case where `tx_size` matches the coding block (no internal
//! subdivision), coarser than spec for blocks whose transform tree splits further. Consistent
//! with `advanced_features`' own documented per-block approximations (e.g. CDEF direction is
//! frame-level, not per-block, since per-CTU direction needs full pixel decode).
//!
//! # Known simplification: reported filter level ignores per-block delta combination
//!
//! `strength` on each edge is the frame-level plane filter level from `LoopFilterInfo` (matching
//! the edge's orientation), not the full per-block effective level the spec computes by combining
//! it with `ref_deltas`/`mode_deltas`/segmentation `SEG_LVL_ALT_LF` (segmentation feature values
//! aren't retained by this parser -- see `frame_header_full`'s `CodedLossless` doc for the same
//! documented gap). `boundary_strength` (the BS value itself) is unaffected -- it depends only on
//! `skip`/`mode`/`ref_frames`/`mv`, not filter levels.

use bitvue_engine::BitvueError;

use super::cu_parser::{parse_all_coding_units, CuSpatialIndex};
use super::parser::ParsedFrame;
use crate::frame_header::LoopFilterInfo;
use crate::tile::{CodingUnit, PredictionMode};

/// Sample granularity (pixels) for locating each coding unit's neighbor across an edge --
/// matches the spec's 4-pixel loop-filter grid.
const SAMPLE: u32 = 4;

/// A full-pixel motion-vector difference, in this crate's quarter-pel `MotionVector` units (spec
/// 7.14.2 uses 1/8-pel units and a threshold of 8; this codebase's MVs are quarter-pel, so the
/// equivalent one-pixel threshold is 4).
const MV_DIFF_ONE_PIXEL_QPEL: i32 = 4;

#[derive(Debug, Clone)]
pub struct DeblockingEdge {
    pub x: u32,
    pub y: u32,
    pub length: u32,
    pub vertical: bool,
    pub boundary_strength: u8,
    pub filtered: bool,
    pub strength: u8,
}

#[derive(Debug, Clone)]
pub struct DeblockingData {
    pub width: u32,
    pub height: u32,
    pub edges: Vec<DeblockingEdge>,
    pub loop_filter: LoopFilterInfo,
}

fn is_intra_mode(mode: PredictionMode) -> bool {
    matches!(
        mode,
        PredictionMode::DcPred
            | PredictionMode::VPred
            | PredictionMode::HPred
            | PredictionMode::D45Pred
            | PredictionMode::D135Pred
            | PredictionMode::D113Pred
            | PredictionMode::D157Pred
            | PredictionMode::D203Pred
            | PredictionMode::D67Pred
            | PredictionMode::SmoothPred
            | PredictionMode::SmoothVPred
            | PredictionMode::SmoothHPred
            | PredictionMode::PaethPred
    )
}

/// Derive boundary strength (0/1/2) for the edge between two adjacent coding units, per AV1 spec
/// Section 7.14.2 -- see this module's doc for the `skip`-as-residual-proxy reasoning.
fn boundary_strength(a: &CodingUnit, b: &CodingUnit) -> u8 {
    if is_intra_mode(a.mode) || is_intra_mode(b.mode) {
        return 2;
    }
    if !a.skip || !b.skip {
        return 1;
    }
    if a.ref_frames != b.ref_frames {
        return 1;
    }
    for i in 0..2 {
        if (a.mv[i].x - b.mv[i].x).abs() >= MV_DIFF_ONE_PIXEL_QPEL
            || (a.mv[i].y - b.mv[i].y).abs() >= MV_DIFF_ONE_PIXEL_QPEL
        {
            return 1;
        }
    }
    0
}

pub fn extract_deblocking_data_from_parsed(
    parsed: &ParsedFrame,
    loop_filter: &LoopFilterInfo,
) -> Result<DeblockingData, BitvueError> {
    let width = parsed.dimensions.width;
    let height = parsed.dimensions.height;

    // Filter fully disabled for this frame (both luma level entries are 0, per spec 7.14.1) --
    // no edges get filtered regardless of block content.
    if loop_filter.level[0] == 0 && loop_filter.level[1] == 0 {
        return Ok(DeblockingData {
            width,
            height,
            edges: Vec::new(),
            loop_filter: loop_filter.clone(),
        });
    }

    let coding_units = parse_all_coding_units(parsed)?;
    if coding_units.is_empty() {
        return Ok(DeblockingData {
            width,
            height,
            edges: Vec::new(),
            loop_filter: loop_filter.clone(),
        });
    }

    let grid_w = width.div_ceil(SAMPLE);
    let grid_h = height.div_ceil(SAMPLE);
    let index = CuSpatialIndex::new(&coding_units, grid_w, grid_h, SAMPLE, SAMPLE);

    let mut edges = Vec::new();
    for cu in coding_units.iter() {
        if cu.x > 0 {
            let neighbor_gx = (cu.x - 1) / SAMPLE;
            let neighbor_gy = cu.y / SAMPLE;
            if let Some(neighbor_idx) = index.get_cu_index(neighbor_gx, neighbor_gy) {
                let neighbor = &coding_units[neighbor_idx];
                let bs = boundary_strength(cu, neighbor);
                edges.push(DeblockingEdge {
                    x: cu.x,
                    y: cu.y,
                    length: cu.height.min(height.saturating_sub(cu.y)),
                    vertical: true,
                    boundary_strength: bs,
                    filtered: bs > 0 && loop_filter.level[0] > 0,
                    strength: loop_filter.level[0],
                });
            }
        }
        if cu.y > 0 {
            let neighbor_gx = cu.x / SAMPLE;
            let neighbor_gy = (cu.y - 1) / SAMPLE;
            if let Some(neighbor_idx) = index.get_cu_index(neighbor_gx, neighbor_gy) {
                let neighbor = &coding_units[neighbor_idx];
                let bs = boundary_strength(cu, neighbor);
                edges.push(DeblockingEdge {
                    x: cu.x,
                    y: cu.y,
                    length: cu.width.min(width.saturating_sub(cu.x)),
                    vertical: false,
                    boundary_strength: bs,
                    filtered: bs > 0 && loop_filter.level[1] > 0,
                    strength: loop_filter.level[1],
                });
            }
        }
    }

    Ok(DeblockingData {
        width,
        height,
        edges,
        loop_filter: loop_filter.clone(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tile::{MotionVector, RefFrame, TxSize};

    fn cu(x: u32, y: u32, w: u32, h: u32, mode: PredictionMode, skip: bool) -> CodingUnit {
        CodingUnit {
            x,
            y,
            width: w,
            height: h,
            skip,
            mode,
            ref_frames: [RefFrame::Intra, RefFrame::Intra],
            mv: [MotionVector::zero(), MotionVector::zero()],
            tx_size: TxSize::from_dimensions(w, h),
            qp: None,
            residual: None,
        }
    }

    #[test]
    fn boundary_strength_is_2_across_any_intra_edge() {
        let a = cu(0, 0, 8, 8, PredictionMode::DcPred, true);
        let b = cu(8, 0, 8, 8, PredictionMode::NewMv, true);
        assert_eq!(boundary_strength(&a, &b), 2);
    }

    #[test]
    fn boundary_strength_is_1_when_either_side_is_not_skipped() {
        let mut a = cu(0, 0, 8, 8, PredictionMode::NewMv, false);
        let mut b = cu(8, 0, 8, 8, PredictionMode::NewMv, true);
        a.ref_frames = [RefFrame::Last, RefFrame::Intra];
        b.ref_frames = [RefFrame::Last, RefFrame::Intra];
        assert_eq!(boundary_strength(&a, &b), 1);
    }

    #[test]
    fn boundary_strength_is_1_when_ref_frames_differ() {
        let mut a = cu(0, 0, 8, 8, PredictionMode::NewMv, true);
        let mut b = cu(8, 0, 8, 8, PredictionMode::NewMv, true);
        a.ref_frames = [RefFrame::Last, RefFrame::Intra];
        b.ref_frames = [RefFrame::Golden, RefFrame::Intra];
        assert_eq!(boundary_strength(&a, &b), 1);
    }

    #[test]
    fn boundary_strength_is_0_when_skipped_same_ref_and_mv_close() {
        let mut a = cu(0, 0, 8, 8, PredictionMode::NewMv, true);
        let mut b = cu(8, 0, 8, 8, PredictionMode::NewMv, true);
        a.ref_frames = [RefFrame::Last, RefFrame::Intra];
        b.ref_frames = [RefFrame::Last, RefFrame::Intra];
        a.mv[0] = MotionVector::new(4, 0);
        b.mv[0] = MotionVector::new(6, 0); // 2 qpel diff < 4 threshold
        assert_eq!(boundary_strength(&a, &b), 0);
    }

    #[test]
    fn boundary_strength_is_1_when_mv_diff_reaches_one_pixel() {
        let mut a = cu(0, 0, 8, 8, PredictionMode::NewMv, true);
        let mut b = cu(8, 0, 8, 8, PredictionMode::NewMv, true);
        a.ref_frames = [RefFrame::Last, RefFrame::Intra];
        b.ref_frames = [RefFrame::Last, RefFrame::Intra];
        a.mv[0] = MotionVector::new(0, 0);
        b.mv[0] = MotionVector::new(4, 0); // exactly 1 pixel (4 qpel) diff
        assert_eq!(boundary_strength(&a, &b), 1);
    }

    #[test]
    fn extract_deblocking_data_returns_no_edges_when_filter_disabled() {
        // ParsedFrame::parse(&[]) always succeeds with scaffold defaults (see parser.rs).
        let parsed = ParsedFrame::parse(&[]).unwrap();
        let loop_filter = LoopFilterInfo::default(); // level [0,0,0,0] => disabled
        let result = extract_deblocking_data_from_parsed(&parsed, &loop_filter).unwrap();
        assert!(result.edges.is_empty());
    }
}
