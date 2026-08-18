//! Above/left neighbor-state tracking for entropy-context derivation.
//!
//! Per AV1 spec Section 9.3 (Function `get_ctx`) and rav1d's `BlockContext`
//! (`memorysafety/rav1d`, BSD-2-Clause, `src/env.rs`) -- covers the `skip` flag's context (see
//! `SymbolDecoder::read_skip`'s doc), key-frame `intra_mode`'s context (see
//! `SymbolDecoder::read_intra_mode`'s doc), real partition-context (per-8x8 bitmask, ported from
//! dav1d's edge-index tree), and the real spatial+temporal reference-motion-vector-candidate
//! subsystem (`refmvs`, ported from `src/refmvs.c`) backing `inter_mode`/`compound_mode`/DRL
//! context -- see `docs/DEVELOPMENT_PHASES.md` Phase 4's AV1 entropy-decoding notes for the full
//! history. Residual (`coeff_base`/`coeff_br`/etc.) context is real for neighbor/position axes
//! but still approximates the plane (luma-only) and qindex-bucket (first-bucket-only) axes --
//! see `SpatialRefContext`'s residual CDF selection and `symbol/cdf.rs`.
//!
//! Units throughout are 4x4 pixels (spec's context-array granularity).

/// Maps a raw intra prediction-mode symbol (0..=12, spec `y_mode`/`uv_mode` values -- matches
/// `bitvue_av1_codec::tile::PredictionMode`'s intra-variant declaration order exactly) to one of
/// 5 mode-context classes used to index the key-frame `kfym` CDF. Source: rav1d
/// `DAV1D_INTRA_MODE_CONTEXT` (`memorysafety/rav1d`, BSD-2-Clause, `src/tables.rs`).
const INTRA_MODE_CONTEXT: [u8; 13] = [0, 1, 2, 3, 4, 4, 4, 4, 3, 0, 1, 2, 0];

/// `DAV1D_AL_PART_CTX[dir][bl][bp]` -- the bitmask written into `above_partition`/`left_partition`
/// after a `partition` symbol `bp` is decided at level `bl`, one row per `bl` (0=128x128..4=8x8,
/// rav1d's `BlockLevel` convention -- opposite of `block_size_log2`, see `partition_bl`), one
/// column per `PartitionType` (0=None..9=Vert4, matches this crate's `PartitionType` enum
/// ordering exactly). `0xff` marks a partition type that's invalid at that level (never actually
/// written in practice, since callers only reach `set_partition` for partition types
/// `PartitionType::is_allowed` at the block's own size already validated). Source: rav1d
/// `DAV1D_AL_PART_CTX` (`memorysafety/rav1d`, BSD-2-Clause, `src/tables.rs`).
const PARTITION_CTX_TABLE: [[[u8; 10]; 5]; 2] = [
    // above
    [
        [0x00, 0x00, 0x10, 0xff, 0x00, 0x10, 0x10, 0x10, 0xff, 0xff],
        [0x10, 0x10, 0x18, 0xff, 0x10, 0x18, 0x18, 0x18, 0x10, 0x1c],
        [0x18, 0x18, 0x1c, 0xff, 0x18, 0x1c, 0x1c, 0x1c, 0x18, 0x1e],
        [0x1c, 0x1c, 0x1e, 0xff, 0x1c, 0x1e, 0x1e, 0x1e, 0x1c, 0x1f],
        [0x1e, 0x1e, 0x1f, 0x1f, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff],
    ],
    // left
    [
        [0x00, 0x10, 0x00, 0xff, 0x10, 0x10, 0x00, 0x10, 0xff, 0xff],
        [0x10, 0x18, 0x10, 0xff, 0x18, 0x18, 0x10, 0x18, 0x1c, 0x10],
        [0x18, 0x1c, 0x18, 0xff, 0x1c, 0x1c, 0x18, 0x1c, 0x1e, 0x18],
        [0x1c, 0x1e, 0x1c, 0xff, 0x1e, 0x1e, 0x1c, 0x1e, 0x1f, 0x1c],
        [0x1e, 0x1f, 0x1e, 0x1f, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff],
    ],
];

/// `DAV1D_SKIP_CTX[min(la,4)][min(ll,4)]` -- `txb_skip`'s context table, indexed by the OR-reduced
/// above/left `cul_level` values (see `TileContext::txb_skip_context`'s doc). Source: rav1d
/// `dav1d_skip_ctx` (`memorysafety/rav1d`, BSD-2-Clause, `src/tables.rs`).
const DAV1D_SKIP_CTX: [[u8; 5]; 5] = [
    [1, 2, 2, 2, 3],
    [2, 4, 4, 4, 5],
    [2, 4, 4, 4, 5],
    [2, 4, 4, 4, 5],
    [3, 5, 5, 5, 6],
];

/// Maps `block_size_log2` (2..=7, this crate's CDF-lookup convention) to rav1d's `BlockLevel`
/// (0=128x128..4=8x8 -- opposite numeric order). Only valid for `block_size_log2` in 3..=7 (8x8
/// and up); 4x4 blocks (log2=2) never read a `partition` symbol at all, so have no `BlockLevel`.
pub fn partition_bl(block_size_log2: u8) -> u8 {
    7 - block_size_log2.clamp(3, 7)
}

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
struct SpatialRefCell {
    /// `false` for intra blocks (rav1d: `RefMvsBlock.mv[0].is_invalid()`) or unwritten cells --
    /// never contributes to a match.
    valid: bool,
    /// rav1d's 0..=6 `ref` encoding (see `TileContext`'s `above_ref0`/`left_ref0` doc).
    ref0: i8,
    /// rav1d's 0..=6 `ref` encoding, or `-1` for a single-ref block.
    ref1: i8,
    /// Whether this block's decoded mode contains a `NEWMV` component (rav1d: `RefMvsBlock.mf`
    /// bit 1 -- `mode == NEWMV` for single-ref, or any compound mode with a "New" L0/L1 component
    /// for compound; matches this crate's `PredictionMode::l0_mv_kind`/`l1_mv_kind` returning
    /// `Some(MvKind::New)`).
    is_newmv: bool,
    /// This block's own decoded MVs (rav1d `RefMvsBlock.mv.mv[0]/[1]`) -- only consumed by
    /// `single_ref_mv_stack` (DRL's real candidate list, `SymbolDecoder::read_drl_bit`'s doc),
    /// unused by the match-count-only `scan` this struct's other fields feed.
    mv0: crate::tile::coding_unit::MotionVector,
    mv1: crate::tile::coding_unit::MotionVector,
    /// This block's own footprint (rav1d derives this from `RefMvsBlock.bs` via
    /// `dav1d_block_dimensions`) -- needed by `single_ref_mv_stack`'s real neighbor-width-aware
    /// row/col stepping (`scan_row`/`scan_col`'s doc), which -- unlike `scan`'s per-cell OR --
    /// cannot get an equivalent result from per-cell iteration alone (candidate weight depends on
    /// the actual overlap length with a neighbor, not just presence/absence of a match).
    width_4x4: u8,
    height_4x4: u8,
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
}

impl SpatialRefContext {
    pub fn new(width_4x4: u32, height_4x4: u32) -> Self {
        let width_4x4 = width_4x4.max(1);
        let height_4x4 = height_4x4.max(1);
        Self {
            width_4x4,
            height_4x4,
            cells: vec![SpatialRefCell::default(); (width_4x4 * height_4x4) as usize],
            temporal: None,
        }
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
        self.temporal = Some(TemporalMvContext { projected, pocdiff });
    }

    fn cell(&self, x4: u32, y4: u32) -> Option<&SpatialRefCell> {
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

    /// Whether a decoded cell's ref matches the query block's `(ref0, ref1)` -- rav1d
    /// `add_spatial_candidate`: single-ref (`ref1 < 0`) matches if the cell's `ref0` *or* `ref1`
    /// equals the query's `ref0`; compound matches only on an exact `(ref0, ref1)` pair match.
    fn ref_matches(cell: &SpatialRefCell, ref0: i8, ref1: i8) -> bool {
        if !cell.valid {
            return false;
        }
        if ref1 < 0 {
            cell.ref0 == ref0 || cell.ref1 == ref0
        } else {
            cell.ref0 == ref0 && cell.ref1 == ref1
        }
    }

    /// Spatial neighbor scan shared by `inter_mode_context`/`compound_mode_context`. Returns
    /// `(nearest_match, ref_match_count, have_newmv)`, matching rav1d `rav1d_refmvs_find`'s
    /// same-named locals (`src/refmvs.rs`) computed purely from the "primary" above-row/left-
    /// column/top-right/top-left scans plus the "secondary" (2/3-units-back) row/column scans --
    /// everything except the temporal (`add_temporal_candidate`) contribution, see this struct's
    /// doc. `w4`/`h4` are the query block's own width/height in 4x4 units, capped to 16 (rav1d:
    /// `cmp::min(bw4, 16)`/`cmp::min(bh4, 16)` -- the tile-bound clamp rav1d also applies is
    /// skipped here, matching the rest of this crate's "whole frame as one tile" simplification).
    fn scan(&self, x4: u32, y4: u32, bw4: u32, bh4: u32, ref0: i8, ref1: i8) -> (u8, u8, bool) {
        let w4 = bw4.clamp(1, 16);
        let h4 = bh4.clamp(1, 16);
        let mut have_newmv = false;
        let mut have_row_mvs = false;
        let mut have_col_mvs = false;

        // Primary above-row scan.
        if y4 > 0 {
            for x in x4..x4 + w4 {
                if let Some(cell) = self.cell(x, y4 - 1) {
                    if Self::ref_matches(cell, ref0, ref1) {
                        have_row_mvs = true;
                        have_newmv |= cell.is_newmv;
                    }
                }
            }
        }
        // Primary left-column scan.
        if x4 > 0 {
            for y in y4..y4 + h4 {
                if let Some(cell) = self.cell(x4 - 1, y) {
                    if Self::ref_matches(cell, ref0, ref1) {
                        have_col_mvs = true;
                        have_newmv |= cell.is_newmv;
                    }
                }
            }
        }
        // Top-right corner (single cell, one unit right of the block's own top-right 4x4).
        if y4 > 0 {
            if let Some(cell) = self.cell(x4 + bw4.max(1), y4 - 1) {
                if Self::ref_matches(cell, ref0, ref1) {
                    have_row_mvs = true;
                    have_newmv |= cell.is_newmv;
                }
            }
        }

        let nearest_match = u8::from(have_row_mvs) + u8::from(have_col_mvs);

        // Top-left corner -- contributes to `have_row_mvs` only, never `have_newmv` (rav1d uses a
        // throwaway `have_dummy_newmv_match` for this candidate, see `rav1d_refmvs_find`).
        if x4 > 0 && y4 > 0 {
            if let Some(cell) = self.cell(x4 - 1, y4 - 1) {
                if Self::ref_matches(cell, ref0, ref1) {
                    have_row_mvs = true;
                }
            }
        }

        // "Secondary" row/column scans at 2 and 3 units back -- also `have_newmv`-inert.
        for n in 2..=3u32 {
            let back = 2 * n - 1;
            if y4 >= back {
                let ry = y4 - back;
                for x in x4..x4 + w4 {
                    if let Some(cell) = self.cell(x, ry) {
                        if Self::ref_matches(cell, ref0, ref1) {
                            have_row_mvs = true;
                        }
                    }
                }
            }
            if x4 >= back {
                let cx = x4 - back;
                for y in y4..y4 + h4 {
                    if let Some(cell) = self.cell(cx, y) {
                        if Self::ref_matches(cell, ref0, ref1) {
                            have_col_mvs = true;
                        }
                    }
                }
            }
        }

        let ref_match_count = u8::from(have_row_mvs) + u8::from(have_col_mvs);
        (nearest_match, ref_match_count, have_newmv)
    }

    /// `(refmv_ctx, newmv_ctx)` per rav1d `rav1d_refmvs_find`'s context build-up (`src/refmvs.rs`).
    fn refmv_newmv_ctx(
        &self,
        x4: u32,
        y4: u32,
        bw4: u32,
        bh4: u32,
        ref0: i8,
        ref1: i8,
    ) -> (u8, u8) {
        let (nearest_match, ref_match_count, have_newmv) = self.scan(x4, y4, bw4, bh4, ref0, ref1);
        match nearest_match {
            0 => (ref_match_count.min(2), u8::from(ref_match_count > 0)),
            1 => ((ref_match_count * 3).min(4), 3 - u8::from(have_newmv)),
            _ => (5, 5 - u8::from(have_newmv)), // nearest_match == 2 (max possible)
        }
    }

    /// Packed single-ref `inter_mode` context (spec 5.11.23): `newmv_mode`'s CDF index is
    /// `ctx & 7`, `globalmv_mode`'s is `ctx >> 3 & 1`, `refmv_mode`'s is `ctx >> 4 & 15`. Source:
    /// rav1d `*ctx = refmv_ctx << 4 | globalmv_ctx << 3 | newmv_ctx` (`src/refmvs.rs`).
    /// `globalmv_ctx` is real when a temporal motion-field context was set on this struct (see
    /// this function's body / `set_temporal_context`), falling back to the frame header's
    /// `use_ref_frame_mvs` flag directly otherwise (`rav1d_refmvs_find` initializes its own
    /// `globalmv_ctx` local to exactly this value before ever overriding it with a real temporal
    /// candidate).
    pub fn inter_mode_context(
        &self,
        x4: u32,
        y4: u32,
        bw4: u32,
        bh4: u32,
        ref0: i8,
        use_ref_frame_mvs: bool,
    ) -> u16 {
        let (refmv_ctx, newmv_ctx) = self.refmv_newmv_ctx(x4, y4, bw4, bh4, ref0, -1);
        // Real `globalmv_ctx` (rav1d `add_temporal_candidate`'s `!(x|y)` sample, `refmvs.c:206-
        // 207`): the temporal candidate at this block's own top-left 8x8 cell, compared against
        // the global-motion predictor -- approximated as always-invalid/zero (this crate's
        // established `global_motion_params` bit-skip-only approximation, matching
        // `single_ref_mv_stack`'s temporal scan doc), so the comparison reduces to `mv != zero`.
        // Falls back to the pre-existing `use_ref_frame_mvs`-flag approximation when no temporal
        // context was set (this struct's doc) -- unchanged behavior for every caller that doesn't
        // opt in.
        let globalmv_ctx = match &self.temporal {
            Some(t) if use_ref_frame_mvs && ref0 >= 0 => {
                let x8 = x4 >> 1;
                let y8 = y4 >> 1;
                match t.projected.get(x8, y8) {
                    Some(cell) => {
                        let mv = crate::tile::motion_field::mv_projection(
                            cell.mv,
                            t.pocdiff[ref0 as usize],
                            cell.ref2ref,
                        );
                        u16::from(mv.x != 0 || mv.y != 0)
                    }
                    None => u16::from(use_ref_frame_mvs),
                }
            }
            _ => u16::from(use_ref_frame_mvs),
        };
        (refmv_ctx as u16) << 4 | globalmv_ctx << 3 | newmv_ctx as u16
    }

    /// `compound_mode` context (spec 5.11.24, 0..=7): fully real, no temporal dependency. Source:
    /// rav1d's compound remap of `refmv_ctx`/`newmv_ctx` (`src/refmvs.rs`):
    /// `match refmv_ctx >> 1 { 0 => min(newmv_ctx,1), 1 => 1+min(newmv_ctx,3), _ => clamp(3+newmv_ctx,4,7) }`.
    pub fn compound_mode_context(
        &self,
        x4: u32,
        y4: u32,
        bw4: u32,
        bh4: u32,
        ref0: i8,
        ref1: i8,
    ) -> u8 {
        let (refmv_ctx, newmv_ctx) = self.refmv_newmv_ctx(x4, y4, bw4, bh4, ref0, ref1);
        match refmv_ctx >> 1 {
            0 => newmv_ctx.min(1),
            1 => 1 + newmv_ctx.min(3),
            _ => (3 + newmv_ctx).clamp(4, 7),
        }
    }

    /// This position's single-ref candidate, if its stored ref matches `ref0` -- rav1d
    /// `add_spatial_candidate`'s `for n in 0..2 { if b.ref.ref[n] == ref.ref[0] { ... } }`: checks
    /// BOTH of the neighbor's own ref slots (a *compound* neighbor can still contribute to a
    /// *single-ref* query, via whichever of its two refs happens to match).
    fn single_ref_candidate_mv(
        cell: &SpatialRefCell,
        ref0: i8,
    ) -> Option<crate::tile::coding_unit::MotionVector> {
        if !cell.valid {
            return None;
        }
        if cell.ref0 == ref0 {
            Some(cell.mv0)
        } else if cell.ref1 == ref0 {
            Some(cell.mv1)
        } else {
            None
        }
    }

    /// Merge-or-append one candidate into the real weighted DRL stack (rav1d
    /// `add_spatial_candidate`'s single-ref branch): identical-MV candidates accumulate weight
    /// instead of duplicating (matches by real MV *value*, not by which neighbor position found
    /// it).
    fn push_mv_candidate(
        stack: &mut [MvStackEntry; 8],
        cnt: &mut usize,
        weight: i32,
        mv: crate::tile::coding_unit::MotionVector,
    ) {
        for cand in &mut stack[..*cnt] {
            if cand.mv == mv {
                cand.weight += weight;
                return;
            }
        }
        if *cnt < 8 {
            stack[*cnt] = MvStackEntry { mv, weight };
            *cnt += 1;
        }
    }

    /// This position's compound candidate, if its stored ref PAIR exactly matches `(ref0, ref1)`
    /// -- rav1d `add_spatial_candidate`'s compound branch (`refmvs.c:73`, `b->ref.pair ==
    /// ref.pair`): unlike the single-ref lookup above, there is no partial/either-slot fallback
    /// here at the spatial-scan stage (that only happens in the separate, deliberately-omitted
    /// `add_compound_extended_candidate` fallback -- `compound_mv_stack`'s doc).
    fn compound_candidate_mv(
        cell: &SpatialRefCell,
        ref0: i8,
        ref1: i8,
    ) -> Option<[crate::tile::coding_unit::MotionVector; 2]> {
        if cell.valid && cell.ref0 == ref0 && cell.ref1 == ref1 {
            Some([cell.mv0, cell.mv1])
        } else {
            None
        }
    }

    /// Compound counterpart of `push_mv_candidate` -- identical-pair candidates accumulate weight
    /// instead of duplicating (rav1d `add_spatial_candidate`'s compound branch, `mvstack[n].mv.n
    /// == cand_mv.n` comparing the whole packed pair at once).
    fn push_compound_mv_candidate(
        stack: &mut [CompoundMvStackEntry; 8],
        cnt: &mut usize,
        weight: i32,
        mv: [crate::tile::coding_unit::MotionVector; 2],
    ) {
        for cand in &mut stack[..*cnt] {
            if cand.mv == mv {
                cand.weight += weight;
                return;
            }
        }
        if *cnt < 8 {
            stack[*cnt] = CompoundMvStackEntry { mv, weight };
            *cnt += 1;
        }
    }

    /// Real neighbor-width-aware row scan (rav1d `scan_row`, `src/refmvs.rs`) -- unlike `scan`'s
    /// per-cell OR (sufficient for a boolean match-count, `SpatialRefContext`'s doc), DRL's real
    /// weight needs the actual overlap length with each distinct neighbor along the row, so this
    /// steps by each neighbor's own stored width instead of visiting every 4x4 cell independently.
    #[allow(clippy::too_many_arguments)]
    fn scan_row_weighted(
        &self,
        stack: &mut [MvStackEntry; 8],
        cnt: &mut usize,
        ref0: i8,
        row_y4: u32,
        x4: u32,
        bw4: u32,
        w4: u32,
        max_rows: i32,
        step: u32,
    ) {
        let Some(first) = self.cell(x4, row_y4) else {
            return;
        };
        let mut cand = *first;
        let mut cand_bw4 = (cand.width_4x4 as u32).max(1);
        let mut len = step.max(bw4.min(cand_bw4));

        if bw4 <= cand_bw4 {
            let weight = if bw4 == 1 {
                2
            } else {
                (cand.height_4x4 as u32).clamp(2, (2 * max_rows.max(1)) as u32)
            };
            if let Some(mv) = Self::single_ref_candidate_mv(&cand, ref0) {
                Self::push_mv_candidate(stack, cnt, (len * weight) as i32, mv);
            }
            return;
        }

        let mut x = 0u32;
        loop {
            if let Some(mv) = Self::single_ref_candidate_mv(&cand, ref0) {
                Self::push_mv_candidate(stack, cnt, (len * 2) as i32, mv);
            }
            x += len;
            if x >= w4 {
                return;
            }
            let Some(next) = self.cell(x4 + x, row_y4) else {
                return;
            };
            cand = *next;
            cand_bw4 = (cand.width_4x4 as u32).max(1);
            len = step.max(cand_bw4);
        }
    }

    /// Real neighbor-height-aware column scan -- `scan_row_weighted`'s doc, transposed (rav1d
    /// `scan_col`).
    #[allow(clippy::too_many_arguments)]
    fn scan_col_weighted(
        &self,
        stack: &mut [MvStackEntry; 8],
        cnt: &mut usize,
        ref0: i8,
        col_x4: u32,
        y4: u32,
        bh4: u32,
        h4: u32,
        max_cols: i32,
        step: u32,
    ) {
        let Some(first) = self.cell(col_x4, y4) else {
            return;
        };
        let mut cand = *first;
        let mut cand_bh4 = (cand.height_4x4 as u32).max(1);
        let mut len = step.max(bh4.min(cand_bh4));

        if bh4 <= cand_bh4 {
            let weight = if bh4 == 1 {
                2
            } else {
                (cand.width_4x4 as u32).clamp(2, (2 * max_cols.max(1)) as u32)
            };
            if let Some(mv) = Self::single_ref_candidate_mv(&cand, ref0) {
                Self::push_mv_candidate(stack, cnt, (len * weight) as i32, mv);
            }
            return;
        }

        let mut y = 0u32;
        loop {
            if let Some(mv) = Self::single_ref_candidate_mv(&cand, ref0) {
                Self::push_mv_candidate(stack, cnt, (len * 2) as i32, mv);
            }
            y += len;
            if y >= h4 {
                return;
            }
            let Some(next) = self.cell(col_x4, y4 + y) else {
                return;
            };
            cand = *next;
            cand_bh4 = (cand.height_4x4 as u32).max(1);
            len = step.max(cand_bh4);
        }
    }

    /// Compound counterpart of `scan_row_weighted` -- identical neighbor-width-aware stepping and
    /// weight math, only the per-cell candidate extraction differs (`compound_candidate_mv`'s
    /// exact-pair match instead of `single_ref_candidate_mv`'s single-ref match).
    #[allow(clippy::too_many_arguments)]
    fn scan_row_weighted_compound(
        &self,
        stack: &mut [CompoundMvStackEntry; 8],
        cnt: &mut usize,
        ref0: i8,
        ref1: i8,
        row_y4: u32,
        x4: u32,
        bw4: u32,
        w4: u32,
        max_rows: i32,
        step: u32,
    ) {
        let Some(first) = self.cell(x4, row_y4) else {
            return;
        };
        let mut cand = *first;
        let mut cand_bw4 = (cand.width_4x4 as u32).max(1);
        let mut len = step.max(bw4.min(cand_bw4));

        if bw4 <= cand_bw4 {
            let weight = if bw4 == 1 {
                2
            } else {
                (cand.height_4x4 as u32).clamp(2, (2 * max_rows.max(1)) as u32)
            };
            if let Some(mv) = Self::compound_candidate_mv(&cand, ref0, ref1) {
                Self::push_compound_mv_candidate(stack, cnt, (len * weight) as i32, mv);
            }
            return;
        }

        let mut x = 0u32;
        loop {
            if let Some(mv) = Self::compound_candidate_mv(&cand, ref0, ref1) {
                Self::push_compound_mv_candidate(stack, cnt, (len * 2) as i32, mv);
            }
            x += len;
            if x >= w4 {
                return;
            }
            let Some(next) = self.cell(x4 + x, row_y4) else {
                return;
            };
            cand = *next;
            cand_bw4 = (cand.width_4x4 as u32).max(1);
            len = step.max(cand_bw4);
        }
    }

    /// Compound counterpart of `scan_col_weighted` -- `scan_row_weighted_compound`'s doc,
    /// transposed.
    #[allow(clippy::too_many_arguments)]
    fn scan_col_weighted_compound(
        &self,
        stack: &mut [CompoundMvStackEntry; 8],
        cnt: &mut usize,
        ref0: i8,
        ref1: i8,
        col_x4: u32,
        y4: u32,
        bh4: u32,
        h4: u32,
        max_cols: i32,
        step: u32,
    ) {
        let Some(first) = self.cell(col_x4, y4) else {
            return;
        };
        let mut cand = *first;
        let mut cand_bh4 = (cand.height_4x4 as u32).max(1);
        let mut len = step.max(bh4.min(cand_bh4));

        if bh4 <= cand_bh4 {
            let weight = if bh4 == 1 {
                2
            } else {
                (cand.width_4x4 as u32).clamp(2, (2 * max_cols.max(1)) as u32)
            };
            if let Some(mv) = Self::compound_candidate_mv(&cand, ref0, ref1) {
                Self::push_compound_mv_candidate(stack, cnt, (len * weight) as i32, mv);
            }
            return;
        }

        let mut y = 0u32;
        loop {
            if let Some(mv) = Self::compound_candidate_mv(&cand, ref0, ref1) {
                Self::push_compound_mv_candidate(stack, cnt, (len * 2) as i32, mv);
            }
            y += len;
            if y >= h4 {
                return;
            }
            let Some(next) = self.cell(col_x4, y4 + y) else {
                return;
            };
            cand = *next;
            cand_bh4 = (cand.height_4x4 as u32).max(1);
            len = step.max(cand_bh4);
        }
    }

    /// Real weighted single-ref DRL candidate stack (spec 7.10.2's `RefMvStack`, single-ref only
    /// -- see `SymbolDecoder::read_drl_bit`'s doc for what this crate deliberately omits: temporal
    /// candidates, compound extension, and the single-ref "non-self-reference" `sign_bias`
    /// extension). Returns `(stack, cnt)` -- `cnt` (real spec's `NumMvFound`) gates whether DRL
    /// bits are read at all; `stack[0]`/`stack[1]` are always safe to read as MV predictors
    /// regardless of `cnt` (default-zero, matching this crate's existing GLOBALMV-as-zero
    /// fallback -- `crate::tile::mv_prediction::MvPredictorContext::predict_global_mv`'s doc).
    /// `get_drl_context`'s doc for how the real weight values this builds are consumed.
    pub fn single_ref_mv_stack(
        &self,
        x4: u32,
        y4: u32,
        bw4: u32,
        bh4: u32,
        ref0: i8,
        use_ref_frame_mvs: bool,
    ) -> ([MvStackEntry; 8], usize) {
        let mut stack = [MvStackEntry::default(); 8];
        let mut cnt = 0usize;
        let w4 = bw4.clamp(1, 16);
        let h4 = bh4.clamp(1, 16);

        let have_top = y4 > 0;
        let have_left = x4 > 0;
        let max_rows = if have_top {
            y4.div_ceil(2).min(2 + u32::from(bh4 > 1)) as i32
        } else {
            0
        };
        let max_cols = if have_left {
            x4.div_ceil(2).min(2 + u32::from(bw4 > 1)) as i32
        } else {
            0
        };

        if have_top {
            self.scan_row_weighted(
                &mut stack,
                &mut cnt,
                ref0,
                y4 - 1,
                x4,
                bw4,
                w4,
                max_rows,
                if bw4 >= 16 { 4 } else { 1 },
            );
        }
        if have_left {
            self.scan_col_weighted(
                &mut stack,
                &mut cnt,
                ref0,
                x4 - 1,
                y4,
                bh4,
                h4,
                max_cols,
                if bh4 >= 16 { 4 } else { 1 },
            );
        }
        // Top-right corner.
        if have_top {
            if let Some(cell) = self.cell(x4 + bw4.max(1), y4 - 1) {
                if let Some(mv) = Self::single_ref_candidate_mv(cell, ref0) {
                    Self::push_mv_candidate(&mut stack, &mut cnt, 4, mv);
                }
            }
        }

        // Real spec bumps every candidate found so far (the "nearest" group: top row + left col +
        // top-right) by a flat +640 -- `get_drl_context`'s `>= 640` threshold exists specifically
        // to distinguish this group from the lower-weight "secondary" group added below, so the
        // exact pre-bump weight magnitude stops mattering the moment this runs.
        for cand in &mut stack[..cnt] {
            cand.weight += 640;
        }

        // Temporal candidates (spec 7.10, rav1d `add_temporal_candidate`'s call site,
        // `refmvs.c:416-431` -- main grid scan only, see `crate::tile::motion_field::
        // add_temporal_candidates`'s doc for the small extra-corner-samples simplification).
        // Weight 2, same low tier as the secondary spatial group below -- both get sorted
        // together by the final whole-stack sort, matching real dav1d's structural ordering.
        if use_ref_frame_mvs && ref0 >= 0 {
            if let Some(temporal) = &self.temporal {
                let by8 = y4 >> 1;
                let bx8 = x4 >> 1;
                let w8 = ((w4 + 1) >> 1).min(8);
                let h8 = ((h4 + 1) >> 1).min(8);
                let step_h = if bw4 >= 16 { 2 } else { 1 };
                let step_v = if bh4 >= 16 { 2 } else { 1 };
                for mv in crate::tile::motion_field::add_temporal_candidates(
                    &temporal.projected,
                    temporal.pocdiff[ref0 as usize],
                    bx8,
                    by8,
                    w8,
                    h8,
                    step_h,
                    step_v,
                ) {
                    Self::push_mv_candidate(&mut stack, &mut cnt, 2, mv);
                }
            }
        }

        // Top-left corner (secondary group).
        if have_top && have_left {
            if let Some(cell) = self.cell(x4 - 1, y4 - 1) {
                if let Some(mv) = Self::single_ref_candidate_mv(cell, ref0) {
                    Self::push_mv_candidate(&mut stack, &mut cnt, 4, mv);
                }
            }
        }
        // "Secondary" row/col scans 2-3 units further back -- approximated via this same
        // neighbor-width-aware stepping at the real spec offsets, rather than porting rav1d's
        // separate 8x8-resolution indexing for this specific sub-scan (`single_ref_mv_stack`'s
        // doc: these entries stay well under the 640 threshold either way, so this only risks a
        // rare tie-break-ordering difference among already-low-weight secondary candidates, never
        // the primary-vs-secondary classification `get_drl_context` actually depends on).
        for n in 2..=3u32 {
            let back = 2 * n - 1;
            if have_top && y4 >= back {
                self.scan_row_weighted(
                    &mut stack,
                    &mut cnt,
                    ref0,
                    y4 - back,
                    x4,
                    bw4,
                    w4,
                    (1 + max_rows - n as i32).max(1),
                    if bw4 >= 16 { 4 } else { 2 },
                );
            }
            if have_left && x4 >= back {
                self.scan_col_weighted(
                    &mut stack,
                    &mut cnt,
                    ref0,
                    x4 - back,
                    y4,
                    bh4,
                    h4,
                    (1 + max_cols - n as i32).max(1),
                    if bh4 >= 16 { 4 } else { 2 },
                );
            }
        }

        // Sort each group (nearest, then secondary) by weight descending, matching real spec --
        // `nearest_cnt` isn't tracked separately here since every "nearest" entry's weight is
        // already `>= 640` (the bump above) and every "secondary" entry's is `< 640` (never
        // bumped), so a single whole-stack sort by weight produces the identical grouped-and-
        // ordered result without needing the boundary index.
        stack[..cnt].sort_by_key(|c| -c.weight);

        (stack, cnt)
    }

    /// Real weighted compound DRL candidate stack (spec 7.10.2's `RefMvStack`, compound pairs --
    /// `single_ref_mv_stack`'s doc for the shared spatial/temporal weight structure this mirrors).
    /// Candidates are joint `[MotionVector; 2]` pairs from neighbors whose stored ref PAIR exactly
    /// matches `(ref0, ref1)` (`compound_candidate_mv`'s doc) -- real spec's separate, deliberately
    /// **not implemented** `add_compound_extended_candidate` fallback (rav1d `refmvs.c:239-`, only
    /// triggered when `cnt < 2` after this scan) would additionally pad from non-self-reference
    /// neighbors and finally from the global-motion predictor; omitting it means a compound CU with
    /// fewer than 2 exact-pair-matching neighbors gets `CompoundMvStackEntry::default()` (zero MV)
    /// for the missing slot(s) instead of that real fallback value -- doesn't affect bitstream
    /// position (pure value computation, spec's own fallback reads no extra bits either), matches
    /// this crate's existing "GLOBALMV predictor approximated as zero" precedent
    /// (`crate::tile::mv_prediction::MvPredictorContext::predict_global_mv`'s doc).
    pub fn compound_mv_stack(
        &self,
        x4: u32,
        y4: u32,
        bw4: u32,
        bh4: u32,
        ref0: i8,
        ref1: i8,
        use_ref_frame_mvs: bool,
    ) -> ([CompoundMvStackEntry; 8], usize) {
        let mut stack = [CompoundMvStackEntry::default(); 8];
        let mut cnt = 0usize;
        let w4 = bw4.clamp(1, 16);
        let h4 = bh4.clamp(1, 16);

        let have_top = y4 > 0;
        let have_left = x4 > 0;
        let max_rows = if have_top {
            y4.div_ceil(2).min(2 + u32::from(bh4 > 1)) as i32
        } else {
            0
        };
        let max_cols = if have_left {
            x4.div_ceil(2).min(2 + u32::from(bw4 > 1)) as i32
        } else {
            0
        };

        if have_top {
            self.scan_row_weighted_compound(
                &mut stack,
                &mut cnt,
                ref0,
                ref1,
                y4 - 1,
                x4,
                bw4,
                w4,
                max_rows,
                if bw4 >= 16 { 4 } else { 1 },
            );
        }
        if have_left {
            self.scan_col_weighted_compound(
                &mut stack,
                &mut cnt,
                ref0,
                ref1,
                x4 - 1,
                y4,
                bh4,
                h4,
                max_cols,
                if bh4 >= 16 { 4 } else { 1 },
            );
        }
        // Top-right corner.
        if have_top {
            if let Some(cell) = self.cell(x4 + bw4.max(1), y4 - 1) {
                if let Some(mv) = Self::compound_candidate_mv(cell, ref0, ref1) {
                    Self::push_compound_mv_candidate(&mut stack, &mut cnt, 4, mv);
                }
            }
        }

        // Real spec bumps every candidate found so far (the "nearest" group) by a flat +640 --
        // `get_compound_drl_context`'s doc, same threshold/rationale as the single-ref version.
        for cand in &mut stack[..cnt] {
            cand.weight += 640;
        }

        // Temporal candidates (spec 7.10, rav1d `add_temporal_candidate`'s compound branch,
        // `refmvs.c:216-232` -- main grid scan only, same scope note as
        // `crate::tile::motion_field::add_temporal_compound_candidates`'s doc). Weight 2, same low
        // tier as the secondary spatial group below.
        if use_ref_frame_mvs && ref0 >= 0 && ref1 >= 0 {
            if let Some(temporal) = &self.temporal {
                let by8 = y4 >> 1;
                let bx8 = x4 >> 1;
                let w8 = ((w4 + 1) >> 1).min(8);
                let h8 = ((h4 + 1) >> 1).min(8);
                let step_h = if bw4 >= 16 { 2 } else { 1 };
                let step_v = if bh4 >= 16 { 2 } else { 1 };
                for mv in crate::tile::motion_field::add_temporal_compound_candidates(
                    &temporal.projected,
                    temporal.pocdiff[ref0 as usize],
                    temporal.pocdiff[ref1 as usize],
                    bx8,
                    by8,
                    w8,
                    h8,
                    step_h,
                    step_v,
                ) {
                    Self::push_compound_mv_candidate(&mut stack, &mut cnt, 2, mv);
                }
            }
        }

        // Top-left corner (secondary group).
        if have_top && have_left {
            if let Some(cell) = self.cell(x4 - 1, y4 - 1) {
                if let Some(mv) = Self::compound_candidate_mv(cell, ref0, ref1) {
                    Self::push_compound_mv_candidate(&mut stack, &mut cnt, 4, mv);
                }
            }
        }
        // "Secondary" row/col scans 2-3 units further back -- same approximation/rationale as
        // `single_ref_mv_stack`'s doc.
        for n in 2..=3u32 {
            let back = 2 * n - 1;
            if have_top && y4 >= back {
                self.scan_row_weighted_compound(
                    &mut stack,
                    &mut cnt,
                    ref0,
                    ref1,
                    y4 - back,
                    x4,
                    bw4,
                    w4,
                    (1 + max_rows - n as i32).max(1),
                    if bw4 >= 16 { 4 } else { 2 },
                );
            }
            if have_left && x4 >= back {
                self.scan_col_weighted_compound(
                    &mut stack,
                    &mut cnt,
                    ref0,
                    ref1,
                    x4 - back,
                    y4,
                    bh4,
                    h4,
                    (1 + max_cols - n as i32).max(1),
                    if bh4 >= 16 { 4 } else { 2 },
                );
            }
        }

        // Sort each group (nearest, then secondary) by weight descending -- same rationale as
        // `single_ref_mv_stack`'s doc.
        stack[..cnt].sort_by_key(|c| -c.weight);

        (stack, cnt)
    }
}

/// One candidate in `SpatialRefContext::single_ref_mv_stack`'s real weighted DRL stack.
#[derive(Debug, Clone, Copy, Default)]
pub struct MvStackEntry {
    pub mv: crate::tile::coding_unit::MotionVector,
    weight: i32,
}

/// One candidate in `SpatialRefContext::compound_mv_stack`'s real weighted DRL stack -- same
/// shape as `MvStackEntry`, but a joint L0/L1 pair (real spec's compound candidates are pairs
/// from one unified search, not two independent single-ref ones -- `compound_mv_stack`'s doc).
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct CompoundMvStackEntry {
    pub mv: [crate::tile::coding_unit::MotionVector; 2],
    weight: i32,
}

/// `drl_bit`'s real context (0..=2) for a compound candidate stack -- identical logic to
/// `get_drl_context`, duplicated rather than made generic since it only ever reads `.weight`
/// (confirmed by that function's body) and `CompoundMvStackEntry`/`MvStackEntry` are otherwise
/// unrelated types.
pub fn get_compound_drl_context(stack: &[CompoundMvStackEntry; 8], idx: usize) -> u8 {
    let w0 = stack[idx.min(7)].weight;
    let w1 = stack[(idx + 1).min(7)].weight;
    if w0 >= 640 {
        u8::from(w1 < 640)
    } else if w1 < 640 {
        2
    } else {
        0
    }
}

/// `drl_bit`'s real context (0..=2), spec 7.10.2.10's `DrlCtxStack` comparison -- source: rav1d
/// `get_drl_context` (`memorysafety/rav1d`, BSD-2-Clause, `src/env.rs`). `idx`: the DRL position
/// being decided between (`0` when choosing NEAREST-vs-NEARER, `1` for NEARER-vs-NEAR, `2` for
/// NEAR-vs-NEARISH) -- compares `stack[idx]`'s weight against `stack[idx+1]`'s.
pub fn get_drl_context(stack: &[MvStackEntry; 8], idx: usize) -> u8 {
    let w0 = stack[idx.min(7)].weight;
    let w1 = stack[(idx + 1).min(7)].weight;
    if w0 >= 640 {
        u8::from(w1 < 640)
    } else if w1 < 640 {
        2
    } else {
        0
    }
}

/// Tracks above/left neighbor state for one tile, at 4x4-unit granularity.
///
/// `above_*` arrays span the tile's full width and persist for the whole tile (matches spec: the
/// above-context row is only reset at a new tile, not at every superblock row). `left_*` arrays
/// span the tile's full height and are addressed with the same absolute 4x4 coordinates as the
/// `above_*` ones -- real dav1d instead sizes its left-context column to one superblock and
/// addresses it with row-relative offsets (a memory/cache optimization for a real-time decoder);
/// this crate isn't performance-constrained the same way, so `start_superblock_row` simply clears
/// the whole array at each new superblock row, which is behaviorally equivalent (spec's
/// left-context only ever remembers state from within the current superblock row of a
/// raster-scanned tile) without needing relative-offset bookkeeping at every call site.
pub struct TileContext {
    above_skip: Vec<bool>,
    left_skip: Vec<bool>,
    /// `skip_mode` (spec 5.11.5) above/left context -- real dav1d `t->a->skip_mode[bx4]`/
    /// `t->l.skip_mode[by4]`, same direct-sum-no-have-top/left-branch shape as `above_skip`/
    /// `left_skip` (`skip_mode_context`'s doc).
    above_skip_mode: Vec<bool>,
    left_skip_mode: Vec<bool>,
    /// Raw intra-mode symbol (0..=12) of the last block covering this 4x4 position, defaulting
    /// to `0` (`DC_PRED`) -- matches dav1d's `BlockContext::mode` default/edge behavior (an
    /// unwritten position reads as `DC_PRED`'s context class, per spec `INTRA_MODE_CONTEXT[0]`).
    above_mode: Vec<u8>,
    left_mode: Vec<u8>,
    /// `partition` context bitmask, one byte per 8x8 unit (not 4x4, like every other field here --
    /// see `PARTITION_CTX_TABLE`'s doc). Sized to half the tile's 4x4-unit extent (rounded up).
    above_partition: Vec<u8>,
    left_partition: Vec<u8>,
    /// `ref_frame()` (spec 5.11.25) context state, one entry per 4x4 unit -- mirrors rav1d
    /// `BlockContext`'s `intra`/`comp_type`/`ref[0]`/`ref[1]` fields (`src/env.rs`). `ref0`/`ref1`
    /// use rav1d's 0..=6 encoding (0..=3 = forward LAST/LAST2/LAST3/GOLDEN, 4..=6 = backward
    /// BWDREF/ALTREF2/ALTREF -- one less than `RefFrame`'s own discriminant, which reserves 0 for
    /// `Intra`; see `set_ref_frames`). All four `above_ref_*`/`left_ref_*` reader methods below
    /// take an explicit `have_top`/`have_left` (computed as `y4 > 0`/`x4 > 0` -- correct because,
    /// unlike `skip`/`mode`, several of rav1d's `ref_frame` context formulas (`get_comp_ctx`,
    /// `get_comp_dir_ctx`) return values for the "no neighbor" case that a mere default array
    /// value can't reproduce, and reading `ref0`/`ref1` as an array index in `uni_comp_ref_p1`'s
    /// formula needs a hard read guard, not a hopeful default) rather than the "default array
    /// value" shortcut `skip_context`/`intra_mode_context`/`partition_context` use.
    above_ref_intra: Vec<bool>,
    left_ref_intra: Vec<bool>,
    above_ref_comp: Vec<bool>,
    left_ref_comp: Vec<bool>,
    above_ref0: Vec<i8>,
    left_ref0: Vec<i8>,
    above_ref1: Vec<i8>,
    left_ref1: Vec<i8>,
    /// `comp_type` (spec 5.11.28) above/left context, real dav1d `CompInterType` numeric encoding
    /// (`NONE=0, WEIGHTED_AVG=1, AVG=2, SEG=3, WEDGE=4` -- verified via the `>=` comparisons
    /// `get_mask_comp_ctx`/`get_jnt_comp_ctx` make against `COMP_INTER_AVG`/`COMP_INTER_SEG`, not
    /// assumed) -- feeds `mask_comp_context`/`jnt_comp_context`. Default `0` (NONE), matching
    /// dav1d's own tile-start reset.
    above_comp_type: Vec<u8>,
    left_comp_type: Vec<u8>,
    /// `filter[dir]` (spec 5.11.30) above/left context -- last chosen subpel filter per direction
    /// (`[0]`=horizontal, `[1]`=vertical), sentinel `3` (`DAV1D_N_SWITCHABLE_FILTERS`) = "no
    /// filter recorded here" (matches real dav1d's own sentinel, `get_filter_ctx`'s doc).
    above_filter: [Vec<u8>; 2],
    left_filter: [Vec<u8>; 2],
    /// `inter_mode`/`compound_mode` context -- see `SpatialRefContext`'s doc (full-grid, not
    /// above/left arrays, and never reset per superblock row).
    spatial_ref: SpatialRefContext,
    /// Intra `tx_size()` (spec 5.11.15/16) context state: the resolved transform's size *class*
    /// (0..=4, `TxSize`'s own discriminant order -- matches rav1d's `TxfmInfo.lw`/`.lh` exactly
    /// for a square transform, so no separate log2 conversion is needed). Defaults to `-1`
    /// (`i8`, never a real class) so an unwritten position can never satisfy the context
    /// formula's `>=` comparison -- matches rav1d's own tile-start reset value for this array
    /// (`memorysafety/rav1d`, BSD-2-Clause, `src/decode.rs`'s `tx_intra` reset), unlike
    /// `above_mode`'s "default reads as a real class" shortcut.
    above_tx_class: Vec<i8>,
    left_tx_class: Vec<i8>,
    /// `txb_skip`/`dc_sign` (spec 8.3.2 `get_txb_skip_ctx`/`get_dc_sign_ctx`) context state, one
    /// entry per 4x4 unit -- unpacked equivalent of rav1d's single combined context byte
    /// (`min(cul_level,63) | (dc_sign_category<<6)`, `memorysafety/rav1d`'s C predecessor
    /// `src/recon_tmpl.c`'s `get_skip_ctx`/`get_dc_sign_ctx`/write side around `decode_coefs`).
    /// `cul_level`: `min(63, sum of absolute coefficient levels)` for the last transform block
    /// covering this position, defaulting to `0` (no coefficients seen). `dc_sign_category`:
    /// `0`=negative DC sign, `1`=neutral (no DC coefficient, or an all-zero/`txb_skip` block),
    /// `2`=positive DC sign -- defaults to `1`, matching rav1d's `0x40` tile/row-boundary reset
    /// value (`>> 6 == 1`). Only ever written/read for key-frame, non-IntraBC coding units (see
    /// `parse_coding_unit`) -- inter blocks and IntraBC still use a heuristic `tx_size`
    /// (`TxSize::from_dimensions`), so their transform-block boundaries don't reliably match the
    /// real encoder's; deriving neighbor context from them previously caused real decode
    /// corruption (see `SymbolDecoder::read_residual_block`'s doc), so those CUs keep the older
    /// fixed-context-0 fallback and never touch these arrays.
    above_cul_level: Vec<u8>,
    left_cul_level: Vec<u8>,
    above_dc_sign_category: Vec<u8>,
    left_dc_sign_category: Vec<u8>,
    /// Inter/IntraBC `read_var_tx_size()` (spec 5.11.17/18, `SymbolDecoder::read_txfm_split`)
    /// above/left context: the leaf transform size *class* last written at each position (0..=4,
    /// `TxSize`'s discriminant order). Defaults to `0` (`TxSize::Tx4x4`'s own class, matching
    /// rav1d's `TxfmSize` `#[derive(Default)]` -- its `.tx` array's own tile-start reset value,
    /// `memorysafety/rav1d`'s `env.rs` `BlockContext.tx` field): `read_txfm_split`'s context
    /// formula (`stored < candidate`) only ever evaluates `candidate` at sizes `>4x4` (spec only
    /// reads `txfm_split` when the candidate is bigger than `TX_4X4`), so a `0` default and any
    /// sentinel strictly below every real class are behaviorally identical for every case that
    /// matters here. Distinct from `above_tx_class`/`left_tx_class` (intra `tx_size()`'s own
    /// separate context array -- real rav1d keeps these as two genuinely independent fields,
    /// `tx_intra` vs `tx`, not one shared array). Currently only written by var-tx leaves (inter,
    /// non-IntraBC -- see `parse_coding_unit`'s doc), never by intra `tx_size()`, so an inter CU's
    /// context lookup against an intra above/left neighbor sees the unwritten default rather than
    /// that neighbor's real chosen size -- a known context-derivation approximation (doesn't
    /// affect bit-position sync, only which adaptive CDF entry gets selected), not a bug.
    above_var_tx: Vec<i8>,
    left_var_tx: Vec<i8>,
    /// Chroma-plane (`[0]`=U, `[1]`=V, real separate arrays per `recon_tmpl.c`'s `t->a->ccoef[pl]`/
    /// `t->l.ccoef[pl]`, not shared between planes) counterparts to `above_cul_level`/
    /// `left_cul_level`/`above_dc_sign_category`/`left_dc_sign_category`, indexed at the same
    /// coordinate scale as the luma arrays (a chroma tile's position is tracked as its luma-CU
    /// origin's `x4/2`/`y4/2`, not a truly independent chroma-plane grid -- an approximation
    /// consistent with this crate's other chroma simplifications, harmless here since only
    /// relative above/left adjacency matters, not absolute physical distance). Added to fix a real
    /// desync bug: `read_chroma_residual_block` previously had no `ctx` parameter at all, so every
    /// chroma transform block in a CU (up to 4, for a 128x128 luma block's 2x2-tiled 64x64 chroma
    /// plane) hit the exact same global CDF slot and over-adapted it relative to what a real
    /// encoder (using real per-position context, spec/rav1d `get_skip_ctx`'s chroma branch)
    /// assumes -- see `TileContext::txb_skip_context_chroma`'s doc.
    above_cul_level_chroma: [Vec<u8>; 2],
    left_cul_level_chroma: [Vec<u8>; 2],
    above_dc_sign_category_chroma: [Vec<u8>; 2],
    left_dc_sign_category_chroma: [Vec<u8>; 2],

    /// `seg_pred` above/left context (spec 5.11.9/5.11.10, real dav1d `t->a->seg_pred[bx4]`/
    /// `t->l.seg_pred[by4]`) -- unlike this struct's other above/left arrays, only ever holds a
    /// single bit per position (whether that position's CU used temporal segment-id prediction),
    /// reset per superblock row like the rest.
    above_seg_pred: Vec<bool>,
    left_seg_pred: Vec<bool>,
    /// `segment_id` per-4x4-unit grid across the *whole tile* (not a thin above/left strip like
    /// this struct's other context arrays) -- real dav1d `get_cur_frame_segid` (`src/env.h`) needs
    /// a genuine above-LEFT diagonal lookup (`cur_seg_map[-(stride+1)]`), which a same-column
    /// above-row array can't provide once a same-row neighbor to the left has already overwritten
    /// that column's "above" slot with a same-row value (see `segment_id_context`'s doc for the
    /// full reasoning) -- so this crate mirrors dav1d's own choice of a real per-tile grid instead
    /// of trying to force the above/left-strip pattern to fit. `-1` sentinel (`i16`, not `i8`, so
    /// the 0..=7 real segment id range plus this sentinel never risk conflating with a real value)
    /// marks "not yet decoded" -- matches `AvailU`/`AvailL` being false for that position (real
    /// spec's `have_top`/`have_left`, simplified to tile-local `x4>0`/`y4>0` -- this crate's
    /// established single-tile-only precedent, see `TileContext`'s own doc history).
    seg_id_grid: Vec<i16>,
    seg_id_grid_stride: u32,

    /// Real palette (spec 5.11.46) above/left context state -- `pal_sz` (`[0]`=Y, `[1]`=UV,
    /// shared by U and V since they're always read together) and `pal_colors` (`[0]`=Y, `[1]`=U,
    /// `[2]`=V, up to 8 colors each) mirror dav1d's `t->a->pal_sz`/`t->pal_sz_uv`/`t->al_pal`
    /// (`src/decode.c`/`src/env.h`) index-for-index. **Chroma is indexed at the same LUMA `x4`/
    /// `y4` positions as Y**, not chroma-scaled -- verified against dav1d's own comment at this
    /// exact array's write site ("see aomedia bug 2183 for why we use luma coordinates here"),
    /// not assumed; this is a real, deliberate dav1d/spec choice, not this crate's usual
    /// chroma-scale-approximation pattern (`TileContext`'s chroma residual field doc).
    above_pal_sz: [Vec<u8>; 2],
    left_pal_sz: [Vec<u8>; 2],
    above_pal_colors: [Vec<[u16; 8]>; 3],
    left_pal_colors: [Vec<[u16; 8]>; 3],
}

impl TileContext {
    /// `tile_width_4x4`/`tile_height_4x4`: tile dimensions in 4x4 units.
    pub fn new(tile_width_4x4: u32, tile_height_4x4: u32) -> Self {
        Self {
            above_skip: vec![false; tile_width_4x4.max(1) as usize],
            left_skip: vec![false; tile_height_4x4.max(1) as usize],
            above_skip_mode: vec![false; tile_width_4x4.max(1) as usize],
            left_skip_mode: vec![false; tile_height_4x4.max(1) as usize],
            above_mode: vec![0; tile_width_4x4.max(1) as usize],
            left_mode: vec![0; tile_height_4x4.max(1) as usize],
            above_partition: vec![0; tile_width_4x4.div_ceil(2).max(1) as usize],
            left_partition: vec![0; tile_height_4x4.div_ceil(2).max(1) as usize],
            // Default `true` (intra), not `false` -- an unwritten slot must never look like a
            // real forward-ref (`ref0` default `0` = LAST) neighbor to the count-based context
            // functions. In real causal decode order this default is never actually read where
            // `have_top`/`have_left` is true (see the struct doc), but defaulting to `true` keeps
            // that invariant even under non-causal (e.g. test) access patterns, at zero cost.
            above_ref_intra: vec![true; tile_width_4x4.max(1) as usize],
            left_ref_intra: vec![true; tile_height_4x4.max(1) as usize],
            above_ref_comp: vec![false; tile_width_4x4.max(1) as usize],
            left_ref_comp: vec![false; tile_height_4x4.max(1) as usize],
            above_ref0: vec![0; tile_width_4x4.max(1) as usize],
            left_ref0: vec![0; tile_height_4x4.max(1) as usize],
            above_ref1: vec![0; tile_width_4x4.max(1) as usize],
            left_ref1: vec![0; tile_height_4x4.max(1) as usize],
            above_comp_type: vec![0; tile_width_4x4.max(1) as usize],
            left_comp_type: vec![0; tile_height_4x4.max(1) as usize],
            above_filter: [
                vec![3; tile_width_4x4.max(1) as usize],
                vec![3; tile_width_4x4.max(1) as usize],
            ],
            left_filter: [
                vec![3; tile_height_4x4.max(1) as usize],
                vec![3; tile_height_4x4.max(1) as usize],
            ],
            spatial_ref: SpatialRefContext::new(tile_width_4x4, tile_height_4x4),
            above_tx_class: vec![-1; tile_width_4x4.max(1) as usize],
            left_tx_class: vec![-1; tile_height_4x4.max(1) as usize],
            above_cul_level: vec![0; tile_width_4x4.max(1) as usize],
            left_cul_level: vec![0; tile_height_4x4.max(1) as usize],
            above_dc_sign_category: vec![1; tile_width_4x4.max(1) as usize],
            left_dc_sign_category: vec![1; tile_height_4x4.max(1) as usize],
            above_var_tx: vec![0; tile_width_4x4.max(1) as usize],
            left_var_tx: vec![0; tile_height_4x4.max(1) as usize],
            above_cul_level_chroma: [
                vec![0; tile_width_4x4.max(1) as usize],
                vec![0; tile_width_4x4.max(1) as usize],
            ],
            left_cul_level_chroma: [
                vec![0; tile_height_4x4.max(1) as usize],
                vec![0; tile_height_4x4.max(1) as usize],
            ],
            above_dc_sign_category_chroma: [
                vec![1; tile_width_4x4.max(1) as usize],
                vec![1; tile_width_4x4.max(1) as usize],
            ],
            left_dc_sign_category_chroma: [
                vec![1; tile_height_4x4.max(1) as usize],
                vec![1; tile_height_4x4.max(1) as usize],
            ],
            above_seg_pred: vec![false; tile_width_4x4.max(1) as usize],
            left_seg_pred: vec![false; tile_height_4x4.max(1) as usize],
            seg_id_grid: vec![-1; (tile_width_4x4.max(1) * tile_height_4x4.max(1)) as usize],
            seg_id_grid_stride: tile_width_4x4.max(1),
            above_pal_sz: [
                vec![0; tile_width_4x4.max(1) as usize],
                vec![0; tile_width_4x4.max(1) as usize],
            ],
            left_pal_sz: [
                vec![0; tile_height_4x4.max(1) as usize],
                vec![0; tile_height_4x4.max(1) as usize],
            ],
            above_pal_colors: [
                vec![[0; 8]; tile_width_4x4.max(1) as usize],
                vec![[0; 8]; tile_width_4x4.max(1) as usize],
                vec![[0; 8]; tile_width_4x4.max(1) as usize],
            ],
            left_pal_colors: [
                vec![[0; 8]; tile_height_4x4.max(1) as usize],
                vec![[0; 8]; tile_height_4x4.max(1) as usize],
                vec![[0; 8]; tile_height_4x4.max(1) as usize],
            ],
        }
    }

    /// Reset the left-context arrays at the start of each new superblock row.
    pub fn start_superblock_row(&mut self) {
        self.left_skip.iter_mut().for_each(|v| *v = false);
        self.left_skip_mode.iter_mut().for_each(|v| *v = false);
        self.left_mode.iter_mut().for_each(|v| *v = 0);
        self.left_partition.iter_mut().for_each(|v| *v = 0);
        self.left_ref_intra.iter_mut().for_each(|v| *v = true);
        self.left_ref_comp.iter_mut().for_each(|v| *v = false);
        self.left_ref0.iter_mut().for_each(|v| *v = 0);
        self.left_ref1.iter_mut().for_each(|v| *v = 0);
        self.left_comp_type.iter_mut().for_each(|v| *v = 0);
        for dir in 0..2 {
            self.left_filter[dir].iter_mut().for_each(|v| *v = 3);
        }
        self.left_tx_class.iter_mut().for_each(|v| *v = -1);
        self.left_cul_level.iter_mut().for_each(|v| *v = 0);
        self.left_dc_sign_category.iter_mut().for_each(|v| *v = 1);
        self.left_var_tx.iter_mut().for_each(|v| *v = 0);
        for plane in 0..2 {
            self.left_cul_level_chroma[plane]
                .iter_mut()
                .for_each(|v| *v = 0);
            self.left_dc_sign_category_chroma[plane]
                .iter_mut()
                .for_each(|v| *v = 1);
        }
        self.left_seg_pred.iter_mut().for_each(|v| *v = false);
        // `seg_id_grid` deliberately NOT reset here -- it's a real per-tile grid (this field's
        // doc), not an above/left strip; positions above the current row must stay readable for
        // `segment_id_context`'s above/above-left lookups.
        for plane in 0..2 {
            self.left_pal_sz[plane].iter_mut().for_each(|v| *v = 0);
        }
        for plane in 0..3 {
            self.left_pal_colors[plane]
                .iter_mut()
                .for_each(|v| *v = [0; 8]);
        }
    }

    /// `partition` context index (0..=3) for a block at absolute 8x8-unit position `(x8, y8)`,
    /// per spec/rav1d: `has_bl(above_partition[x8]) + 2*has_bl(left_partition[y8])`, where
    /// `has_bl` extracts the bit for the current level `bl` (0=128x128..4=8x8, see
    /// `partition_bl`) from the neighbor's stored bitmask.
    pub fn partition_context(&self, x8: u32, y8: u32, bl: u8) -> u8 {
        let has_bl = |v: u8| (v >> (4 - bl)) & 1;
        let above = self.above_partition.get(x8 as usize).copied().unwrap_or(0);
        let left = self.left_partition.get(y8 as usize).copied().unwrap_or(0);
        has_bl(above) + 2 * has_bl(left)
    }

    /// Record a decoded `partition` symbol across the block's 8x8-unit footprint (`hsz8` units
    /// wide/tall -- `2^(block_size_log2-3)`), for future context lookups. Per spec/rav1d, the
    /// written value isn't the raw symbol but `PARTITION_CTX_TABLE[dir][bl][partition_symbol]` --
    /// see that table's doc for why this is a lookup, not the symbol value itself.
    pub fn set_partition(&mut self, x8: u32, y8: u32, hsz8: u32, bl: u8, partition_symbol: u8) {
        let above_val = PARTITION_CTX_TABLE[0][bl as usize][partition_symbol as usize];
        let left_val = PARTITION_CTX_TABLE[1][bl as usize][partition_symbol as usize];
        let x_end = (x8 + hsz8).min(self.above_partition.len() as u32);
        for x in x8..x_end {
            self.above_partition[x as usize] = above_val;
        }
        let y_end = (y8 + hsz8).min(self.left_partition.len() as u32);
        for y in y8..y_end {
            self.left_partition[y as usize] = left_val;
        }
    }

    /// Key-frame `intra_mode` context: `(above_mode_class, left_mode_class)`, each 0..=4, for a
    /// block at absolute 4x4 position `(x4, y4)` -- per spec/dav1d:
    /// `INTRA_MODE_CONTEXT[above_mode[x4]]`, `INTRA_MODE_CONTEXT[left_mode[y4]]`.
    pub fn intra_mode_context(&self, x4: u32, y4: u32) -> (u8, u8) {
        let above_raw = self.above_mode.get(x4 as usize).copied().unwrap_or(0);
        let left_raw = self.left_mode.get(y4 as usize).copied().unwrap_or(0);
        (
            INTRA_MODE_CONTEXT[above_raw as usize],
            INTRA_MODE_CONTEXT[left_raw as usize],
        )
    }

    /// Record a decoded intra-mode symbol across the block's 4x4-unit footprint, for future
    /// context lookups.
    pub fn set_mode(&mut self, x4: u32, y4: u32, width_4x4: u32, height_4x4: u32, mode_symbol: u8) {
        let x_end = (x4 + width_4x4).min(self.above_mode.len() as u32);
        for x in x4..x_end {
            self.above_mode[x as usize] = mode_symbol;
        }
        let y_end = (y4 + height_4x4).min(self.left_mode.len() as u32);
        for y in y4..y_end {
            self.left_mode[y as usize] = mode_symbol;
        }
    }

    /// Intra `tx_size()` context (0..=2) for a block at absolute 4x4 position `(x4, y4)` with
    /// max transform class `max_tx_class` (0..=4, `TxSize`'s discriminant order) -- per
    /// spec/rav1d `get_tx_ctx`: `(left_tx_class[y4] >= max_tx_class) + (above_tx_class[x4] >=
    /// max_tx_class)`. An unwritten neighbor (`-1`, see the struct field doc) never satisfies
    /// `>=` for any real class, so it correctly contributes `0` without needing an explicit
    /// `have_top`/`have_left` check (same "default array value" pattern as `skip_context`).
    pub fn tx_size_context(&self, x4: u32, y4: u32, max_tx_class: u8) -> u8 {
        let above = self.above_tx_class.get(x4 as usize).copied().unwrap_or(-1);
        let left = self.left_tx_class.get(y4 as usize).copied().unwrap_or(-1);
        u8::from(left >= max_tx_class as i8) + u8::from(above >= max_tx_class as i8)
    }

    /// Record a decoded (or table-derived, for non-`Switchable` `TxMode`s) transform's size
    /// class across the block's 4x4-unit footprint, for future `tx_size_context` lookups.
    pub fn set_tx_class(
        &mut self,
        x4: u32,
        y4: u32,
        width_4x4: u32,
        height_4x4: u32,
        tx_class: u8,
    ) {
        let tx_class = tx_class as i8;
        let x_end = (x4 + width_4x4).min(self.above_tx_class.len() as u32);
        for x in x4..x_end {
            self.above_tx_class[x as usize] = tx_class;
        }
        let y_end = (y4 + height_4x4).min(self.left_tx_class.len() as u32);
        for y in y4..y_end {
            self.left_tx_class[y as usize] = tx_class;
        }
    }

    /// `read_txfm_split` context `(a, l)` pair (each `0` or `1`) at absolute 4x4 position
    /// `(x4, y4)` for a candidate split of size `candidate_width_class`/`candidate_height_class`
    /// (each 0..=4, `tx_size_class` of the candidate's width/height in pixels) -- per spec/rav1d
    /// `read_tx_tree` (`src/decode.c`): `a = above_var_tx[x4] < candidate_width_class`, `l =
    /// left_var_tx[y4] < candidate_height_class` -- **width for above, height for left**, verified
    /// against the real C directly (`t->a->tx[bx4] < txw` / `t->l.tx[by4] < txh`), not assumed.
    /// For a square candidate these two class values are equal, matching this function's pre-rect
    /// single-`candidate_class` behavior exactly. Caller sums `a + l` for the CDF's `0..=2`
    /// context index (see `above_var_tx`'s doc for why the `0` default is safe).
    pub fn var_tx_context(
        &self,
        x4: u32,
        y4: u32,
        candidate_width_class: u8,
        candidate_height_class: u8,
    ) -> (u8, u8) {
        let above = self.above_var_tx.get(x4 as usize).copied().unwrap_or(0);
        let left = self.left_var_tx.get(y4 as usize).copied().unwrap_or(0);
        (
            u8::from(above < candidate_width_class as i8),
            u8::from(left < candidate_height_class as i8),
        )
    }

    /// Record a var-tx leaf's size across its own 4x4-unit footprint, for future
    /// `var_tx_context` lookups -- see `above_var_tx`'s doc. `width_class`/`height_class`: real
    /// dav1d stores the leaf's own width-derived class into the above-context array and its own
    /// height-derived class into the left-context array *separately* (not one shared class, see
    /// `var_tx_context`'s doc) -- for a square leaf these are equal, matching pre-rect behavior.
    pub fn set_var_tx_class(
        &mut self,
        x4: u32,
        y4: u32,
        width_4x4: u32,
        height_4x4: u32,
        width_class: u8,
        height_class: u8,
    ) {
        let width_class = width_class as i8;
        let height_class = height_class as i8;
        let x_end = (x4 + width_4x4).min(self.above_var_tx.len() as u32);
        for x in x4..x_end {
            self.above_var_tx[x as usize] = width_class;
        }
        let y_end = (y4 + height_4x4).min(self.left_var_tx.len() as u32);
        for y in y4..y_end {
            self.left_var_tx[y as usize] = height_class;
        }
    }

    /// `seg_pred` context index (0..=2) at absolute 4x4 position `(x4, y4)` -- per spec/rav1d:
    /// `above_seg_pred[x4] as u8 + left_seg_pred[y4] as u8`.
    pub fn seg_pred_context(&self, x4: u32, y4: u32) -> u8 {
        let above = self
            .above_seg_pred
            .get(x4 as usize)
            .copied()
            .unwrap_or(false);
        let left = self
            .left_seg_pred
            .get(y4 as usize)
            .copied()
            .unwrap_or(false);
        u8::from(above) + u8::from(left)
    }

    /// Record a CU's `seg_pred` flag across its 4x4-unit footprint, for future `seg_pred_context`
    /// lookups.
    pub fn set_seg_pred(&mut self, x4: u32, y4: u32, width_4x4: u32, height_4x4: u32, val: bool) {
        let x_end = (x4 + width_4x4).min(self.above_seg_pred.len() as u32);
        for x in x4..x_end {
            self.above_seg_pred[x as usize] = val;
        }
        let y_end = (y4 + height_4x4).min(self.left_seg_pred.len() as u32);
        for y in y4..y_end {
            self.left_seg_pred[y as usize] = val;
        }
    }

    /// Read one cell of the `seg_id_grid` (`-1` when out of bounds or not yet decoded).
    fn seg_id_grid_get(&self, x4: u32, y4: u32) -> i16 {
        let idx = y4 as usize * self.seg_id_grid_stride as usize + x4 as usize;
        self.seg_id_grid.get(idx).copied().unwrap_or(-1)
    }

    /// Real `segment_id` context + prediction (spec 5.11.9/5.11.10, real dav1d
    /// `get_cur_frame_segid`, `src/env.h` -- ported index-for-index, not approximated): looks up
    /// the above (`a`), left (`l`), and above-left (`al`) cells directly from `seg_id_grid`
    /// (`AvailU`/`AvailL` simplified to tile-local `y4>0`/`x4>0`, this crate's established
    /// single-tile precedent). Returns `(ctx, pred)`: `ctx` 0..=2 (`2` if all three neighbors
    /// agree, `1` if any two agree, `0` otherwise or if a neighbor is unavailable), `pred` is
    /// `a` when `a == al`, else `l` -- when only one of `AvailU`/`AvailL` holds, `ctx = 0` and
    /// `pred` is whichever single neighbor is available (`0` if neither).
    pub fn segment_id_context(&self, x4: u32, y4: u32) -> (u8, u8) {
        let have_top = y4 > 0;
        let have_left = x4 > 0;
        if have_top && have_left {
            let l = self.seg_id_grid_get(x4 - 1, y4);
            let a = self.seg_id_grid_get(x4, y4 - 1);
            let al = self.seg_id_grid_get(x4 - 1, y4 - 1);
            let ctx = if l == a && al == l {
                2
            } else if l == a || al == l || a == al {
                1
            } else {
                0
            };
            let pred = if a == al { a } else { l };
            (ctx, pred.max(0) as u8)
        } else {
            let pred = if have_left {
                self.seg_id_grid_get(x4 - 1, y4)
            } else if have_top {
                self.seg_id_grid_get(x4, y4 - 1)
            } else {
                0
            };
            (0, pred.max(0) as u8)
        }
    }

    /// Record a CU's real decoded `segment_id` across its 4x4-unit footprint, for future
    /// `segment_id_context` lookups (and any later frame's temporal prediction, when that lands --
    /// see `SegmentationInfo`'s doc for the cross-frame gap this doesn't yet close).
    pub fn set_segment_id(
        &mut self,
        x4: u32,
        y4: u32,
        width_4x4: u32,
        height_4x4: u32,
        seg_id: u8,
    ) {
        let stride = self.seg_id_grid_stride;
        let x_end = (x4 + width_4x4).min(stride);
        let y_end = y4 + height_4x4;
        for y in y4..y_end {
            let row_start = y as usize * stride as usize;
            if row_start >= self.seg_id_grid.len() {
                break;
            }
            for x in x4..x_end {
                if let Some(cell) = self.seg_id_grid.get_mut(row_start + x as usize) {
                    *cell = seg_id as i16;
                }
            }
        }
    }

    /// `has_palette_y` context (spec 5.11.46, real dav1d `t->a->pal_sz[bx4] > 0` / `t->l.pal_sz[
    /// by4] > 0`, gated by real `AvailU`/`AvailL` -- this crate's established tile-local
    /// `y4>0`/`x4>0` simplification, same as `segment_id_context`'s).
    pub fn has_palette_y_context(&self, x4: u32, y4: u32) -> u8 {
        let mut ctx = 0u8;
        if y4 > 0 && self.above_pal_sz[0].get(x4 as usize).copied().unwrap_or(0) > 0 {
            ctx += 1;
        }
        if x4 > 0 && self.left_pal_sz[0].get(y4 as usize).copied().unwrap_or(0) > 0 {
            ctx += 1;
        }
        ctx
    }

    /// Real stored above palette state for one color plane's cache derivation
    /// (`SymbolDecoder::read_palette_colors`'s doc) -- `color_plane` 0=Y/1=U/2=V, `size_plane`
    /// 0=Y/1=UV (U and V always share the same size). Returns `(colors, count)`.
    pub fn pal_above(&self, color_plane: usize, size_plane: usize, x4: u32) -> ([u16; 8], u8) {
        let count = self.above_pal_sz[size_plane.min(1)]
            .get(x4 as usize)
            .copied()
            .unwrap_or(0);
        let colors = self.above_pal_colors[color_plane.min(2)]
            .get(x4 as usize)
            .copied()
            .unwrap_or([0; 8]);
        (colors, count)
    }

    /// Real stored left palette state -- see `pal_above`'s doc.
    pub fn pal_left(&self, color_plane: usize, size_plane: usize, y4: u32) -> ([u16; 8], u8) {
        let count = self.left_pal_sz[size_plane.min(1)]
            .get(y4 as usize)
            .copied()
            .unwrap_or(0);
        let colors = self.left_pal_colors[color_plane.min(2)]
            .get(y4 as usize)
            .copied()
            .unwrap_or([0; 8]);
        (colors, count)
    }

    /// Record a CU's real palette size across its 4x4-unit footprint (`size_plane` 0=Y/1=UV, real
    /// dav1d's `al_pal`/`pal_sz` write always uses the CU's *luma* footprint even for chroma --
    /// `above_pal_sz`'s doc), for future `has_palette_y_context`/`pal_above`/`pal_left` lookups.
    pub fn set_pal_size(
        &mut self,
        size_plane: usize,
        x4: u32,
        y4: u32,
        width_4x4: u32,
        height_4x4: u32,
        sz: u8,
    ) {
        let size_plane = size_plane.min(1);
        let x_end = (x4 + width_4x4).min(self.above_pal_sz[size_plane].len() as u32);
        for x in x4..x_end {
            self.above_pal_sz[size_plane][x as usize] = sz;
        }
        let y_end = (y4 + height_4x4).min(self.left_pal_sz[size_plane].len() as u32);
        for y in y4..y_end {
            self.left_pal_sz[size_plane][y as usize] = sz;
        }
    }

    /// Record a CU's real decoded palette colors across its 4x4-unit (luma) footprint --
    /// `color_plane` 0=Y/1=U/2=V, see `set_pal_size`'s doc.
    pub fn set_pal_colors(
        &mut self,
        color_plane: usize,
        x4: u32,
        y4: u32,
        width_4x4: u32,
        height_4x4: u32,
        colors: [u16; 8],
    ) {
        let color_plane = color_plane.min(2);
        let x_end = (x4 + width_4x4).min(self.above_pal_colors[color_plane].len() as u32);
        for x in x4..x_end {
            self.above_pal_colors[color_plane][x as usize] = colors;
        }
        let y_end = (y4 + height_4x4).min(self.left_pal_colors[color_plane].len() as u32);
        for y in y4..y_end {
            self.left_pal_colors[color_plane][y as usize] = colors;
        }
    }

    /// `txb_skip` (all_zero) context index for one transform block at absolute 4x4 position
    /// `(x4, y4)`, `tx_w4`/`tx_h4` 4x4 units wide/tall -- per spec/rav1d `get_skip_ctx`'s luma
    /// branch. `is_single_tx_block`: true when this transform block is the coding block's only
    /// one (`tx_cols == 1 && tx_rows == 1`), matching rav1d's `b_dim[2] == t_dim->lw && b_dim[3]
    /// == t_dim->lh` immediate-zero case -- no neighbor lookup needed there, context is always
    /// `0`. Otherwise: OR-reduce `cul_level` across the `tx_w4` above-row / `tx_h4` left-column
    /// positions this transform block's row/column spans (a real bitwise OR of the raw values,
    /// not a boolean "any nonzero" -- see this method's own struct-field doc), then index
    /// `DAV1D_SKIP_CTX[min(la,4)][min(ll,4)]`.
    pub fn txb_skip_context(
        &self,
        x4: u32,
        y4: u32,
        tx_w4: u32,
        tx_h4: u32,
        is_single_tx_block: bool,
    ) -> u8 {
        if is_single_tx_block {
            return 0;
        }
        let la = (0..tx_w4)
            .map(|i| {
                self.above_cul_level
                    .get((x4 + i) as usize)
                    .copied()
                    .unwrap_or(0)
            })
            .fold(0u32, |acc, v| acc | v as u32);
        let ll = (0..tx_h4)
            .map(|i| {
                self.left_cul_level
                    .get((y4 + i) as usize)
                    .copied()
                    .unwrap_or(0)
            })
            .fold(0u32, |acc, v| acc | v as u32);
        DAV1D_SKIP_CTX[la.min(4) as usize][ll.min(4) as usize]
    }

    /// `dc_sign` context index (0..=2) for one transform block at absolute 4x4 position
    /// `(x4, y4)`, `tx_w4`/`tx_h4` 4x4 units wide/tall -- per spec/rav1d `get_dc_sign_ctx`'s luma
    /// branch: sum `(dc_sign_category - 1)` across the above-row and left-column positions this
    /// transform block spans (a real per-position sum, not an OR -- neutral positions contribute
    /// `0`, negative `-1`, positive `+1`), then classify the sign of that sum into `{0,1,2}`.
    pub fn dc_sign_context(&self, x4: u32, y4: u32, tx_w4: u32, tx_h4: u32) -> u8 {
        let above_sum: i32 = (0..tx_w4)
            .map(|i| {
                self.above_dc_sign_category
                    .get((x4 + i) as usize)
                    .copied()
                    .unwrap_or(1) as i32
                    - 1
            })
            .sum();
        let left_sum: i32 = (0..tx_h4)
            .map(|i| {
                self.left_dc_sign_category
                    .get((y4 + i) as usize)
                    .copied()
                    .unwrap_or(1) as i32
                    - 1
            })
            .sum();
        let s = above_sum + left_sum;
        if s < 0 {
            0
        } else if s == 0 {
            1
        } else {
            2
        }
    }

    /// Record one decoded transform block's `cul_level`/`dc_sign` state across its 4x4-unit
    /// footprint, for future `txb_skip_context`/`dc_sign_context` lookups. `cul_level`: `min(63,
    /// sum of absolute coefficient levels)` (already clamped by the caller). `dc_sign_symbol`:
    /// the raw `dc_sign` bit read for this block's DC coefficient (`Some(1)`=negative,
    /// `Some(0)`=positive), or `None` when no `dc_sign` bit was read at all -- an all-zero
    /// (`txb_skip`) block, or a block whose DC position happened to decode to a zero level --
    /// both cases mean "neutral" (category `1`), matching rav1d's `all_skip`/`dc_tok==0` paths
    /// (see the struct field's doc).
    pub fn set_residual_ctx(
        &mut self,
        x4: u32,
        y4: u32,
        tx_w4: u32,
        tx_h4: u32,
        cul_level: u8,
        dc_sign_symbol: Option<u8>,
    ) {
        let category = match dc_sign_symbol {
            None => 1,
            Some(1) => 0,
            Some(_) => 2,
        };
        let x_end = (x4 + tx_w4).min(self.above_cul_level.len() as u32);
        for x in x4..x_end {
            self.above_cul_level[x as usize] = cul_level;
            self.above_dc_sign_category[x as usize] = category;
        }
        let y_end = (y4 + tx_h4).min(self.left_cul_level.len() as u32);
        for y in y4..y_end {
            self.left_cul_level[y as usize] = cul_level;
            self.left_dc_sign_category[y as usize] = category;
        }
    }

    /// Chroma `txb_skip` context index -- local remap of spec/rav1d's real chroma branch of
    /// `get_skip_ctx` (`recon_tmpl.c:68-100`, `memorysafety/rav1d`/`videolan/dav1d`,
    /// BSD-2-Clause): real ctx there is `7 + not_one_blk*3 + ca + cl` (13-context table shared
    /// with luma's 7); since this crate's chroma CDF table is a separate `[tx_size_class][0..=5]`
    /// array (no shared luma/chroma axis), the `+7` is dropped and `ca`/`cl` are computed as a
    /// plain boolean OR across the tx block's above/left footprint (`cul_level != 0` = "neighbor
    /// wasn't all-zero") rather than dav1d's bit-packed `0x40`-sentinel trick -- same result,
    /// simpler representation. `not_one_blk`: true when this CU's chroma plane needed more than
    /// one chroma transform block (`chroma_tile_count > 1` at the call site) -- spec: whether the
    /// chroma prediction block exceeds one max-chroma-tx-size tile.
    pub fn txb_skip_context_chroma(
        &self,
        plane: usize,
        cx4: u32,
        cy4: u32,
        tx_w4: u32,
        tx_h4: u32,
        not_one_blk: bool,
    ) -> u8 {
        let above = &self.above_cul_level_chroma[plane.min(1)];
        let left = &self.left_cul_level_chroma[plane.min(1)];
        let ca = (0..tx_w4).any(|i| above.get((cx4 + i) as usize).copied().unwrap_or(0) != 0);
        let cl = (0..tx_h4).any(|i| left.get((cy4 + i) as usize).copied().unwrap_or(0) != 0);
        u8::from(not_one_blk) * 3 + u8::from(ca) + u8::from(cl)
    }

    /// Chroma `dc_sign` context -- identical algorithm to `dc_sign_context` (spec/rav1d's
    /// `get_dc_sign_ctx` doesn't differ between luma/chroma except which above/left array it's
    /// given), applied to `plane`'s own category arrays.
    pub fn dc_sign_context_chroma(
        &self,
        plane: usize,
        cx4: u32,
        cy4: u32,
        tx_w4: u32,
        tx_h4: u32,
    ) -> u8 {
        let above = &self.above_dc_sign_category_chroma[plane.min(1)];
        let left = &self.left_dc_sign_category_chroma[plane.min(1)];
        let above_sum: i32 = (0..tx_w4)
            .map(|i| above.get((cx4 + i) as usize).copied().unwrap_or(1) as i32 - 1)
            .sum();
        let left_sum: i32 = (0..tx_h4)
            .map(|i| left.get((cy4 + i) as usize).copied().unwrap_or(1) as i32 - 1)
            .sum();
        let s = above_sum + left_sum;
        if s < 0 {
            0
        } else if s == 0 {
            1
        } else {
            2
        }
    }

    /// Chroma counterpart to `set_residual_ctx`, per plane (`0`=U, `1`=V).
    #[allow(clippy::too_many_arguments)]
    pub fn set_residual_ctx_chroma(
        &mut self,
        plane: usize,
        cx4: u32,
        cy4: u32,
        tx_w4: u32,
        tx_h4: u32,
        cul_level: u8,
        dc_sign_symbol: Option<u8>,
    ) {
        let category = match dc_sign_symbol {
            None => 1,
            Some(1) => 0,
            Some(_) => 2,
        };
        let plane = plane.min(1);
        let above = &mut self.above_cul_level_chroma[plane];
        let above_cat = &mut self.above_dc_sign_category_chroma[plane];
        let x_end = (cx4 + tx_w4).min(above.len() as u32);
        for x in cx4..x_end {
            above[x as usize] = cul_level;
            above_cat[x as usize] = category;
        }
        let left = &mut self.left_cul_level_chroma[plane];
        let left_cat = &mut self.left_dc_sign_category_chroma[plane];
        let y_end = (cy4 + tx_h4).min(left.len() as u32);
        for y in cy4..y_end {
            left[y as usize] = cul_level;
            left_cat[y as usize] = category;
        }
    }

    /// `skip` context index (0..=2) for a block at absolute 4x4 position `(x4, y4)` -- per
    /// spec/dav1d: `above_skip[x4] as u8 + left_skip[y4] as u8`.
    pub fn skip_context(&self, x4: u32, y4: u32) -> u8 {
        let above = self.above_skip.get(x4 as usize).copied().unwrap_or(false);
        let left = self.left_skip.get(y4 as usize).copied().unwrap_or(false);
        u8::from(above) + u8::from(left)
    }

    /// Record a decoded `skip` flag across the block's 4x4-unit footprint, for future context
    /// lookups -- per spec/dav1d, every 4x4 unit the block covers gets the same value in both the
    /// above-row and left-column arrays (`CaseSet::set_disjoint`/`case.set_disjoint` in rav1d).
    pub fn set_skip(&mut self, x4: u32, y4: u32, width_4x4: u32, height_4x4: u32, skip: bool) {
        let x_end = (x4 + width_4x4).min(self.above_skip.len() as u32);
        for x in x4..x_end {
            self.above_skip[x as usize] = skip;
        }
        let y_end = (y4 + height_4x4).min(self.left_skip.len() as u32);
        for y in y4..y_end {
            self.left_skip[y as usize] = skip;
        }
    }

    /// `skip_mode` context index (0..=2) -- identical shape to `skip_context`, per dav1d
    /// `smctx = t->a->skip_mode[bx4] + t->l.skip_mode[by4]` (no `have_top`/`have_left` branch,
    /// unlike `intra_ctx`).
    pub fn skip_mode_context(&self, x4: u32, y4: u32) -> u8 {
        let above = self
            .above_skip_mode
            .get(x4 as usize)
            .copied()
            .unwrap_or(false);
        let left = self
            .left_skip_mode
            .get(y4 as usize)
            .copied()
            .unwrap_or(false);
        u8::from(above) + u8::from(left)
    }

    /// Record a decoded `skip_mode` flag across the block's 4x4-unit footprint -- mirrors
    /// `set_skip` exactly, separate array (`skip_mode_context`'s doc).
    pub fn set_skip_mode(
        &mut self,
        x4: u32,
        y4: u32,
        width_4x4: u32,
        height_4x4: u32,
        skip_mode: bool,
    ) {
        let x_end = (x4 + width_4x4).min(self.above_skip_mode.len() as u32);
        for x in x4..x_end {
            self.above_skip_mode[x as usize] = skip_mode;
        }
        let y_end = (y4 + height_4x4).min(self.left_skip_mode.len() as u32);
        for y in y4..y_end {
            self.left_skip_mode[y as usize] = skip_mode;
        }
    }

    /// `is_inter` context index (0..=3, real spec's `IsInterCtx`) -- whether the above/left
    /// neighbors were themselves intra-coded, source: rav1d `get_intra_ctx` (`memorysafety/rav1d`,
    /// BSD-2-Clause, `src/env.rs`). Reuses `above_ref_intra`/`left_ref_intra` (the same array
    /// `ref_frame()`'s own context functions read -- real dav1d's `BlockContext.intra` is one
    /// shared field serving both purposes, see that field's doc) rather than a dedicated array.
    /// Unlike `skip_context`/`skip_mode_context`, this real spec formula explicitly branches on
    /// `have_top`/`have_left` instead of a bare sum -- `have_left && have_top` folds `ctx==2` to
    /// `3` (skipping `2`, which is otherwise reachable from the other branches), so a plain
    /// `left+above` (that a "default array value" shortcut would produce for an unwritten neighbor)
    /// is NOT behaviorally equivalent here.
    pub fn intra_ctx(&self, x4: u32, y4: u32) -> u8 {
        let have_left = x4 > 0;
        let have_top = y4 > 0;
        let above = u8::from(
            self.above_ref_intra
                .get(x4 as usize)
                .copied()
                .unwrap_or(true),
        );
        let left = u8::from(
            self.left_ref_intra
                .get(y4 as usize)
                .copied()
                .unwrap_or(true),
        );
        if have_left {
            if have_top {
                let ctx = left + above;
                ctx + u8::from(ctx == 2)
            } else {
                left * 2
            }
        } else if have_top {
            above * 2
        } else {
            0
        }
    }

    /// Single-position `intra` reads (`above_ref_intra[x4]`/`left_ref_intra[y4]`) -- used by
    /// `crate::tile::coding_unit::has_overlappable_neighbors`'s real above/left "any inter
    /// neighbor" edge scan (spec 5.11.27's `findoddzero` over `t->a->intra`/`t->l.intra`).
    pub fn above_is_intra(&self, x4: u32) -> bool {
        self.above_ref_intra
            .get(x4 as usize)
            .copied()
            .unwrap_or(true)
    }
    pub fn left_is_intra(&self, y4: u32) -> bool {
        self.left_ref_intra
            .get(y4 as usize)
            .copied()
            .unwrap_or(true)
    }

    /// Record whether a block was intra-coded, for future `intra_ctx` lookups -- only touches
    /// `above_ref_intra`/`left_ref_intra` (NOT `above_ref_comp`/`above_ref0`/`above_ref1`, unlike
    /// `set_ref_frames`): real dav1d's entropy-pass context-set macro for a genuine intra CU
    /// within an inter frame writes only `edge->intra`/`edge->skip_mode` (`src/decode.c`,
    /// `rep_macro(edge->intra, off, 1)` -- no `ref`/`comp_type` write at all, since an intra
    /// block has no reference-frame data to record), so mirroring `set_ref_frames`'s full write
    /// here would touch fields real spec never updates for this case. The inter path keeps using
    /// `set_ref_frames(..., is_intra: false, ...)` for its own (correct, existing) write of all
    /// four fields together.
    pub fn set_intra_flag(
        &mut self,
        x4: u32,
        y4: u32,
        width_4x4: u32,
        height_4x4: u32,
        is_intra: bool,
    ) {
        let x_end = (x4 + width_4x4).min(self.above_ref_intra.len() as u32);
        for x in x4..x_end {
            self.above_ref_intra[x as usize] = is_intra;
        }
        let y_end = (y4 + height_4x4).min(self.left_ref_intra.len() as u32);
        for y in y4..y_end {
            self.left_ref_intra[y as usize] = is_intra;
        }
    }

    /// Record a decoded `ref_frame()` result across the block's 4x4-unit footprint, for future
    /// `ref_frame` context lookups. `ref0`/`ref1` use rav1d's 0..=6 encoding (see the struct
    /// fields' doc) -- pass `ref1 = -1` for single-reference blocks (`comp = false`).
    #[allow(clippy::too_many_arguments)]
    pub fn set_ref_frames(
        &mut self,
        x4: u32,
        y4: u32,
        width_4x4: u32,
        height_4x4: u32,
        is_intra: bool,
        comp: bool,
        ref0: i8,
        ref1: i8,
    ) {
        let x_end = (x4 + width_4x4).min(self.above_ref_intra.len() as u32);
        for x in x4..x_end {
            self.above_ref_intra[x as usize] = is_intra;
            self.above_ref_comp[x as usize] = comp;
            self.above_ref0[x as usize] = ref0;
            self.above_ref1[x as usize] = ref1;
        }
        let y_end = (y4 + height_4x4).min(self.left_ref_intra.len() as u32);
        for y in y4..y_end {
            self.left_ref_intra[y as usize] = is_intra;
            self.left_ref_comp[y as usize] = comp;
            self.left_ref0[y as usize] = ref0;
            self.left_ref1[y as usize] = ref1;
        }
    }

    /// Record a decoded `comp_type` (spec 5.11.28) across the block's 4x4-unit footprint --
    /// `comp_type`'s doc for the real dav1d numeric encoding this expects.
    pub fn set_comp_type(
        &mut self,
        x4: u32,
        y4: u32,
        width_4x4: u32,
        height_4x4: u32,
        comp_type: u8,
    ) {
        let x_end = (x4 + width_4x4).min(self.above_comp_type.len() as u32);
        for x in x4..x_end {
            self.above_comp_type[x as usize] = comp_type;
        }
        let y_end = (y4 + height_4x4).min(self.left_comp_type.len() as u32);
        for y in y4..y_end {
            self.left_comp_type[y as usize] = comp_type;
        }
    }

    /// `mask_comp` context (0..=5) -- is-this-compound-masked (seg/wedge) likelihood, source:
    /// rav1d `get_mask_comp_ctx` (`memorysafety/rav1d`, BSD-2-Clause, `src/env.rs`).
    pub fn mask_comp_context(&self, x4: u32, y4: u32) -> u8 {
        let a_comp_type = self.above_comp_type.get(x4 as usize).copied().unwrap_or(0);
        let l_comp_type = self.left_comp_type.get(y4 as usize).copied().unwrap_or(0);
        let a_ref0 = self.above_ref0.get(x4 as usize).copied().unwrap_or(0);
        let l_ref0 = self.left_ref0.get(y4 as usize).copied().unwrap_or(0);
        let a_ctx = if a_comp_type >= 3 {
            1
        } else if a_ref0 == 6 {
            3
        } else {
            0
        };
        let l_ctx = if l_comp_type >= 3 {
            1
        } else if l_ref0 == 6 {
            3
        } else {
            0
        };
        (a_ctx + l_ctx).min(5)
    }

    /// `jnt_comp` context (0..=5) -- source: rav1d `get_jnt_comp_ctx`. Real spec also factors a
    /// POC-distance-derived `offset` term (`d0 == d1`, comparing the current frame's and both
    /// references' display-order distance) this crate approximates as always `0` (`offset` term
    /// omitted) -- this crate doesn't track cross-frame `OrderHint`/reference-frame POC state (no
    /// real DPB), same scope limit as `crate::frame_header_full::SegmentationInfo`'s missing
    /// per-segment feature data. A real, documented approximation, not a bug fix candidate without
    /// building that state first.
    pub fn jnt_comp_context(&self, x4: u32, y4: u32) -> u8 {
        let a_comp_type = self.above_comp_type.get(x4 as usize).copied().unwrap_or(0);
        let l_comp_type = self.left_comp_type.get(y4 as usize).copied().unwrap_or(0);
        let a_ref0 = self.above_ref0.get(x4 as usize).copied().unwrap_or(0);
        let l_ref0 = self.left_ref0.get(y4 as usize).copied().unwrap_or(0);
        let a_ctx = u8::from(a_comp_type >= 2 || a_ref0 == 6);
        let l_ctx = u8::from(l_comp_type >= 2 || l_ref0 == 6);
        a_ctx + l_ctx
    }

    /// Record a decoded subpel `filter` across the block's 4x4-unit footprint, for the given
    /// direction (`0`=horizontal, `1`=vertical).
    pub fn set_filter(
        &mut self,
        x4: u32,
        y4: u32,
        width_4x4: u32,
        height_4x4: u32,
        dir: usize,
        filter: u8,
    ) {
        let dir = dir.min(1);
        let x_end = (x4 + width_4x4).min(self.above_filter[dir].len() as u32);
        for x in x4..x_end {
            self.above_filter[dir][x as usize] = filter;
        }
        let y_end = (y4 + height_4x4).min(self.left_filter[dir].len() as u32);
        for y in y4..y_end {
            self.left_filter[dir][y as usize] = filter;
        }
    }

    /// `filter` context (0..=7) -- source: rav1d `get_filter_ctx`. `comp`: whether this block is
    /// compound. `dir`: `0`=horizontal, `1`=vertical. `ref0`: this block's own first reference
    /// (rav1d 0..=6 encoding) -- a neighbor's recorded filter only counts if that neighbor
    /// actually used this same reference (`sentinel 3` otherwise, matching real dav1d).
    pub fn filter_context(&self, x4: u32, y4: u32, comp: bool, dir: usize, ref0: i8) -> u8 {
        let dir = dir.min(1);
        let xi = x4 as usize;
        let yi = y4 as usize;
        let a_matches = self.above_ref0.get(xi).copied().unwrap_or(-1) == ref0
            || self.above_ref1.get(xi).copied().unwrap_or(-1) == ref0;
        let l_matches = self.left_ref0.get(yi).copied().unwrap_or(-1) == ref0
            || self.left_ref1.get(yi).copied().unwrap_or(-1) == ref0;
        let a_filter = if a_matches {
            self.above_filter[dir].get(xi).copied().unwrap_or(3)
        } else {
            3
        };
        let l_filter = if l_matches {
            self.left_filter[dir].get(yi).copied().unwrap_or(3)
        } else {
            3
        };
        let comp = u8::from(comp);
        if a_filter == l_filter {
            comp * 4 + a_filter
        } else if a_filter == 3 {
            comp * 4 + l_filter
        } else if l_filter == 3 {
            comp * 4 + a_filter
        } else {
            comp * 4 + 3
        }
    }

    /// Approximate `find_matching_ref` (spec/dav1d's real above/left scan for a matching-single-
    /// reference neighbor, used to gate `motion_mode`'s warp eligibility) -- real dav1d scans
    /// EVERY distinct neighbor block touching this CU's above/left edge (a real per-4x4-unit
    /// `refmvs` grid this crate doesn't maintain, see `read_motion_mode`'s doc for why a full port
    /// is deferred). This checks only the SINGLE above/left neighbor at this CU's own origin
    /// (`above_ref0[x4]`/`left_ref0[y4]`, the same arrays `comp_mode_context` etc. already
    /// maintain) instead of the full edge -- exact whenever the neighbor's block boundary aligns
    /// with this CU's full edge (the common case), approximate (may miss a real match, never
    /// invents a false one) when a neighbor is smaller and only partially covers the edge. A real,
    /// documented approximation, not a bug.
    pub fn has_matching_single_ref(
        &self,
        x4: u32,
        y4: u32,
        have_top: bool,
        have_left: bool,
        ref0: i8,
    ) -> bool {
        let above = have_top
            && self.above_ref0.get(x4 as usize).copied().unwrap_or(-1) == ref0
            && !self
                .above_ref_comp
                .get(x4 as usize)
                .copied()
                .unwrap_or(false)
            && !self
                .above_ref_intra
                .get(x4 as usize)
                .copied()
                .unwrap_or(true);
        let left = have_left
            && self.left_ref0.get(y4 as usize).copied().unwrap_or(-1) == ref0
            && !self
                .left_ref_comp
                .get(y4 as usize)
                .copied()
                .unwrap_or(false)
            && !self
                .left_ref_intra
                .get(y4 as usize)
                .copied()
                .unwrap_or(true);
        above || left
    }

    /// `comp_mode` context (0..=4) -- whether this block is likely single- or compound-reference,
    /// derived from the above/left neighbors' reference counts. Source: rav1d `get_comp_ctx`
    /// (`memorysafety/rav1d`, BSD-2-Clause, `src/env.rs`).
    pub fn comp_mode_context(&self, x4: u32, y4: u32) -> u8 {
        let have_top = y4 > 0;
        let have_left = x4 > 0;
        let xi = x4 as usize;
        let yi = y4 as usize;
        let a_comp = || self.above_ref_comp.get(xi).copied().unwrap_or(false);
        let l_comp = || self.left_ref_comp.get(yi).copied().unwrap_or(false);
        let a_ref0 = || self.above_ref0.get(xi).copied().unwrap_or(0);
        let l_ref0 = || self.left_ref0.get(yi).copied().unwrap_or(0);

        if have_top {
            if have_left {
                if a_comp() {
                    if l_comp() {
                        4
                    } else {
                        2 + u8::from(l_ref0() >= 4)
                    }
                } else if l_comp() {
                    2 + u8::from(a_ref0() >= 4)
                } else {
                    u8::from((l_ref0() >= 4) ^ (a_ref0() >= 4))
                }
            } else if a_comp() {
                3
            } else {
                u8::from(a_ref0() >= 4)
            }
        } else if have_left {
            if l_comp() {
                3
            } else {
                u8::from(l_ref0() >= 4)
            }
        } else {
            1
        }
    }

    /// `comp_ref_type` context (0..=4) -- unidirectional vs bidirectional compound likelihood.
    /// Source: rav1d `get_comp_dir_ctx` (`memorysafety/rav1d`, BSD-2-Clause, `src/env.rs`), the
    /// most heavily branched of the `ref_frame` context functions.
    pub fn comp_ref_type_context(&self, x4: u32, y4: u32) -> u8 {
        let have_top = y4 > 0;
        let have_left = x4 > 0;
        let xi = x4 as usize;
        let yi = y4 as usize;
        let (a_intra, a_comp, a_ref0, a_ref1) = (
            self.above_ref_intra.get(xi).copied().unwrap_or(true),
            self.above_ref_comp.get(xi).copied().unwrap_or(false),
            self.above_ref0.get(xi).copied().unwrap_or(0),
            self.above_ref1.get(xi).copied().unwrap_or(0),
        );
        let (l_intra, l_comp, l_ref0, l_ref1) = (
            self.left_ref_intra.get(yi).copied().unwrap_or(true),
            self.left_ref_comp.get(yi).copied().unwrap_or(false),
            self.left_ref0.get(yi).copied().unwrap_or(0),
            self.left_ref1.get(yi).copied().unwrap_or(0),
        );
        let has_uni_comp = |ref0: i8, ref1: i8| (ref0 < 4) == (ref1 < 4);

        if have_top && have_left {
            if a_intra && l_intra {
                return 2;
            }
            if a_intra || l_intra {
                let (edge_comp, edge_ref0, edge_ref1) = if a_intra {
                    (l_comp, l_ref0, l_ref1)
                } else {
                    (a_comp, a_ref0, a_ref1)
                };
                if !edge_comp {
                    return 2;
                }
                return 1 + 2 * u8::from(has_uni_comp(edge_ref0, edge_ref1));
            }

            if !a_comp && !l_comp {
                1 + 2 * u8::from((a_ref0 >= 4) == (l_ref0 >= 4))
            } else if !a_comp || !l_comp {
                let (edge_ref0, edge_ref1) = if a_comp {
                    (a_ref0, a_ref1)
                } else {
                    (l_ref0, l_ref1)
                };
                if !has_uni_comp(edge_ref0, edge_ref1) {
                    1
                } else {
                    3 + u8::from((a_ref0 >= 4) == (l_ref0 >= 4))
                }
            } else {
                let a_uni = has_uni_comp(a_ref0, a_ref1);
                let l_uni = has_uni_comp(l_ref0, l_ref1);
                if !a_uni && !l_uni {
                    0
                } else if !a_uni || !l_uni {
                    2
                } else {
                    3 + u8::from((a_ref0 == 4) == (l_ref0 == 4))
                }
            }
        } else if have_top || have_left {
            let (edge_intra, edge_comp, edge_ref0, edge_ref1) = if have_left {
                (l_intra, l_comp, l_ref0, l_ref1)
            } else {
                (a_intra, a_comp, a_ref0, a_ref1)
            };
            if edge_intra || !edge_comp {
                2
            } else {
                4 * u8::from(has_uni_comp(edge_ref0, edge_ref1))
            }
        } else {
            2
        }
    }

    /// Shared neighbor-counting core for `single_ref_p1`/`single_ref_p2`/`single_ref_p3`/
    /// `single_ref_p4`/`single_ref_p5`/`single_ref_p6` and their compound-CDF reuses
    /// (`uni_comp_ref`/`comp_ref`/`comp_ref_p1`/`comp_ref_p2`/`comp_bwdref`/`comp_bwdref_p1`).
    /// `bucket` maps a neighbor's raw rav1d-encoded ref value (0..=6) to `Some(bucket index)` to
    /// count, or `None` to ignore it (mirrors each rav1d function's own `if ref0 < N`/`>= N`/
    /// `^ N < N` guard before indexing its `cnt` array).
    fn ref_count_context(
        &self,
        x4: u32,
        y4: u32,
        num_buckets: usize,
        bucket: impl Fn(i8) -> Option<usize>,
    ) -> [u8; 4] {
        let have_top = y4 > 0;
        let have_left = x4 > 0;
        let xi = x4 as usize;
        let yi = y4 as usize;
        let mut cnt = [0u8; 4];
        debug_assert!(num_buckets <= 4);
        let above_intra = self.above_ref_intra.get(xi).copied().unwrap_or(true);
        let left_intra = self.left_ref_intra.get(yi).copied().unwrap_or(true);

        if have_top && !above_intra {
            if let Some(b) = bucket(self.above_ref0.get(xi).copied().unwrap_or(0)) {
                cnt[b] += 1;
            }
            if self.above_ref_comp.get(xi).copied().unwrap_or(false) {
                if let Some(b) = bucket(self.above_ref1.get(xi).copied().unwrap_or(0)) {
                    cnt[b] += 1;
                }
            }
        }
        if have_left && !left_intra {
            if let Some(b) = bucket(self.left_ref0.get(yi).copied().unwrap_or(0)) {
                cnt[b] += 1;
            }
            if self.left_ref_comp.get(yi).copied().unwrap_or(false) {
                if let Some(b) = bucket(self.left_ref1.get(yi).copied().unwrap_or(0)) {
                    cnt[b] += 1;
                }
            }
        }
        cnt
    }

    /// `single_ref_p1`/`uni_comp_ref` context (0..=2) -- forward vs backward reference-group
    /// likelihood. Source: rav1d `av1_get_ref_ctx` (`memorysafety/rav1d`, BSD-2-Clause,
    /// `src/env.rs`).
    pub fn single_ref_p1_context(&self, x4: u32, y4: u32) -> u8 {
        let cnt = self.ref_count_context(x4, y4, 2, |r| Some(usize::from(r >= 4)));
        cmp_counts(cnt[0], cnt[1])
    }

    /// `single_ref_p3`/`comp_ref` context (0..=2) -- among forward refs, LAST/LAST2 vs
    /// LAST3/GOLDEN. Source: rav1d `av1_get_fwd_ref_ctx` (`memorysafety/rav1d`, BSD-2-Clause,
    /// `src/env.rs`).
    pub fn single_ref_p3_context(&self, x4: u32, y4: u32) -> u8 {
        let cnt =
            self.ref_count_context(x4, y4, 4, |r| if r < 4 { Some(r as usize) } else { None });
        cmp_counts(cnt[0] + cnt[1], cnt[2] + cnt[3])
    }

    /// `single_ref_p4`/`comp_ref_p1` context (0..=2) -- LAST vs LAST2. Source: rav1d
    /// `av1_get_fwd_ref_1_ctx` (`memorysafety/rav1d`, BSD-2-Clause, `src/env.rs`).
    pub fn single_ref_p4_context(&self, x4: u32, y4: u32) -> u8 {
        let cnt =
            self.ref_count_context(x4, y4, 2, |r| if r < 2 { Some(r as usize) } else { None });
        cmp_counts(cnt[0], cnt[1])
    }

    /// `single_ref_p5`/`comp_ref_p2`/`uni_comp_ref_p2` context (0..=2) -- LAST3 vs GOLDEN.
    /// Source: rav1d `av1_get_fwd_ref_2_ctx` (`memorysafety/rav1d`, BSD-2-Clause, `src/env.rs`).
    pub fn single_ref_p5_context(&self, x4: u32, y4: u32) -> u8 {
        let cnt = self.ref_count_context(x4, y4, 2, |r| {
            if (r ^ 2) < 2 {
                Some((r - 2) as usize)
            } else {
                None
            }
        });
        cmp_counts(cnt[0], cnt[1])
    }

    /// `single_ref_p2`/`comp_bwdref` context (0..=2) -- among backward refs, BWDREF/ALTREF2 vs
    /// ALTREF. Source: rav1d `av1_get_bwd_ref_ctx` (`memorysafety/rav1d`, BSD-2-Clause,
    /// `src/env.rs`).
    pub fn single_ref_p2_context(&self, x4: u32, y4: u32) -> u8 {
        let cnt = self.ref_count_context(x4, y4, 3, |r| {
            if r >= 4 {
                Some((r - 4) as usize)
            } else {
                None
            }
        });
        cmp_counts(cnt[0] + cnt[1], cnt[2])
    }

    /// `single_ref_p6`/`comp_bwdref_p1` context (0..=2) -- BWDREF vs ALTREF2. Source: rav1d
    /// `av1_get_bwd_ref_1_ctx` (`memorysafety/rav1d`, BSD-2-Clause, `src/env.rs`).
    pub fn single_ref_p6_context(&self, x4: u32, y4: u32) -> u8 {
        let cnt = self.ref_count_context(x4, y4, 3, |r| {
            if r >= 4 {
                Some((r - 4) as usize)
            } else {
                None
            }
        });
        cmp_counts(cnt[0], cnt[1])
    }

    /// `uni_comp_ref_p1` context (0..=2) -- LAST2 vs LAST3/GOLDEN (grouped), among unidirectional
    /// compound candidates. Source: rav1d `av1_get_uni_p1_ctx` (`memorysafety/rav1d`,
    /// BSD-2-Clause, `src/env.rs`) -- the one function of the nine whose bucket count (3, indices
    /// 0..=2 for ref values 1..=3) doesn't match its `cnt` array width (also 3): unlike the other
    /// `ref_count_context` uses, out-of-range ref values here are silently dropped (rav1d's
    /// `cnt.get_mut(...)` returns `None` and the increment is skipped) rather than counted, since
    /// LAST (ref value 0) legitimately isn't part of this context's model at all.
    pub fn uni_comp_ref_p1_context(&self, x4: u32, y4: u32) -> u8 {
        let mut cnt = self.ref_count_context(x4, y4, 3, |r| {
            let idx = r - 1;
            if (0..3).contains(&idx) {
                Some(idx as usize)
            } else {
                None
            }
        });
        cnt[1] += cnt[2];
        cmp_counts(cnt[0], cnt[1])
    }

    /// Record a decoded **inter** block's ref/mode state for future `inter_mode`/`compound_mode`
    /// context lookups -- see `SpatialRefContext::set_block`'s doc (never call for intra blocks).
    #[allow(clippy::too_many_arguments)]
    #[allow(clippy::too_many_arguments)]
    pub fn set_spatial_ref_block(
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
        self.spatial_ref.set_block(
            x4, y4, width_4x4, height_4x4, ref0, ref1, is_newmv, mv0, mv1,
        );
    }

    /// Packed single-ref `inter_mode` context -- see `SpatialRefContext::inter_mode_context`.
    pub fn inter_mode_context(
        &self,
        x4: u32,
        y4: u32,
        bw4: u32,
        bh4: u32,
        ref0: i8,
        use_ref_frame_mvs: bool,
    ) -> u16 {
        self.spatial_ref
            .inter_mode_context(x4, y4, bw4, bh4, ref0, use_ref_frame_mvs)
    }

    /// Real weighted single-ref DRL candidate stack -- see `SpatialRefContext::single_ref_mv_stack`.
    pub fn single_ref_mv_stack(
        &self,
        x4: u32,
        y4: u32,
        bw4: u32,
        bh4: u32,
        ref0: i8,
        use_ref_frame_mvs: bool,
    ) -> ([MvStackEntry; 8], usize) {
        self.spatial_ref
            .single_ref_mv_stack(x4, y4, bw4, bh4, ref0, use_ref_frame_mvs)
    }

    /// Real weighted compound DRL candidate stack -- see `SpatialRefContext::compound_mv_stack`.
    #[allow(clippy::too_many_arguments)]
    pub fn compound_mv_stack(
        &self,
        x4: u32,
        y4: u32,
        bw4: u32,
        bh4: u32,
        ref0: i8,
        ref1: i8,
        use_ref_frame_mvs: bool,
    ) -> ([CompoundMvStackEntry; 8], usize) {
        self.spatial_ref
            .compound_mv_stack(x4, y4, bw4, bh4, ref0, ref1, use_ref_frame_mvs)
    }

    /// Opt this frame's parse into real temporal MV candidates -- see
    /// `SpatialRefContext::set_temporal_context`.
    pub fn set_temporal_context(
        &mut self,
        projected: crate::tile::motion_field::ProjectedMotionField,
        pocdiff: [i32; 7],
    ) {
        self.spatial_ref.set_temporal_context(projected, pocdiff);
    }

    /// `compound_mode` context -- see `SpatialRefContext::compound_mode_context`.
    pub fn compound_mode_context(
        &self,
        x4: u32,
        y4: u32,
        bw4: u32,
        bh4: u32,
        ref0: i8,
        ref1: i8,
    ) -> u8 {
        self.spatial_ref
            .compound_mode_context(x4, y4, bw4, bh4, ref0, ref1)
    }
}

/// 3-way ordinal comparison of two neighbor reference-frame counts, used by every count-based
/// `ref_frame` context function above. Source: rav1d `cmp_counts` (`memorysafety/rav1d`,
/// BSD-2-Clause, `src/env.rs`).
fn cmp_counts(c1: u8, c2: u8) -> u8 {
    match c1.cmp(&c2) {
        std::cmp::Ordering::Less => 0,
        std::cmp::Ordering::Equal => 1,
        std::cmp::Ordering::Greater => 2,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::symbol::SymbolDecoder;
    use crate::tile::coding_unit::MotionVector;

    #[test]
    fn test_skip_context_starts_at_zero_with_no_neighbors() {
        let ctx = TileContext::new(16, 16);
        assert_eq!(ctx.skip_context(0, 0), 0);
    }

    #[test]
    fn test_skip_context_reflects_above_neighbor() {
        let mut ctx = TileContext::new(16, 16);
        ctx.set_skip(0, 0, 4, 4, true);
        // A block directly to the right, same 4x4 row -- shares the above-context column but not
        // the left-context row, so only the "above" contribution applies here since we're
        // querying the SAME above-row position that was written.
        assert_eq!(ctx.skip_context(0, 5), 1);
    }

    #[test]
    fn test_skip_context_reflects_left_neighbor() {
        let mut ctx = TileContext::new(16, 16);
        ctx.set_skip(0, 0, 4, 4, true);
        assert_eq!(ctx.skip_context(5, 0), 1);
    }

    #[test]
    fn test_skip_context_sums_both_neighbors() {
        let mut ctx = TileContext::new(16, 16);
        ctx.set_skip(0, 0, 4, 4, true);
        assert_eq!(ctx.skip_context(0, 0), 2);
    }

    #[test]
    fn test_start_superblock_row_resets_left_but_not_above() {
        let mut ctx = TileContext::new(16, 16);
        ctx.set_skip(0, 0, 4, 4, true);
        ctx.start_superblock_row();
        // Left context cleared...
        assert_eq!(ctx.skip_context(5, 0), 0);
        // ...but above context (tile-lifetime) persists.
        assert_eq!(ctx.skip_context(0, 5), 1);
    }

    #[test]
    fn test_set_skip_clamps_to_array_bounds() {
        // A block whose footprint would run past the tile/superblock edge must not panic.
        let mut ctx = TileContext::new(4, 4);
        ctx.set_skip(2, 2, 100, 100, true);
        assert_eq!(ctx.skip_context(3, 3), 2);
    }

    #[test]
    fn test_intra_mode_context_defaults_to_dc_pred_class() {
        // No neighbors written yet -- both default to raw mode 0 (DC_PRED), class 0.
        let ctx = TileContext::new(16, 16);
        assert_eq!(ctx.intra_mode_context(0, 0), (0, 0));
    }

    #[test]
    fn test_intra_mode_context_maps_through_intra_mode_context_table() {
        let mut ctx = TileContext::new(16, 16);
        // Raw mode 8 (D67_PRED) -> class 3 per INTRA_MODE_CONTEXT.
        ctx.set_mode(0, 0, 4, 4, 8);
        assert_eq!(ctx.intra_mode_context(0, 5).0, 3); // above contribution only
        assert_eq!(ctx.intra_mode_context(5, 0).1, 3); // left contribution only
    }

    #[test]
    fn test_intra_mode_context_combines_distinct_above_and_left_classes() {
        let mut ctx = TileContext::new(16, 16);
        // A block at (x4=0, y4=0..4) sets above_mode[0..4] AND left_mode[0..4] -- to isolate the
        // two contributions for a query at (x4=0, y4=4), use two non-overlapping blocks: one
        // covering only above_mode[0] (a block above-and-to-the-left, x4=0 y4=0..4), one covering
        // only left_mode[4] (a block directly above the query's row, y4=4 x4=4..8 -- outside the
        // query's own x4=0 column, so it can't also perturb above_mode[0]).
        ctx.set_mode(0, 0, 4, 4, 8); // above_mode[0..4]=8, left_mode[0..4]=8
        ctx.set_mode(4, 4, 4, 4, 4); // above_mode[4..8]=4, left_mode[4..8]=4
                                     // Query (0, 4): above_mode[0]=8 (class 3, untouched by the second call), left_mode[4]=4
                                     // (class 4, set by the second call).
        assert_eq!(ctx.intra_mode_context(0, 4), (3, 4));
    }

    #[test]
    fn test_start_superblock_row_resets_left_mode_but_not_above_mode() {
        let mut ctx = TileContext::new(16, 16);
        ctx.set_mode(0, 0, 4, 4, 8);
        ctx.start_superblock_row();
        assert_eq!(ctx.intra_mode_context(5, 0).1, 0); // left reset to DC_PRED class
        assert_eq!(ctx.intra_mode_context(0, 5).0, 3); // above persists
    }

    #[test]
    fn test_intra_mode_context_table_matches_rav1d_dav1d_intra_mode_context() {
        // Source: rav1d `DAV1D_INTRA_MODE_CONTEXT` (`src/tables.rs`). Pinning the exact table
        // here catches an accidental edit independent of the round-trip tests above.
        assert_eq!(INTRA_MODE_CONTEXT, [0, 1, 2, 3, 4, 4, 4, 4, 3, 0, 1, 2, 0]);
    }

    #[test]
    fn test_partition_context_starts_at_zero_with_no_neighbors() {
        let ctx = TileContext::new(16, 16);
        assert_eq!(ctx.partition_context(0, 0, 4), 0);
    }

    #[test]
    fn test_partition_context_reflects_above_split_at_8x8() {
        let mut ctx = TileContext::new(16, 16);
        // Split (symbol 3) at 8x8 (bl=4) sets bit 0 of PARTITION_CTX_TABLE's value (0x1f), which
        // `has_bl` for bl=4 reads directly off bit 0.
        ctx.set_partition(0, 0, 1, 4, 3);
        // Query at the same above-column (x8=0), different row -- above contribution only.
        assert_eq!(ctx.partition_context(0, 5, 4), 1);
    }

    #[test]
    fn test_partition_context_reflects_left_split_at_8x8() {
        let mut ctx = TileContext::new(16, 16);
        ctx.set_partition(0, 0, 1, 4, 3);
        assert_eq!(ctx.partition_context(5, 0, 4), 2);
    }

    #[test]
    fn test_partition_context_sums_both_neighbors() {
        let mut ctx = TileContext::new(16, 16);
        ctx.set_partition(0, 0, 1, 4, 3);
        assert_eq!(ctx.partition_context(0, 0, 4), 3);
    }

    #[test]
    fn test_partition_context_none_clears_the_bit() {
        let mut ctx = TileContext::new(16, 16);
        ctx.set_partition(0, 0, 1, 4, 3); // Split first
        ctx.set_partition(0, 0, 1, 4, 0); // None overwrites (0x1e has bit 0 clear)
        assert_eq!(ctx.partition_context(0, 5, 4), 0);
    }

    #[test]
    fn test_start_superblock_row_resets_left_partition_but_not_above() {
        let mut ctx = TileContext::new(16, 16);
        ctx.set_partition(0, 0, 1, 4, 3);
        ctx.start_superblock_row();
        assert_eq!(ctx.partition_context(5, 0, 4), 0); // left reset
        assert_eq!(ctx.partition_context(0, 5, 4), 1); // above persists
    }

    #[test]
    fn test_set_partition_clamps_to_array_bounds() {
        let mut ctx = TileContext::new(4, 4);
        ctx.set_partition(0, 0, 100, 4, 3); // footprint far exceeds the array; must not panic
        assert_eq!(ctx.partition_context(0, 0, 4), 3);
    }

    #[test]
    fn test_partition_ctx_table_matches_rav1d_dav1d_al_part_ctx() {
        // Source: rav1d `DAV1D_AL_PART_CTX` (`src/tables.rs`). Pinning both extremes (128x128's
        // mostly-0xff row, and 8x8's real row) catches an accidental edit.
        assert_eq!(
            PARTITION_CTX_TABLE[0][0],
            [0x00, 0x00, 0x10, 0xff, 0x00, 0x10, 0x10, 0x10, 0xff, 0xff]
        );
        assert_eq!(
            PARTITION_CTX_TABLE[1][4],
            [0x1e, 0x1f, 0x1e, 0x1f, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff]
        );
    }

    #[test]
    fn test_partition_bl_maps_block_size_log2_to_rav1d_block_level() {
        assert_eq!(partition_bl(7), 0); // 128x128 -> rav1d BlockLevel 0
        assert_eq!(partition_bl(3), 4); // 8x8 -> rav1d BlockLevel 4
        assert_eq!(partition_bl(5), 2); // 32x32 -> rav1d BlockLevel 2
    }

    // ref_frame context tests. `ref0`/`ref1` values below use rav1d's raw 0..=6 encoding (see
    // `TileContext`'s struct doc): 0=LAST, 1=LAST2, 2=LAST3, 3=GOLDEN, 4=BWDREF, 5=ALTREF2,
    // 6=ALTREF.

    #[test]
    fn test_comp_mode_context_no_neighbors_returns_one() {
        // Neither have_top nor have_left -- rav1d's `get_comp_ctx` early-returns 1.
        let ctx = TileContext::new(16, 16);
        assert_eq!(ctx.comp_mode_context(0, 0), 1);
    }

    #[test]
    fn test_comp_mode_context_top_only_forward_ref_is_zero() {
        let mut ctx = TileContext::new(16, 16);
        ctx.set_ref_frames(0, 0, 4, 4, false, false, 0, -1); // LAST, single-ref
        assert_eq!(ctx.comp_mode_context(0, 5), 0); // above only, ref0 < 4
    }

    #[test]
    fn test_comp_mode_context_top_only_backward_ref_is_one() {
        let mut ctx = TileContext::new(16, 16);
        ctx.set_ref_frames(0, 0, 4, 4, false, false, 4, -1); // BWDREF, single-ref
        assert_eq!(ctx.comp_mode_context(0, 5), 1); // above only, ref0 >= 4
    }

    #[test]
    fn test_comp_mode_context_left_only_compound_is_three() {
        let mut ctx = TileContext::new(16, 16);
        ctx.set_ref_frames(0, 0, 4, 4, false, true, 0, 4);
        assert_eq!(ctx.comp_mode_context(5, 0), 3); // left only, compound
    }

    #[test]
    fn test_comp_mode_context_both_compound_returns_four() {
        let mut ctx = TileContext::new(16, 16);
        // To get both have_top and have_left true at the query position (5, 5), write the above
        // contribution at column x4=5 (via a block at (5, 0)) and the left contribution at row
        // y4=5 (via a block at (0, 5)) -- each call also writes the other array's irrelevant
        // range, which doesn't affect this query.
        ctx.set_ref_frames(5, 0, 4, 4, false, true, 0, 4); // above_comp[5..9]=true
        ctx.set_ref_frames(0, 5, 4, 4, false, true, 0, 4); // left_comp[5..9]=true
        assert_eq!(ctx.comp_mode_context(5, 5), 4);
    }

    #[test]
    fn test_comp_ref_type_context_no_neighbors_returns_two() {
        let ctx = TileContext::new(16, 16);
        assert_eq!(ctx.comp_ref_type_context(0, 0), 2);
    }

    #[test]
    fn test_comp_ref_type_context_both_intra_returns_two() {
        let mut ctx = TileContext::new(16, 16);
        // Same both-neighbor query-position trick as `test_comp_mode_context_both_compound...`.
        ctx.set_ref_frames(5, 0, 4, 4, true, false, 0, -1); // above_ref_intra[5..9]=true
        ctx.set_ref_frames(0, 5, 4, 4, true, false, 0, -1); // left_ref_intra[5..9]=true
        assert_eq!(ctx.comp_ref_type_context(5, 5), 2);
    }

    #[test]
    fn test_single_ref_p1_context_no_neighbors_is_tie() {
        // cmp_counts(0, 0) == 1 (Equal).
        let ctx = TileContext::new(16, 16);
        assert_eq!(ctx.single_ref_p1_context(0, 0), 1);
    }

    #[test]
    fn test_single_ref_p1_context_counts_forward_vs_backward() {
        let mut ctx = TileContext::new(16, 16);
        // Both-neighbor query-position trick (see `test_comp_mode_context_both_compound...`):
        // above contribution at column x4=5 (query x4), left contribution at row y4=5 (query y4).
        ctx.set_ref_frames(5, 0, 4, 4, false, false, 0, -1); // LAST (forward), above[5..9]
        ctx.set_ref_frames(0, 5, 4, 4, false, false, 4, -1); // BWDREF (backward), left[5..9]
                                                             // cnt[0]=1 (forward), cnt[1]=1 (backward) -> cmp_counts(1,1) == 1 (Equal).
        assert_eq!(ctx.single_ref_p1_context(5, 5), 1);
    }

    #[test]
    fn test_single_ref_p1_context_both_forward_is_greater() {
        let mut ctx = TileContext::new(16, 16);
        ctx.set_ref_frames(5, 0, 4, 4, false, false, 0, -1); // above[5..9]
        ctx.set_ref_frames(0, 5, 4, 4, false, false, 1, -1); // left[5..9]
                                                             // cnt[0]=2, cnt[1]=0 -> cmp_counts(2,0) == 2 (Greater).
        assert_eq!(ctx.single_ref_p1_context(5, 5), 2);
    }

    #[test]
    fn test_single_ref_p1_context_excludes_intra_neighbor() {
        let mut ctx = TileContext::new(16, 16);
        // An intra neighbor must not contribute to either bucket.
        ctx.set_ref_frames(0, 0, 4, 4, true, false, 0, -1);
        assert_eq!(ctx.single_ref_p1_context(0, 5), 1); // still a tie (0 vs 0)
    }

    #[test]
    fn test_single_ref_p3_context_groups_last_last2_vs_last3_golden() {
        let mut ctx = TileContext::new(16, 16);
        ctx.set_ref_frames(5, 0, 4, 4, false, false, 0, -1); // LAST -> bucket 0, above[5..9]
        ctx.set_ref_frames(0, 5, 4, 4, false, false, 3, -1); // GOLDEN -> bucket 3, left[5..9]
                                                             // cnt[0]+cnt[1]=1, cnt[2]+cnt[3]=1 -> tie.
        assert_eq!(ctx.single_ref_p3_context(5, 5), 1);
    }

    #[test]
    fn test_single_ref_p5_context_last3_vs_golden() {
        let mut ctx = TileContext::new(16, 16);
        ctx.set_ref_frames(0, 0, 4, 4, false, false, 3, -1); // GOLDEN -> bucket 1
                                                             // cnt[0]=0 (LAST3), cnt[1]=1 (GOLDEN) -> cmp_counts(0,1) == 0 (Less).
        assert_eq!(ctx.single_ref_p5_context(0, 5), 0);
    }

    #[test]
    fn test_uni_comp_ref_p1_context_groups_last2_vs_last3_golden() {
        let mut ctx = TileContext::new(16, 16);
        ctx.set_ref_frames(0, 0, 4, 4, false, false, 1, -1); // LAST2 -> bucket 0
                                                             // cnt[0]=1, cnt[1]=0 (bucket 1/2 empty, merged via cnt[1]+=cnt[2]) -> cmp_counts(1,0)=2.
        assert_eq!(ctx.uni_comp_ref_p1_context(0, 5), 2);
    }

    #[test]
    fn test_uni_comp_ref_p1_context_excludes_last() {
        let mut ctx = TileContext::new(16, 16);
        // LAST (ref value 0) isn't part of this context's model -- rav1d's `cnt.get_mut(-1)`
        // silently drops it (see `uni_comp_ref_p1_context`'s doc).
        ctx.set_ref_frames(0, 0, 4, 4, false, false, 0, -1);
        assert_eq!(ctx.uni_comp_ref_p1_context(0, 5), 1); // 0 vs 0, tie
    }

    #[test]
    fn test_start_superblock_row_resets_left_ref_state() {
        let mut ctx = TileContext::new(16, 16);
        // Query at x4=5 (have_left=true), y4=0 (have_top=false) so only the left contribution
        // matters; writing at (0, 0) sets left_ref0[0..4], covering the query's y4=0.
        ctx.set_ref_frames(0, 0, 4, 4, false, false, 4, -1); // BWDREF, left_ref0[0..4]=4
        assert_eq!(ctx.single_ref_p1_context(5, 0), 0); // Less: 0 forward vs 1 backward
        ctx.start_superblock_row();
        // After reset, left_ref_intra defaults back to `true` (see `start_superblock_row`'s doc),
        // so the stale backward-ref value at left_ref0[0] is excluded again -- back to a tie.
        assert_eq!(ctx.single_ref_p1_context(5, 0), 1);
    }

    // `inter_mode`/`compound_mode` (`SpatialRefContext`) tests.

    #[test]
    fn test_inter_mode_context_no_neighbors_is_zero() {
        let ctx = SpatialRefContext::new(16, 16);
        assert_eq!(ctx.inter_mode_context(0, 0, 4, 4, 0, false), 0);
    }

    #[test]
    fn test_inter_mode_context_use_ref_frame_mvs_sets_globalmv_bit() {
        // No spatial neighbors (refmv_ctx=0, newmv_ctx=0) isolates globalmv_ctx: packed layout is
        // `refmv_ctx << 4 | globalmv_ctx << 3 | newmv_ctx`, so globalmv_ctx alone should produce
        // exactly bit 3 (value 8).
        let ctx = SpatialRefContext::new(16, 16);
        assert_eq!(ctx.inter_mode_context(0, 0, 4, 4, 0, false), 0);
        assert_eq!(ctx.inter_mode_context(0, 0, 4, 4, 0, true), 8);
    }

    #[test]
    fn test_inter_mode_context_top_forward_match_not_newmv() {
        let mut ctx = SpatialRefContext::new(16, 16);
        ctx.set_block(
            0,
            0,
            4,
            4,
            0,
            -1,
            false,
            MotionVector::zero(),
            MotionVector::zero(),
        ); // LAST, covers x4=0..4, y4=0..4
           // Query directly below: top row scan hits row y4=0, columns 0..4 -- a match.
           // nearest_match=1 -> refmv_ctx=min(1*3,4)=3, newmv_ctx=3-0=3 -> packed = 3<<4|3 = 51.
        assert_eq!(ctx.inter_mode_context(0, 1, 4, 4, 0, false), 51);
    }

    #[test]
    fn test_inter_mode_context_top_match_is_newmv_lowers_newmv_ctx() {
        let mut ctx = SpatialRefContext::new(16, 16);
        ctx.set_block(
            0,
            0,
            4,
            4,
            0,
            -1,
            true,
            MotionVector::zero(),
            MotionVector::zero(),
        );
        // Same as above but is_newmv=true -> newmv_ctx = 3-1=2 -> packed = 3<<4|2 = 50.
        assert_eq!(ctx.inter_mode_context(0, 1, 4, 4, 0, false), 50);
    }

    #[test]
    fn test_inter_mode_context_no_ref_match_is_zero() {
        let mut ctx = SpatialRefContext::new(16, 16);
        ctx.set_block(
            0,
            0,
            4,
            4,
            4,
            -1,
            false,
            MotionVector::zero(),
            MotionVector::zero(),
        ); // GOLDEN
           // Query for a different ref (LAST) -- no match at all.
        assert_eq!(ctx.inter_mode_context(0, 1, 4, 4, 0, false), 0);
    }

    #[test]
    fn test_inter_mode_context_single_ref_matches_either_slot_of_compound_neighbor() {
        let mut ctx = SpatialRefContext::new(16, 16);
        ctx.set_block(
            0,
            0,
            4,
            4,
            2,
            5,
            false,
            MotionVector::zero(),
            MotionVector::zero(),
        ); // compound neighbor: LAST3 + ALTREF2
           // Single-ref query for ref0=5 (ALTREF2) matches via the neighbor's ref1 slot.
        assert_eq!(ctx.inter_mode_context(0, 1, 4, 4, 5, false), 51);
    }

    #[test]
    fn test_compound_mode_context_no_neighbors_is_zero() {
        let ctx = SpatialRefContext::new(16, 16);
        assert_eq!(ctx.compound_mode_context(0, 0, 4, 4, 0, 4), 0);
    }

    #[test]
    fn test_compound_mode_context_exact_pair_required() {
        let mut ctx = SpatialRefContext::new(16, 16);
        ctx.set_block(
            0,
            0,
            4,
            4,
            0,
            4,
            false,
            MotionVector::zero(),
            MotionVector::zero(),
        ); // compound (LAST, BWDREF)
           // Swapped pair must NOT match (rav1d: exact `RefMvsRefPair` equality).
        assert_eq!(ctx.compound_mode_context(0, 1, 4, 4, 4, 0), 0);
        // Exact pair matches: nearest_match=1 -> refmv_ctx=3, refmv_ctx>>1=1 -> 1+min(newmv_ctx,3).
        // newmv_ctx=3 (not newmv) -> 1+3=4.
        assert_eq!(ctx.compound_mode_context(0, 1, 4, 4, 0, 4), 4);
    }

    #[test]
    fn test_compound_mode_context_both_sides_match_saturates_to_seven() {
        let mut ctx = SpatialRefContext::new(16, 16);
        // Isolated single-cell placements: row 4 (above the query row) and column 4 (left of the
        // query column), so both the top and left primary scans find exactly one match each.
        ctx.set_block(
            5,
            4,
            4,
            1,
            0,
            4,
            false,
            MotionVector::zero(),
            MotionVector::zero(),
        );
        ctx.set_block(
            4,
            5,
            1,
            4,
            0,
            4,
            false,
            MotionVector::zero(),
            MotionVector::zero(),
        );
        // nearest_match=2 -> refmv_ctx=5, refmv_ctx>>1=2 -> clamp(3+newmv_ctx,4,7).
        // newmv_ctx=5-0=5 -> clamp(8,4,7)=7.
        assert_eq!(ctx.compound_mode_context(5, 5, 4, 4, 0, 4), 7);
    }

    #[test]
    fn test_compound_mv_stack_requires_exact_pair_match() {
        let mut ctx = SpatialRefContext::new(16, 16);
        let mv_a = MotionVector::new(4, 8);
        let mv_b = MotionVector::new(-2, 6);
        // A neighbor with the SWAPPED pair (BWDREF, LAST) at the query's (LAST, BWDREF) position
        // must not contribute at all -- real spec's exact `RefMvsRefPair` equality (no partial
        // single-slot fallback at the spatial-scan stage, `compound_candidate_mv`'s doc).
        ctx.set_block(0, 0, 4, 4, 4, 0, false, mv_a, mv_b);
        let (stack, cnt) = ctx.compound_mv_stack(0, 1, 4, 4, 0, 4, false);
        assert_eq!(cnt, 0, "swapped ref pair must not match");
        assert_eq!(stack[0], CompoundMvStackEntry::default());

        // The exact pair DOES contribute, with both MVs preserved as a joint pair (not
        // independently re-derived).
        let mut ctx2 = SpatialRefContext::new(16, 16);
        ctx2.set_block(0, 0, 4, 4, 0, 4, false, mv_a, mv_b);
        let (stack2, cnt2) = ctx2.compound_mv_stack(0, 1, 4, 4, 0, 4, false);
        assert_eq!(cnt2, 1);
        assert_eq!(stack2[0].mv, [mv_a, mv_b]);
    }

    #[test]
    fn test_compound_mv_stack_dedups_identical_pairs_by_weight() {
        let mut ctx = SpatialRefContext::new(16, 16);
        let mv_pair = [MotionVector::new(1, 1), MotionVector::new(2, 2)];
        // Two separate 1x1 neighbors (row above at x4=0 and x4=1) with the IDENTICAL pair --
        // real spec accumulates weight into one entry rather than pushing a duplicate
        // (`push_compound_mv_candidate`'s doc).
        ctx.set_block(0, 0, 1, 1, 0, 4, false, mv_pair[0], mv_pair[1]);
        ctx.set_block(1, 0, 1, 1, 0, 4, false, mv_pair[0], mv_pair[1]);
        let (stack, cnt) = ctx.compound_mv_stack(0, 1, 2, 4, 0, 4, false);
        assert_eq!(
            cnt, 1,
            "identical pairs must merge into a single stack entry"
        );
        assert_eq!(stack[0].mv, mv_pair);
    }

    #[test]
    fn test_get_compound_drl_context_matches_weight_thresholds() {
        let mut stack = [CompoundMvStackEntry::default(); 8];
        // Both entries in the "nearest" (>= 640) tier -> ctx 0.
        stack[0].weight = 700;
        stack[1].weight = 650;
        assert_eq!(get_compound_drl_context(&stack, 0), 0);
        // idx in nearest tier, idx+1 in secondary tier (< 640) -> ctx 1.
        stack[1].weight = 100;
        assert_eq!(get_compound_drl_context(&stack, 0), 1);
        // Both in secondary tier -> ctx 2.
        stack[0].weight = 50;
        assert_eq!(get_compound_drl_context(&stack, 0), 2);
    }

    #[test]
    fn test_inter_mode_context_top_left_corner_contributes_but_not_to_newmv() {
        let mut ctx = SpatialRefContext::new(16, 16);
        // Only the top-left corner cell (x4-1, y4-1) matches, and it's a newmv block -- rav1d
        // discards this candidate's newmv contribution (`have_dummy_newmv_match`).
        ctx.set_block(
            4,
            4,
            1,
            1,
            0,
            -1,
            true,
            MotionVector::zero(),
            MotionVector::zero(),
        );
        // nearest_match=0 (top-left isn't scanned until after nearest_match is computed) ->
        // refmv_ctx=min(ref_match_count,2), newmv_ctx=(ref_match_count>0).
        // ref_match_count=1 (top-left counted into have_row_mvs) -> refmv_ctx=1, newmv_ctx=1.
        // packed = 1<<4|1 = 17 -- and critically NOT lowered by the neighbor's newmv flag.
        assert_eq!(ctx.inter_mode_context(5, 5, 4, 4, 0, false), 17);
    }

    #[test]
    fn test_inter_mode_context_secondary_row_scan_finds_distant_match() {
        let mut ctx = SpatialRefContext::new(16, 16);
        // 3 rows above the query, out of reach of the primary top scan (row y4-1) but within the
        // secondary n=2 scan's `back = 2*2-1 = 3` reach.
        ctx.set_block(
            0,
            2,
            4,
            1,
            0,
            -1,
            false,
            MotionVector::zero(),
            MotionVector::zero(),
        );
        // Primary top scan (row 4) and top-right/top-left find nothing -> nearest_match=0.
        // Secondary scan finds the match -> ref_match_count=1 -> refmv_ctx=1, newmv_ctx=1.
        assert_eq!(ctx.inter_mode_context(0, 5, 4, 4, 0, false), 17);
    }

    #[test]
    fn test_set_block_intra_default_never_matches() {
        // A cell that's never written (the "intra block" simplification -- see `set_block`'s doc)
        // stays `valid: false` and never contributes, regardless of what ref0/ref1 it queries for.
        let ctx = SpatialRefContext::new(16, 16);
        assert_eq!(ctx.inter_mode_context(5, 5, 4, 4, 0, false), 0);
        assert_eq!(ctx.compound_mode_context(5, 5, 4, 4, 0, 4), 0);
    }

    // `tx_size()` (intra) tests.

    #[test]
    fn test_tx_size_context_no_neighbors_is_zero() {
        let ctx = TileContext::new(16, 16);
        assert_eq!(ctx.tx_size_context(0, 0, 4), 0);
    }

    #[test]
    fn test_tx_size_context_above_neighbor_at_least_as_large_contributes() {
        let mut ctx = TileContext::new(16, 16);
        ctx.set_tx_class(0, 0, 4, 4, 4); // neighbor's tx class = Tx64x64 (4)
                                         // Query at (0, 5): above-only (x4=0 means have_left irrelevant here since query itself
                                         // reads left_tx_class[y4=5], untouched -> only above contributes).
        assert_eq!(ctx.tx_size_context(0, 5, 3), 1); // 4 >= 3
    }

    #[test]
    fn test_tx_size_context_left_neighbor_smaller_does_not_contribute() {
        let mut ctx = TileContext::new(16, 16);
        ctx.set_tx_class(0, 0, 4, 4, 1); // neighbor's tx class = Tx8x8 (1)
        assert_eq!(ctx.tx_size_context(5, 0, 3), 0); // 1 < 3
    }

    #[test]
    fn test_tx_size_context_sums_both_neighbors() {
        let mut ctx = TileContext::new(16, 16);
        // Isolated placements: above contribution at column x4=5 (query x4), left at row y4=5.
        ctx.set_tx_class(5, 0, 4, 4, 4);
        ctx.set_tx_class(0, 5, 4, 4, 4);
        assert_eq!(ctx.tx_size_context(5, 5, 3), 2);
    }

    #[test]
    fn test_start_superblock_row_resets_left_tx_class_but_not_above() {
        let mut ctx = TileContext::new(16, 16);
        ctx.set_tx_class(0, 0, 4, 4, 4);
        ctx.start_superblock_row();
        assert_eq!(ctx.tx_size_context(5, 0, 3), 0); // left reset to -1
        assert_eq!(ctx.tx_size_context(0, 5, 3), 1); // above persists
    }

    // `read_var_tx_size` (inter/IntraBC var-tx) context tests.

    #[test]
    fn test_var_tx_context_no_neighbors_is_zero_zero() {
        let ctx = TileContext::new(16, 16);
        // Default `0` < any real candidate class > 0 -- both should contribute.
        assert_eq!(ctx.var_tx_context(0, 0, 3, 3), (1, 1));
    }

    #[test]
    fn test_var_tx_context_neighbor_at_least_as_large_does_not_contribute() {
        let mut ctx = TileContext::new(16, 16);
        ctx.set_var_tx_class(0, 0, 1, 1, 3, 3); // neighbor leaf class = Tx32x32 (3)
        assert_eq!(ctx.var_tx_context(0, 1, 3, 3), (0, 1)); // above: 3 < 3 false; left default 0<3 true
    }

    #[test]
    fn test_var_tx_context_neighbor_smaller_contributes() {
        let mut ctx = TileContext::new(16, 16);
        ctx.set_var_tx_class(0, 0, 1, 1, 1, 1); // neighbor leaf class = Tx8x8 (1)
        assert_eq!(ctx.var_tx_context(0, 1, 3, 3), (1, 1)); // above: 1 < 3 true
    }

    #[test]
    fn test_var_tx_context_width_and_height_classes_tracked_independently() {
        // A rectangular neighbor (e.g. 32x16, width_class=3, height_class=2): above-context sees
        // the wider dim, left-context sees the shorter one -- distinct from either alone.
        let mut ctx = TileContext::new(16, 16);
        ctx.set_var_tx_class(0, 0, 1, 1, 3, 2);
        // above: candidate width_class=3, neighbor above=3 -> 3<3 false.
        // left: candidate height_class=3, neighbor left=2 -> 2<3 true.
        assert_eq!(ctx.var_tx_context(0, 0, 3, 3), (0, 1));
    }

    #[test]
    fn test_read_tx_size_class_zero_reads_zero_bits() {
        let data = vec![0x80, 0x00, 0xFF, 0xFF, 0xAA, 0xBB];
        let mut decoder = SymbolDecoder::new(&data).unwrap();
        let before = (
            decoder.decoder.range,
            decoder.decoder.value,
            decoder.decoder.cnt,
        );
        let resolved = decoder.read_tx_size(0, 0).unwrap();
        assert_eq!(resolved, 0);
        assert_eq!(
            (
                decoder.decoder.range,
                decoder.decoder.value,
                decoder.decoder.cnt
            ),
            before
        );
    }

    #[test]
    fn test_read_tx_size_class_reads_real_bits_and_never_exceeds_max() {
        let data = vec![0x80, 0x00, 0xFF, 0xFF, 0xAA, 0xBB];
        let mut decoder = SymbolDecoder::new(&data).unwrap();
        let before = (
            decoder.decoder.range,
            decoder.decoder.value,
            decoder.decoder.cnt,
        );
        let resolved = decoder.read_tx_size(4, 1).unwrap();
        assert!(resolved <= 4);
        assert_ne!(
            (
                decoder.decoder.range,
                decoder.decoder.value,
                decoder.decoder.cnt
            ),
            before
        );
    }

    #[test]
    fn test_txb_skip_context_single_tx_block_is_always_zero() {
        let mut ctx = TileContext::new(16, 16);
        ctx.set_residual_ctx(0, 0, 4, 4, 63, None);
        assert_eq!(ctx.txb_skip_context(0, 0, 4, 4, true), 0);
    }

    #[test]
    fn test_txb_skip_context_no_neighbors_is_table_zero_zero() {
        let ctx = TileContext::new(16, 16);
        assert_eq!(
            ctx.txb_skip_context(0, 0, 2, 2, false),
            DAV1D_SKIP_CTX[0][0]
        );
    }

    #[test]
    fn test_txb_skip_context_above_neighbor_ors_in() {
        let mut ctx = TileContext::new(16, 16);
        ctx.set_residual_ctx(0, 0, 4, 4, 5, None);
        // Query at (0, 5): above-only (left_cul_level[5] untouched by the write above).
        assert_eq!(
            ctx.txb_skip_context(0, 5, 2, 2, false),
            DAV1D_SKIP_CTX[4][0]
        );
    }

    #[test]
    fn test_txb_skip_context_left_neighbor_ors_in() {
        let mut ctx = TileContext::new(16, 16);
        ctx.set_residual_ctx(0, 0, 4, 4, 5, None);
        // Query at (5, 0): left-only (above_cul_level[5] untouched by the write above).
        assert_eq!(
            ctx.txb_skip_context(5, 0, 2, 2, false),
            DAV1D_SKIP_CTX[0][4]
        );
    }

    #[test]
    fn test_txb_skip_context_combines_above_and_left() {
        let mut ctx = TileContext::new(16, 16);
        ctx.set_residual_ctx(0, 0, 2, 2, 5, None);
        assert_eq!(
            ctx.txb_skip_context(0, 0, 2, 2, false),
            DAV1D_SKIP_CTX[4][4]
        );
    }

    #[test]
    fn test_dc_sign_context_no_neighbors_is_neutral() {
        let ctx = TileContext::new(16, 16);
        assert_eq!(ctx.dc_sign_context(0, 0, 2, 2), 1);
    }

    #[test]
    fn test_dc_sign_context_positive_neighbors_is_positive() {
        let mut ctx = TileContext::new(16, 16);
        ctx.set_residual_ctx(0, 0, 2, 2, 10, Some(0)); // dc_sign=0 -> positive category
        assert_eq!(ctx.dc_sign_context(0, 0, 2, 2), 2);
    }

    #[test]
    fn test_dc_sign_context_negative_neighbors_is_negative() {
        let mut ctx = TileContext::new(16, 16);
        ctx.set_residual_ctx(0, 0, 2, 2, 10, Some(1)); // dc_sign=1 -> negative category
        assert_eq!(ctx.dc_sign_context(0, 0, 2, 2), 0);
    }

    #[test]
    fn test_dc_sign_context_sums_both_neighbors_and_can_cancel() {
        let mut ctx = TileContext::new(16, 16);
        ctx.set_residual_ctx(5, 0, 1, 1, 5, Some(0)); // above[5] positive
        ctx.set_residual_ctx(0, 5, 1, 1, 5, Some(1)); // left[5] negative
        assert_eq!(ctx.dc_sign_context(5, 5, 1, 1), 1); // +1 and -1 cancel to neutral
    }

    #[test]
    fn test_set_residual_ctx_all_zero_block_writes_neutral_state() {
        let mut ctx = TileContext::new(16, 16);
        ctx.set_residual_ctx(0, 0, 2, 2, 0, None);
        assert_eq!(
            ctx.txb_skip_context(0, 0, 2, 2, false),
            DAV1D_SKIP_CTX[0][0]
        );
        assert_eq!(ctx.dc_sign_context(0, 0, 2, 2), 1);
    }

    #[test]
    fn test_start_superblock_row_resets_left_residual_ctx_but_not_above() {
        let mut ctx = TileContext::new(16, 16);
        ctx.set_residual_ctx(0, 0, 2, 2, 20, Some(0));
        assert_eq!(
            ctx.txb_skip_context(0, 0, 2, 2, false),
            DAV1D_SKIP_CTX[4][4]
        );
        ctx.start_superblock_row();
        assert_eq!(
            ctx.txb_skip_context(0, 0, 2, 2, false),
            DAV1D_SKIP_CTX[4][0]
        );
    }
}
