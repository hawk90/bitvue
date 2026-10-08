//! The reference-motion-vector map and the contexts/candidate stacks derived from it (dav1d
//! `refmvs`, `src/refmvs.c`). This file holds the state and how blocks are recorded in it; the
//! `impl` blocks that read it are split by purpose:
//!
//! - `match_ref`: whether an edge neighbour matches a reference (`motion_mode`'s warp gate);
//! - `mode_context`: `inter_mode` / `compound_mode` contexts;
//! - `candidates`: the neighbour scans and the merge-by-value stack primitives;
//! - `single_stack` / `compound_stack`: the candidate stacks themselves.

mod candidates;
mod compound_stack;
mod match_ref;
mod mode_context;
mod single_stack;
#[cfg(test)]
mod tests;

use super::drl::{CompoundMvStackEntry, MvStackEntry};

/// One 4x4 cell's reference-frame/mode state for `inter_mode`/`compound_mode` context (spec
/// 5.11.23/5.11.24's `newmv_ctx`/`refmv_ctx`/`compound_mode`'s CDF index). Source: rav1d
/// `RefMvsBlock` (`memorysafety/rav1d`, BSD-2-Clause, `src/refmvs.rs`), reduced to only the
/// fields `rav1d_refmvs_find`'s context derivation (not its motion-vector CANDIDATE LIST, which
/// this crate doesn't replicate -- see `crate::tile::mv_prediction::MvPredictorContext` for
/// Bitvue's separate, simpler MV-value predictor) actually reads: whether a decoded block's
/// stored ref matches the current block's ref (any block width works, since a wider block just
/// repeats the same ref/mode across its own footprint -- see `SpatialRefContext`'s doc for why
/// per-4x4-cell scanning doesn't need rav1d's block-width-aware step optimization at all).
#[derive(Clone, Copy, Default)]
pub(in crate::tile::context) struct SpatialRefCell {
    /// `false` for intra blocks (rav1d: `RefMvsBlock.mv[0].is_invalid()`) or unwritten cells --
    /// never contributes to a match.
    pub(in crate::tile::context) valid: bool,
    /// rav1d's 0..=6 `ref` encoding (see `TileContext`'s `above_ref0`/`left_ref0` doc).
    pub(in crate::tile::context) ref0: i8,
    /// rav1d's 0..=6 `ref` encoding, or `-1` for a single-ref block.
    pub(in crate::tile::context) ref1: i8,
    /// Whether this block's decoded mode contains a `NEWMV` component (rav1d: `RefMvsBlock.mf`
    /// bit 1 -- `mode == NEWMV` for single-ref, or any compound mode with a "New" L0/L1 component
    /// for compound; matches this crate's `PredictionMode::l0_mv_kind`/`l1_mv_kind` returning
    /// `Some(MvKind::New)`).
    pub(in crate::tile::context) is_newmv: bool,
    /// This block's own decoded MVs (rav1d `RefMvsBlock.mv.mv[0]/[1]`) -- only consumed by
    /// `single_ref_mv_stack` (DRL's real candidate list, `SymbolDecoder::read_drl_bit`'s doc),
    /// unused by the match-count-only `scan` this struct's other fields feed.
    pub(in crate::tile::context) mv0: crate::tile::coding_unit::MotionVector,
    pub(in crate::tile::context) mv1: crate::tile::coding_unit::MotionVector,
    /// This block's own footprint (rav1d derives this from `RefMvsBlock.bs` via
    /// `dav1d_block_dimensions`) -- needed by `single_ref_mv_stack`'s real neighbor-width-aware
    /// row/col stepping (`scan_row`/`scan_col`'s doc), which -- unlike `scan`'s per-cell OR --
    /// cannot get an equivalent result from per-cell iteration alone (candidate weight depends on
    /// the actual overlap length with a neighbor, not just presence/absence of a match).
    pub(in crate::tile::context) width_4x4: u8,
    pub(in crate::tile::context) height_4x4: u8,
}

/// Real above/left/secondary-neighbor `inter_mode`/`compound_mode` context, per AV1 spec
/// 5.11.23/5.11.24 and rav1d's `rav1d_refmvs_find` (`memorysafety/rav1d`, BSD-2-Clause,
/// `src/refmvs.rs`) -- **spatial only**. `globalmv_ctx` (the third component of single-ref
/// `inter_mode`'s packed context) additionally depends on a temporal motion-field projected from
/// a *different* decoded frame (rav1d: `add_temporal_candidate`, gated on the frame header's
/// `use_ref_frame_mvs` AND on `rf.n_mfmvs > 0`, itself gated on at least one reference slot
/// actually having a *saved* motion field from decoding that reference -- spec 7.9's
/// `motion_field_estimation`) -- a separate, decoder-wide cross-frame subsystem this doesn't
/// implement (no reconstruction/motion compensation, so no per-frame motion field is ever saved).
/// `inter_mode_context` takes `use_ref_frame_mvs` (the frame header flag) directly as
/// `globalmv_ctx`'s value: rav1d's own `globalmv_ctx` local is *initialized* to exactly this flag
/// and only gets overridden away from it when a real temporal candidate is found at that specific
/// query position, so this matches rav1d's true value in every case except "a temporal candidate
/// existed and disagreed" -- a documented, deliberately partial approximation (confirmed with the
/// user before implementing the spatial piece alone; the temporal half remains a follow-up, not a
/// bug), strictly closer than the earlier hardcoded `0`. `compound_mode`'s context has no
/// temporal dependency at all, so it's fully real.
///
/// Unlike `TileContext`'s above/left arrays (`skip`/`mode`/`partition`/`ref_frame`), this needs a
/// full `width x height` grid, not just one row + one column: rav1d's neighbor scan reaches a
/// top-right cell, a top-left cell, and "secondary" rows/columns 2-3 units further back, all of
/// which can be genuinely above-*and*-to-the-side of the query block, not purely above or purely
/// left. Persists for the whole tile (no `start_superblock_row` reset -- the secondary scans need
/// history from earlier superblock rows, unlike `skip`/`mode`/`partition`'s one-row lookback).
///
/// rav1d additionally uses candidate blocks' *widths* to skip redundant same-block re-scans (a
/// performance optimization for its real-time decoder). This doesn't affect the result: since a
/// wider block repeats the same `ref`/`is_newmv` across every 4x4 cell it covers, scanning every
/// cell individually and OR-ing the match outcome is behaviorally identical to rav1d's
/// width-aware stepping -- so `SpatialRefCell` doesn't need to store block dimensions at all.
pub struct SpatialRefContext {
    width_4x4: u32,
    height_4x4: u32,
    cells: Vec<SpatialRefCell>,
    /// Frame extent in 4x4 units (dav1d `rt->tile_col.end`/`tile_row.end`, clipped to `iw4`/`ih4`):
    /// neighbour scans stop there and candidate MVs are clamped against it. Defaults to the
    /// allocated grid; `set_frame_extent` narrows it to the real frame.
    col_end: u32,
    row_end: u32,
    /// Frame size in 4x4 units (dav1d `iw4`/`ih4`), the bound candidate MVs are clamped against.
    iw4: u32,
    ih4: u32,
    /// `ref_frame_sign_bias[ref]` for `LAST..=ALTREF` (dav1d `rf->sign_bias`): whether the
    /// reference lies after the current frame. All `false` when the caller has no reference
    /// order hints (stateless parses), which makes the sign-flip of extended candidates a no-op.
    sign_bias: [bool; 7],
    /// Real temporal motion field for this frame (spec 7.9/7.10, [`crate::tile::motion_field`]),
    /// set once via [`SpatialRefContext::set_temporal_context`] before any block is decoded --
    /// `None` for every existing caller that doesn't opt in (the crate's stateless, single-frame
    /// parse path), preserving this struct's prior spatial-only/`use_ref_frame_mvs`-flag-only
    /// behavior exactly. See this struct's own doc for why this piece needs cross-frame state.
    temporal: Option<TemporalMvContext>,
}

/// Per-frame temporal-candidate inputs, set once and read by every block's
/// `single_ref_mv_stack`/`inter_mode_context` call -- see [`crate::tile::motion_field`]'s module
/// doc for how these are produced.
struct TemporalMvContext {
    projected: crate::tile::motion_field::ProjectedMotionField,
    /// This frame's own `pocdiff[0..=6]` (spec 7.9.2, this crate's 0..=6 `ref0` convention) --
    /// `add_temporal_candidate`'s numerator (`refmvs.c:201`).
    pocdiff: [i32; 7],
    /// Frame MV precision for `fix_mv_precision` of projected candidates.
    allow_high_precision_mv: bool,
    force_integer_mv: bool,
}

impl SpatialRefContext {
    pub fn new(width_4x4: u32, height_4x4: u32) -> Self {
        let width_4x4 = width_4x4.max(1);
        let height_4x4 = height_4x4.max(1);
        Self {
            width_4x4,
            height_4x4,
            cells: vec![SpatialRefCell::default(); (width_4x4 * height_4x4) as usize],
            col_end: width_4x4,
            row_end: height_4x4,
            iw4: width_4x4,
            ih4: height_4x4,
            sign_bias: [false; 7],
            temporal: None,
        }
    }

    /// Sets the real frame extent in 4x4 units. See the `col_end` field.
    pub fn set_frame_extent(&mut self, width: u32, height: u32) {
        // Tile end is in 8x8-aligned 4x4 units (dav1d `f->bw`/`f->bh`); the clamp bound is exact.
        self.col_end = (2 * width.div_ceil(8)).min(self.width_4x4);
        self.row_end = (2 * height.div_ceil(8)).min(self.height_4x4);
        // dav1d `rf->iw4 = iw8 << 1`: the clamp bound is 8x8-aligned too.
        self.iw4 = 2 * width.div_ceil(8);
        self.ih4 = 2 * height.div_ceil(8);
    }

    /// Sets the per-reference sign bias. See the `sign_bias` field.
    pub fn set_sign_bias(&mut self, sign_bias: [bool; 7]) {
        self.sign_bias = sign_bias;
    }

    /// Opt this frame's parse into real temporal MV candidates -- see [`crate::tile::motion_field`]
    /// for how `projected`/`pocdiff` are computed. Not called by any existing (stateless,
    /// single-frame) production call site; only the sequential test harness that threads
    /// [`crate::tile::motion_field::MotionFieldState`] across frames calls this.
    pub fn set_temporal_context(
        &mut self,
        projected: crate::tile::motion_field::ProjectedMotionField,
        pocdiff: [i32; 7],
    ) {
        // `pocdiff` is current - reference; a reference after the current frame has sign bias 1.
        self.sign_bias = std::array::from_fn(|i| pocdiff[i] < 0);
        self.temporal = Some(TemporalMvContext {
            projected,
            pocdiff,
            allow_high_precision_mv: true,
            force_integer_mv: false,
        });
    }

    /// Sets the frame's MV precision (`allow_high_precision_mv`, `force_integer_mv`), which
    /// projected temporal candidates are rounded to. Call after `set_temporal_context`.
    pub fn set_mv_precision(&mut self, allow_high_precision_mv: bool, force_integer_mv: bool) {
        if let Some(t) = &mut self.temporal {
            t.allow_high_precision_mv = allow_high_precision_mv;
            t.force_integer_mv = force_integer_mv;
        }
    }

    pub(in crate::tile::context) fn cell(&self, x4: u32, y4: u32) -> Option<&SpatialRefCell> {
        if x4 >= self.width_4x4 || y4 >= self.height_4x4 {
            return None;
        }
        self.cells.get((y4 * self.width_4x4 + x4) as usize)
    }

    /// Record a decoded **inter** block's ref/mode state across its 4x4-unit footprint. Never
    /// called for intra blocks -- rav1d's own `splat_mv` is likewise only invoked from the
    /// inter-block decode path (`decode.rs`), leaving intra-covered cells at their default
    /// `valid: false` for the lifetime of the tile (every cell is visited by exactly one coding
    /// block during a tile's decode, so "never written" and "written by an intra block" coincide).
    #[allow(clippy::too_many_arguments)]
    pub fn set_block(
        &mut self,
        x4: u32,
        y4: u32,
        width_4x4: u32,
        height_4x4: u32,
        ref0: i8,
        ref1: i8,
        is_newmv: bool,
        mv0: crate::tile::coding_unit::MotionVector,
        mv1: crate::tile::coding_unit::MotionVector,
    ) {
        let cell = SpatialRefCell {
            valid: true,
            ref0,
            ref1,
            is_newmv,
            mv0,
            mv1,
            width_4x4: width_4x4.min(255) as u8,
            height_4x4: height_4x4.min(255) as u8,
        };
        let x_end = (x4 + width_4x4).min(self.width_4x4);
        let y_end = (y4 + height_4x4).min(self.height_4x4);
        for y in y4..y_end {
            for x in x4..x_end {
                self.cells[(y * self.width_4x4 + x) as usize] = cell;
            }
        }
    }

    /// Records an intra block of an inter frame (dav1d `splat_intraref`): no MV, but its size
    /// still steers the width-aware neighbour scans.
    pub fn set_intra_block(&mut self, x4: u32, y4: u32, width_4x4: u32, height_4x4: u32) {
        let cell = SpatialRefCell {
            ref1: -1,
            width_4x4: width_4x4.min(255) as u8,
            height_4x4: height_4x4.min(255) as u8,
            ..SpatialRefCell::default()
        };
        let x_end = (x4 + width_4x4).min(self.width_4x4);
        let y_end = (y4 + height_4x4).min(self.height_4x4);
        for y in y4..y_end {
            for x in x4..x_end {
                self.cells[(y * self.width_4x4 + x) as usize] = cell;
            }
        }
    }
}
