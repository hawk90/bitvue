//! Real AV1 temporal MV candidates -- spec 7.9 (motion field motion vector storage process) and
//! 7.10 (motion field estimation process), used by [`crate::tile::context::SpatialRefContext::
//! single_ref_mv_stack`]'s temporal scan.
//!
//! Source: dav1d's own C reference implementation (`memorysafety`-adjacent, VideoLAN/Two Orioles,
//! BSD-2-Clause), `src/refmvs.c`/`refmvs.h` -- ported directly (this crate's established
//! "verify against reference source, don't reconstruct from memory" discipline), not from the
//! spec text alone, because the real algorithm has load-bearing implementation detail the spec's
//! prose alone under-specifies (the up-to-3-source priority selection, the chained
//! reference-of-reference projection). Every function below cites the exact `refmvs.c` lines it
//! ports.
//!
//! **Unit note**: dav1d stores MVs in 1/8-luma-sample units (`mv.x`/`mv.y`, spec's native MV
//! precision). This crate's [`crate::tile::coding_unit::MotionVector`] uses 1/4-sample ("quarter-
//! pel") units instead (see that type's doc) -- half as fine. Every magnitude/clip constant ported
//! from `refmvs.c` below is therefore halved from its literal C value, and every `* 8` subpel-scale
//! factor becomes `* 4`. This is a pure unit rescale, not a behavioral change.
//!
//! **Scope**: implemented for correctness only, exercised by a sequential full-fixture test
//! harness (`overlay_extraction::cu_parser`'s `temporal_mv` tests) -- not wired into
//! `bitvue-sidecar`'s per-request commands, which remain stateless/random-access (see this
//! session's `docs/DEVELOPMENT_PHASES.md` entry for the scope discussion).

use crate::overlay_extraction::cu_parser::CuSpatialIndex;
use crate::tile::coding_unit::{CodingUnit, MotionVector, RefFrame};

/// One saved candidate per 8x8 luma unit -- mirrors dav1d's `refmvs_temporal_block` (`refmvs.h:43-47`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SavedMv {
    pub mv: MotionVector,
    /// Real AV1 ref-frame value, `1..=7` (`RefFrame::Last..=RefFrame::AltRef`) -- never
    /// `RefFrame::Intra`, matching dav1d's `ref: u8` which is only ever written for inter blocks
    /// (`save_tmvs_c`, `refmvs.c:794-798` stores `ref = 0` i.e. "empty" otherwise, modeled here as
    /// `None` in the containing grid instead of a sentinel ref value).
    pub ref_frame: RefFrame,
}

/// 8x8-luma-unit-resolution grid, one [`SavedMv`] (or `None` = empty) per cell -- dav1d's
/// `rf->rp`/`rf->rp_proj` (`refmvs.h:81-84`), sized via `iw8`/`ih8` (`refmvs.c:819-822`).
#[derive(Debug, Clone)]
pub struct MotionFieldGrid {
    cells: Vec<Option<SavedMv>>,
    pub cols_8x8: u32,
    pub rows_8x8: u32,
}

impl MotionFieldGrid {
    fn empty(cols_8x8: u32, rows_8x8: u32) -> Self {
        Self {
            cells: vec![None; (cols_8x8 as usize) * (rows_8x8 as usize)],
            cols_8x8,
            rows_8x8,
        }
    }

    pub fn get(&self, x8: u32, y8: u32) -> Option<SavedMv> {
        if x8 >= self.cols_8x8 || y8 >= self.rows_8x8 {
            return None;
        }
        self.cells[(y8 * self.cols_8x8 + x8) as usize]
    }

    fn set(&mut self, x8: u32, y8: u32, value: Option<SavedMv>) {
        if x8 < self.cols_8x8 && y8 < self.rows_8x8 {
            self.cells[(y8 * self.cols_8x8 + x8) as usize] = value;
        }
    }
}

/// dav1d's `mv_projection` (`refmvs.c:175-191`) -- scales `mv` by the ratio `num/den` of two
/// order-hint deltas, spec 7.9.3's rounding/clip. `den` must be in `1..32`(dav1d asserts this --
/// callers here only ever pass a `pocdiff`/`ref2ref` value already clamped to that range, see
/// `select_motion_field_sources`).
pub(crate) fn mv_projection(mv: MotionVector, num: i32, den: i32) -> MotionVector {
    const DIV_MULT: [i32; 32] = [
        0, 16384, 8192, 5461, 4096, 3276, 2730, 2340, 2048, 1820, 1638, 1489, 1365, 1260, 1170,
        1092, 1024, 963, 910, 862, 819, 780, 744, 712, 682, 655, 630, 606, 585, 564, 546, 528,
    ];
    debug_assert!((1..32).contains(&den));
    debug_assert!((-32..32).contains(&num));
    let frac = num * DIV_MULT[den as usize];
    let y = mv.y * frac;
    let x = mv.x * frac;
    // Real dav1d clips to 0x3fff (1/8-pel units) -- halved for this crate's quarter-pel units
    // (see module doc). `(v + 8192 + (v>>31)) >> 14` is dav1d's round-to-nearest-ties-away-from-
    // zero via arithmetic shift; ported verbatim since it's unit-independent (round/shift on the
    // already-scaled product, before the unit-dependent clip).
    const CLIP: i32 = 0x3fff / 2;
    let round = |v: i32| ((v + 8192 + (v >> 31)) >> 14).clamp(-CLIP, CLIP);
    MotionVector::new(round(x), round(y))
}

/// dav1d `save_tmvs_c` (`refmvs.c:763-802`, spec 7.9 storage). `mfmv_sign[r]`: whether ref index
/// `r` (0..=6, this crate's `ref0` convention -- see `crate::tile::context`'s doc, `RefFrame`
/// discriminant minus 1) is a valid "forward-looking" reference for *this* frame (dav1d's
/// `mfmv_sign[i] = poc_diff < 0`, `refmvs.c:849` -- computed by the caller from `relative_dist`).
pub fn store_motion_field(
    coding_units: &[CodingUnit],
    frame_width: u32,
    frame_height: u32,
    mfmv_sign: &[bool; 7],
) -> MotionFieldGrid {
    let cols_8x8 = frame_width.div_ceil(8).max(1);
    let rows_8x8 = frame_height.div_ceil(8).max(1);
    // 4x4-unit-granularity index (not 8x8) so the bottom-right-of-the-2x2-group sample below can
    // land precisely on that specific 4x4 sub-position even when the 8x8 region straddles more
    // than one CU (real for any block smaller than 8x8, e.g. 4x4/4x8/8x4) -- an 8x8-granularity
    // index's "first CU wins" tiebreak (`CuSpatialIndex::new`'s doc) wouldn't reliably pick that
    // exact sub-position.
    let cols_4x4 = frame_width.div_ceil(4).max(1);
    let rows_4x4 = frame_height.div_ceil(4).max(1);
    let index = CuSpatialIndex::new(coding_units, cols_4x4, rows_4x4, 4, 4);

    let mut grid = MotionFieldGrid::empty(cols_8x8, rows_8x8);
    for y8 in 0..rows_8x8 {
        for x8 in 0..cols_8x8 {
            // Bottom-right 4x4 sub-block of the 2x2 group forming this 8x8 cell
            // (`save_tmvs_c`'s `b[x*2+1]`, `refmvs.c:773`).
            let gx = (2 * x8 + 1).min(cols_4x4 - 1);
            let gy = (2 * y8 + 1).min(rows_4x4 - 1);
            let Some(cu_idx) = index.get_cu_index(gx, gy) else {
                continue;
            };
            grid.set(x8, y8, saved_mv_for_cu(&coding_units[cu_idx], mfmv_sign));
        }
    }
    grid
}

/// Compound-`ref[1]`-preferred-over-`ref[0]` selection + sign/magnitude gating, `save_tmvs_c`'s
/// per-cell body (`refmvs.c:776-798`). Returns `None` for intra blocks and blocks whose eligible
/// ref fails the gate (dav1d's "empty" `ref=0` cell).
fn saved_mv_for_cu(cu: &CodingUnit, mfmv_sign: &[bool; 7]) -> Option<SavedMv> {
    // Magnitude gate is `< 4096` in dav1d's 1/8-pel units -- halved for this crate's quarter-pel
    // `MotionVector` (module doc).
    const MAG_LIMIT: i32 = 4096 / 2;
    let eligible = |rf: RefFrame, mv: MotionVector| -> Option<SavedMv> {
        if rf == RefFrame::Intra {
            return None;
        }
        let idx0 = rf as usize - 1; // RefFrame::Last(1)..=AltRef(7) -> 0..=6
        if !mfmv_sign[idx0] {
            return None;
        }
        if mv.x.unsigned_abs().max(mv.y.unsigned_abs()) >= MAG_LIMIT as u32 {
            return None;
        }
        Some(SavedMv { mv, ref_frame: rf })
    };
    eligible(cu.ref_frames[1], cu.mv[1]).or_else(|| eligible(cu.ref_frames[0], cu.mv[0]))
}

/// One selected temporal source, `dav1d_refmvs_init_frame`'s per-source computed state
/// (`refmvs.c:882-898`).
#[derive(Debug, Clone, Copy)]
pub struct MfmvSource {
    /// Physical DPB slot (0..=7) this source's saved grid comes from.
    pub slot: usize,
    /// That source's order-hint delta to the CURRENT frame (`mfmv_ref2cur`).
    pub ref2cur: i32,
    /// That source's OWN 7 references' order-hint deltas back to the source itself
    /// (`mfmv_ref2ref[n][0..7]`), indexed by the *stored block's* ref (0..=6). `0` = invalid/skip
    /// this cell (dav1d: `if (!ref2ref) continue;`, `refmvs.c:727`).
    pub ref2ref: [i32; 7],
}

/// Per-physical-DPB-slot state a sequential test harness threads across frames: this crate's
/// existing [`crate::frame_header_full::RefFrameState`] (order hints) plus the two additional
/// pieces spec 7.9/7.10 need that nothing in this crate tracked before this module: each slot's
/// *own* 7 reference order hints (snapshotted when that slot was last refreshed) and each slot's
/// saved [`MotionFieldGrid`].
#[derive(Debug, Clone, Default)]
pub struct MotionFieldState {
    pub ref_state: crate::frame_header_full::RefFrameState,
    /// slot -> that slot's own 7 refs' order hints, as they were when the frame that refreshed
    /// this slot was itself parsed.
    ref_ref_order_hint: [[u32; 7]; 8],
    grids: [Option<MotionFieldGrid>; 8],
}

impl MotionFieldState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Called once per frame, after that frame's CUs are parsed, for every slot set in
    /// `refresh_frame_flags`. `prev_ref_order_hint`: a snapshot of `self.ref_state`'s order hints
    /// taken by the caller *before* this frame's own `parse_frame_header_full` call (which, fed
    /// `&mut self.ref_state` directly per the existing threading pattern, already applies this
    /// frame's own `ref_order_hint` update as a side effect -- by the time this method runs,
    /// `self.ref_state` reflects the POST-update state, too late to read this frame's own
    /// resolved references from). This mirrors `dav1d_refmvs_init_frame`'s `ref_ref_poc` being the
    /// state as of when the frame owning a slot was itself decoded, not after.
    pub fn update(
        &mut self,
        prev_ref_order_hint: &[u32; 8],
        refresh_frame_flags: u8,
        ref_frame_idx: Option<&[u8; 7]>,
        grid: MotionFieldGrid,
    ) {
        let this_frame_ref_hints: [u32; 7] = match ref_frame_idx {
            Some(idx) => std::array::from_fn(|m| prev_ref_order_hint[idx[m] as usize]),
            None => [0; 7], // intra frame: no references, matches dav1d leaving ref_ref_poc unused
        };
        for i in 0..8 {
            if (refresh_frame_flags >> i) & 1 == 1 {
                self.ref_ref_order_hint[i] = this_frame_ref_hints;
                self.grids[i] = Some(grid.clone());
            }
        }
        // ref_state.ref_order_hint's own update already happened as a side effect of the caller's
        // parse_frame_header_full(..., &mut self.ref_state) call -- nothing to do here.
    }
}

/// `dav1d_refmvs_init_frame`'s source-selection loop (`refmvs.c:855-898`) -- selects up to 3
/// "mfmv" sources by priority (`last` unless `alt-of-last == gold`; then `bwd`/`altref2`/`altref`
/// each gated by a forward-in-time check; then `last2` as a final fallback), and computes each
/// selected source's `ref2cur`/`ref2ref`. `ref_frame_idx`: this frame's own slot mapping (`0..=6`
/// logical ref -> `0..=7` physical slot, real spec `ref_frame_idx[REFS_PER_FRAME]`).
/// `ref_order_hint`: a snapshot taken *before* this frame's own `parse_frame_header_full` call --
/// see [`MotionFieldState::update`]'s doc for why `state.ref_state` itself can't be used here.
pub fn select_motion_field_sources(
    state: &MotionFieldState,
    ref_order_hint: &[u32; 8],
    ref_frame_idx: &[u8; 7],
    cur_order_hint: u32,
    enable_order_hint: bool,
    order_hint_bits: u32,
) -> Vec<MfmvSource> {
    use crate::frame_header_full::relative_dist;
    let dist = |a: u32, b: u32| relative_dist(a, b, enable_order_hint, order_hint_bits);

    // Logical ref index -> physical slot, dav1d's `rp_ref[n]` naming: 0=LAST, 1=LAST2, 4=BWDREF,
    // 5=ALTREF2, 6=ALTREF (refmvs.c:859-880 references these fixed logical positions directly).
    let slot_of = |logical: usize| ref_frame_idx[logical] as usize;
    let has_grid = |slot: usize| state.grids[slot].is_some();

    let ref_poc = |logical: usize| ref_order_hint[slot_of(logical)];

    let mut candidates: Vec<usize> = Vec::with_capacity(3); // logical ref indices, priority order
    let mut total = 2usize;
    // last (0) unless alt-of-last == gold: dav1d compares this LAST slot's own ref[6] (its ALTREF)
    // order hint against the current frame's GOLD (logical 3) order hint (refmvs.c:859-861).
    let last_slot = slot_of(0);
    if has_grid(last_slot) && state.ref_ref_order_hint[last_slot][6] != ref_poc(3) {
        candidates.push(0);
        total = 3;
    }
    for &logical in &[4usize, 5, 6] {
        if candidates.len() >= total {
            break;
        }
        let slot = slot_of(logical);
        if has_grid(slot) && dist(ref_poc(logical), cur_order_hint) > 0 {
            candidates.push(logical);
        }
    }
    if candidates.len() < total && has_grid(slot_of(1)) {
        candidates.push(1); // last2 fallback
    }

    let mut sources = Vec::with_capacity(candidates.len());
    for logical in candidates {
        let slot = slot_of(logical);
        let rpoc = ref_poc(logical);
        let diff1 = dist(rpoc, cur_order_hint);
        if diff1.unsigned_abs() > 31 {
            continue; // INVALID_REF2CUR, refmvs.c:886-887
        }
        let diff1 = diff1 as i32;
        let ref2cur = if logical < 4 { -diff1 } else { diff1 };
        let ref2ref: [i32; 7] = std::array::from_fn(|m| {
            let diff2 = dist(rpoc, state.ref_ref_order_hint[slot][m]);
            if !(0..=31).contains(&diff2) {
                0
            } else {
                diff2 as i32
            }
        });
        sources.push(MfmvSource {
            slot,
            ref2cur,
            ref2ref,
        });
    }
    sources
}

/// One cell of a [`ProjectedMotionField`] -- **not** a final MV. Real dav1d's `load_tmvs_c`
/// (`refmvs.c:690-761`) uses `mv_projection` only to compute a *position* offset (where to place
/// this entry in the current frame's grid); the value it stores is the saved cell's *original,
/// unrescaled* MV (`rp_proj[...].mv = rb->mv`, `refmvs.c:741`) plus the `ref2ref` order-hint delta
/// that produced that offset (`rp_proj[...].ref = ref2ref`, `refmvs.c:742`) -- repurposed as the
/// denominator for a *second* `mv_projection` at candidate-consumption time
/// (`add_temporal_candidate`, `refmvs.c:201`, numerator = the current block's own ref's
/// `pocdiff`). The actual MV rescale happens exactly once, at consumption, not here.
#[derive(Debug, Clone, Copy)]
pub struct ProjectedMv {
    pub mv: MotionVector,
    pub ref2ref: i32,
}

#[derive(Debug, Clone)]
pub struct ProjectedMotionField {
    cells: Vec<Option<ProjectedMv>>,
    pub cols_8x8: u32,
    pub rows_8x8: u32,
}

impl ProjectedMotionField {
    fn empty(cols_8x8: u32, rows_8x8: u32) -> Self {
        Self {
            cells: vec![None; (cols_8x8 as usize) * (rows_8x8 as usize)],
            cols_8x8,
            rows_8x8,
        }
    }

    pub fn get(&self, x8: u32, y8: u32) -> Option<ProjectedMv> {
        if x8 >= self.cols_8x8 || y8 >= self.rows_8x8 {
            return None;
        }
        self.cells[(y8 * self.cols_8x8 + x8) as usize]
    }

    fn set(&mut self, x8: u32, y8: u32, value: ProjectedMv) {
        if x8 < self.cols_8x8 && y8 < self.rows_8x8 {
            self.cells[(y8 * self.cols_8x8 + x8) as usize] = Some(value);
        }
    }
}

/// dav1d `load_tmvs_c`'s per-source, per-cell projection (`refmvs.c:690-761`, spec 7.10
/// estimation) -- plain per-cell loop, not the identical-neighbor-run splat dav1d uses for
/// performance (an equivalent, simpler simplification for this crate's non-realtime use).
///
/// Scope note: dav1d also re-derives each cell's `bx8/by8`-block-size run so the *position*
/// offset is applied consistently across a whole run of identical neighboring cells
/// (`refmvs.c:744-757`) -- here every cell is projected independently, which is behaviorally
/// identical (each cell's own saved MV determines its own offset either way) and only differs in
/// CPU cost, not output.
pub fn project_motion_field(
    sources: &[MfmvSource],
    state: &MotionFieldState,
    cols_8x8: u32,
    rows_8x8: u32,
) -> ProjectedMotionField {
    let mut out = ProjectedMotionField::empty(cols_8x8, rows_8x8);
    for src in sources {
        let Some(saved) = &state.grids[src.slot] else {
            continue;
        };
        for y in 0..rows_8x8.min(saved.rows_8x8) {
            for x in 0..cols_8x8.min(saved.cols_8x8) {
                let Some(cell) = saved.get(x, y) else {
                    continue;
                };
                let ref2ref = src.ref2ref[cell.ref_frame as usize - 1];
                if ref2ref == 0 {
                    continue;
                }
                let offset = mv_projection(cell.mv, src.ref2cur, ref2ref);
                // Position offset: eighth-pel-to-8x8-cell shift is `>>6` in dav1d (8 px/cell * 8
                // subpel-units/px = 64). This crate's quarter-pel units: 8 px/cell * 4
                // subpel-units/px = 32 = `>>5`.
                let off_x = (offset.x.unsigned_abs() >> 5) as i32 * offset.x.signum();
                let off_y = (offset.y.unsigned_abs() >> 5) as i32 * offset.y.signum();
                let px = x as i32 + off_x;
                let py = y as i32 + off_y;
                if px < 0 || py < 0 || px >= cols_8x8 as i32 || py >= rows_8x8 as i32 {
                    continue;
                }
                out.set(
                    px as u32,
                    py as u32,
                    ProjectedMv {
                        mv: cell.mv,
                        ref2ref,
                    },
                );
            }
        }
    }
    out
}

/// dav1d `add_temporal_candidate` + its call site's main grid scan (`refmvs.c:193-237`,
/// `416-431`) -- the *second* (and only value-changing) `mv_projection`, using the current
/// block's own ref's `pocdiff` as numerator against the cell's stored `ref2ref` denominator.
/// Returns weight-2 candidates (spec: real dav1d dedups exact-MV matches by bumping weight
/// instead of pushing a duplicate -- callers should do the same, matching this crate's existing
/// `single_ref_mv_stack`/`push_mv_candidate` dedup behavior for spatial candidates).
///
/// `x8_start`/`y8_start`/`w8`/`h8`: the current block's footprint in 8x8 units (`by4>>1`/`bx4>>1`/
/// `imin((w4+1)>>1,8)`/`imin((h4+1)>>1,8)`, `refmvs.c:420-424`). `step_h`/`step_v`: `2` for blocks
/// >=16 4x4-units wide/tall, else `1` (`refmvs.c:423`).
///
/// Scope note: dav1d also samples up to 3 extra positions (bottom-left/bottom-right/top-right-ish,
/// tile/superblock-boundary-clamped) for blocks with `2 <= min(bw4,bh4)` and `max(bw4,bh4) < 16`
/// (`refmvs.c:432-451`) -- omitted here (documented, not silent): these only ever add further
/// low-weight (2) candidates on top of the main grid scan already covering the block's own
/// footprint, so omitting them narrows candidate-set completeness for medium-sized blocks without
/// affecting bit-position sync (weight-2 candidates never change `nearest_match`/`refmv_ctx`/
/// `newmv_ctx`, which only count spatial matches -- `dav1d_refmvs_find`'s doc in this crate's
/// `context.rs`).
pub fn add_temporal_candidates(
    projected: &ProjectedMotionField,
    pocdiff_ref0: i32,
    x8_start: u32,
    y8_start: u32,
    w8: u32,
    h8: u32,
    step_h: u32,
    step_v: u32,
) -> Vec<MotionVector> {
    let mut out = Vec::new();
    let mut y = 0;
    while y < h8 {
        let mut x = 0;
        while x < w8 {
            if let Some(cell) = projected.get(x8_start + x, y8_start + y) {
                out.push(mv_projection(cell.mv, pocdiff_ref0, cell.ref2ref));
            }
            x += step_h;
        }
        y += step_v;
    }
    out
}

/// Compound counterpart of [`add_temporal_candidates`] -- dav1d `add_temporal_candidate`'s
/// `ref.ref[1] != -1` branch (`refmvs.c:216-232`): the *same* grid cell is projected against
/// **both** refs' `pocdiff` (via the same [`mv_projection`] this crate already uses for the
/// single-ref case) and pushed as one joint pair, not two independent single MVs. Same scope note
/// as [`add_temporal_candidates`] (main grid scan only, no extra corner samples) and same
/// weight-2/caller-dedups-by-value contract.
pub fn add_temporal_compound_candidates(
    projected: &ProjectedMotionField,
    pocdiff_ref0: i32,
    pocdiff_ref1: i32,
    x8_start: u32,
    y8_start: u32,
    w8: u32,
    h8: u32,
    step_h: u32,
    step_v: u32,
) -> Vec<[MotionVector; 2]> {
    let mut out = Vec::new();
    let mut y = 0;
    while y < h8 {
        let mut x = 0;
        while x < w8 {
            if let Some(cell) = projected.get(x8_start + x, y8_start + y) {
                out.push([
                    mv_projection(cell.mv, pocdiff_ref0, cell.ref2ref),
                    mv_projection(cell.mv, pocdiff_ref1, cell.ref2ref),
                ]);
            }
            x += step_h;
        }
        y += step_v;
    }
    out
}
