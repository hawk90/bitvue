//! The single-reference candidate stack (dav1d `refmvs_find`).

use super::neighbour_scan::{ScanExtent, ScanQuery};
use super::*;

impl SpatialRefContext {
    /// Real weighted single-ref candidate stack, a port of dav1d's `dav1d_refmvs_find` for a
    /// single reference (`src/refmvs.c`): `(stack, cnt)` where `cnt` (spec `NumMvFound`) gates the
    /// DRL bits and `stack[0..2]` are always safe to read (zero-filled past `cnt`, standing in for
    /// the global MV). Structure, in dav1d's order:
    ///
    /// 1. top row, left column, top-right neighbour -- the "nearest" group, bumped by +640 so
    ///    `get_drl_context` can tell it apart from the rest;
    /// 2. temporal candidates (weight 2);
    /// 3. top-left neighbour, then the "secondary" rows/columns 3 and 5 units back. These are
    ///    gated by `n_rows`/`n_cols` (how many rows/columns the previous scans already covered)
    ///    and read from the odd row/column of each 8x8 pair (`| 1`);
    /// 4. each group sorted by weight, descending, stably;
    /// 5. fewer than 2 candidates: neighbours using other references are added (sign-flipped by
    ///    `sign_bias`) with weight 2;
    /// 6. clamp to the frame, pad with the global MV.
    ///
    /// Global motion: `GLOBALMV` neighbours contribute the block's global MV instead of their own
    /// (see [`Self::add_spatial_candidate`]).
    pub fn single_ref_mv_stack(
        &self,
        x4: u32,
        y4: u32,
        bw4: u32,
        bh4: u32,
        ref0: i8,
        use_ref_frame_mvs: bool,
    ) -> ([MvStackEntry; 8], usize) {
        let gmv = self.global_candidate(ref0, x4, y4, bw4, bh4);
        let query = ScanQuery {
            x4,
            y4,
            bw4,
            bh4,
            ref0,
            ref1: -1,
            gmvs: [gmv, None],
        };
        let extent = self.scan_extent(&query);
        let mut stack = [MvStackEntry::default(); 8];
        let mut cnt = 0usize;

        let add =
            |stack: &mut [MvStackEntry; 8], cnt: &mut usize, cell: &SpatialRefCell, weight| {
                Self::add_spatial_candidate(stack, cnt, weight, cell, ref0, gmv)
            };

        let (mut n_rows, mut n_cols) =
            self.scan_nearest(&query, &extent, &mut |cell, weight, _| {
                add(&mut stack, &mut cnt, cell, weight)
            });

        let nearest_cnt = cnt;
        for cand in &mut stack[..nearest_cnt] {
            cand.weight += 640;
        }

        if use_ref_frame_mvs {
            self.add_single_temporal(&query, &extent, &mut stack, &mut cnt);
        }
        self.scan_above_left(&query, n_rows, n_cols, &mut |cell, weight, _| {
            add(&mut stack, &mut cnt, cell, weight)
        });
        self.scan_secondary(
            &query,
            &extent,
            &mut n_rows,
            &mut n_cols,
            &mut |cell, weight, _| add(&mut stack, &mut cnt, cell, weight),
        );

        // Sort the nearest group, then the rest. Stable and descending, like dav1d's bubble sort
        // (which only swaps on a strictly lower weight).
        stack[..nearest_cnt].sort_by_key(|c| -c.weight);
        stack[nearest_cnt..cnt].sort_by_key(|c| -c.weight);

        self.add_other_ref_candidates(
            &mut stack, &mut cnt, x4, y4, extent.w4, extent.h4, ref0, n_rows, n_cols,
        );

        self.clamp_stack_to_frame(&mut stack, cnt, &query);

        // Missing candidates read as the global MV (spec 7.10.2: `RefStackMv[idx][0] =
        // GlobalMvs[0]` for `idx` in `NumMvFound..2`); `cnt` still counts only real ones.
        let global = self.global_mv(ref0, x4, y4, bw4, bh4);
        for entry in &mut stack[cnt.min(2)..2] {
            entry.mv = global;
        }

        (stack, cnt)
    }

    /// Temporal candidates (`refmvs.c:416-452`), weight 2.
    fn add_single_temporal(
        &self,
        q: &ScanQuery,
        extent: &ScanExtent,
        stack: &mut [MvStackEntry; 8],
        cnt: &mut usize,
    ) {
        if q.ref0 < 0 {
            return;
        }
        let Some(temporal) = &self.temporal else {
            return;
        };
        for (mv, _) in Self::temporal_samples(
            temporal,
            q.x4,
            q.y4,
            q.bw4,
            q.bh4,
            extent.w4,
            extent.h4,
            q.ref0,
            self.col_start,
            self.col_end,
            self.row_end,
        ) {
            Self::push_mv_candidate(stack, cnt, 2, mv);
        }
    }

    /// Neighbours using other references, added with their motion vector sign-flipped when needed.
    #[allow(clippy::too_many_arguments)]
    fn add_other_ref_candidates(
        &self,
        stack: &mut [MvStackEntry; 8],
        cnt: &mut usize,
        x4: u32,
        y4: u32,
        w4: u32,
        h4: u32,
        ref0: i8,
        n_rows: u32,
        n_cols: u32,
    ) {
        // Neighbours using other references, sign-flipped when that reference lies on the other
        // side of the current frame.
        if *cnt < 2 && ref0 >= 0 {
            let sign = self.sign_bias[ref0 as usize];
            let sz4 = w4.min(h4);
            if n_rows != u32::MAX {
                let mut x = 0;
                while x < sz4 && *cnt < 2 {
                    let Some(cell) = self.cell(x4 + x, y4 - 1) else {
                        break;
                    };
                    self.add_single_extended_candidate(stack, cnt, cell, sign);
                    x += u32::from(cell.width_4x4).max(1);
                }
            }
            if n_cols != u32::MAX {
                let mut y = 0;
                while y < sz4 && *cnt < 2 {
                    let Some(cell) = self.cell(x4 - 1, y4 + y) else {
                        break;
                    };
                    self.add_single_extended_candidate(stack, cnt, cell, sign);
                    y += u32::from(cell.height_4x4).max(1);
                }
            }
        }
    }

    /// Clamps every candidate to the frame.
    fn clamp_stack_to_frame(&self, stack: &mut [MvStackEntry; 8], cnt: usize, q: &ScanQuery) {
        let (left, right, top, bottom) = self.mv_clamp_bounds(q);
        for cand in &mut stack[..cnt] {
            cand.mv.x = cand.mv.x.clamp(left, right);
            cand.mv.y = cand.mv.y.clamp(top, bottom);
        }
    }

    /// Projected, precision-fixed temporal samples for a block -- see
    /// `motion_field::add_temporal_candidates`. The flag marks the block's own top-left sample.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn temporal_samples(
        t: &TemporalMvContext,
        x4: u32,
        y4: u32,
        bw4: u32,
        bh4: u32,
        w4: u32,
        h4: u32,
        ref0: i8,
        col_start: u32,
        col_end: u32,
        row_end: u32,
    ) -> Vec<(crate::tile::coding_unit::MotionVector, bool)> {
        crate::tile::motion_field::add_temporal_candidates(
            &t.projected,
            t.pocdiff[ref0 as usize],
            crate::tile::motion_field::TemporalBlock {
                bx4: x4,
                by4: y4,
                bw4,
                bh4,
                w4,
                h4,
                col_start,
                col_end,
                row_end,
            },
        )
        .into_iter()
        .map(|(mv, first)| {
            (
                crate::tile::motion_field::fix_mv_precision(
                    mv,
                    t.allow_high_precision_mv,
                    t.force_integer_mv,
                ),
                first,
            )
        })
        .collect()
    }

    /// dav1d `add_single_extended_candidate`: a neighbour's MVs for any reference, negated when
    /// that reference's `sign_bias` differs from the target's, appended (weight 2) unless already
    /// on the stack.
    pub(super) fn add_single_extended_candidate(
        &self,
        stack: &mut [MvStackEntry; 8],
        cnt: &mut usize,
        cell: &SpatialRefCell,
        sign: bool,
    ) {
        for (cand_ref, cand_mv) in [(cell.ref0, cell.mv0), (cell.ref1, cell.mv1)] {
            if !cell.valid || cand_ref < 0 {
                break;
            }
            let mut mv = cand_mv;
            if sign != self.sign_bias[cand_ref as usize] {
                mv.x = -mv.x;
                mv.y = -mv.y;
            }
            if !stack[..*cnt].iter().any(|c| c.mv == mv) {
                stack[*cnt] = MvStackEntry { mv, weight: 2 };
                *cnt += 1;
            }
        }
    }
}
