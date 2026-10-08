//! Neighbour scans and the merge-by-value stack primitives (dav1d `scan_row` / `scan_col` / `add_spatial_candidate`).

use super::*;

impl SpatialRefContext {
    /// This position's single-ref candidate, if its stored ref matches `ref0` -- rav1d
    /// `add_spatial_candidate`'s `for n in 0..2 { if b.ref.ref[n] == ref.ref[0] { ... } }`: checks
    /// BOTH of the neighbor's own ref slots (a *compound* neighbor can still contribute to a
    /// *single-ref* query, via whichever of its two refs happens to match).
    pub(super) fn single_ref_candidate_mv(
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

    /// dav1d `add_spatial_candidate` (single reference): the neighbour's MV for `ref0`, if it has
    /// one in either slot, merged into the stack by value.
    pub(super) fn add_spatial_candidate(
        stack: &mut [MvStackEntry; 8],
        cnt: &mut usize,
        weight: i32,
        cell: &SpatialRefCell,
        ref0: i8,
    ) {
        if let Some(mv) = Self::single_ref_candidate_mv(cell, ref0) {
            Self::push_mv_candidate(stack, cnt, weight, mv);
        }
    }

    /// Merge-or-append one candidate into the real weighted DRL stack (rav1d
    /// `add_spatial_candidate`'s single-ref branch): identical-MV candidates accumulate weight
    /// instead of duplicating (matches by real MV *value*, not by which neighbor position found
    /// it).
    pub(super) fn push_mv_candidate(
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
    /// here at the spatial-scan stage (that only happens in the separate `cnt<2` fallback --
    /// `fill_compound_extended_candidates`'s doc).
    pub(super) fn compound_candidate_mv(
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
    pub(super) fn push_compound_mv_candidate(
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

    /// dav1d `scan_row` (`src/refmvs.c`): walks the neighbours along one row, stepping by each
    /// neighbour's own width, and calls `visit(neighbour, weight)` for each. A neighbour at least
    /// as wide as the block is visited once with a weight that grows with its height; narrower
    /// ones are visited one by one with weight `2 * overlap`. Returns how many rows the scan
    /// covered (`weight >> 1`, or 1), which gates the secondary scans. `step` is 4 for blocks
    /// 16 or more units wide, which skips small neighbours -- exactly as dav1d does.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn walk_row(
        &self,
        row_y4: u32,
        x4: u32,
        bw4: u32,
        w4: u32,
        max_rows: i32,
        step: u32,
        visit: &mut dyn FnMut(&SpatialRefCell, i32),
    ) -> u32 {
        let Some(first) = self.cell(x4, row_y4) else {
            return 1;
        };
        let mut cand = *first;
        let mut len = step.max(bw4.min(u32::from(cand.width_4x4).max(1)));

        if bw4 <= u32::from(cand.width_4x4).max(1) {
            let weight = if bw4 == 1 {
                2
            } else {
                u32::from(cand.height_4x4)
                    .min((2 * max_rows).max(0) as u32)
                    .max(2)
            };
            visit(&cand, (len * weight) as i32);
            return weight >> 1;
        }

        let mut x = 0u32;
        loop {
            visit(&cand, (len * 2) as i32);
            x += len;
            if x >= w4 {
                return 1;
            }
            let Some(next) = self.cell(x4 + x, row_y4) else {
                return 1;
            };
            cand = *next;
            len = step.max(u32::from(cand.width_4x4).max(1));
        }
    }

    /// dav1d `scan_col`: [`Self::walk_row`] transposed.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn walk_col(
        &self,
        col_x4: u32,
        y4: u32,
        bh4: u32,
        h4: u32,
        max_cols: i32,
        step: u32,
        visit: &mut dyn FnMut(&SpatialRefCell, i32),
    ) -> u32 {
        let Some(first) = self.cell(col_x4, y4) else {
            return 1;
        };
        let mut cand = *first;
        let mut len = step.max(bh4.min(u32::from(cand.height_4x4).max(1)));

        if bh4 <= u32::from(cand.height_4x4).max(1) {
            let weight = if bh4 == 1 {
                2
            } else {
                u32::from(cand.width_4x4)
                    .min((2 * max_cols).max(0) as u32)
                    .max(2)
            };
            visit(&cand, (len * weight) as i32);
            return weight >> 1;
        }

        let mut y = 0u32;
        loop {
            visit(&cand, (len * 2) as i32);
            y += len;
            if y >= h4 {
                return 1;
            }
            let Some(next) = self.cell(col_x4, y4 + y) else {
                return 1;
            };
            cand = *next;
            len = step.max(u32::from(cand.height_4x4).max(1));
        }
    }

    /// `walk_row` feeding the single-reference candidate stack.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn scan_row_weighted(
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
    ) -> u32 {
        self.walk_row(row_y4, x4, bw4, w4, max_rows, step, &mut |cell, weight| {
            Self::add_spatial_candidate(stack, cnt, weight, cell, ref0);
        })
    }

    /// `walk_col` feeding the single-reference candidate stack.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn scan_col_weighted(
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
    ) -> u32 {
        self.walk_col(col_x4, y4, bh4, h4, max_cols, step, &mut |cell, weight| {
            Self::add_spatial_candidate(stack, cnt, weight, cell, ref0);
        })
    }

    /// Compound counterpart of `scan_row_weighted` -- identical neighbor-width-aware stepping and
    /// weight math, only the per-cell candidate extraction differs (`compound_candidate_mv`'s
    /// exact-pair match instead of `single_ref_candidate_mv`'s single-ref match).
    #[allow(clippy::too_many_arguments)]
    pub(super) fn scan_row_weighted_compound(
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
    pub(super) fn scan_col_weighted_compound(
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
}
