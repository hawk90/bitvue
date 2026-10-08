//! The compound candidate stack (dav1d `refmvs_find`, compound branch).

use super::*;

impl SpatialRefContext {
    /// Real weighted compound DRL candidate stack (spec 7.10.2's `RefMvStack`, compound pairs --
    /// `single_ref_mv_stack`'s doc for the shared spatial/temporal weight structure this mirrors).
    /// Candidates are joint `[MotionVector; 2]` pairs from neighbors whose stored ref PAIR exactly
    /// matches `(ref0, ref1)` (`compound_candidate_mv`'s doc); if fewer than 2 such exact-pair
    /// matches are found, `fill_compound_extended_candidates` pads the rest -- see that function's
    /// doc for the (partial, sign-bias-less) extended-candidate fallback this runs.
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

        if cnt < 2 {
            self.fill_compound_extended_candidates(
                x4, y4, w4, h4, have_top, have_left, ref0, ref1, &mut stack, &mut cnt,
            );
        }

        (stack, cnt)
    }

    /// Partial port of rav1d's `add_compound_extended_candidate` (`refmvs.c:769-`) -- the cnt<2
    /// fallback `compound_mv_stack`'s doc describes. Scans the same top-row/left-col neighbor
    /// footprint as the main scan above, but (unlike `compound_candidate_mv`, which requires an
    /// EXACT ref-pair match) accepts any neighbor whose ref matches just ONE of our two refs
    /// individually -- e.g. a single-ref neighbor referencing only `ref0` still contributes its
    /// one MV to component 0's fill list even with `ref1` empty. Each of the two missing stack
    /// components is filled independently (up to 2 matches each), matching real spec/rav1d.
    ///
    /// **Narrower than rav1d**: rav1d also recycles a genuinely non-matching-ref neighbor's MV as
    /// a sign-flipped last-resort "diff" source (`ref_frame_sign_bias`, spec 5.9.14) before
    /// falling back to global motion. `ref_frame_sign_bias` is *derived* from cross-frame
    /// `RefOrderHint` state (`sign(relative_dist(RefOrderHint[ref], OrderHint))`) that this
    /// stateless single-frame parser doesn't carry -- same cross-frame-state gap as
    /// `crate::tile::motion_field`'s doc, not something a per-block fallback can source on its
    /// own. This omits that "diff" tier entirely and goes straight from same-ref matches to the
    /// global-motion fallback, which -- like `crate::tile::mv_prediction::MvPredictorContext::
    /// predict_global_mv`'s doc -- is approximated as zero (`gm_params` values are parsed for
    /// bit-position only, never stored by this crate). Pure value computation either way: doesn't
    /// affect bitstream position, only how close an already-under-populated (cnt<2, itself a rare
    /// edge case) stack slot's displayed/DRL-context-feeding MV is to the real decoder's.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn fill_compound_extended_candidates(
        &self,
        x4: u32,
        y4: u32,
        w4: u32,
        h4: u32,
        have_top: bool,
        have_left: bool,
        ref0: i8,
        ref1: i8,
        stack: &mut [CompoundMvStackEntry; 8],
        cnt: &mut usize,
    ) {
        let sz4 = w4.min(h4);
        let mut same = [[crate::tile::coding_unit::MotionVector::zero(); 2]; 2];
        let mut same_cnt = [0usize; 2];

        if have_top {
            let mut x = 0u32;
            while x < sz4 {
                let Some(cell) = self.cell(x4 + x, y4 - 1) else {
                    break;
                };
                Self::accumulate_extended_match(cell, ref0, ref1, &mut same, &mut same_cnt);
                x += (cell.width_4x4 as u32).max(1);
            }
        }
        if have_left {
            let mut y = 0u32;
            while y < sz4 {
                let Some(cell) = self.cell(x4 - 1, y4 + y) else {
                    break;
                };
                Self::accumulate_extended_match(cell, ref0, ref1, &mut same, &mut same_cnt);
                y += (cell.height_4x4 as u32).max(1);
            }
        }

        // Global-motion fallback (approximated as zero -- this function's own doc) for any
        // component that still has fewer than 2 same-ref matches.
        let ext0 = [same[0][0], same[1][0]];
        let ext1 = [same[0][1], same[1][1]];

        match *cnt {
            0 => {
                stack[0] = CompoundMvStackEntry {
                    mv: ext0,
                    weight: 2,
                };
                stack[1] = CompoundMvStackEntry {
                    mv: ext1,
                    weight: 2,
                };
                *cnt = 2;
            }
            1 => {
                // If the first extended candidate duplicates the already-real stack[0], use the
                // second extended candidate instead (rav1d: "if the first extended was the same
                // as the non-extended one, then replace it with the second extended one").
                let second = if stack[0].mv == ext0 { ext1 } else { ext0 };
                stack[1] = CompoundMvStackEntry {
                    mv: second,
                    weight: 2,
                };
                *cnt = 2;
            }
            _ => {}
        }
    }

    /// One neighbor cell's contribution to `fill_compound_extended_candidates`'s per-component
    /// same-ref fill lists -- see that function's doc.
    pub(super) fn accumulate_extended_match(
        cell: &SpatialRefCell,
        ref0: i8,
        ref1: i8,
        same: &mut [[crate::tile::coding_unit::MotionVector; 2]; 2],
        same_cnt: &mut [usize; 2],
    ) {
        if !cell.valid {
            return;
        }
        if cell.ref0 == ref0 && same_cnt[0] < 2 {
            same[0][same_cnt[0]] = cell.mv0;
            same_cnt[0] += 1;
        }
        if cell.ref0 == ref1 && same_cnt[1] < 2 {
            same[1][same_cnt[1]] = cell.mv0;
            same_cnt[1] += 1;
        }
        if cell.ref1 >= 0 {
            if cell.ref1 == ref0 && same_cnt[0] < 2 {
                same[0][same_cnt[0]] = cell.mv1;
                same_cnt[0] += 1;
            }
            if cell.ref1 == ref1 && same_cnt[1] < 2 {
                same[1][same_cnt[1]] = cell.mv1;
                same_cnt[1] += 1;
            }
        }
    }
}
