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

    /// The `cnt < 2` fallback of dav1d's `refmvs_find` for a compound query: neighbours on the top
    /// row and left column whose references only partly match are used per component, so the
    /// stack is padded to two entries. Port of `add_compound_extended_candidate` and the merge
    /// that follows it (`src/refmvs.c`):
    ///
    /// - a neighbour reference equal to `ref0` is a "same" candidate for component 0, and (turned
    ///   around when the two references' sign biases differ) a "diff" candidate for component 1;
    ///   a reference equal to `ref1` is the mirror image; any other reference is a "diff"
    ///   candidate for both components, turned around where the sign bias differs;
    /// - each component takes its "same" candidates first, then its "diff" ones, then the
    ///   global motion vector (zero here: no global-motion parameters are kept).
    ///
    /// Pure value computation: it does not move the bitstream position, but the values feed the
    /// candidates of later blocks and the stack's DRL contexts.
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
        let sign = [
            self.sign_bias[ref0 as usize % 7],
            self.sign_bias[ref1 as usize % 7],
        ];
        let mut lists = ExtendedLists::default();

        if have_top {
            let mut x = 0u32;
            while x < sz4 {
                let Some(cell) = self.cell(x4 + x, y4 - 1) else {
                    break;
                };
                self.add_compound_extended_candidate(&mut lists, cell, sign, ref0, ref1);
                x += (cell.width_4x4 as u32).max(1);
            }
        }
        if have_left {
            let mut y = 0u32;
            while y < sz4 {
                let Some(cell) = self.cell(x4 - 1, y4 + y) else {
                    break;
                };
                self.add_compound_extended_candidate(&mut lists, cell, sign, ref0, ref1);
                y += (cell.height_4x4 as u32).max(1);
            }
        }

        // Merge: per component, the "same" candidates, then the "diff" ones, then the global
        // motion vector.
        let zero = crate::tile::coding_unit::MotionVector::zero();
        let merge = |comp: usize| -> [crate::tile::coding_unit::MotionVector; 2] {
            let candidates = lists.same[..lists.same_count[comp]]
                .iter()
                .map(|c| c[comp])
                .chain(lists.diff[..lists.diff_count[comp]].iter().map(|c| c[comp]));
            let mut merged = [zero; 2];
            for (slot, mv) in merged.iter_mut().zip(candidates) {
                *slot = mv;
            }
            merged
        };
        let (comp0, comp1) = (merge(0), merge(1));
        let ext0 = [comp0[0], comp1[0]];
        let ext1 = [comp0[1], comp1[1]];

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
                // second one instead.
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

    /// One neighbour's contribution to the "same" and "diff" lists -- dav1d's
    /// `add_compound_extended_candidate`.
    fn add_compound_extended_candidate(
        &self,
        lists: &mut ExtendedLists,
        cell: &SpatialRefCell,
        sign: [bool; 2],
        ref0: i8,
        ref1: i8,
    ) {
        if !cell.valid {
            return;
        }
        for (cand_ref, cand_mv) in [(cell.ref0, cell.mv0), (cell.ref1, cell.mv1)] {
            if cand_ref < 0 {
                break;
            }
            let bias = self.sign_bias[cand_ref as usize % 7];
            let flipped = crate::tile::coding_unit::MotionVector::new(-cand_mv.x, -cand_mv.y);
            let turned = |comp: usize| if sign[comp] ^ bias { flipped } else { cand_mv };
            if cand_ref == ref0 {
                lists.push_same(0, cand_mv);
                lists.push_diff(1, turned(1));
            } else if cand_ref == ref1 {
                lists.push_same(1, cand_mv);
                lists.push_diff(0, turned(0));
            } else {
                lists.push_diff(0, turned(0));
                lists.push_diff(1, turned(1));
            }
        }
    }
}

/// The per-component candidate lists `fill_compound_extended_candidates` collects: up to two
/// "same" and two "diff" motion vectors for each of the two components.
#[derive(Default)]
struct ExtendedLists {
    same: [[crate::tile::coding_unit::MotionVector; 2]; 2], // [candidate][component]
    same_count: [usize; 2],
    diff: [[crate::tile::coding_unit::MotionVector; 2]; 2],
    diff_count: [usize; 2],
}

impl ExtendedLists {
    fn push_same(&mut self, comp: usize, mv: crate::tile::coding_unit::MotionVector) {
        if self.same_count[comp] < 2 {
            self.same[self.same_count[comp]][comp] = mv;
            self.same_count[comp] += 1;
        }
    }

    fn push_diff(&mut self, comp: usize, mv: crate::tile::coding_unit::MotionVector) {
        if self.diff_count[comp] < 2 {
            self.diff[self.diff_count[comp]][comp] = mv;
            self.diff_count[comp] += 1;
        }
    }
}
