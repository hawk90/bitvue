//! Projection of the selected sources onto the current frame (spec 7.10).

use super::state::{MfmvSource, MotionFieldState};
use crate::tile::coding_unit::MotionVector;

/// dav1d's `mv_projection` (`refmvs.c:175-191`) -- scales `mv` by the ratio `num/den` of two
/// order-hint deltas, spec 7.9.3's rounding/clip. `den` must be in `1..32`(dav1d asserts this --
/// callers here only ever pass a `pocdiff`/`ref2ref` value already clamped to that range, see
/// `select_motion_field_sources`).
pub(crate) fn mv_projection(mv: MotionVector, num: i32, den: i32) -> MotionVector {
    const DIV_MULT: [i32; 32] = [
        0, 16384, 8192, 5461, 4096, 3276, 2730, 2340, 2048, 1820, 1638, 1489, 1365, 1260, 1170,
        1092, 1024, 963, 910, 862, 819, 780, 744, 712, 682, 655, 630, 606, 585, 564, 546, 528,
    ];
    debug_assert!((1..32).contains(&den));
    debug_assert!((-32..32).contains(&num));
    let frac = num * DIV_MULT[den as usize];
    let y = mv.y * frac;
    let x = mv.x * frac;
    // Real dav1d clips to 0x3fff (1/8-pel units). `(v + 8192 + (v>>31)) >> 14` is dav1d's round-to-nearest-ties-away-from-
    // zero via arithmetic shift; ported verbatim since it's unit-independent (round/shift on the
    // already-scaled product, before the unit-dependent clip).
    const CLIP: i32 = 0x3fff;
    let round = |v: i32| ((v + 8192 + (v >> 31)) >> 14).clamp(-CLIP, CLIP);
    MotionVector::new(round(x), round(y))
}

/// One cell of a [`ProjectedMotionField`] -- **not** a final MV. Real dav1d's `load_tmvs_c`
/// (`refmvs.c:690-761`) uses `mv_projection` only to compute a *position* offset (where to place
/// this entry in the current frame's grid); the value it stores is the saved cell's *original,
/// unrescaled* MV (`rp_proj[...].mv = rb->mv`, `refmvs.c:741`) plus the `ref2ref` order-hint delta
/// that produced that offset (`rp_proj[...].ref = ref2ref`, `refmvs.c:742`) -- repurposed as the
/// denominator for a *second* `mv_projection` at candidate-consumption time
/// (`add_temporal_candidate`, `refmvs.c:201`, numerator = the current block's own ref's
/// `pocdiff`). The actual MV rescale happens exactly once, at consumption, not here.
#[derive(Debug, Clone, Copy)]
pub struct ProjectedMv {
    pub mv: MotionVector,
    pub ref2ref: i32,
}

#[derive(Debug, Clone)]
pub struct ProjectedMotionField {
    cells: Vec<Option<ProjectedMv>>,
    pub cols_8x8: u32,
    pub rows_8x8: u32,
}

impl ProjectedMotionField {
    pub(super) fn empty(cols_8x8: u32, rows_8x8: u32) -> Self {
        Self {
            cells: vec![None; (cols_8x8 as usize) * (rows_8x8 as usize)],
            cols_8x8,
            rows_8x8,
        }
    }

    pub fn get(&self, x8: u32, y8: u32) -> Option<ProjectedMv> {
        if x8 >= self.cols_8x8 || y8 >= self.rows_8x8 {
            return None;
        }
        self.cells[(y8 * self.cols_8x8 + x8) as usize]
    }

    pub(super) fn set(&mut self, x8: u32, y8: u32, value: ProjectedMv) {
        if x8 < self.cols_8x8 && y8 < self.rows_8x8 {
            self.cells[(y8 * self.cols_8x8 + x8) as usize] = Some(value);
        }
    }
}

/// dav1d `load_tmvs_c`'s per-source, per-cell projection (`refmvs.c:690-761`, spec 7.10
/// estimation) -- plain per-cell loop, not the identical-neighbor-run splat dav1d uses for
/// performance (an equivalent, simpler simplification for this crate's non-realtime use).
///
/// Scope note: dav1d also re-derives each cell's `bx8/by8`-block-size run so the *position*
/// offset is applied consistently across a whole run of identical neighboring cells
/// (`refmvs.c:744-757`) -- here every cell is projected independently, which is behaviorally
/// identical (each cell's own saved MV determines its own offset either way) and only differs in
/// CPU cost, not output.
pub fn project_motion_field(
    sources: &[MfmvSource],
    state: &MotionFieldState,
    cols_8x8: u32,
    rows_8x8: u32,
) -> ProjectedMotionField {
    let mut out = ProjectedMotionField::empty(cols_8x8, rows_8x8);
    for src in sources {
        let Some(saved) = &state.grids[src.slot] else {
            continue;
        };
        for y in 0..rows_8x8.min(saved.rows_8x8) {
            for x in 0..cols_8x8.min(saved.cols_8x8) {
                let Some(cell) = saved.get(x, y) else {
                    continue;
                };
                let ref2ref = src.ref2ref[cell.ref_frame as usize - 1];
                if ref2ref == 0 {
                    continue;
                }
                let offset = mv_projection(cell.mv, src.ref2cur, ref2ref);
                // Position offset: eighth-pel-to-8x8-cell shift is `>>6` in dav1d (8 px/cell * 8
                // subpel-units/px = 64).
                // dav1d `apply_sign(abs(offset) >> 6, offset ^ ref_sign)`: forward sources flip.
                let flip = if src.forward { -1 } else { 1 };
                let off_x = (offset.x.unsigned_abs() >> 6) as i32 * offset.x.signum() * flip;
                let off_y = (offset.y.unsigned_abs() >> 6) as i32 * offset.y.signum() * flip;
                let px = x as i32 + off_x;
                let py = y as i32 + off_y;
                // The projection stays inside the superblock row of its source cell (8 cells)
                // and at most one superblock (8 cells) to the left or 16 to the right of it
                // (`load_tmvs_c`: dav1d works one superblock row at a time).
                let (x_sb, y_sb) = ((x & !7) as i32, (y & !7) as i32);
                if py < y_sb
                    || py >= (y_sb + 8).min(rows_8x8 as i32)
                    || px < (x_sb - 8).max(0)
                    || px >= (x_sb + 16).min(cols_8x8 as i32)
                {
                    continue;
                }
                out.set(
                    px as u32,
                    py as u32,
                    ProjectedMv {
                        mv: cell.mv,
                        ref2ref,
                    },
                );
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tile::coding_unit::RefFrame;
    use crate::tile::motion_field::saved::{MotionFieldGrid, SavedMv};

    fn mv(x: i32, y: i32) -> MotionVector {
        MotionVector::new(x, y)
    }

    // ---- mv_projection ----

    /// Spec 7.9.3 `get_mv_projection`, with i64 arithmetic, `Round2Signed` and `Div_Mult[i] =
    /// 16384 / i` built here instead of copied from the table.
    fn reference_projection(v: i32, num: i32, den: i32) -> i32 {
        let div_mult = i64::from(16384 / den);
        let product = i64::from(v) * i64::from(num) * div_mult;
        let rounded = (product.abs() + 8192) >> 14;
        (product.signum() * rounded).clamp(-0x3fff, 0x3fff) as i32
    }

    #[test]
    fn mv_projection_matches_the_spec_formula_for_every_ratio() {
        let values = [
            0, 1, -1, 2, -2, 3, 7, -7, 8, 63, -64, 100, -255, 1000, -1001, 4094, -4095, 4095,
        ];
        for den in 1..32 {
            for num in -31..32 {
                for &x in &values {
                    for &y in &[0, 5, -4095, 4095] {
                        let got = mv_projection(mv(x, y), num, den);
                        assert_eq!(
                            (got.x, got.y),
                            (
                                reference_projection(x, num, den),
                                reference_projection(y, num, den)
                            ),
                            "mv ({x},{y}) * {num}/{den}"
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn mv_projection_keeps_the_components_apart() {
        let got = mv_projection(mv(100, -300), 1, 1);
        assert_eq!((got.x, got.y), (100, -300));
        let got = mv_projection(mv(100, -300), 2, 1);
        assert_eq!((got.x, got.y), (200, -600));
        let got = mv_projection(mv(100, -300), 1, 2);
        assert_eq!((got.x, got.y), (50, -150));
    }

    #[test]
    fn mv_projection_clamps_to_the_14_bit_range_on_both_signs() {
        // 4095 * 31 = 126945 -> far past 0x3fff.
        let got = mv_projection(mv(4095, -4095), 31, 1);
        assert_eq!((got.x, got.y), (0x3fff, -0x3fff));
        // Just inside the range is untouched: 8191 * 2 = 16382.
        let got = mv_projection(mv(8191, -8191), 2, 1);
        assert_eq!((got.x, got.y), (16382, -16382));
        // Exactly at the limit: 8192 * 2 = 16384 -> 16383.
        let got = mv_projection(mv(8192, -8192), 2, 1);
        assert_eq!((got.x, got.y), (0x3fff, -0x3fff));
    }

    #[test]
    fn mv_projection_by_zero_or_of_zero_is_zero_and_equal_ratios_keep_the_vector() {
        for den in 1..32 {
            let got = mv_projection(mv(1234, -987), 0, den);
            assert_eq!((got.x, got.y), (0, 0), "num 0 over {den}");
            let got = mv_projection(mv(0, 0), 17, den);
            assert_eq!((got.x, got.y), (0, 0), "zero vector over {den}");
            // A ratio of one: the reciprocal table is rounded down, so small vectors come back
            // unchanged and large ones lose at most `|v| * den / 16384` (rounded up).
            for v in [-8, -1, 1, 2, 8] {
                let got = mv_projection(mv(v, -v), den, den);
                assert_eq!((got.x, got.y), (v, -v), "{v} * {den}/{den}");
            }
            for v in [-4095, -3000, 3000, 4095] {
                let got = mv_projection(mv(v, -v), den, den);
                let slack = (v.abs() * den + 16383) / 16384;
                assert!(
                    (got.x.abs() - v.abs()).abs() <= slack,
                    "{v} * {den}/{den} -> {}",
                    got.x
                );
                assert!(got.x.abs() <= v.abs(), "never grows: {v} * {den}/{den}");
            }
        }
    }

    #[test]
    fn mv_projection_rounds_halves_away_from_zero_and_is_odd_symmetric() {
        // 1/2 of 3 = 1.5 -> 2, and of -3 = -1.5 -> -2.
        let got = mv_projection(mv(3, -3), 1, 2);
        assert_eq!((got.x, got.y), (2, -2));
        // 1/2 of 1 = 0.5 -> 1 (away from zero).
        let got = mv_projection(mv(1, -1), 1, 2);
        assert_eq!((got.x, got.y), (1, -1));
        for den in 1..32 {
            for num in 1..32 {
                for v in [1, 5, 13, 99, 1000, 4095] {
                    let pos = mv_projection(mv(v, v), num, den);
                    let neg = mv_projection(mv(-v, -v), num, den);
                    assert_eq!((neg.x, neg.y), (-pos.x, -pos.y), "{v} * {num}/{den}");
                    let flipped = mv_projection(mv(v, v), -num, den);
                    assert_eq!(
                        (flipped.x, flipped.y),
                        (-pos.x, -pos.y),
                        "{v} * -{num}/{den}"
                    );
                }
            }
        }
    }

    // ---- ProjectedMotionField ----

    fn projected(mv: MotionVector, ref2ref: i32) -> ProjectedMv {
        ProjectedMv { mv, ref2ref }
    }

    #[test]
    fn a_new_field_is_empty_and_set_get_are_row_major() {
        let mut field = ProjectedMotionField::empty(3, 2);
        assert_eq!((field.cols_8x8, field.rows_8x8), (3, 2));
        for y in 0..2 {
            for x in 0..3 {
                assert!(field.get(x, y).is_none());
            }
        }
        // Distinct values per cell so a transposed index is visible.
        for y in 0..2 {
            for x in 0..3 {
                field.set(
                    x,
                    y,
                    projected(mv(x as i32, y as i32), (y * 3 + x) as i32 + 1),
                );
            }
        }
        for y in 0..2 {
            for x in 0..3 {
                let cell = field.get(x, y).unwrap();
                assert_eq!((cell.mv.x, cell.mv.y), (x as i32, y as i32));
                assert_eq!(cell.ref2ref, (y * 3 + x) as i32 + 1);
            }
        }
    }

    #[test]
    fn reads_and_writes_outside_the_field_do_nothing() {
        let mut field = ProjectedMotionField::empty(3, 2);
        // A full field, so a missing bound on one axis would wrap into a neighbouring row or
        // run past the vector.
        for y in 0..2 {
            for x in 0..3 {
                field.set(x, y, projected(mv(1, 1), 1));
            }
        }
        for (x, y) in [(3, 0), (3, 1), (0, 2), (2, 2), (u32::MAX, 0), (0, u32::MAX)] {
            assert!(field.get(x, y).is_none(), "get ({x},{y})");
        }
        let before: Vec<_> = (0..2)
            .flat_map(|y| (0..3).map(move |x| (x, y)))
            .map(|(x, y)| field.get(x, y).map(|c| c.ref2ref))
            .collect();
        for (x, y) in [(3, 0), (0, 2), (3, 2), (u32::MAX, u32::MAX)] {
            field.set(x, y, projected(mv(9, 9), 9));
        }
        let after: Vec<_> = (0..2)
            .flat_map(|y| (0..3).map(move |x| (x, y)))
            .map(|(x, y)| field.get(x, y).map(|c| c.ref2ref))
            .collect();
        assert_eq!(before, after);
    }

    #[test]
    fn an_empty_field_has_no_cells() {
        let field = ProjectedMotionField::empty(0, 0);
        assert!(field.get(0, 0).is_none());
        let field = ProjectedMotionField::empty(4, 0);
        assert!(field.get(0, 0).is_none() && field.get(3, 0).is_none());
    }

    // ---- project_motion_field ----

    fn grid(cols: u32, rows: u32, cells: &[(u32, u32, MotionVector, RefFrame)]) -> MotionFieldGrid {
        let mut grid = MotionFieldGrid::empty(cols, rows);
        for &(x, y, mv, ref_frame) in cells {
            grid.set(x, y, Some(SavedMv { mv, ref_frame }));
        }
        grid
    }

    fn state_with(slots: &[(usize, MotionFieldGrid)]) -> MotionFieldState {
        let mut state = MotionFieldState::new();
        for (slot, grid) in slots {
            state.grids[*slot] = Some(std::sync::Arc::new(grid.clone()));
        }
        state
    }

    /// A source whose every reference has the same `ref2ref`, with `ref2cur == ref2ref` so the
    /// position offset equals the saved vector (a ratio of exactly 1/1 is exact in the table).
    fn source(slot: usize, forward: bool) -> MfmvSource {
        MfmvSource {
            slot,
            ref2cur: 1,
            ref2ref: [1; 7],
            forward,
        }
    }

    /// The cells with a value, as `(x, y, mv, ref2ref)`.
    fn filled(field: &ProjectedMotionField) -> Vec<(u32, u32, (i32, i32), i32)> {
        let mut out = Vec::new();
        for y in 0..field.rows_8x8 {
            for x in 0..field.cols_8x8 {
                if let Some(c) = field.get(x, y) {
                    out.push((x, y, (c.mv.x, c.mv.y), c.ref2ref));
                }
            }
        }
        out
    }

    #[test]
    fn without_sources_or_saved_grids_the_field_is_empty_but_sized() {
        let field = project_motion_field(&[], &MotionFieldState::new(), 5, 3);
        assert_eq!((field.cols_8x8, field.rows_8x8), (5, 3));
        assert!(filled(&field).is_empty());

        // A source whose slot holds no grid is skipped.
        let field = project_motion_field(&[source(2, false)], &MotionFieldState::new(), 5, 3);
        assert!(filled(&field).is_empty());
    }

    #[test]
    fn a_small_vector_projects_onto_its_own_cell_with_its_original_value() {
        // |offset| < 64 -> a zero cell shift; the stored vector is the saved one, not rescaled.
        let state = state_with(&[(0, grid(4, 4, &[(2, 1, mv(40, -60), RefFrame::Last)]))]);
        for forward in [false, true] {
            let field = project_motion_field(&[source(0, forward)], &state, 4, 4);
            assert_eq!(
                filled(&field),
                vec![(2, 1, (40, -60), 1)],
                "forward={forward}"
            );
        }
    }

    #[test]
    fn the_value_stored_is_the_saved_vector_and_the_ref2ref_not_the_projected_offset() {
        // ref2cur 2 over ref2ref 4: the offset is half the vector, but the cell keeps the vector.
        let state = state_with(&[(0, grid(4, 4, &[(1, 1, mv(256, 0), RefFrame::Last)]))]);
        let src = MfmvSource {
            slot: 0,
            ref2cur: 2,
            ref2ref: [4; 7],
            forward: false,
        };
        let field = project_motion_field(&[src], &state, 4, 4);
        // offset = 128 -> 2 cells right.
        assert_eq!(filled(&field), vec![(3, 1, (256, 0), 4)]);
    }

    #[test]
    fn backward_sources_move_with_the_vector_and_forward_sources_against_it() {
        let state = state_with(&[(
            0,
            grid(
                16,
                8,
                &[
                    (5, 3, mv(128, 64), RefFrame::Last),   // +2, +1
                    (5, 6, mv(-128, -64), RefFrame::Last), // -2, -1
                ],
            ),
        )]);
        let back = project_motion_field(&[source(0, false)], &state, 16, 8);
        assert_eq!(
            filled(&back),
            vec![(7, 4, (128, 64), 1), (3, 5, (-128, -64), 1)]
        );
        let fwd = project_motion_field(&[source(0, true)], &state, 16, 8);
        assert_eq!(
            filled(&fwd),
            vec![(3, 2, (128, 64), 1), (7, 7, (-128, -64), 1)]
        );
    }

    #[test]
    fn each_axis_shifts_by_its_own_component_in_whole_cells() {
        // 63 px-eighths is not yet a cell, 64 is one, 127 is still one, 128 is two.
        let state = state_with(&[(
            0,
            grid(
                16,
                8,
                &[
                    (4, 3, mv(63, 0), RefFrame::Last),
                    (5, 3, mv(64, 0), RefFrame::Last),
                    (6, 3, mv(127, 0), RefFrame::Last),
                    (7, 3, mv(128, 0), RefFrame::Last),
                    (4, 5, mv(0, 63), RefFrame::Last),
                    (5, 5, mv(0, 64), RefFrame::Last),
                    (6, 5, mv(0, 127), RefFrame::Last),
                    (7, 5, mv(0, 128), RefFrame::Last),
                ],
            ),
        )]);
        let field = project_motion_field(&[source(0, false)], &state, 16, 8);
        let targets: Vec<(u32, u32)> = filled(&field).iter().map(|c| (c.0, c.1)).collect();
        assert_eq!(
            targets,
            vec![
                (4, 3), // 63 -> 0
                (6, 3), // 64 -> +1 from x=5
                (7, 3), // 127 -> +1 from x=6
                (9, 3), // 128 -> +2 from x=7
                (4, 5), // 63 -> 0
                (5, 6), // 64 -> +1 from y=5
                (6, 6), // 127 -> +1 from y=5
                (7, 7), // 128 -> +2 from y=5
            ]
        );
    }

    #[test]
    fn the_reference_of_the_saved_cell_picks_its_ref2ref() {
        // Seven cells, one per reference, each with its own ref2ref; ref2ref 0 skips the cell.
        let refs = [
            RefFrame::Last,
            RefFrame::Last2,
            RefFrame::Last3,
            RefFrame::Golden,
            RefFrame::BwdRef,
            RefFrame::AltRef2,
            RefFrame::AltRef,
        ];
        let cells: Vec<_> = refs
            .iter()
            .enumerate()
            .map(|(i, &r)| (i as u32, 0, mv(8, 8), r))
            .collect();
        let state = state_with(&[(0, grid(8, 1, &cells))]);
        let src = MfmvSource {
            slot: 0,
            ref2cur: 1,
            ref2ref: [3, 5, 0, 7, 9, 11, 13],
            forward: false,
        };
        let field = project_motion_field(&[src], &state, 8, 1);
        assert_eq!(
            filled(&field),
            vec![
                (0, 0, (8, 8), 3),
                (1, 0, (8, 8), 5),
                // Last3 has ref2ref 0: skipped.
                (3, 0, (8, 8), 7),
                (4, 0, (8, 8), 9),
                (5, 0, (8, 8), 11),
                (6, 0, (8, 8), 13),
            ]
        );
    }

    /// Projects one cell at `(x, y)` of a big grid by `(dx, dy)` cells (backward source) and says
    /// whether it lands.
    fn lands(x: u32, y: u32, dx: i32, dy: i32, cols: u32, rows: u32) -> Option<(u32, u32)> {
        let state = state_with(&[(
            0,
            grid(cols, rows, &[(x, y, mv(dx * 64, dy * 64), RefFrame::Last)]),
        )]);
        let field = project_motion_field(&[source(0, false)], &state, cols, rows);
        filled(&field).first().map(|c| (c.0, c.1))
    }

    #[test]
    fn a_projection_stays_in_the_source_superblock_row() {
        // Source cell (10, 11) is in superblock row 1 (cells 8..16).
        assert_eq!(lands(10, 11, 0, -3, 40, 40), Some((10, 8)));
        assert_eq!(
            lands(10, 11, 0, -4, 40, 40),
            None,
            "one row above the superblock row"
        );
        assert_eq!(lands(10, 11, 0, 4, 40, 40), Some((10, 15)));
        assert_eq!(
            lands(10, 11, 0, 5, 40, 40),
            None,
            "one row below the superblock row"
        );
        // First superblock row: nothing above row 0.
        assert_eq!(lands(3, 0, 0, 7, 40, 40), Some((3, 7)));
        assert_eq!(lands(3, 0, 0, 8, 40, 40), None);
        assert_eq!(lands(3, 0, 0, -1, 40, 40), None);
    }

    #[test]
    fn a_projection_reaches_one_superblock_left_and_two_right() {
        // Source cell (17, 2) is in superblock column 2 (cells 16..24): window x in 8..32.
        assert_eq!(lands(17, 2, -9, 0, 64, 16), Some((8, 2)));
        assert_eq!(lands(17, 2, -10, 0, 64, 16), None);
        assert_eq!(lands(17, 2, 14, 0, 64, 16), Some((31, 2)));
        assert_eq!(lands(17, 2, 15, 0, 64, 16), None);
        // First superblock column: nothing left of column 0.
        assert_eq!(lands(2, 2, -2, 0, 64, 16), Some((0, 2)));
        assert_eq!(lands(2, 2, -3, 0, 64, 16), None);
    }

    #[test]
    fn a_projection_never_leaves_the_frame() {
        // 20 columns x 10 rows: window clipped by the frame, not only by the superblock.
        assert_eq!(lands(19, 9, 0, 0, 20, 10), Some((19, 9)));
        assert_eq!(lands(18, 8, 1, 1, 20, 10), Some((19, 9)));
        assert_eq!(lands(18, 8, 2, 0, 20, 10), None, "column 20 is outside");
        assert_eq!(lands(18, 8, 0, 2, 20, 10), None, "row 10 is outside");
    }

    #[test]
    fn a_later_cell_or_source_overwrites_an_earlier_one_on_the_same_target() {
        // Two cells of one source landing on (4, 1): scan order is rows then columns, so the
        // later one is (6, 1).
        let both = state_with(&[(
            0,
            grid(
                8,
                4,
                &[
                    (4, 1, mv(0, 0), RefFrame::Last),
                    (6, 1, mv(-128, 0), RefFrame::Last2),
                ],
            ),
        )]);
        let src = MfmvSource {
            slot: 0,
            ref2cur: 2,
            ref2ref: [1, 2, 3, 4, 5, 6, 7],
            forward: false,
        };
        let field = project_motion_field(&[src], &both, 8, 4);
        assert_eq!(filled(&field), vec![(4, 1, (-128, 0), 2)]);

        // Two sources: the second wins.
        let two = state_with(&[
            (0, grid(8, 4, &[(4, 1, mv(0, 0), RefFrame::Last)])),
            (1, grid(8, 4, &[(4, 1, mv(8, 0), RefFrame::Last)])),
        ]);
        let field = project_motion_field(&[source(0, false), source(1, false)], &two, 8, 4);
        assert_eq!(filled(&field), vec![(4, 1, (8, 0), 1)]);
        let field = project_motion_field(&[source(1, false), source(0, false)], &two, 8, 4);
        assert_eq!(filled(&field), vec![(4, 1, (0, 0), 1)]);
    }

    #[test]
    fn grids_of_a_different_size_are_read_only_where_both_have_cells() {
        // Saved grid 6x4, output 4x6: only the 4x4 overlap is read.
        let state = state_with(&[(
            0,
            grid(
                6,
                4,
                &[
                    (1, 1, mv(4, 4), RefFrame::Last),
                    (5, 1, mv(4, 4), RefFrame::Last),
                    (1, 3, mv(4, 4), RefFrame::Last),
                ],
            ),
        )]);
        let field = project_motion_field(&[source(0, false)], &state, 4, 6);
        assert_eq!(filled(&field), vec![(1, 1, (4, 4), 1), (1, 3, (4, 4), 1)]);

        // Output larger than the saved grid: no reads past it.
        let field = project_motion_field(&[source(0, false)], &state, 8, 8);
        assert_eq!(filled(&field).len(), 3);
    }

    /// Cells of a bigger saved grid outside the output are not read, even if their offset would
    /// bring them back inside it.
    #[test]
    fn saved_cells_beyond_the_output_are_not_projected_back_in() {
        let state = state_with(&[(
            0,
            grid(
                8,
                8,
                &[
                    (5, 1, mv(-128, 0), RefFrame::Last), // column 5 of a 4-column output
                    (1, 5, mv(0, -128), RefFrame::Last), // row 5 of a 4-row output
                    (2, 2, mv(0, 0), RefFrame::Last),    // inside: the one that is read
                ],
            ),
        )]);
        let field = project_motion_field(&[source(0, false)], &state, 4, 4);
        assert_eq!(filled(&field), vec![(2, 2, (0, 0), 1)]);
    }

    #[test]
    fn a_source_without_a_grid_does_not_stop_the_later_sources() {
        let state = state_with(&[(1, grid(4, 4, &[(2, 2, mv(8, 8), RefFrame::Last)]))]);
        let field = project_motion_field(&[source(0, false), source(1, false)], &state, 4, 4);
        assert_eq!(filled(&field), vec![(2, 2, (8, 8), 1)]);
    }

    #[test]
    fn the_superblock_window_follows_the_source_column_in_whole_superblocks() {
        // Columns 4..8 are still superblock column 0: the window ends at column 15 for them.
        assert_eq!(lands(6, 2, 9, 0, 64, 16), Some((15, 2)));
        assert_eq!(lands(6, 2, 10, 0, 64, 16), None);
        assert_eq!(lands(4, 2, 11, 0, 64, 16), Some((15, 2)));
        assert_eq!(lands(4, 2, 12, 0, 64, 16), None);
        // And the left edge of superblock column 1 starts at column 8, not column 4.
        assert_eq!(lands(10, 2, -10, 0, 64, 16), Some((0, 2)));
        assert_eq!(lands(10, 2, -11, 0, 64, 16), None);
    }

    #[test]
    fn cells_without_a_saved_vector_stay_empty() {
        let mut g = grid(4, 1, &[(1, 0, mv(8, 0), RefFrame::Last)]);
        g.set(2, 0, None);
        let state = state_with(&[(0, g)]);
        let field = project_motion_field(&[source(0, false)], &state, 4, 1);
        assert_eq!(filled(&field), vec![(1, 0, (8, 0), 1)]);
    }

    #[test]
    fn a_zero_sized_output_is_empty_and_never_panics() {
        let state = state_with(&[(0, grid(4, 4, &[(1, 1, mv(8, 8), RefFrame::Last)]))]);
        for (c, r) in [(0, 0), (0, 4), (4, 0)] {
            let field = project_motion_field(&[source(0, false)], &state, c, r);
            assert!(filled(&field).is_empty(), "{c}x{r}");
        }
    }

    #[test]
    fn random_inputs_never_panic_and_stay_inside_the_window() {
        let mut seed = 0x1234_5678_9abc_def0_u64;
        let mut next = move || {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            seed
        };
        let refs = [RefFrame::Last, RefFrame::Golden, RefFrame::AltRef];
        for _ in 0..200 {
            let (cols, rows) = ((next() % 40) as u32 + 1, (next() % 20) as u32 + 1);
            let cells: Vec<_> = (0..30)
                .map(|_| {
                    (
                        (next() % u64::from(cols)) as u32,
                        (next() % u64::from(rows)) as u32,
                        mv((next() % 8191) as i32 - 4095, (next() % 8191) as i32 - 4095),
                        refs[(next() % 3) as usize],
                    )
                })
                .collect();
            let state = state_with(&[(3, grid(cols, rows, &cells))]);
            let src = MfmvSource {
                slot: 3,
                ref2cur: (next() % 63) as i32 - 31,
                ref2ref: std::array::from_fn(|_| (next() % 31) as i32 + 1),
                forward: next() % 2 == 0,
            };
            let field = project_motion_field(&[src], &state, cols, rows);
            for (x, y, _, ref2ref) in filled(&field) {
                assert!(x < cols && y < rows && (1..32).contains(&ref2ref));
            }
        }
    }
}
