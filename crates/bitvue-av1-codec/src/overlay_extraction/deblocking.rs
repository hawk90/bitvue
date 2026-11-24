//! Loop filter (deblocking) edges of an AV1 frame, per spec 7.14.2-7.14.5 (luma plane).
//!
//! AV1 has no boundary strength: whether a transform-block edge is filtered, how wide the filter can
//! be, and how strong it is all follow from the block data and the frame header:
//!
//! * an edge is a candidate where the samples cross a transform block edge (`isTxEdge`); it is
//!   filtered if it also crosses a prediction block edge, or the block has a residual, or it is
//!   intra (`applyFilter`). Edges inside a skipped inter block stay unfiltered;
//! * the largest filter that can be used is the smaller transform dimension on either side, capped
//!   at 16 samples (`filterSize`, 7.14.3): 4, 8 or 16;
//! * the strength level comes from the frame level of the edge's direction, adjusted by the
//!   block's segment feature, reference frame and mode (7.14.5). A block with level 0 takes the
//!   level of the block across the edge; an edge whose level is still 0 is not filtered.
//!
//! The filter that is actually applied to each sample line (narrow, 8 or 16 wide) depends on the
//! sample values (`flatMask`, `flatMask2`, 7.14.6.2), which are not known without the pre-filter
//! picture, so `filter_size` is the largest filter the edge allows, not the one used.
//!
//! Only luma edges are reported (the guide this follows draws luma boundaries).
//!
//! # Known limitation: `delta_lf`
//!
//! A frame with `delta_lf_present` adds a per-block `DeltaLF` to the base level. The tile parser
//! reads those values only to stay in step and does not keep them, so on such frames the levels
//! here use `DeltaLF = 0` and `DeltaData::delta_lf_ignored` is set.

use bitvue_engine::BitvueError;

use super::cu_parser::parse_all_coding_units;
use super::parser::ParsedFrame;
use crate::frame_header::{FrameHeader, LoopFilterInfo};
use crate::frame_header_full::SegmentationInfo;
use crate::tile::coding_unit::TxBlock;
use crate::tile::{CodingUnit, PredictionMode, RefFrame};

/// Luma sample size of one mode-info (4x4) cell, `MI_SIZE`.
const MI_SIZE: u32 = 4;

/// Largest filter level, `MAX_LOOP_FILTER`.
const MAX_LOOP_FILTER: i32 = 63;

/// Segmentation feature `SEG_LVL_ALT_LF_Y_V`; the luma horizontal-edge feature follows it (spec
/// 7.14.5: `SEG_LVL_ALT_LF_Y_V + i` with `i` the pass).
const SEG_LVL_ALT_LF_Y_V: usize = 1;

/// One run of loop filter edge along a block boundary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeblockingEdge {
    pub x: u32,
    pub y: u32,
    /// Length along the edge in luma samples (a multiple of 4, clipped to the frame).
    pub length: u32,
    /// `true` for a vertical edge (between horizontally adjacent blocks).
    pub vertical: bool,
    /// The largest filter the edge allows: 4, 8 or 16 samples (spec `filterSize`).
    pub filter_size: u8,
    /// `applyFilter` and a non-zero level: samples across this edge are filtered.
    pub filtered: bool,
    /// The filter level (`lvl`, 0-63) after segment, reference and mode adjustments.
    pub level: u8,
}

#[derive(Debug, Clone)]
pub struct DeblockingData {
    pub width: u32,
    pub height: u32,
    pub edges: Vec<DeblockingEdge>,
    pub loop_filter: LoopFilterInfo,
    /// `delta_lf_present` is set but the per-block `DeltaLF` is not kept: levels use 0 (see the
    /// module doc).
    pub delta_lf_ignored: bool,
}

/// The frame-level inputs of the edge derivation, from the frame header.
pub struct FilterParams<'a> {
    pub width: u32,
    pub height: u32,
    pub loop_filter: &'a LoopFilterInfo,
    pub segmentation: &'a SegmentationInfo,
    pub coded_lossless: bool,
}

/// What the loop filter needs to know about one 4x4 luma cell.
#[derive(Clone, Copy, Default)]
struct Cell {
    /// `Block_Width`/`Block_Height` of the block's `MiSize`
    block_w: u32,
    block_h: u32,
    /// `Tx_Width`/`Tx_Height` of `LoopfilterTxSizes`
    tx_w: u32,
    tx_h: u32,
    skip: bool,
    is_intra: bool,
    /// `lvl` for vertical (`[0]`) and horizontal (`[1]`) edges
    level: [u8; 2],
}

/// `modeType` of spec 7.14.4: 1 for inter modes other than the global-motion ones.
fn mode_type(mode: PredictionMode) -> usize {
    usize::from(
        mode.is_inter()
            && mode != PredictionMode::GlobalMv
            && mode != PredictionMode::GlobalGlobalMv,
    )
}

/// The filter level of spec 7.14.5 for one block and edge direction (`pass` 0 vertical, 1
/// horizontal), with `DeltaLF` taken as 0.
fn filter_level(
    lf: &LoopFilterInfo,
    segmentation: &SegmentationInfo,
    segment_id: usize,
    ref_frame: RefFrame,
    mode_type: usize,
    pass: usize,
) -> u8 {
    let base = i32::from(lf.level[pass]).clamp(0, MAX_LOOP_FILTER);
    let mut level = base;
    let feature = SEG_LVL_ALT_LF_Y_V + pass;
    if segmentation.enabled && segmentation.feature_enabled[segment_id][feature] {
        level = (level + i32::from(segmentation.feature_data[segment_id][feature]))
            .clamp(0, MAX_LOOP_FILTER);
    }
    if lf.delta_enabled {
        let n_shift = level >> 5;
        let ref_delta = i32::from(lf.ref_deltas[ref_frame as usize]);
        level += if ref_frame == RefFrame::Intra {
            ref_delta << n_shift
        } else {
            (ref_delta << n_shift) + (i32::from(lf.mode_deltas[mode_type]) << n_shift)
        };
        level = level.clamp(0, MAX_LOOP_FILTER);
    }
    level as u8
}

/// The luma transform blocks of a coding unit as `(x, y, width, height)` rectangles in samples.
/// A unit without a block list uses the largest transform of its size (spec `Max_Tx_Size_Rect`,
/// 64 at most; 4x4 when lossless), as `read_tx_size` does for a skipped inter block.
fn transform_blocks(cu: &CodingUnit, coded_lossless: bool) -> Vec<(u32, u32, u32, u32)> {
    if let Some(blocks) = cu.tx_blocks.as_ref().filter(|b| !b.is_empty()) {
        return blocks
            .iter()
            .map(|b: &TxBlock| (b.x4 * MI_SIZE, b.y4 * MI_SIZE, b.width_px, b.height_px))
            .collect();
    }
    let (w, h) = if coded_lossless {
        (4, 4)
    } else {
        (cu.width.min(64), cu.height.min(64))
    };
    let mut out = Vec::new();
    for y in (cu.y..cu.y + cu.height).step_by(h as usize) {
        for x in (cu.x..cu.x + cu.width).step_by(w as usize) {
            out.push((x, y, w, h));
        }
    }
    out
}

/// The cells of a frame of `width` x `height` luma samples, filled from its coding units.
fn build_cells(params: &FilterParams, coding_units: &[CodingUnit]) -> (Vec<Cell>, usize, usize) {
    let (width, height) = (params.width, params.height);
    // MiCols/MiRows: the frame size in 4x4 cells, rounded up to a multiple of 8 samples
    let mi_cols = (2 * width.div_ceil(8)) as usize;
    let mi_rows = (2 * height.div_ceil(8)) as usize;
    let mut cells = vec![Cell::default(); mi_cols * mi_rows];
    let mut fill = |x: u32, y: u32, w: u32, h: u32, f: &mut dyn FnMut(&mut Cell)| {
        for row in (y / MI_SIZE) as usize..(y + h).div_ceil(MI_SIZE) as usize {
            for col in (x / MI_SIZE) as usize..(x + w).div_ceil(MI_SIZE) as usize {
                if row < mi_rows && col < mi_cols {
                    f(&mut cells[row * mi_cols + col]);
                }
            }
        }
    };

    for cu in coding_units {
        let ref_frame = cu.ref_frames[0];
        let mode_type = mode_type(cu.mode);
        let segment_id = usize::from(cu.segment_id).min(7);
        let level = [
            filter_level(
                params.loop_filter,
                params.segmentation,
                segment_id,
                ref_frame,
                mode_type,
                0,
            ),
            filter_level(
                params.loop_filter,
                params.segmentation,
                segment_id,
                ref_frame,
                mode_type,
                1,
            ),
        ];
        fill(cu.x, cu.y, cu.width, cu.height, &mut |c| {
            c.block_w = cu.width;
            c.block_h = cu.height;
            c.skip = cu.skip;
            c.is_intra = ref_frame == RefFrame::Intra;
            c.level = level;
        });
        for (x, y, w, h) in transform_blocks(cu, params.coded_lossless) {
            fill(x, y, w, h, &mut |c| {
                c.tx_w = w;
                c.tx_h = h;
            });
        }
    }
    (cells, mi_cols, mi_rows)
}

/// The edges of one frame, from its decoded coding units and header.
pub fn deblocking_edges(params: &FilterParams, coding_units: &[CodingUnit]) -> Vec<DeblockingEdge> {
    let (width, height) = (params.width, params.height);
    let (cells, mi_cols, mi_rows) = build_cells(params, coding_units);
    let cell = |row: usize, col: usize| cells[row * mi_cols + col];

    let mut edges = Vec::new();
    for pass in 0..2usize {
        let vertical = pass == 0;
        // Walk along the edge so consecutive cells with the same result merge into one run.
        let (outer, inner) = if vertical {
            (mi_cols, mi_rows)
        } else {
            (mi_rows, mi_cols)
        };
        for across in 0..outer {
            let mut run: Option<DeblockingEdge> = None;
            for along in 0..inner {
                let (row, col) = if vertical {
                    (along, across)
                } else {
                    (across, along)
                };
                let (x, y) = (col as u32 * MI_SIZE, row as u32 * MI_SIZE);
                let edge = edge_at(&cell, row, col, x, y, pass, width, height);
                run = match (run.take(), edge) {
                    (Some(mut r), Some(e)) if same_kind(&r, &e) => {
                        r.length += MI_SIZE;
                        Some(r)
                    }
                    (Some(r), e) => {
                        edges.push(r);
                        e
                    }
                    (None, e) => e,
                };
            }
            edges.extend(run);
        }
    }
    edges
}

fn same_kind(a: &DeblockingEdge, b: &DeblockingEdge) -> bool {
    a.vertical == b.vertical
        && a.filter_size == b.filter_size
        && a.filtered == b.filtered
        && a.level == b.level
        && if a.vertical {
            a.x == b.x && a.y + a.length == b.y
        } else {
            a.y == b.y && a.x + a.length == b.x
        }
}

/// Spec 7.14.2 for one 4x4 cell and direction; `None` where there is no transform edge to
/// consider (including everything off screen or on the frame's own border).
#[allow(clippy::too_many_arguments)]
fn edge_at(
    cell: &dyn Fn(usize, usize) -> Cell,
    row: usize,
    col: usize,
    x: u32,
    y: u32,
    pass: usize,
    width: u32,
    height: u32,
) -> Option<DeblockingEdge> {
    let vertical = pass == 0;
    let on_screen = x < width && y < height && if vertical { x != 0 } else { y != 0 };
    if !on_screen {
        return None;
    }
    let cur = cell(row, col);
    if cur.block_w == 0 {
        return None; // no coding unit covers this cell
    }
    let prev = if vertical {
        cell(row, col - 1)
    } else {
        cell(row - 1, col)
    };
    if prev.block_w == 0 {
        return None;
    }

    let (pos, block_len, tx_len, prev_tx_len) = if vertical {
        (x, cur.block_w, cur.tx_w, prev.tx_w)
    } else {
        (y, cur.block_h, cur.tx_h, prev.tx_h)
    };
    let is_block_edge = pos % block_len == 0;
    let is_tx_edge = tx_len != 0 && pos % tx_len == 0;
    if !is_tx_edge {
        return None;
    }
    let apply_filter = is_block_edge || !cur.skip || cur.is_intra;
    let filter_size = tx_len.min(prev_tx_len).min(16) as u8;
    let mut level = cur.level[pass];
    if level == 0 {
        level = prev.level[pass];
    }
    Some(DeblockingEdge {
        x,
        y,
        length: MI_SIZE,
        vertical,
        filter_size,
        filtered: apply_filter && level > 0,
        level,
    })
}

pub fn extract_deblocking_data_from_parsed(
    parsed: &ParsedFrame,
    header: &FrameHeader,
) -> Result<DeblockingData, BitvueError> {
    let width = header.width;
    let height = header.height;
    let without_edges = || DeblockingData {
        width,
        height,
        edges: Vec::new(),
        loop_filter: header.loop_filter.clone(),
        delta_lf_ignored: header.delta_lf_present,
    };

    // Filter fully disabled for this frame (both luma level entries are 0, per spec 7.14.1) --
    // no edges get filtered regardless of block content.
    if header.loop_filter.level[0] == 0 && header.loop_filter.level[1] == 0 {
        return Ok(without_edges());
    }

    let coding_units = parse_all_coding_units(parsed)?;
    if coding_units.is_empty() {
        return Ok(without_edges());
    }

    Ok(DeblockingData {
        edges: deblocking_edges(
            &FilterParams {
                width,
                height,
                loop_filter: &header.loop_filter,
                segmentation: &header.segmentation,
                coded_lossless: parsed.coded_lossless,
            },
            &coding_units,
        ),
        ..without_edges()
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tile::{MotionVector, PaletteInfo, TxSize};

    fn cu(x: u32, y: u32, w: u32, h: u32) -> CodingUnit {
        CodingUnit {
            x,
            y,
            width: w,
            height: h,
            skip: false,
            skip_mode: false,
            segment_id: 0,
            mode: PredictionMode::DcPred,
            ref_frames: [RefFrame::Intra, RefFrame::Intra],
            use_intrabc: false,
            mv: [MotionVector::zero(), MotionVector::zero()],
            tx_size: TxSize::from_dimensions(w, h),
            tx_blocks: None,
            qp: None,
            residual: None,
            palette: PaletteInfo::default(),
            bits: 0.0,
            symbols: 0,
            partition_bits: 0.0,
            partition_symbols: 0.0,
        }
    }

    fn inter(mut c: CodingUnit) -> CodingUnit {
        c.ref_frames = [RefFrame::Last, RefFrame::Intra];
        c.mode = PredictionMode::NewMv;
        c
    }

    fn with_tx(mut c: CodingUnit, w: u32, h: u32) -> CodingUnit {
        let mut blocks = Vec::new();
        for y in (c.y..c.y + c.height).step_by(h as usize) {
            for x in (c.x..c.x + c.width).step_by(w as usize) {
                blocks.push(TxBlock {
                    x4: x / 4,
                    y4: y / 4,
                    width_px: w,
                    height_px: h,
                });
            }
        }
        c.tx_blocks = Some(blocks);
        c
    }

    /// Frame-level inputs for a `width` x `height` frame with the given luma levels.
    struct Frame {
        width: u32,
        height: u32,
        loop_filter: LoopFilterInfo,
        segmentation: SegmentationInfo,
    }

    fn header(width: u32, height: u32, level: [u8; 2]) -> Frame {
        Frame {
            width,
            height,
            loop_filter: LoopFilterInfo {
                level: [level[0], level[1], 0, 0],
                ..Default::default()
            },
            segmentation: SegmentationInfo::default(),
        }
    }

    fn edges_for(f: &Frame, units: &[CodingUnit], coded_lossless: bool) -> Vec<DeblockingEdge> {
        deblocking_edges(
            &FilterParams {
                width: f.width,
                height: f.height,
                loop_filter: &f.loop_filter,
                segmentation: &f.segmentation,
                coded_lossless,
            },
            units,
        )
    }

    fn edges(f: &Frame, units: &[CodingUnit]) -> Vec<DeblockingEdge> {
        edges_for(f, units, false)
    }

    fn find(edges: &[DeblockingEdge], x: u32, y: u32, vertical: bool) -> &DeblockingEdge {
        edges
            .iter()
            .find(|e| {
                e.vertical == vertical
                    && e.x <= x
                    && x < e.x + if vertical { 1 } else { e.length }
                    && e.y <= y
                    && y < e.y + if vertical { e.length } else { 1 }
            })
            .unwrap_or_else(|| {
                panic!(
                    "no {} edge at ({x},{y}) in {edges:?}",
                    if vertical { "vertical" } else { "horizontal" }
                )
            })
    }

    /// Two intra 16x16 blocks side by side with 16x16 transforms: a filtered, 16-wide edge at x = 16
    /// along the whole block height, merged into one run.
    #[test]
    fn a_block_edge_between_large_transforms_allows_the_16_sample_filter() {
        let h = header(32, 16, [20, 20]);
        let e = edges(&h, &[cu(0, 0, 16, 16), cu(16, 0, 16, 16)]);
        assert_eq!(
            e,
            vec![DeblockingEdge {
                x: 16,
                y: 0,
                length: 16,
                vertical: true,
                filter_size: 16,
                filtered: true,
                level: 20,
            }]
        );
    }

    /// The smaller transform dimension across the edge limits the filter (spec 7.14.3), and 32 or
    /// 64 transforms are capped at 16.
    #[test]
    fn filter_size_is_the_smaller_transform_dimension_capped_at_16() {
        let h = header(96, 32, [20, 20]);
        let e = edges(
            &h,
            &[
                cu(0, 0, 32, 32),                 // 32x32 transform
                with_tx(cu(32, 0, 32, 32), 8, 8), // 8x8 transforms
                with_tx(cu(64, 0, 32, 32), 4, 4), // 4x4 transforms
            ],
        );
        assert_eq!(find(&e, 32, 0, true).filter_size, 8, "32 | 8");
        assert_eq!(find(&e, 64, 0, true).filter_size, 4, "8 | 4");
        // inside the 8x8 block, between its own 8x8 transforms
        assert_eq!(find(&e, 40, 0, true).filter_size, 8);
        // a 32 | 32 edge would be 16: two 32x32 blocks
        let e = edges(
            &header(64, 32, [20, 20]),
            &[cu(0, 0, 32, 32), cu(32, 0, 32, 32)],
        );
        assert_eq!(find(&e, 32, 0, true).filter_size, 16);
    }

    /// Rectangular transforms: the dimension across the edge counts (`Tx_Width` for a vertical
    /// edge, `Tx_Height` for a horizontal one).
    #[test]
    fn a_rectangular_transform_limits_each_direction_by_its_own_dimension() {
        let h = header(32, 32, [20, 20]);
        // two 16x8 blocks stacked, with 16x8 transforms: the vertical edge between columns would
        // see width 16, the horizontal edge between them sees height 8
        let e = edges(
            &h,
            &[
                cu(0, 0, 16, 8),
                cu(0, 8, 16, 8),
                cu(16, 0, 16, 8),
                cu(16, 8, 16, 8),
            ],
        );
        assert_eq!(find(&e, 16, 0, true).filter_size, 16);
        assert_eq!(find(&e, 0, 8, false).filter_size, 8);
    }

    /// Spec 7.14.2 `applyFilter`: a transform edge inside a skipped inter block is not a block edge
    /// and the block has no residual, so it is reported but not filtered; with a residual (or
    /// intra) it is filtered.
    #[test]
    fn transform_edges_inside_a_skipped_inter_block_are_not_filtered() {
        let h = header(32, 32, [20, 20]);
        let mut skipped = inter(with_tx(cu(0, 0, 32, 32), 16, 16));
        skipped.skip = true;
        let e = edges(&h, &[skipped.clone()]);
        assert!(!find(&e, 16, 0, true).filtered);
        assert!(!find(&e, 0, 16, false).filtered);

        let mut coded = skipped.clone();
        coded.skip = false;
        let e = edges(&h, &[coded]);
        assert!(find(&e, 16, 0, true).filtered);

        let mut intra = with_tx(cu(0, 0, 32, 32), 16, 16);
        intra.skip = true;
        assert!(find(&edges(&h, &[intra]), 16, 0, true).filtered, "intra");
    }

    /// A block edge is always filtered (when the level allows), skipped or not.
    #[test]
    fn a_block_edge_is_filtered_even_between_skipped_inter_blocks() {
        let h = header(32, 16, [20, 20]);
        let (mut a, mut b) = (inter(cu(0, 0, 16, 16)), inter(cu(16, 0, 16, 16)));
        a.skip = true;
        b.skip = true;
        assert!(find(&edges(&h, &[a, b]), 16, 0, true).filtered);
    }

    /// Spec 7.14.5: base level per direction, segment feature, then reference/mode deltas with
    /// `nShift = lvlSeg >> 5`.
    #[test]
    fn the_level_follows_segment_reference_and_mode_adjustments() {
        let mut lf = LoopFilterInfo {
            level: [10, 40, 0, 0],
            delta_enabled: true,
            ..Default::default()
        };
        lf.ref_deltas[RefFrame::Intra as usize] = 1;
        lf.ref_deltas[RefFrame::Last as usize] = -2;
        lf.mode_deltas = [0, 3];
        let seg = SegmentationInfo::default();
        let level = |lf: &LoopFilterInfo, seg: &SegmentationInfo, r, m, pass| {
            filter_level(lf, seg, 0, r, m, pass)
        };
        // level 10 < 32: nShift 0
        assert_eq!(level(&lf, &seg, RefFrame::Intra, 0, 0), 11);
        assert_eq!(level(&lf, &seg, RefFrame::Last, 1, 0), 10 - 2 + 3);
        assert_eq!(level(&lf, &seg, RefFrame::Last, 0, 0), 10 - 2);
        // level 40 >= 32: nShift 1 doubles the deltas
        assert_eq!(level(&lf, &seg, RefFrame::Intra, 0, 1), 42);
        assert_eq!(level(&lf, &seg, RefFrame::Last, 1, 1), 40 + (3 - 2) * 2);

        // a segment feature adds to the base before the deltas
        let mut seg = SegmentationInfo {
            enabled: true,
            ..Default::default()
        };
        seg.feature_enabled[0][SEG_LVL_ALT_LF_Y_V] = true;
        seg.feature_data[0][SEG_LVL_ALT_LF_Y_V] = 25; // vertical edges only
        assert_eq!(level(&lf, &seg, RefFrame::Intra, 0, 0), 35 + (1 << 1));
        assert_eq!(
            level(&lf, &seg, RefFrame::Intra, 0, 1),
            42,
            "horizontal feature is off"
        );

        // both clamp to 0..=63
        seg.feature_data[0][SEG_LVL_ALT_LF_Y_V] = -100;
        assert_eq!(level(&lf, &seg, RefFrame::Last, 0, 0), 0);
        lf.level[0] = 63;
        lf.ref_deltas[RefFrame::Intra as usize] = 63;
        assert_eq!(
            level(&lf, &SegmentationInfo::default(), RefFrame::Intra, 0, 0),
            63
        );
        // without delta_enabled the deltas are ignored
        lf.delta_enabled = false;
        assert_eq!(
            level(&lf, &SegmentationInfo::default(), RefFrame::Last, 1, 0),
            63
        );
    }

    /// Global-motion modes use the first mode delta, other inter modes the second.
    #[test]
    fn global_motion_modes_use_mode_delta_zero() {
        assert_eq!(mode_type(PredictionMode::GlobalMv), 0);
        assert_eq!(mode_type(PredictionMode::GlobalGlobalMv), 0);
        assert_eq!(mode_type(PredictionMode::NearestMv), 1);
        assert_eq!(mode_type(PredictionMode::NewNewMv), 1);
        assert_eq!(mode_type(PredictionMode::DcPred), 0);
    }

    /// A block with level 0 takes the level of the block across the edge; with both 0 the edge is
    /// reported but not filtered.
    #[test]
    fn a_zero_level_block_borrows_the_level_across_the_edge() {
        let mut lf_h = header(32, 16, [0, 0]);
        lf_h.loop_filter.delta_enabled = true;
        lf_h.loop_filter.ref_deltas[RefFrame::Last as usize] = 9;
        // left block is intra (level 0 + 0), right block is inter (level 0 + 9)
        let e = edges(&lf_h, &[cu(0, 0, 16, 16), inter(cu(16, 0, 16, 16))]);
        let edge = find(&e, 16, 0, true);
        assert_eq!(edge.level, 9);
        assert!(edge.filtered);
        // both sides level 0
        let e = edges(
            &header(32, 16, [0, 5]),
            &[cu(0, 0, 16, 16), cu(16, 0, 16, 16)],
        );
        let edge = find(&e, 16, 0, true);
        assert_eq!(
            (edge.level, edge.filtered),
            (0, false),
            "vertical level is 0"
        );
    }

    #[test]
    fn each_direction_uses_its_own_frame_level() {
        let h = header(32, 32, [7, 30]);
        let e = edges(
            &h,
            &[
                cu(0, 0, 16, 16),
                cu(16, 0, 16, 16),
                cu(0, 16, 16, 16),
                cu(16, 16, 16, 16),
            ],
        );
        assert_eq!(find(&e, 16, 0, true).level, 7);
        assert_eq!(find(&e, 0, 16, false).level, 30);
    }

    /// Nothing is reported on the frame border (`x == 0`, `y == 0`) or past the visible edge, and
    /// runs are cut at the frame's edge.
    #[test]
    fn edges_on_the_border_or_off_screen_are_not_reported() {
        // 20x20 frame (coded area 24x24): the second block column starts at x = 16
        let h = header(20, 20, [20, 20]);
        let e = edges(
            &h,
            &[
                cu(0, 0, 16, 16),
                cu(16, 0, 16, 16),
                cu(0, 16, 16, 16),
                cu(16, 16, 16, 16),
            ],
        );
        assert!(e.iter().all(|e| e.x < 20 && e.y < 20), "{e:?}");
        assert!(e
            .iter()
            .all(|e| !(e.vertical && e.x == 0) && !(!e.vertical && e.y == 0)));
        // the vertical edge at x = 16 runs down to the last visible cell (y < FrameHeight, spec
        // `onScreen`), not to the 8-aligned coded area
        let v = find(&e, 16, 0, true);
        assert_eq!(v.y + v.length, 20);
        assert!(e
            .iter()
            .all(|e| e.y + if e.vertical { e.length } else { 0 } <= 20));
    }

    /// Edges of a different kind do not merge, equal neighbours do.
    #[test]
    fn runs_merge_only_while_the_edge_stays_the_same() {
        let h = header(32, 16, [20, 20]);
        // the right block has 16x16 then... use two stacked 16x8 blocks with different transforms
        let e = edges(
            &h,
            &[
                cu(0, 0, 16, 16),
                with_tx(cu(16, 0, 16, 8), 4, 4),
                cu(16, 8, 16, 8),
            ],
        );
        let upper = find(&e, 16, 0, true);
        let lower = find(&e, 16, 8, true);
        assert_eq!((upper.filter_size, upper.length), (4, 8));
        assert_eq!((lower.filter_size, lower.length), (16, 8));
    }

    /// A unit without a transform block list (a skipped inter block) uses its largest transform:
    /// no internal edges, and 4x4 when lossless.
    #[test]
    fn a_unit_without_transform_blocks_uses_the_largest_transform_or_4x4_when_lossless() {
        let h = header(64, 32, [20, 20]);
        let mut big = inter(cu(0, 0, 64, 32));
        big.skip = true;
        // 64x32 block: one 64x32 transform, no internal vertical edge
        assert!(edges(&h, &[big.clone()])
            .iter()
            .all(|e| !(e.vertical && e.x > 0)));
        // lossless: every 4x4 is a transform
        let e = edges_for(&h, &[big], true);
        assert_eq!(find(&e, 4, 0, true).filter_size, 4);
        assert!(
            !find(&e, 4, 0, true).filtered,
            "inside a skipped inter block"
        );
    }
}
