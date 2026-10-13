//! Per-slot motion field state and the choice of up to three temporal sources (spec 7.9/7.10).

use super::saved::MotionFieldGrid;

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
    /// Whether this source is one of the four references before the current frame in the
    /// reference list (LAST..GOLD, dav1d `ref - 4 < 0`): the projected position moves the
    /// opposite way for those.
    pub forward: bool,
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
    pub(super) ref_ref_order_hint: [[u32; 7]; 8],
    pub(super) grids: [Option<std::sync::Arc<MotionFieldGrid>>; 8],
    /// slot -> the CDFs the frame that last refreshed this slot saved (spec `save_cdfs`), which a
    /// later frame with that slot as its `primary_ref_frame` starts from.
    cdfs: [Option<std::sync::Arc<crate::symbol::CdfContext>>; 8],
}

impl MotionFieldState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Whether `slot` holds motion vectors a later frame could project.
    pub fn has_motion_field(&self, slot: u8) -> bool {
        self.grids[slot as usize & 7].is_some()
    }

    /// The CDFs saved in `slot`, if any frame has refreshed it.
    pub fn saved_cdf(&self, slot: u8) -> Option<&crate::symbol::CdfContext> {
        self.cdfs[slot as usize & 7].as_deref()
    }

    /// Saves `cdf` into every slot set in `refresh_frame_flags`.
    pub fn store_cdf(&mut self, refresh_frame_flags: u8, cdf: &crate::symbol::CdfContext) {
        let shared = std::sync::Arc::new(cdf.clone());
        for slot in 0..8 {
            if refresh_frame_flags & (1 << slot) != 0 {
                self.cdfs[slot] = Some(std::sync::Arc::clone(&shared));
            }
        }
    }

    /// What a `show_existing_frame` of a key frame in `slot` does to the other slots (spec 7.21,
    /// dav1d `obu.c`): they all take the shown frame's header state and CDFs, and none of them
    /// keeps motion vectors (`refmvs` is dropped).
    pub fn refresh_all_from(&mut self, slot: u8) {
        let slot = slot as usize & 7;
        self.ref_state.copy_slot_to_all(slot);
        let (cdf, hints) = (self.cdfs[slot].clone(), self.ref_ref_order_hint[slot]);
        for i in 0..8 {
            self.cdfs[i] = cdf.clone();
            self.ref_ref_order_hint[i] = hints;
            self.grids[i] = None;
        }
    }

    /// Called once per frame, after that frame's CUs are parsed, for every slot set in
    /// `refresh_frame_flags`. `prev_ref_order_hint`: a snapshot of `self.ref_state`'s order hints
    /// taken by the caller *before* this frame's own `parse_frame_header_full` call (which, fed
    /// `&mut self.ref_state` directly per the existing threading pattern, already applies this
    /// frame's own `ref_order_hint` update as a side effect -- by the time this method runs,
    /// `self.ref_state` reflects the POST-update state, too late to read this frame's own
    /// resolved references from). `has_motion`: whether the frame keeps motion vectors for later
    /// frames -- dav1d allocates them only for inter frames and frames with `allow_intrabc`, so a
    /// slot holding any other intra frame offers no temporal candidates (`rp_ref[i] == NULL`).
    /// This mirrors `dav1d_refmvs_init_frame`'s `ref_ref_poc` being the
    /// state as of when the frame owning a slot was itself decoded, not after.
    pub fn update(
        &mut self,
        prev_ref_order_hint: &[u32; 8],
        refresh_frame_flags: u8,
        ref_frame_idx: Option<&[u8; 7]>,
        grid: MotionFieldGrid,
        has_motion: bool,
    ) {
        let this_frame_ref_hints: [u32; 7] = match ref_frame_idx {
            Some(idx) => std::array::from_fn(|m| prev_ref_order_hint[idx[m] as usize]),
            None => [0; 7], // intra frame: no references, matches dav1d leaving ref_ref_poc unused
        };
        let grid = std::sync::Arc::new(grid);
        for i in 0..8 {
            if (refresh_frame_flags >> i) & 1 == 1 {
                self.ref_ref_order_hint[i] = this_frame_ref_hints;
                self.grids[i] = has_motion.then(|| std::sync::Arc::clone(&grid));
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
            forward: logical < 4,
        });
    }
    sources
}
