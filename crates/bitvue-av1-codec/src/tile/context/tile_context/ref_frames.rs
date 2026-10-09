//! `comp_mode`, `comp_ref_type` and `ref_frame` (single / unidirectional-compound) contexts.

use super::TileContext;

/// dav1d compares the reference with `(unsigned) ref >= 4U` in the two branches where one
/// neighbour is compound: an intra neighbour (-1) therefore counts as backward there, unlike in
/// the other branches, which compare signed.
fn is_backward_or_intra(ref0: i8) -> bool {
    (ref0 as u8) >= 4
}

impl TileContext {
    /// `comp_mode` context (0..=4) -- whether this block is likely single- or compound-reference,
    /// derived from the above/left neighbors' reference counts. Source: rav1d `get_comp_ctx`
    /// (`memorysafety/rav1d`, BSD-2-Clause, `src/env.rs`).
    pub fn comp_mode_context(&self, x4: u32, y4: u32) -> u8 {
        let have_top = self.has_top(y4);
        let have_left = self.has_left(x4);
        let xi = x4 as usize;
        let yi = y4 as usize;
        let a_comp = || self.above_ref_comp.get(xi).copied().unwrap_or(false);
        let l_comp = || self.left_ref_comp.get(yi).copied().unwrap_or(false);
        let a_ref0 = || self.above_ref0.get(xi).copied().unwrap_or(0);
        let l_ref0 = || self.left_ref0.get(yi).copied().unwrap_or(0);

        if have_top {
            if have_left {
                if a_comp() {
                    if l_comp() {
                        4
                    } else {
                        2 + u8::from(is_backward_or_intra(l_ref0()))
                    }
                } else if l_comp() {
                    2 + u8::from(is_backward_or_intra(a_ref0()))
                } else {
                    u8::from((l_ref0() >= 4) ^ (a_ref0() >= 4))
                }
            } else if a_comp() {
                3
            } else {
                u8::from(a_ref0() >= 4)
            }
        } else if have_left {
            if l_comp() {
                3
            } else {
                u8::from(l_ref0() >= 4)
            }
        } else {
            1
        }
    }

    /// `comp_ref_type` context (0..=4) -- unidirectional vs bidirectional compound likelihood.
    /// Source: rav1d `get_comp_dir_ctx` (`memorysafety/rav1d`, BSD-2-Clause, `src/env.rs`), the
    /// most heavily branched of the `ref_frame` context functions.
    pub fn comp_ref_type_context(&self, x4: u32, y4: u32) -> u8 {
        let have_top = self.has_top(y4);
        let have_left = self.has_left(x4);
        let xi = x4 as usize;
        let yi = y4 as usize;
        let (a_intra, a_comp, a_ref0, a_ref1) = (
            self.above_ref_intra.get(xi).copied().unwrap_or(true),
            self.above_ref_comp.get(xi).copied().unwrap_or(false),
            self.above_ref0.get(xi).copied().unwrap_or(0),
            self.above_ref1.get(xi).copied().unwrap_or(0),
        );
        let (l_intra, l_comp, l_ref0, l_ref1) = (
            self.left_ref_intra.get(yi).copied().unwrap_or(true),
            self.left_ref_comp.get(yi).copied().unwrap_or(false),
            self.left_ref0.get(yi).copied().unwrap_or(0),
            self.left_ref1.get(yi).copied().unwrap_or(0),
        );
        let has_uni_comp = |ref0: i8, ref1: i8| (ref0 < 4) == (ref1 < 4);

        if have_top && have_left {
            if a_intra && l_intra {
                return 2;
            }
            if a_intra || l_intra {
                let (edge_comp, edge_ref0, edge_ref1) = if a_intra {
                    (l_comp, l_ref0, l_ref1)
                } else {
                    (a_comp, a_ref0, a_ref1)
                };
                if !edge_comp {
                    return 2;
                }
                return 1 + 2 * u8::from(has_uni_comp(edge_ref0, edge_ref1));
            }

            if !a_comp && !l_comp {
                1 + 2 * u8::from((a_ref0 >= 4) == (l_ref0 >= 4))
            } else if !a_comp || !l_comp {
                let (edge_ref0, edge_ref1) = if a_comp {
                    (a_ref0, a_ref1)
                } else {
                    (l_ref0, l_ref1)
                };
                if !has_uni_comp(edge_ref0, edge_ref1) {
                    1
                } else {
                    3 + u8::from((a_ref0 >= 4) == (l_ref0 >= 4))
                }
            } else {
                let a_uni = has_uni_comp(a_ref0, a_ref1);
                let l_uni = has_uni_comp(l_ref0, l_ref1);
                if !a_uni && !l_uni {
                    0
                } else if !a_uni || !l_uni {
                    2
                } else {
                    3 + u8::from((a_ref0 == 4) == (l_ref0 == 4))
                }
            }
        } else if have_top || have_left {
            let (edge_intra, edge_comp, edge_ref0, edge_ref1) = if have_left {
                (l_intra, l_comp, l_ref0, l_ref1)
            } else {
                (a_intra, a_comp, a_ref0, a_ref1)
            };
            if edge_intra || !edge_comp {
                2
            } else {
                4 * u8::from(has_uni_comp(edge_ref0, edge_ref1))
            }
        } else {
            2
        }
    }

    /// Shared neighbor-counting core for `single_ref_p1`/`single_ref_p2`/`single_ref_p3`/
    /// `single_ref_p4`/`single_ref_p5`/`single_ref_p6` and their compound-CDF reuses
    /// (`uni_comp_ref`/`comp_ref`/`comp_ref_p1`/`comp_ref_p2`/`comp_bwdref`/`comp_bwdref_p1`).
    /// `bucket` maps a neighbor's raw rav1d-encoded ref value (0..=6) to `Some(bucket index)` to
    /// count, or `None` to ignore it (mirrors each rav1d function's own `if ref0 < N`/`>= N`/
    /// `^ N < N` guard before indexing its `cnt` array).
    fn ref_count_context(
        &self,
        x4: u32,
        y4: u32,
        num_buckets: usize,
        bucket: impl Fn(i8) -> Option<usize>,
    ) -> [u8; 4] {
        let have_top = self.has_top(y4);
        let have_left = self.has_left(x4);
        let xi = x4 as usize;
        let yi = y4 as usize;
        let mut cnt = [0u8; 4];
        debug_assert!(num_buckets <= 4);
        let above_intra = self.above_ref_intra.get(xi).copied().unwrap_or(true);
        let left_intra = self.left_ref_intra.get(yi).copied().unwrap_or(true);

        if have_top && !above_intra {
            if let Some(b) = bucket(self.above_ref0.get(xi).copied().unwrap_or(0)) {
                cnt[b] += 1;
            }
            if self.above_ref_comp.get(xi).copied().unwrap_or(false) {
                if let Some(b) = bucket(self.above_ref1.get(xi).copied().unwrap_or(0)) {
                    cnt[b] += 1;
                }
            }
        }
        if have_left && !left_intra {
            if let Some(b) = bucket(self.left_ref0.get(yi).copied().unwrap_or(0)) {
                cnt[b] += 1;
            }
            if self.left_ref_comp.get(yi).copied().unwrap_or(false) {
                if let Some(b) = bucket(self.left_ref1.get(yi).copied().unwrap_or(0)) {
                    cnt[b] += 1;
                }
            }
        }
        cnt
    }

    /// `single_ref_p1`/`uni_comp_ref` context (0..=2) -- forward vs backward reference-group
    /// likelihood. Source: rav1d `av1_get_ref_ctx` (`memorysafety/rav1d`, BSD-2-Clause,
    /// `src/env.rs`).
    pub fn single_ref_p1_context(&self, x4: u32, y4: u32) -> u8 {
        let cnt = self.ref_count_context(x4, y4, 2, |r| Some(usize::from(r >= 4)));
        cmp_counts(cnt[0], cnt[1])
    }

    /// `single_ref_p3`/`comp_ref` context (0..=2) -- among forward refs, LAST/LAST2 vs
    /// LAST3/GOLDEN. Source: rav1d `av1_get_fwd_ref_ctx` (`memorysafety/rav1d`, BSD-2-Clause,
    /// `src/env.rs`).
    pub fn single_ref_p3_context(&self, x4: u32, y4: u32) -> u8 {
        let cnt =
            self.ref_count_context(x4, y4, 4, |r| if r < 4 { Some(r as usize) } else { None });
        cmp_counts(cnt[0] + cnt[1], cnt[2] + cnt[3])
    }

    /// `single_ref_p4`/`comp_ref_p1` context (0..=2) -- LAST vs LAST2. Source: rav1d
    /// `av1_get_fwd_ref_1_ctx` (`memorysafety/rav1d`, BSD-2-Clause, `src/env.rs`).
    pub fn single_ref_p4_context(&self, x4: u32, y4: u32) -> u8 {
        let cnt =
            self.ref_count_context(x4, y4, 2, |r| if r < 2 { Some(r as usize) } else { None });
        cmp_counts(cnt[0], cnt[1])
    }

    /// `single_ref_p5`/`comp_ref_p2`/`uni_comp_ref_p2` context (0..=2) -- LAST3 vs GOLDEN.
    /// Source: rav1d `av1_get_fwd_ref_2_ctx` (`memorysafety/rav1d`, BSD-2-Clause, `src/env.rs`).
    pub fn single_ref_p5_context(&self, x4: u32, y4: u32) -> u8 {
        let cnt = self.ref_count_context(x4, y4, 2, |r| {
            if (r ^ 2) < 2 {
                Some((r - 2) as usize)
            } else {
                None
            }
        });
        cmp_counts(cnt[0], cnt[1])
    }

    /// `single_ref_p2`/`comp_bwdref` context (0..=2) -- among backward refs, BWDREF/ALTREF2 vs
    /// ALTREF. Source: rav1d `av1_get_bwd_ref_ctx` (`memorysafety/rav1d`, BSD-2-Clause,
    /// `src/env.rs`).
    pub fn single_ref_p2_context(&self, x4: u32, y4: u32) -> u8 {
        let cnt = self.ref_count_context(x4, y4, 3, |r| {
            if r >= 4 {
                Some((r - 4) as usize)
            } else {
                None
            }
        });
        cmp_counts(cnt[0] + cnt[1], cnt[2])
    }

    /// `single_ref_p6`/`comp_bwdref_p1` context (0..=2) -- BWDREF vs ALTREF2. Source: rav1d
    /// `av1_get_bwd_ref_1_ctx` (`memorysafety/rav1d`, BSD-2-Clause, `src/env.rs`).
    pub fn single_ref_p6_context(&self, x4: u32, y4: u32) -> u8 {
        let cnt = self.ref_count_context(x4, y4, 3, |r| {
            if r >= 4 {
                Some((r - 4) as usize)
            } else {
                None
            }
        });
        cmp_counts(cnt[0], cnt[1])
    }

    /// `uni_comp_ref_p1` context (0..=2) -- LAST2 vs LAST3/GOLDEN (grouped), among unidirectional
    /// compound candidates. Source: rav1d `av1_get_uni_p1_ctx` (`memorysafety/rav1d`,
    /// BSD-2-Clause, `src/env.rs`) -- the one function of the nine whose bucket count (3, indices
    /// 0..=2 for ref values 1..=3) doesn't match its `cnt` array width (also 3): unlike the other
    /// `ref_count_context` uses, out-of-range ref values here are silently dropped (rav1d's
    /// `cnt.get_mut(...)` returns `None` and the increment is skipped) rather than counted, since
    /// LAST (ref value 0) legitimately isn't part of this context's model at all.
    pub fn uni_comp_ref_p1_context(&self, x4: u32, y4: u32) -> u8 {
        let mut cnt = self.ref_count_context(x4, y4, 3, |r| {
            let idx = r - 1;
            if (0..3).contains(&idx) {
                Some(idx as usize)
            } else {
                None
            }
        });
        cnt[1] += cnt[2];
        cmp_counts(cnt[0], cnt[1])
    }
}

/// 3-way ordinal comparison of two neighbor reference-frame counts, used by every count-based
/// `ref_frame` context function above. Source: rav1d `cmp_counts` (`memorysafety/rav1d`,
/// BSD-2-Clause, `src/env.rs`).
fn cmp_counts(c1: u8, c2: u8) -> u8 {
    match c1.cmp(&c2) {
        std::cmp::Ordering::Less => 0,
        std::cmp::Ordering::Equal => 1,
        std::cmp::Ordering::Greater => 2,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ref_frame context tests. `ref0`/`ref1` values below use rav1d's raw 0..=6 encoding (see
    // `TileContext`'s struct doc): 0=LAST, 1=LAST2, 2=LAST3, 3=GOLDEN, 4=BWDREF, 5=ALTREF2,
    // 6=ALTREF.

    #[test]
    fn test_comp_mode_context_no_neighbors_returns_one() {
        // Neither have_top nor have_left -- rav1d's `get_comp_ctx` early-returns 1.
        let ctx = TileContext::new(16, 16);
        assert_eq!(ctx.comp_mode_context(0, 0), 1);
    }

    #[test]
    fn test_comp_mode_context_top_only_forward_ref_is_zero() {
        let mut ctx = TileContext::new(16, 16);
        ctx.set_ref_frames(0, 0, 4, 4, false, false, 0, -1); // LAST, single-ref
        assert_eq!(ctx.comp_mode_context(0, 5), 0); // above only, ref0 < 4
    }

    #[test]
    fn test_comp_mode_context_top_only_backward_ref_is_one() {
        let mut ctx = TileContext::new(16, 16);
        ctx.set_ref_frames(0, 0, 4, 4, false, false, 4, -1); // BWDREF, single-ref
        assert_eq!(ctx.comp_mode_context(0, 5), 1); // above only, ref0 >= 4
    }

    #[test]
    fn test_comp_mode_context_left_only_compound_is_three() {
        let mut ctx = TileContext::new(16, 16);
        ctx.set_ref_frames(0, 0, 4, 4, false, true, 0, 4);
        assert_eq!(ctx.comp_mode_context(5, 0), 3); // left only, compound
    }

    /// dav1d compares `(unsigned) ref >= 4U` where one neighbour is compound, so an intra
    /// neighbour (reference -1) counts as backward there: context 3, not 2. Where neither
    /// neighbour is compound the comparison is signed and an intra neighbour is not backward.
    #[test]
    fn test_comp_mode_context_intra_neighbour_beside_a_compound_one_counts_as_backward() {
        let mut ctx = TileContext::new(16, 16);
        ctx.set_ref_frames(5, 0, 4, 4, false, true, 0, 4); // above: compound
        ctx.set_ref_frames(0, 5, 4, 4, true, false, -1, -1); // left: intra
        assert_eq!(ctx.comp_mode_context(5, 5), 3);

        let mut ctx = TileContext::new(16, 16);
        ctx.set_ref_frames(5, 0, 4, 4, true, false, -1, -1); // above: intra
        ctx.set_ref_frames(0, 5, 4, 4, false, true, 0, 4); // left: compound
        assert_eq!(ctx.comp_mode_context(5, 5), 3);

        // Neither compound: intra (-1) is not >= 4, LAST is not either -> 0.
        let mut ctx = TileContext::new(16, 16);
        ctx.set_ref_frames(5, 0, 4, 4, true, false, -1, -1);
        ctx.set_ref_frames(0, 5, 4, 4, false, false, 0, -1);
        assert_eq!(ctx.comp_mode_context(5, 5), 0);
    }

    #[test]
    fn test_comp_mode_context_both_compound_returns_four() {
        let mut ctx = TileContext::new(16, 16);
        // To get both have_top and have_left true at the query position (5, 5), write the above
        // contribution at column x4=5 (via a block at (5, 0)) and the left contribution at row
        // y4=5 (via a block at (0, 5)) -- each call also writes the other array's irrelevant
        // range, which doesn't affect this query.
        ctx.set_ref_frames(5, 0, 4, 4, false, true, 0, 4); // above_comp[5..9]=true
        ctx.set_ref_frames(0, 5, 4, 4, false, true, 0, 4); // left_comp[5..9]=true
        assert_eq!(ctx.comp_mode_context(5, 5), 4);
    }

    #[test]
    fn test_comp_ref_type_context_no_neighbors_returns_two() {
        let ctx = TileContext::new(16, 16);
        assert_eq!(ctx.comp_ref_type_context(0, 0), 2);
    }

    #[test]
    fn test_comp_ref_type_context_both_intra_returns_two() {
        let mut ctx = TileContext::new(16, 16);
        // Same both-neighbor query-position trick as `test_comp_mode_context_both_compound...`.
        ctx.set_ref_frames(5, 0, 4, 4, true, false, 0, -1); // above_ref_intra[5..9]=true
        ctx.set_ref_frames(0, 5, 4, 4, true, false, 0, -1); // left_ref_intra[5..9]=true
        assert_eq!(ctx.comp_ref_type_context(5, 5), 2);
    }

    #[test]
    fn test_single_ref_p1_context_no_neighbors_is_tie() {
        // cmp_counts(0, 0) == 1 (Equal).
        let ctx = TileContext::new(16, 16);
        assert_eq!(ctx.single_ref_p1_context(0, 0), 1);
    }

    #[test]
    fn test_single_ref_p1_context_counts_forward_vs_backward() {
        let mut ctx = TileContext::new(16, 16);
        // Both-neighbor query-position trick (see `test_comp_mode_context_both_compound...`):
        // above contribution at column x4=5 (query x4), left contribution at row y4=5 (query y4).
        ctx.set_ref_frames(5, 0, 4, 4, false, false, 0, -1); // LAST (forward), above[5..9]
        ctx.set_ref_frames(0, 5, 4, 4, false, false, 4, -1); // BWDREF (backward), left[5..9]
                                                             // cnt[0]=1 (forward), cnt[1]=1 (backward) -> cmp_counts(1,1) == 1 (Equal).
        assert_eq!(ctx.single_ref_p1_context(5, 5), 1);
    }

    #[test]
    fn test_single_ref_p1_context_both_forward_is_greater() {
        let mut ctx = TileContext::new(16, 16);
        ctx.set_ref_frames(5, 0, 4, 4, false, false, 0, -1); // above[5..9]
        ctx.set_ref_frames(0, 5, 4, 4, false, false, 1, -1); // left[5..9]
                                                             // cnt[0]=2, cnt[1]=0 -> cmp_counts(2,0) == 2 (Greater).
        assert_eq!(ctx.single_ref_p1_context(5, 5), 2);
    }

    #[test]
    fn test_single_ref_p1_context_excludes_intra_neighbor() {
        let mut ctx = TileContext::new(16, 16);
        // An intra neighbor must not contribute to either bucket.
        ctx.set_ref_frames(0, 0, 4, 4, true, false, 0, -1);
        assert_eq!(ctx.single_ref_p1_context(0, 5), 1); // still a tie (0 vs 0)
    }

    #[test]
    fn test_single_ref_p3_context_groups_last_last2_vs_last3_golden() {
        let mut ctx = TileContext::new(16, 16);
        ctx.set_ref_frames(5, 0, 4, 4, false, false, 0, -1); // LAST -> bucket 0, above[5..9]
        ctx.set_ref_frames(0, 5, 4, 4, false, false, 3, -1); // GOLDEN -> bucket 3, left[5..9]
                                                             // cnt[0]+cnt[1]=1, cnt[2]+cnt[3]=1 -> tie.
        assert_eq!(ctx.single_ref_p3_context(5, 5), 1);
    }

    #[test]
    fn test_single_ref_p5_context_last3_vs_golden() {
        let mut ctx = TileContext::new(16, 16);
        ctx.set_ref_frames(0, 0, 4, 4, false, false, 3, -1); // GOLDEN -> bucket 1
                                                             // cnt[0]=0 (LAST3), cnt[1]=1 (GOLDEN) -> cmp_counts(0,1) == 0 (Less).
        assert_eq!(ctx.single_ref_p5_context(0, 5), 0);
    }

    #[test]
    fn test_uni_comp_ref_p1_context_groups_last2_vs_last3_golden() {
        let mut ctx = TileContext::new(16, 16);
        ctx.set_ref_frames(0, 0, 4, 4, false, false, 1, -1); // LAST2 -> bucket 0
                                                             // cnt[0]=1, cnt[1]=0 (bucket 1/2 empty, merged via cnt[1]+=cnt[2]) -> cmp_counts(1,0)=2.
        assert_eq!(ctx.uni_comp_ref_p1_context(0, 5), 2);
    }

    #[test]
    fn test_uni_comp_ref_p1_context_excludes_last() {
        let mut ctx = TileContext::new(16, 16);
        // LAST (ref value 0) isn't part of this context's model -- rav1d's `cnt.get_mut(-1)`
        // silently drops it (see `uni_comp_ref_p1_context`'s doc).
        ctx.set_ref_frames(0, 0, 4, 4, false, false, 0, -1);
        assert_eq!(ctx.uni_comp_ref_p1_context(0, 5), 1); // 0 vs 0, tie
    }

    #[test]
    fn test_start_superblock_row_resets_left_ref_state() {
        let mut ctx = TileContext::new(16, 16);
        // Query at x4=5 (have_left=true), y4=0 (have_top=false) so only the left contribution
        // matters; writing at (0, 0) sets left_ref0[0..4], covering the query's y4=0.
        ctx.set_ref_frames(0, 0, 4, 4, false, false, 4, -1); // BWDREF, left_ref0[0..4]=4
        assert_eq!(ctx.single_ref_p1_context(5, 0), 0); // Less: 0 forward vs 1 backward
        ctx.start_superblock_row();
        // After reset, left_ref_intra defaults back to `true` (see `start_superblock_row`'s doc),
        // so the stale backward-ref value at left_ref0[0] is excluded again -- back to a tie.
        assert_eq!(ctx.single_ref_p1_context(5, 0), 1);
    }
}
