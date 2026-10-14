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
        let have_left = self.has_left(x4);
        let have_top = self.has_top(y4);
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

    /// `mask_comp`'s context: a neighbour that is masked-compound (`comp_type` 3 = SEG, 4 =
    /// WEDGE) adds one, an ALTREF neighbour (`ref0 == 6`) adds three, a plain or averaged
    /// neighbour adds nothing; the sum is capped at 5.
    #[test]
    fn mask_comp_context_counts_masked_neighbours_and_altref() {
        let mut ctx = TileContext::new(16, 16);
        assert_eq!(ctx.mask_comp_context(5, 5), 0, "nothing decoded yet");

        // Above neighbour (x4 = 5): an averaged compound block, comp_type 2 -- still not masked.
        ctx.set_comp_type(5, 0, 4, 4, 2);
        assert_eq!(ctx.mask_comp_context(5, 5), 0);

        // SEG and WEDGE both count as masked.
        ctx.set_comp_type(5, 0, 4, 4, 3);
        assert_eq!(ctx.mask_comp_context(5, 5), 1, "SEG above");
        ctx.set_comp_type(5, 0, 4, 4, 4);
        assert_eq!(ctx.mask_comp_context(5, 5), 1, "WEDGE above");

        // Left neighbour (y4 = 5) masked too: both contribute.
        ctx.set_comp_type(0, 5, 4, 4, 4);
        assert_eq!(ctx.mask_comp_context(5, 5), 2);
    }

    #[test]
    fn mask_comp_context_adds_three_for_an_altref_neighbour_and_caps_at_five() {
        let mut ctx = TileContext::new(16, 16);
        // ALTREF (ref0 = 6) above, single reference: 3.
        ctx.set_ref_frames(5, 0, 4, 4, false, true, 6, -1);
        assert_eq!(ctx.mask_comp_context(5, 5), 3);

        // ALTREF on both sides: 3 + 3 = 6, capped at 5.
        ctx.set_ref_frames(0, 5, 4, 4, false, true, 6, -1);
        assert_eq!(ctx.mask_comp_context(5, 5), 5);

        // A masked neighbour outranks the ALTREF test on the same side: 1, not 1 + 3.
        let mut masked = TileContext::new(16, 16);
        masked.set_ref_frames(5, 0, 4, 4, false, true, 6, -1);
        masked.set_comp_type(5, 0, 4, 4, 3);
        assert_eq!(masked.mask_comp_context(5, 5), 1);
    }

    // ---- independent reference formulas (dav1d `get_*_ctx`, restated for the tests) ----

    fn reference_intra_ctx(have_left: bool, have_top: bool, left: bool, above: bool) -> u8 {
        let (l, a) = (u8::from(left), u8::from(above));
        match (have_left, have_top) {
            (true, true) => match l + a {
                2 => 3,
                n => n,
            },
            (true, false) => l * 2,
            (false, true) => a * 2,
            (false, false) => 0,
        }
    }

    fn reference_filter_ctx(a_filter: u8, l_filter: u8, comp: bool) -> u8 {
        let base = if comp { 4 } else { 0 };
        if a_filter == l_filter {
            base + a_filter
        } else if a_filter == 3 {
            base + l_filter
        } else if l_filter == 3 {
            base + a_filter
        } else {
            base + 3
        }
    }

    /// A context whose above neighbour at column 5 and left neighbour at row 5 were written
    /// by `fill`, with the tile origin at (1, 1) so that (5, 5) has both neighbours.
    fn ctx_16() -> TileContext {
        let mut ctx = TileContext::new(16, 16);
        ctx.tile_col_start = 1;
        ctx.tile_row_start = 1;
        ctx
    }

    // ---- intra flag ----

    #[test]
    fn unwritten_neighbours_read_as_intra() {
        let ctx = TileContext::new(8, 8);
        assert!(ctx.above_is_intra(0) && ctx.left_is_intra(0));
        assert!(ctx.above_is_intra(7) && ctx.left_is_intra(7));
        // Outside the tile's arrays: still intra, never a panic.
        assert!(ctx.above_is_intra(8) && ctx.left_is_intra(8));
        assert!(ctx.above_is_intra(u32::MAX) && ctx.left_is_intra(u32::MAX));
    }

    #[test]
    fn the_above_and_left_intra_readers_each_read_their_own_array() {
        let mut ctx = TileContext::new(8, 8);
        ctx.above_ref_intra[3] = false;
        assert!(!ctx.above_is_intra(3));
        assert!(
            ctx.left_is_intra(3),
            "the left array at the same index is untouched"
        );
        ctx.above_ref_intra[3] = true;
        ctx.left_ref_intra[4] = false;
        assert!(ctx.above_is_intra(4));
        assert!(!ctx.left_is_intra(4));
    }

    #[test]
    fn intra_ctx_matches_the_reference_for_every_edge_and_neighbour_combination() {
        for have_left in [false, true] {
            for have_top in [false, true] {
                for left in [false, true] {
                    for above in [false, true] {
                        // With the tile origin at (1, 1): index 2 has a neighbour, index 1 is on the edge.
                        let mut ctx = ctx_16();
                        let x4 = if have_left { 2 } else { 1 };
                        let y4 = if have_top { 2 } else { 1 };
                        ctx.above_ref_intra[x4 as usize] = above;
                        ctx.left_ref_intra[y4 as usize] = left;
                        assert_eq!(ctx.has_left(x4), have_left);
                        assert_eq!(ctx.has_top(y4), have_top);
                        assert_eq!(
                            ctx.intra_ctx(x4, y4),
                            reference_intra_ctx(have_left, have_top, left, above),
                            "left_nb={have_left} top_nb={have_top} left_intra={left} above_intra={above}"
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn intra_ctx_skips_the_value_2_only_when_both_neighbours_exist() {
        let mut ctx = ctx_16();
        ctx.set_intra_flag(5, 0, 1, 1, true);
        ctx.set_intra_flag(0, 5, 1, 1, true);
        // Written through the footprint API: above at column 5, left at row 5.
        ctx.above_ref_intra[5] = true;
        ctx.left_ref_intra[5] = true;
        assert_eq!(ctx.intra_ctx(5, 5), 3, "both intra: 2 folds to 3");
        ctx.set_intra_flag(5, 0, 1, 1, false);
        assert_eq!(ctx.intra_ctx(5, 5), 1, "only the left one intra");
        ctx.set_intra_flag(0, 5, 1, 1, false);
        assert_eq!(ctx.intra_ctx(5, 5), 0, "none intra");
        // Only the top neighbour exists (x4 on the tile's left edge): above * 2, the left flag is ignored.
        ctx.above_ref_intra[1] = true;
        ctx.left_ref_intra[5] = false;
        assert_eq!(ctx.intra_ctx(1, 5), 2);
        ctx.above_ref_intra[1] = false;
        ctx.left_ref_intra[5] = true;
        assert_eq!(ctx.intra_ctx(1, 5), 0);
        // Only the left neighbour exists (y4 on the tile's top edge): left * 2.
        ctx.left_ref_intra[1] = true;
        ctx.above_ref_intra[5] = false;
        assert_eq!(ctx.intra_ctx(5, 1), 2);
        // Neither neighbour exists, whatever is stored.
        assert_eq!(ctx.intra_ctx(1, 1), 0);
        // Beyond the arrays both flags read as intra.
        assert_eq!(ctx.intra_ctx(100, 100), 3);
    }

    #[test]
    fn set_intra_flag_writes_only_the_intra_arrays_over_exactly_the_footprint() {
        let mut ctx = TileContext::new(8, 8);
        ctx.set_intra_flag(2, 3, 2, 4, false);
        for x in 0..8usize {
            assert_eq!(ctx.above_ref_intra[x], !(2..4).contains(&x), "above {x}");
        }
        for y in 0..8usize {
            assert_eq!(ctx.left_ref_intra[y], !(3..7).contains(&y), "left {y}");
        }
        // Reference data is untouched, unlike `set_ref_frames`.
        assert!(ctx.above_ref_comp.iter().all(|&c| !c));
        assert!(ctx
            .above_ref0
            .iter()
            .chain(&ctx.above_ref1)
            .all(|&r| r == 0));
        assert!(ctx.left_ref0.iter().chain(&ctx.left_ref1).all(|&r| r == 0));
    }

    #[test]
    fn footprints_that_run_past_the_tile_are_clipped_and_empty_ones_write_nothing() {
        let mut ctx = TileContext::new(4, 4);
        ctx.set_intra_flag(2, 2, 100, 100, false);
        assert_eq!(ctx.above_ref_intra, vec![true, true, false, false]);
        assert_eq!(ctx.left_ref_intra, vec![true, true, false, false]);
        // Starting beyond the end, or zero-sized: nothing changes and nothing panics.
        let mut ctx = TileContext::new(4, 4);
        ctx.set_intra_flag(9, 9, 4, 4, false);
        ctx.set_intra_flag(1, 1, 0, 0, false);
        ctx.set_ref_frames(9, 9, 4, 4, false, true, 3, 4);
        ctx.set_ref_frames(1, 1, 0, 0, false, true, 3, 4);
        ctx.set_comp_type(9, 9, 4, 4, 4);
        ctx.set_comp_type(1, 1, 0, 0, 4);
        ctx.set_filter(9, 9, 4, 4, 0, 1);
        ctx.set_filter(1, 1, 0, 0, 1, 1);
        assert!(ctx
            .above_ref_intra
            .iter()
            .chain(&ctx.left_ref_intra)
            .all(|&i| i));
        assert!(ctx.above_ref0.iter().chain(&ctx.left_ref1).all(|&r| r == 0));
        assert!(ctx
            .above_comp_type
            .iter()
            .chain(&ctx.left_comp_type)
            .all(|&c| c == 0));
        assert!(ctx.above_filter[0]
            .iter()
            .chain(&ctx.left_filter[1])
            .all(|&f| f == 3));
    }

    // ---- reference frames and compound type ----

    #[test]
    fn set_ref_frames_writes_all_four_fields_over_the_footprint() {
        let mut ctx = TileContext::new(8, 8);
        ctx.set_ref_frames(1, 2, 2, 3, false, true, 4, 6);
        for x in 0..8usize {
            let inside = (1..3).contains(&x);
            assert_eq!(ctx.above_ref_intra[x], !inside, "above intra {x}");
            assert_eq!(ctx.above_ref_comp[x], inside, "above comp {x}");
            assert_eq!(
                ctx.above_ref0[x],
                if inside { 4 } else { 0 },
                "above ref0 {x}"
            );
            assert_eq!(
                ctx.above_ref1[x],
                if inside { 6 } else { 0 },
                "above ref1 {x}"
            );
        }
        for y in 0..8usize {
            let inside = (2..5).contains(&y);
            assert_eq!(ctx.left_ref_intra[y], !inside, "left intra {y}");
            assert_eq!(ctx.left_ref_comp[y], inside, "left comp {y}");
            assert_eq!(
                ctx.left_ref0[y],
                if inside { 4 } else { 0 },
                "left ref0 {y}"
            );
            assert_eq!(
                ctx.left_ref1[y],
                if inside { 6 } else { 0 },
                "left ref1 {y}"
            );
        }
        // A later single-reference write replaces them, with -1 for the missing second reference.
        ctx.set_ref_frames(1, 2, 2, 3, true, false, 0, -1);
        assert!(ctx.above_ref_intra[1] && !ctx.above_ref_comp[1]);
        assert_eq!((ctx.above_ref0[1], ctx.above_ref1[1]), (0, -1));
    }

    #[test]
    fn set_comp_type_writes_above_and_left_independently_of_the_reference_data() {
        let mut ctx = TileContext::new(8, 8);
        ctx.set_comp_type(1, 4, 2, 2, 3);
        assert_eq!(ctx.above_comp_type, vec![0, 3, 3, 0, 0, 0, 0, 0]);
        assert_eq!(ctx.left_comp_type, vec![0, 0, 0, 0, 3, 3, 0, 0]);
        assert!(ctx.above_ref0.iter().all(|&r| r == 0));
    }

    #[test]
    fn mask_comp_context_matches_the_reference_for_every_neighbour_type() {
        let side = |comp_type: u8, ref0: i8| -> u8 {
            if comp_type >= 3 {
                1
            } else if ref0 == 6 {
                3
            } else {
                0
            }
        };
        for a_type in 0..=4u8 {
            for l_type in 0..=4u8 {
                for a_ref in [-1i8, 0, 5, 6] {
                    for l_ref in [-1i8, 0, 5, 6] {
                        let mut ctx = TileContext::new(16, 16);
                        ctx.above_comp_type[5] = a_type;
                        ctx.left_comp_type[5] = l_type;
                        ctx.above_ref0[5] = a_ref;
                        ctx.left_ref0[5] = l_ref;
                        let expected = (side(a_type, a_ref) + side(l_type, l_ref)).min(5);
                        assert_eq!(
                            ctx.mask_comp_context(5, 5),
                            expected,
                            "above ({a_type},{a_ref}) left ({l_type},{l_ref})"
                        );
                    }
                }
            }
        }
        // Outside the arrays both sides read as plain neighbours.
        assert_eq!(TileContext::new(4, 4).mask_comp_context(50, 50), 0);
    }

    #[test]
    fn jnt_comp_context_matches_the_reference_for_every_neighbour_type_and_distance_relation() {
        let side = |comp_type: u8, ref0: i8| u8::from(comp_type >= 2 || ref0 == 6);
        for a_type in 0..=4u8 {
            for l_type in 0..=4u8 {
                for a_ref in [0i8, 5, 6] {
                    for l_ref in [0i8, 5, 6] {
                        for equal in [false, true] {
                            let mut ctx = TileContext::new(16, 16);
                            ctx.above_comp_type[5] = a_type;
                            ctx.left_comp_type[5] = l_type;
                            ctx.above_ref0[5] = a_ref;
                            ctx.left_ref0[5] = l_ref;
                            ctx.set_ref_order_distance([
                                3,
                                3,
                                0,
                                0,
                                0,
                                0,
                                if equal { 3 } else { 4 },
                            ]);
                            let expected =
                                3 * u8::from(equal) + side(a_type, a_ref) + side(l_type, l_ref);
                            assert_eq!(
                                ctx.jnt_comp_context(5, 5, 0, 6),
                                expected,
                                "above ({a_type},{a_ref}) left ({l_type},{l_ref}) equal {equal}"
                            );
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn jnt_comp_context_compares_distances_by_magnitude_and_clamps_out_of_range_references() {
        let mut ctx = TileContext::new(16, 16);
        ctx.set_ref_order_distance([5, 0, 0, 0, 0, 0, -5]);
        // |5| == |-5|.
        assert_eq!(ctx.jnt_comp_context(5, 5, 0, 6), 3);
        // A negative or too-large reference index is clamped to the first / last slot.
        assert_eq!(ctx.jnt_comp_context(5, 5, -1, 6), 3, "-1 reads slot 0");
        assert_eq!(ctx.jnt_comp_context(5, 5, 0, 99), 3, "99 reads slot 6");
        assert_eq!(ctx.jnt_comp_context(5, 5, 1, 6), 0, "0 vs 5");
        // Outside the neighbour arrays: plain neighbours.
        assert_eq!(ctx.jnt_comp_context(500, 500, 0, 6), 3);
    }

    // ---- interpolation filter ----

    #[test]
    fn set_filter_writes_one_direction_over_the_footprint_and_clamps_the_direction() {
        let mut ctx = TileContext::new(8, 8);
        ctx.set_filter(2, 1, 3, 2, 0, 1);
        assert_eq!(ctx.above_filter[0], vec![3, 3, 1, 1, 1, 3, 3, 3]);
        assert_eq!(ctx.left_filter[0], vec![3, 1, 1, 3, 3, 3, 3, 3]);
        assert!(ctx.above_filter[1]
            .iter()
            .chain(&ctx.left_filter[1])
            .all(|&f| f == 3));
        // Any direction above 1 is the vertical one.
        ctx.set_filter(0, 0, 1, 1, 7, 2);
        assert_eq!(ctx.above_filter[1][0], 2);
        assert_eq!(ctx.left_filter[1][0], 2);
        assert_eq!(ctx.above_filter[0][0], 3);
    }

    #[test]
    fn filter_context_matches_the_reference_for_every_filter_pair_and_reference_match() {
        for dir in 0..2usize {
            for comp in [false, true] {
                for a_filter in 0..=3u8 {
                    for l_filter in 0..=3u8 {
                        for a_match in [false, true] {
                            for l_match in [false, true] {
                                let mut ctx = TileContext::new(16, 16);
                                // Own reference is 2; a matching neighbour used it as ref0, a
                                // non-matching one used 4 and 5.
                                ctx.above_ref0[5] = if a_match { 2 } else { 4 };
                                ctx.above_ref1[5] = 5;
                                ctx.left_ref0[5] = if l_match { 2 } else { 4 };
                                ctx.left_ref1[5] = 5;
                                ctx.above_filter[dir][5] = a_filter;
                                ctx.left_filter[dir][5] = l_filter;
                                // The other direction holds different values and must not leak.
                                ctx.above_filter[1 - dir][5] = (a_filter + 1) % 3;
                                ctx.left_filter[1 - dir][5] = (l_filter + 2) % 3;
                                let a = if a_match { a_filter } else { 3 };
                                let l = if l_match { l_filter } else { 3 };
                                assert_eq!(
                                    ctx.filter_context(5, 5, comp, dir, 2),
                                    reference_filter_ctx(a, l, comp),
                                    "dir {dir} comp {comp} a {a_filter}/{a_match} l {l_filter}/{l_match}"
                                );
                            }
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn filter_context_accepts_a_match_on_the_neighbours_second_reference() {
        let mut ctx = TileContext::new(16, 16);
        ctx.above_ref0[5] = 1;
        ctx.above_ref1[5] = 2; // compound neighbour whose second reference is ours
        ctx.above_filter[0][5] = 1;
        assert_eq!(ctx.filter_context(5, 5, false, 0, 2), 1);
        assert_eq!(
            ctx.filter_context(5, 5, false, 0, 3),
            3,
            "no neighbour uses reference 3"
        );
        // The left neighbour matches through its first reference only.
        ctx.left_ref0[5] = 2;
        ctx.left_ref1[5] = -1;
        ctx.left_filter[0][5] = 1;
        assert_eq!(ctx.filter_context(5, 5, true, 0, 2), 4 + 1);
    }

    #[test]
    fn filter_context_outside_the_arrays_sees_no_matching_neighbour() {
        let ctx = TileContext::new(4, 4);
        assert_eq!(ctx.filter_context(40, 40, false, 0, 0), 3);
        assert_eq!(ctx.filter_context(40, 40, true, 1, 6), 7);
        // `dir` above 1 is the vertical direction.
        assert_eq!(
            ctx.filter_context(1, 1, false, 9, 0),
            ctx.filter_context(1, 1, false, 1, 0)
        );
    }

    // ---- matching single reference (motion_mode warp gate) ----

    #[test]
    fn has_matching_single_ref_needs_an_existing_inter_single_reference_neighbour_with_our_reference(
    ) {
        for have_top in [false, true] {
            for have_left in [false, true] {
                for a_ok in [false, true] {
                    for l_ok in [false, true] {
                        let mut ctx = TileContext::new(16, 16);
                        // `ok` neighbours: inter, single reference 3. Others differ in one way.
                        ctx.set_ref_frames(5, 0, 1, 1, false, false, if a_ok { 3 } else { 2 }, -1);
                        ctx.set_ref_frames(0, 5, 1, 1, false, false, if l_ok { 3 } else { 2 }, -1);
                        let expected = (have_top && a_ok) || (have_left && l_ok);
                        assert_eq!(
                            ctx.has_matching_single_ref(5, 5, have_top, have_left, 3),
                            expected,
                            "top {have_top} left {have_left} a_ok {a_ok} l_ok {l_ok}"
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn a_compound_or_intra_neighbour_never_counts_as_a_matching_single_reference() {
        let mut ctx = TileContext::new(16, 16);
        // Compound neighbours with a matching first reference.
        ctx.set_ref_frames(5, 0, 1, 1, false, true, 3, 4);
        ctx.set_ref_frames(0, 5, 1, 1, false, true, 3, 4);
        assert!(!ctx.has_matching_single_ref(5, 5, true, true, 3));
        // Intra neighbours that happen to hold the same reference number.
        ctx.set_ref_frames(5, 0, 1, 1, true, false, 3, -1);
        ctx.set_ref_frames(0, 5, 1, 1, true, false, 3, -1);
        assert!(!ctx.has_matching_single_ref(5, 5, true, true, 3));
        // Only one side qualifies: the other being intra / compound does not hide it.
        ctx.set_ref_frames(0, 5, 1, 1, false, false, 3, -1);
        assert!(ctx.has_matching_single_ref(5, 5, true, true, 3));
        assert!(!ctx.has_matching_single_ref(5, 5, true, false, 3));
    }

    #[test]
    fn has_matching_single_ref_outside_the_arrays_is_false() {
        let ctx = TileContext::new(4, 4);
        assert!(!ctx.has_matching_single_ref(50, 50, true, true, 0));
        assert!(!ctx.has_matching_single_ref(50, 50, true, true, -1));
    }
}
