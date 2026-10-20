//! The compound candidate stack (dav1d `refmvs_find`, compound branch).

use super::mode_context::{ScanExtent, ScanQuery};
use super::*;

type Stack = [CompoundMvStackEntry; 8];

impl SpatialRefContext {
    /// Real weighted compound candidate stack: a port of the compound branch of dav1d's
    /// `refmvs_find` with the same structure as [`Self::single_ref_mv_stack`] -- nearest group
    /// (top row, left column, top-right, bumped by +640), temporal pairs, top-left and the
    /// secondary rows/columns, each group sorted by weight, then the `cnt < 2` fallback
    /// ([`Self::fill_compound_extended_candidates`]) and the clamp to the frame. Candidates are
    /// joint pairs from neighbours whose stored reference PAIR equals `(ref0, ref1)` exactly.
    pub fn compound_mv_stack(
        &self,
        x4: u32,
        y4: u32,
        bw4: u32,
        bh4: u32,
        ref0: i8,
        ref1: i8,
        use_ref_frame_mvs: bool,
    ) -> (Stack, usize) {
        let query = ScanQuery {
            x4,
            y4,
            bw4,
            bh4,
            ref0,
            ref1,
        };
        let extent = self.scan_extent(&query);
        let mut stack = Stack::default();
        let mut cnt = 0usize;

        let (mut n_rows, mut n_cols) =
            self.scan_compound_primary(&query, &extent, &mut stack, &mut cnt);

        let nearest_cnt = cnt;
        for cand in &mut stack[..nearest_cnt] {
            cand.weight += 640;
        }

        self.add_temporal_pairs(&mut stack, &mut cnt, &query, &extent, use_ref_frame_mvs);
        self.scan_compound_top_left(&query, n_rows, n_cols, &mut stack, &mut cnt);
        self.scan_secondary(
            &query,
            &extent,
            &mut n_rows,
            &mut n_cols,
            &mut stack,
            &mut cnt,
        );

        // Sort the nearest group, then the rest (stable, descending).
        stack[..nearest_cnt].sort_by_key(|c| -c.weight);
        stack[nearest_cnt..cnt].sort_by_key(|c| -c.weight);

        if cnt < 2 {
            self.fill_compound_extended_candidates(
                x4,
                y4,
                extent.w4,
                extent.h4,
                n_rows != u32::MAX,
                n_cols != u32::MAX,
                ref0,
                ref1,
                &mut stack,
                &mut cnt,
            );
        }

        self.clamp_compound_stack_to_frame(&mut stack, cnt, x4, y4, bw4, bh4);

        (stack, cnt)
    }

    /// Pushes the cell's reference pair onto the stack with `weight` when it matches the query.
    fn add_compound_neighbour(
        q: &ScanQuery,
        stack: &mut Stack,
        cnt: &mut usize,
        cell: &SpatialRefCell,
        weight: i32,
    ) {
        if let Some(mv) = Self::compound_candidate_mv(cell, q.ref0, q.ref1) {
            Self::push_compound_mv_candidate(stack, cnt, weight, mv);
        }
    }

    /// The nearest group: the row above, the column to the left and the top-right cell (only a
    /// decoded cell can be there, dav1d's `EDGE_I444_TOP_HAS_RIGHT`). Returns `(n_rows, n_cols)`,
    /// each `u32::MAX` while the row/column is outside the tile.
    fn scan_compound_primary(
        &self,
        q: &ScanQuery,
        extent: &ScanExtent,
        stack: &mut Stack,
        cnt: &mut usize,
    ) -> (u32, u32) {
        let mut n_rows = u32::MAX;
        let mut n_cols = u32::MAX;
        if q.y4 > self.row_start {
            n_rows = self.walk_row(
                q.y4 - 1,
                q.x4,
                q.bw4,
                extent.w4,
                extent.max_rows,
                if q.bw4 >= 16 { 4 } else { 1 },
                &mut |cell, weight| Self::add_compound_neighbour(q, stack, cnt, cell, weight),
            );
        }
        if q.x4 > self.col_start {
            n_cols = self.walk_col(
                q.x4 - 1,
                q.y4,
                q.bh4,
                extent.h4,
                extent.max_cols,
                if q.bh4 >= 16 { 4 } else { 1 },
                &mut |cell, weight| Self::add_compound_neighbour(q, stack, cnt, cell, weight),
            );
        }
        if n_rows != u32::MAX && q.bw4.max(q.bh4) <= 16 && q.x4 + q.bw4 < self.col_end {
            if let Some(cell) = self.cell(q.x4 + q.bw4, q.y4 - 1) {
                Self::add_compound_neighbour(q, stack, cnt, cell, 4);
            }
        }
        (n_rows, n_cols)
    }

    /// The top-left cell -- only where both the row above and the column to the left exist.
    fn scan_compound_top_left(
        &self,
        q: &ScanQuery,
        n_rows: u32,
        n_cols: u32,
        stack: &mut Stack,
        cnt: &mut usize,
    ) {
        if n_rows != u32::MAX && n_cols != u32::MAX {
            if let Some(cell) = self.cell(q.x4 - 1, q.y4 - 1) {
                Self::add_compound_neighbour(q, stack, cnt, cell, 4);
            }
        }
    }

    /// Temporal pairs: one cell projected against both references.
    fn add_temporal_pairs(
        &self,
        stack: &mut Stack,
        cnt: &mut usize,
        q: &ScanQuery,
        extent: &ScanExtent,
        use_ref_frame_mvs: bool,
    ) {
        if use_ref_frame_mvs && q.ref0 >= 0 && q.ref1 >= 0 {
            if let Some(temporal) = &self.temporal {
                for [mv0, mv1] in crate::tile::motion_field::add_temporal_compound_candidates(
                    &temporal.projected,
                    temporal.pocdiff[q.ref0 as usize],
                    temporal.pocdiff[q.ref1 as usize],
                    crate::tile::motion_field::TemporalBlock {
                        bx4: q.x4,
                        by4: q.y4,
                        bw4: q.bw4,
                        bh4: q.bh4,
                        w4: extent.w4,
                        h4: extent.h4,
                        col_start: self.col_start,
                        col_end: self.col_end,
                        row_end: self.row_end,
                    },
                ) {
                    let fix = |mv| {
                        crate::tile::motion_field::fix_mv_precision(
                            mv,
                            temporal.allow_high_precision_mv,
                            temporal.force_integer_mv,
                        )
                    };
                    Self::push_compound_mv_candidate(stack, cnt, 2, [fix(mv0), fix(mv1)]);
                }
            }
        }
    }

    /// The secondary rows/columns 3 and 5 units back, skipped where an earlier scan already
    /// covered them; extends `n_rows`/`n_cols` by what they walked.
    fn scan_secondary(
        &self,
        q: &ScanQuery,
        extent: &ScanExtent,
        n_rows: &mut u32,
        n_cols: &mut u32,
        stack: &mut Stack,
        cnt: &mut usize,
    ) {
        for n in 2..=3u32 {
            if n > *n_rows && n as i32 <= extent.max_rows {
                *n_rows += self.walk_row(
                    (q.y4 + 1 - 2 * n) | 1,
                    q.x4 | 1,
                    q.bw4,
                    extent.w4,
                    1 + extent.max_rows - n as i32,
                    if q.bw4 >= 16 { 4 } else { 2 },
                    &mut |cell, weight| Self::add_compound_neighbour(q, stack, cnt, cell, weight),
                );
            }
            if n > *n_cols && n as i32 <= extent.max_cols {
                *n_cols += self.walk_col(
                    (q.x4 + 1 - 2 * n) | 1,
                    q.y4 | 1,
                    q.bh4,
                    extent.h4,
                    1 + extent.max_cols - n as i32,
                    if q.bh4 >= 16 { 4 } else { 2 },
                    &mut |cell, weight| Self::add_compound_neighbour(q, stack, cnt, cell, weight),
                );
            }
        }
    }

    /// Clamps every candidate to the frame.
    fn clamp_compound_stack_to_frame(
        &self,
        stack: &mut [CompoundMvStackEntry; 8],
        cnt: usize,
        x4: u32,
        y4: u32,
        bw4: u32,
        bh4: u32,
    ) {
        let left = -((x4 + bw4 + 4) as i32) * 32;
        let right = (self.iw4 as i32 - x4 as i32 + 4) * 32;
        let top = -((y4 + bh4 + 4) as i32) * 32;
        let bottom = (self.ih4 as i32 - y4 as i32 + 4) * 32;
        for cand in &mut stack[..cnt] {
            for mv in &mut cand.mv {
                mv.x = mv.x.clamp(left, right);
                mv.y = mv.y.clamp(top, bottom);
            }
        }
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
