//! The single-reference candidate stack (dav1d `refmvs_find`).

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
    /// 6. clamp to the frame, pad with the global MV (zero here: no global-motion parameters).
    ///
    /// Not covered: global-motion candidates (`mf & 1` blocks) and `fix_mv_precision` of temporal
    /// candidates -- both need frame-header state this context does not carry.
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
        let w4 = bw4.min(16).min(self.col_end.saturating_sub(x4)).max(1);
        let h4 = bh4.min(16).min(self.row_end.saturating_sub(y4)).max(1);

        // `n_rows`/`n_cols` stay `u32::MAX` while the row/column is outside the tile.
        let mut n_rows = u32::MAX;
        let mut n_cols = u32::MAX;
        let max_rows = if y4 > 0 {
            y4.div_ceil(2).min(2 + u32::from(bh4 > 1))
        } else {
            0
        } as i32;
        let max_cols = if x4 > 0 {
            x4.div_ceil(2).min(2 + u32::from(bw4 > 1))
        } else {
            0
        } as i32;

        if y4 > 0 {
            n_rows = self.scan_row_weighted(
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
        if x4 > 0 {
            n_cols = self.scan_col_weighted(
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
        // Top-right: only a decoded cell can be there (dav1d's `EDGE_I444_TOP_HAS_RIGHT`).
        if n_rows != u32::MAX && bw4.max(bh4) <= 16 && x4 + bw4 < self.col_end {
            if let Some(cell) = self.cell(x4 + bw4, y4 - 1) {
                Self::add_spatial_candidate(&mut stack, &mut cnt, 4, cell, ref0);
            }
        }

        let nearest_cnt = cnt;
        for cand in &mut stack[..nearest_cnt] {
            cand.weight += 640;
        }

        // Temporal candidates (`refmvs.c:416-452`).
        if use_ref_frame_mvs && ref0 >= 0 {
            if let Some(temporal) = &self.temporal {
                for (mv, _) in Self::temporal_samples(
                    temporal,
                    x4,
                    y4,
                    bw4,
                    bh4,
                    w4,
                    h4,
                    ref0,
                    self.col_end,
                    self.row_end,
                ) {
                    Self::push_mv_candidate(&mut stack, &mut cnt, 2, mv);
                }
            }
        }

        // Top-left, then the secondary rows/columns -- only where both edges exist.
        if n_rows != u32::MAX && n_cols != u32::MAX {
            if let Some(cell) = self.cell(x4 - 1, y4 - 1) {
                Self::add_spatial_candidate(&mut stack, &mut cnt, 4, cell, ref0);
            }
        }
        for n in 2..=3u32 {
            if n > n_rows && n as i32 <= max_rows {
                n_rows += self.scan_row_weighted(
                    &mut stack,
                    &mut cnt,
                    ref0,
                    (y4 + 1 - 2 * n) | 1,
                    x4 | 1,
                    bw4,
                    w4,
                    1 + max_rows - n as i32,
                    if bw4 >= 16 { 4 } else { 2 },
                );
            }
            if n > n_cols && n as i32 <= max_cols {
                n_cols += self.scan_col_weighted(
                    &mut stack,
                    &mut cnt,
                    ref0,
                    (x4 + 1 - 2 * n) | 1,
                    y4 | 1,
                    bh4,
                    h4,
                    1 + max_cols - n as i32,
                    if bh4 >= 16 { 4 } else { 2 },
                );
            }
        }

        // Sort the nearest group, then the rest. Stable and descending, like dav1d's bubble sort
        // (which only swaps on a strictly lower weight).
        stack[..nearest_cnt].sort_by_key(|c| -c.weight);
        stack[nearest_cnt..cnt].sort_by_key(|c| -c.weight);

        // Neighbours using other references, sign-flipped when that reference lies on the other
        // side of the current frame.
        if cnt < 2 && ref0 >= 0 {
            let sign = self.sign_bias[ref0 as usize];
            let sz4 = w4.min(h4);
            if n_rows != u32::MAX {
                let mut x = 0;
                while x < sz4 && cnt < 2 {
                    let Some(cell) = self.cell(x4 + x, y4 - 1) else {
                        break;
                    };
                    self.add_single_extended_candidate(&mut stack, &mut cnt, cell, sign);
                    x += u32::from(cell.width_4x4).max(1);
                }
            }
            if n_cols != u32::MAX {
                let mut y = 0;
                while y < sz4 && cnt < 2 {
                    let Some(cell) = self.cell(x4 - 1, y4 + y) else {
                        break;
                    };
                    self.add_single_extended_candidate(&mut stack, &mut cnt, cell, sign);
                    y += u32::from(cell.height_4x4).max(1);
                }
            }
        }

        // Clamp to the frame (1/8-sample units: 4 samples per 4x4 unit, 8 units per sample).
        let left = -((x4 + bw4 + 4) as i32) * 32;
        let right = (self.iw4 as i32 - x4 as i32 + 4) * 32;
        let top = -((y4 + bh4 + 4) as i32) * 32;
        let bottom = (self.ih4 as i32 - y4 as i32 + 4) * 32;
        for cand in &mut stack[..cnt] {
            cand.mv.x = cand.mv.x.clamp(left, right);
            cand.mv.y = cand.mv.y.clamp(top, bottom);
        }

        (stack, cnt)
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
