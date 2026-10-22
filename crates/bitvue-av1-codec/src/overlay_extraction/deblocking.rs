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

use super::cu_parser::parse_all_coding_units;
use super::cu_spatial_index::CuSpatialIndex;
use super::parser::ParsedFrame;
use crate::frame_header::LoopFilterInfo;
use crate::tile::{CodingUnit, PredictionMode};

/// Sample granularity (pixels) for locating each coding unit's neighbor across an edge --
/// matches the spec's 4-pixel loop-filter grid.
const SAMPLE: u32 = 4;

/// A full-pixel motion-vector difference in `MotionVector`'s 1/8-sample units (spec 7.14.2).
const MV_DIFF_ONE_PIXEL: i32 = 8;

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
        if (a.mv[i].x - b.mv[i].x).abs() >= MV_DIFF_ONE_PIXEL
            || (a.mv[i].y - b.mv[i].y).abs() >= MV_DIFF_ONE_PIXEL
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
    let without_edges = || DeblockingData {
        width,
        height,
        edges: Vec::new(),
        loop_filter: loop_filter.clone(),
    };

    // Filter fully disabled for this frame (both luma level entries are 0, per spec 7.14.1) --
    // no edges get filtered regardless of block content.
    if loop_filter.level[0] == 0 && loop_filter.level[1] == 0 {
        return Ok(without_edges());
    }

    let coding_units = parse_all_coding_units(parsed)?;
    if coding_units.is_empty() {
        return Ok(without_edges());
    }

    Ok(DeblockingData {
        edges: collect_edges(&coding_units, loop_filter, width, height),
        ..without_edges()
    })
}

/// The edges on the left and top side of every coding unit that has a neighbour there, in coding
/// unit order (left edge first).
fn collect_edges(
    coding_units: &[CodingUnit],
    loop_filter: &LoopFilterInfo,
    width: u32,
    height: u32,
) -> Vec<DeblockingEdge> {
    let grid_w = width.div_ceil(SAMPLE);
    let grid_h = height.div_ceil(SAMPLE);
    let index = CuSpatialIndex::new(coding_units, grid_w, grid_h, SAMPLE, SAMPLE);

    let mut edges = Vec::new();
    for cu in coding_units {
        if cu.x > 0 {
            if let Some(neighbor) = index.get_cu_index((cu.x - 1) / SAMPLE, cu.y / SAMPLE) {
                let length = cu.height.min(height.saturating_sub(cu.y));
                edges.push(edge_to(
                    cu,
                    &coding_units[neighbor],
                    true,
                    length,
                    loop_filter.level[0],
                ));
            }
        }
        if cu.y > 0 {
            if let Some(neighbor) = index.get_cu_index(cu.x / SAMPLE, (cu.y - 1) / SAMPLE) {
                let length = cu.width.min(width.saturating_sub(cu.x));
                edges.push(edge_to(
                    cu,
                    &coding_units[neighbor],
                    false,
                    length,
                    loop_filter.level[1],
                ));
            }
        }
    }
    edges
}

/// The edge along `cu`'s left (`vertical`) or top side, shared with `neighbor`. `level` is the
/// frame's filter level for the edge's orientation.
fn edge_to(
    cu: &CodingUnit,
    neighbor: &CodingUnit,
    vertical: bool,
    length: u32,
    level: u8,
) -> DeblockingEdge {
    let boundary_strength = boundary_strength(cu, neighbor);
    DeblockingEdge {
        x: cu.x,
        y: cu.y,
        length,
        vertical,
        boundary_strength,
        filtered: boundary_strength > 0 && level > 0,
        strength: level,
    }
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
            skip_mode: false,
            segment_id: 0,
            mode,
            ref_frames: [RefFrame::Intra, RefFrame::Intra],
            use_intrabc: false,
            mv: [MotionVector::zero(), MotionVector::zero()],
            tx_size: TxSize::from_dimensions(w, h),
            tx_blocks: None,
            qp: None,
            residual: None,
            palette: crate::tile::PaletteInfo::default(),
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
        a.mv[0] = MotionVector::new(8, 0);
        b.mv[0] = MotionVector::new(12, 0); // 4/8-sample diff < 8 threshold
        assert_eq!(boundary_strength(&a, &b), 0);
    }

    #[test]
    fn boundary_strength_is_1_when_mv_diff_reaches_one_pixel() {
        let mut a = cu(0, 0, 8, 8, PredictionMode::NewMv, true);
        let mut b = cu(8, 0, 8, 8, PredictionMode::NewMv, true);
        a.ref_frames = [RefFrame::Last, RefFrame::Intra];
        b.ref_frames = [RefFrame::Last, RefFrame::Intra];
        a.mv[0] = MotionVector::new(0, 0);
        b.mv[0] = MotionVector::new(8, 0); // exactly 1 pixel (8 eighth-samples) diff
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

    const INTRA_MODES: [PredictionMode; 13] = [
        PredictionMode::DcPred,
        PredictionMode::VPred,
        PredictionMode::HPred,
        PredictionMode::D45Pred,
        PredictionMode::D135Pred,
        PredictionMode::D113Pred,
        PredictionMode::D157Pred,
        PredictionMode::D203Pred,
        PredictionMode::D67Pred,
        PredictionMode::SmoothPred,
        PredictionMode::SmoothVPred,
        PredictionMode::SmoothHPred,
        PredictionMode::PaethPred,
    ];

    /// Two skipped inter blocks that agree on everything, so `boundary_strength` is 0.
    fn matching_inter_pair() -> (CodingUnit, CodingUnit) {
        let mut a = cu(0, 0, 8, 8, PredictionMode::NewMv, true);
        let mut b = cu(8, 0, 8, 8, PredictionMode::NewMv, true);
        a.ref_frames = [RefFrame::Last, RefFrame::Intra];
        b.ref_frames = [RefFrame::Last, RefFrame::Intra];
        assert_eq!(boundary_strength(&a, &b), 0);
        (a, b)
    }

    #[test]
    fn every_intra_mode_on_either_side_gives_strength_2() {
        for intra in INTRA_MODES {
            let (inter, _) = matching_inter_pair();
            let intra_cu = cu(8, 0, 8, 8, intra, true);
            assert_eq!(boundary_strength(&intra_cu, &inter), 2, "{intra:?} as a");
            assert_eq!(boundary_strength(&inter, &intra_cu), 2, "{intra:?} as b");
        }
    }

    #[test]
    fn no_inter_mode_counts_as_intra() {
        for mode in [
            PredictionMode::NewMv,
            PredictionMode::NearestMv,
            PredictionMode::NearMv,
            PredictionMode::GlobalMv,
            PredictionMode::NearestNearestMv,
            PredictionMode::NearNearMv,
            PredictionMode::NearestNewMv,
            PredictionMode::NewNearestMv,
            PredictionMode::NearNewMv,
        ] {
            let (mut a, b) = matching_inter_pair();
            a.mode = mode;
            assert_eq!(boundary_strength(&a, &b), 0, "{mode:?}");
        }
    }

    #[test]
    fn either_side_unskipped_gives_strength_1_in_both_orders() {
        let (mut a, b) = matching_inter_pair();
        a.skip = false;
        assert_eq!(boundary_strength(&a, &b), 1);
        assert_eq!(boundary_strength(&b, &a), 1);
    }

    #[test]
    fn a_difference_in_the_second_reference_gives_strength_1() {
        let (mut a, b) = matching_inter_pair();
        a.ref_frames[1] = RefFrame::Golden;
        assert_eq!(boundary_strength(&a, &b), 1);
    }

    #[test]
    fn mv_threshold_is_exactly_8_for_each_component_of_each_list_in_either_direction() {
        for list in 0..2 {
            for (dx, dy, expected) in [
                (7, 0, 0),
                (8, 0, 1),
                (-7, 0, 0),
                (-8, 0, 1),
                (0, 7, 0),
                (0, 8, 1),
                (0, -7, 0),
                (0, -8, 1),
                (7, 7, 0),
                (100, 100, 1),
            ] {
                let (mut a, b) = matching_inter_pair();
                a.mv[list] = crate::tile::MotionVector::new(dx, dy);
                assert_eq!(
                    boundary_strength(&a, &b),
                    expected,
                    "list {list} diff ({dx},{dy})"
                );
                assert_eq!(boundary_strength(&b, &a), expected, "swapped");
            }
        }
    }

    /// A frame of `width` x `height` whose coding units are `units`, as the decoder would cache them.
    fn frame_with(width: u32, height: u32, units: Vec<CodingUnit>) -> ParsedFrame {
        let mut parsed = ParsedFrame::parse(&[]).unwrap();
        parsed.dimensions.width = width;
        parsed.dimensions.height = height;
        parsed.decoded = Some(super::super::ParsedCodingUnits {
            units: std::sync::Arc::new(units),
            outcome: Default::default(),
        });
        parsed
    }

    fn levels(luma_vertical: u8, luma_horizontal: u8) -> LoopFilterInfo {
        LoopFilterInfo {
            level: [luma_vertical, luma_horizontal, 0, 0],
            ..LoopFilterInfo::default()
        }
    }

    /// Four 8x8 intra blocks tiling a 16x16 frame: (0,0) (8,0) / (0,8) (8,8).
    fn quad() -> Vec<CodingUnit> {
        vec![
            cu(0, 0, 8, 8, PredictionMode::DcPred, true),
            cu(8, 0, 8, 8, PredictionMode::DcPred, true),
            cu(0, 8, 8, 8, PredictionMode::DcPred, true),
            cu(8, 8, 8, 8, PredictionMode::DcPred, true),
        ]
    }

    fn edge_at(data: &DeblockingData, x: u32, y: u32, vertical: bool) -> &DeblockingEdge {
        let found: Vec<_> = data
            .edges
            .iter()
            .filter(|e| e.x == x && e.y == y && e.vertical == vertical)
            .collect();
        assert_eq!(found.len(), 1, "edge ({x},{y}) vertical={vertical}");
        found[0]
    }

    #[test]
    fn extract_reports_one_edge_per_internal_block_boundary_with_its_orientation() {
        let parsed = frame_with(16, 16, quad());
        let data = extract_deblocking_data_from_parsed(&parsed, &levels(10, 20)).unwrap();
        // The top-left block has no left/top neighbour; the others have one or two.
        assert_eq!(data.edges.len(), 4);
        assert_eq!((data.width, data.height), (16, 16));
        let v = edge_at(&data, 8, 0, true);
        assert_eq!(
            (v.length, v.boundary_strength, v.filtered, v.strength),
            (8, 2, true, 10)
        );
        let h = edge_at(&data, 0, 8, false);
        assert_eq!(
            (h.length, h.boundary_strength, h.filtered, h.strength),
            (8, 2, true, 20)
        );
        edge_at(&data, 8, 8, true);
        edge_at(&data, 8, 8, false);
        assert_eq!(data.loop_filter.level, [10, 20, 0, 0]);
    }

    #[test]
    fn each_orientation_uses_its_own_luma_level() {
        let parsed = frame_with(16, 16, quad());
        // Only the horizontal-edge level is set: the filter is enabled, vertical edges stay unfiltered.
        let data = extract_deblocking_data_from_parsed(&parsed, &levels(0, 20)).unwrap();
        assert_eq!(data.edges.len(), 4);
        for e in &data.edges {
            if e.vertical {
                assert_eq!((e.strength, e.filtered), (0, false));
            } else {
                assert_eq!((e.strength, e.filtered), (20, true));
            }
        }
        // And the other way round.
        let data = extract_deblocking_data_from_parsed(&parsed, &levels(10, 0)).unwrap();
        assert_eq!(data.edges.len(), 4);
        for e in &data.edges {
            if e.vertical {
                assert_eq!((e.strength, e.filtered), (10, true));
            } else {
                assert_eq!((e.strength, e.filtered), (0, false));
            }
        }
    }

    #[test]
    fn a_zero_strength_edge_is_not_filtered_even_with_a_nonzero_level() {
        let mut units = quad();
        for unit in &mut units {
            unit.mode = PredictionMode::NewMv;
            unit.ref_frames = [RefFrame::Last, RefFrame::Intra];
        }
        let parsed = frame_with(16, 16, units);
        let data = extract_deblocking_data_from_parsed(&parsed, &levels(10, 10)).unwrap();
        assert_eq!(data.edges.len(), 4);
        for e in &data.edges {
            assert_eq!(
                (e.boundary_strength, e.filtered, e.strength),
                (0, false, 10)
            );
        }
    }

    #[test]
    fn an_edge_hanging_over_the_frame_border_is_clipped_to_the_frame() {
        // 12x12 frame, 8x8 blocks: the blocks at x=8 / y=8 stick out by 4 pixels.
        let parsed = frame_with(12, 12, quad());
        let data = extract_deblocking_data_from_parsed(&parsed, &levels(5, 5)).unwrap();
        // Vertical edge at (8,8): length is the block height clipped to 12-8 = 4.
        assert_eq!(edge_at(&data, 8, 8, true).length, 4);
        // Horizontal edge at (8,8): width clipped the same way.
        assert_eq!(edge_at(&data, 8, 8, false).length, 4);
        // Edges that fit are unclipped.
        assert_eq!(edge_at(&data, 8, 0, true).length, 8);
        assert_eq!(edge_at(&data, 0, 8, false).length, 8);
    }

    #[test]
    fn a_block_whose_neighbour_cell_has_no_coding_unit_gets_no_edge() {
        // Only the block at (8,0) exists: its left neighbour cell is uncovered.
        let units = vec![cu(8, 0, 8, 8, PredictionMode::DcPred, true)];
        let parsed = frame_with(16, 16, units);
        let data = extract_deblocking_data_from_parsed(&parsed, &levels(5, 5)).unwrap();
        assert!(data.edges.is_empty());
        // Same for the top neighbour.
        let units = vec![cu(0, 8, 8, 8, PredictionMode::DcPred, true)];
        let parsed = frame_with(16, 16, units);
        let data = extract_deblocking_data_from_parsed(&parsed, &levels(5, 5)).unwrap();
        assert!(data.edges.is_empty());
    }

    #[test]
    fn an_enabled_filter_with_no_coding_units_reports_no_edges_but_keeps_the_levels() {
        let parsed = frame_with(16, 16, Vec::new());
        let data = extract_deblocking_data_from_parsed(&parsed, &levels(5, 6)).unwrap();
        assert!(data.edges.is_empty());
        assert_eq!((data.width, data.height), (16, 16));
        assert_eq!(data.loop_filter.level, [5, 6, 0, 0]);
    }

    #[test]
    fn a_disabled_filter_reports_no_edges_for_a_frame_that_has_coding_units() {
        let parsed = frame_with(16, 16, quad());
        let data = extract_deblocking_data_from_parsed(&parsed, &levels(0, 0)).unwrap();
        assert!(data.edges.is_empty());
        assert_eq!(data.loop_filter.level, [0, 0, 0, 0]);
    }

    #[test]
    fn a_frame_size_that_is_not_a_multiple_of_4_still_finds_every_neighbour() {
        // 10x10: the 4x4 grid has 3x3 cells (rounded up), not 2x2.
        let parsed = frame_with(10, 10, quad());
        let data = extract_deblocking_data_from_parsed(&parsed, &levels(5, 5)).unwrap();
        assert_eq!(data.edges.len(), 4);
        // Clipped to the 2 pixels that remain inside the frame.
        assert_eq!(edge_at(&data, 8, 8, true).length, 2);
        assert_eq!(edge_at(&data, 8, 8, false).length, 2);

        // The block at x=8 lies in grid column 2, which only exists in the rounded-up grid. Its
        // upper neighbour is the intra block (strength 2); the block left of it, a skipped inter
        // block, would give strength 0 if the lookup wrapped into the previous cell.
        let inter = |x, y| {
            let mut unit = cu(x, y, 8, 8, PredictionMode::NewMv, true);
            unit.ref_frames = [RefFrame::Last, RefFrame::Intra];
            unit
        };
        let units = vec![
            cu(8, 0, 8, 8, PredictionMode::DcPred, true),
            inter(0, 8),
            inter(8, 8),
        ];
        let parsed = frame_with(10, 16, units);
        let data = extract_deblocking_data_from_parsed(&parsed, &levels(5, 5)).unwrap();
        assert_eq!(edge_at(&data, 8, 8, false).boundary_strength, 2);
        assert_eq!(edge_at(&data, 8, 8, true).boundary_strength, 0);
    }

    #[test]
    fn the_block_above_is_found_at_the_blocks_own_left_column_not_the_next_cell() {
        // Two 4-wide blocks above one 8-wide block: the edge sees the one directly above its
        // top-left corner (intra, strength 2), not the neighbour of that block (strength 0).
        let above_left = cu(0, 0, 4, 8, PredictionMode::DcPred, true);
        let mut above_right = cu(4, 0, 4, 8, PredictionMode::NewMv, true);
        above_right.ref_frames = [RefFrame::Last, RefFrame::Intra];
        let mut below = cu(0, 8, 8, 8, PredictionMode::NewMv, true);
        below.ref_frames = [RefFrame::Last, RefFrame::Intra];
        let parsed = frame_with(8, 16, vec![above_left, above_right, below]);
        let data = extract_deblocking_data_from_parsed(&parsed, &levels(5, 5)).unwrap();
        assert_eq!(edge_at(&data, 0, 8, false).boundary_strength, 2);
        // The vertical edge between the two upper blocks sees the intra block on its left too.
        assert_eq!(edge_at(&data, 4, 0, true).boundary_strength, 2);
    }

    #[test]
    fn the_neighbour_is_the_block_covering_the_cell_beside_the_top_left_corner() {
        // A tall block on the left, two short blocks on the right: the left edge of the lower right
        // block sees the tall block, whose strength differs from the one the upper right block sees.
        let mut left = cu(0, 0, 8, 16, PredictionMode::NewMv, true);
        left.ref_frames = [RefFrame::Last, RefFrame::Intra];
        let mut top_right = cu(8, 0, 8, 8, PredictionMode::NewMv, true);
        top_right.ref_frames = [RefFrame::Last, RefFrame::Intra];
        let mut bottom_right = cu(8, 8, 8, 8, PredictionMode::NewMv, true);
        bottom_right.ref_frames = [RefFrame::Golden, RefFrame::Intra];
        let parsed = frame_with(16, 16, vec![left, top_right, bottom_right]);
        let data = extract_deblocking_data_from_parsed(&parsed, &levels(5, 5)).unwrap();
        assert_eq!(edge_at(&data, 8, 0, true).boundary_strength, 0);
        assert_eq!(edge_at(&data, 8, 8, true).boundary_strength, 1);
        // Horizontal edge between the two right blocks differs in reference frame.
        assert_eq!(edge_at(&data, 8, 8, false).boundary_strength, 1);
    }
}
