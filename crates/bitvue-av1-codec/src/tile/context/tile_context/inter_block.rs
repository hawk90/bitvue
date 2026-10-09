//! Intra flag, compound type (`mask_comp` / `jnt_comp`) and interpolation-filter contexts.

use super::TileContext;

impl TileContext {
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

    /// The POC distances of the frame's references -- see `jnt_comp_context`.
    pub fn set_ref_order_distance(&mut self, distance: [i32; 7]) {
        self.ref_order_distance = distance;
    }

    /// `jnt_comp` context (0..=5), dav1d `get_jnt_comp_ctx`: 3 when both references are the same
    /// POC distance from the current frame, plus one for each neighbour that is compound-averaged
    /// or uses ALTREF.
    pub fn jnt_comp_context(&self, x4: u32, y4: u32, ref0: i8, ref1: i8) -> u8 {
        let a_comp_type = self.above_comp_type.get(x4 as usize).copied().unwrap_or(0);
        let l_comp_type = self.left_comp_type.get(y4 as usize).copied().unwrap_or(0);
        let a_ref0 = self.above_ref0.get(x4 as usize).copied().unwrap_or(0);
        let l_ref0 = self.left_ref0.get(y4 as usize).copied().unwrap_or(0);
        let a_ctx = u8::from(a_comp_type >= 2 || a_ref0 == 6);
        let l_ctx = u8::from(l_comp_type >= 2 || l_ref0 == 6);
        let distance = |r: i8| self.ref_order_distance[r.clamp(0, 6) as usize].unsigned_abs();
        let offset = u8::from(distance(ref0) == distance(ref1));
        3 * offset + a_ctx + l_ctx
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
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `jnt_comp`'s context starts at 3 when both references are the same POC distance from the
    /// frame (in either direction), and neighbours add one each.
    #[test]
    fn jnt_comp_context_adds_three_for_references_at_equal_poc_distance() {
        let mut ctx = TileContext::new(16, 16);
        // LAST is 2 frames before, BWDREF 2 after, ALTREF 4 after.
        ctx.set_ref_order_distance([-2, 0, 0, 0, 2, 0, 4]);

        assert_eq!(ctx.jnt_comp_context(0, 0, 0, 4), 3, "|-2| == |2|");
        assert_eq!(ctx.jnt_comp_context(0, 0, 0, 6), 0, "|-2| != |4|");

        // A compound-averaged neighbour above adds one.
        ctx.set_comp_type(5, 0, 4, 4, 2);
        ctx.set_ref_frames(5, 0, 4, 4, false, true, 0, 4);
        assert_eq!(ctx.jnt_comp_context(5, 5, 0, 4), 4);
    }
}
