//! The motion vectors a frame saves for later frames (spec 7.9 storage).

use crate::overlay_extraction::cu_spatial_index::CuSpatialIndex;
use crate::tile::coding_unit::{CodingUnit, MotionVector, RefFrame};

/// One saved candidate per 8x8 luma unit -- mirrors dav1d's `refmvs_temporal_block` (`refmvs.h:43-47`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SavedMv {
    pub mv: MotionVector,
    /// Real AV1 ref-frame value, `1..=7` (`RefFrame::Last..=RefFrame::AltRef`) -- never
    /// `RefFrame::Intra`, matching dav1d's `ref: u8` which is only ever written for inter blocks
    /// (`save_tmvs_c`, `refmvs.c:794-798` stores `ref = 0` i.e. "empty" otherwise, modeled here as
    /// `None` in the containing grid instead of a sentinel ref value).
    pub ref_frame: RefFrame,
}

/// 8x8-luma-unit-resolution grid, one [`SavedMv`] (or `None` = empty) per cell -- dav1d's
/// `rf->rp`/`rf->rp_proj` (`refmvs.h:81-84`), sized via `iw8`/`ih8` (`refmvs.c:819-822`).
#[derive(Debug, Clone)]
pub struct MotionFieldGrid {
    cells: Vec<Option<SavedMv>>,
    pub cols_8x8: u32,
    pub rows_8x8: u32,
}

impl MotionFieldGrid {
    pub(super) fn empty(cols_8x8: u32, rows_8x8: u32) -> Self {
        Self {
            cells: vec![None; (cols_8x8 as usize) * (rows_8x8 as usize)],
            cols_8x8,
            rows_8x8,
        }
    }

    pub fn get(&self, x8: u32, y8: u32) -> Option<SavedMv> {
        if x8 >= self.cols_8x8 || y8 >= self.rows_8x8 {
            return None;
        }
        self.cells[(y8 * self.cols_8x8 + x8) as usize]
    }

    pub(super) fn set(&mut self, x8: u32, y8: u32, value: Option<SavedMv>) {
        if x8 < self.cols_8x8 && y8 < self.rows_8x8 {
            self.cells[(y8 * self.cols_8x8 + x8) as usize] = value;
        }
    }
}

/// dav1d `save_tmvs_c` (`refmvs.c:763-802`, spec 7.9 storage). `mfmv_sign[r]`: whether ref index
/// `r` (0..=6, this crate's `ref0` convention -- see `crate::tile::context`'s doc, `RefFrame`
/// discriminant minus 1) is a valid "forward-looking" reference for *this* frame (dav1d's
/// `mfmv_sign[i] = poc_diff < 0`, `refmvs.c:849` -- computed by the caller from `relative_dist`).
pub fn store_motion_field(
    coding_units: &[CodingUnit],
    frame_width: u32,
    frame_height: u32,
    mfmv_sign: &[bool; 7],
) -> MotionFieldGrid {
    let cols_8x8 = frame_width.div_ceil(8).max(1);
    let rows_8x8 = frame_height.div_ceil(8).max(1);
    // 4x4-unit-granularity index (not 8x8) so the bottom-right-of-the-2x2-group sample below can
    // land precisely on that specific 4x4 sub-position even when the 8x8 region straddles more
    // than one CU (real for any block smaller than 8x8, e.g. 4x4/4x8/8x4) -- an 8x8-granularity
    // index's "first CU wins" tiebreak (`CuSpatialIndex::new`'s doc) wouldn't reliably pick that
    // exact sub-position.
    let cols_4x4 = frame_width.div_ceil(4).max(1);
    let rows_4x4 = frame_height.div_ceil(4).max(1);
    let index = CuSpatialIndex::new(coding_units, cols_4x4, rows_4x4, 4, 4);

    let mut grid = MotionFieldGrid::empty(cols_8x8, rows_8x8);
    for y8 in 0..rows_8x8 {
        for x8 in 0..cols_8x8 {
            // Bottom-right 4x4 sub-block of the 2x2 group forming this 8x8 cell
            // (`save_tmvs_c`'s `b[x*2+1]`, `refmvs.c:773`).
            let gx = (2 * x8 + 1).min(cols_4x4 - 1);
            let gy = (2 * y8 + 1).min(rows_4x4 - 1);
            let Some(cu_idx) = index.get_cu_index(gx, gy) else {
                continue;
            };
            grid.set(x8, y8, saved_mv_for_cu(&coding_units[cu_idx], mfmv_sign));
        }
    }
    grid
}

/// Compound-`ref[1]`-preferred-over-`ref[0]` selection + sign/magnitude gating, `save_tmvs_c`'s
/// per-cell body (`refmvs.c:776-798`). Returns `None` for intra blocks and blocks whose eligible
/// ref fails the gate (dav1d's "empty" `ref=0` cell).
fn saved_mv_for_cu(cu: &CodingUnit, mfmv_sign: &[bool; 7]) -> Option<SavedMv> {
    // Magnitude gate is `< 4096` in 1/8-pel units.
    const MAG_LIMIT: i32 = 4096;
    let eligible = |rf: RefFrame, mv: MotionVector| -> Option<SavedMv> {
        if rf == RefFrame::Intra {
            return None;
        }
        let idx0 = rf as usize - 1; // RefFrame::Last(1)..=AltRef(7) -> 0..=6
        if !mfmv_sign[idx0] {
            return None;
        }
        if mv.x.unsigned_abs().max(mv.y.unsigned_abs()) >= MAG_LIMIT as u32 {
            return None;
        }
        Some(SavedMv { mv, ref_frame: rf })
    };
    eligible(cu.ref_frames[1], cu.mv[1]).or_else(|| eligible(cu.ref_frames[0], cu.mv[0]))
}

#[cfg(test)]
mod tests {
    use super::*;

    const REFS: [RefFrame; 7] = [
        RefFrame::Last,
        RefFrame::Last2,
        RefFrame::Last3,
        RefFrame::Golden,
        RefFrame::BwdRef,
        RefFrame::AltRef2,
        RefFrame::AltRef,
    ];

    fn mv(x: i32, y: i32) -> MotionVector {
        MotionVector::new(x, y)
    }

    fn inter(x: u32, y: u32, w: u32, h: u32, rf: RefFrame, v: MotionVector) -> CodingUnit {
        CodingUnit {
            ref_frames: [rf, RefFrame::Intra],
            mv: [v, MotionVector::zero()],
            ..CodingUnit::new(x, y, w, h)
        }
    }

    fn compound(
        x: u32,
        y: u32,
        w: u32,
        h: u32,
        refs: [RefFrame; 2],
        mvs: [MotionVector; 2],
    ) -> CodingUnit {
        CodingUnit {
            ref_frames: refs,
            mv: mvs,
            ..CodingUnit::new(x, y, w, h)
        }
    }

    const ALL_SIGNS: [bool; 7] = [true; 7];

    fn cell(grid: &MotionFieldGrid, x: u32, y: u32) -> Option<(i32, i32, RefFrame)> {
        grid.get(x, y).map(|s| (s.mv.x, s.mv.y, s.ref_frame))
    }

    // ---- MotionFieldGrid ----

    #[test]
    fn grid_cells_are_row_major_and_outside_reads_and_writes_do_nothing() {
        let mut grid = MotionFieldGrid::empty(3, 2);
        assert_eq!((grid.cols_8x8, grid.rows_8x8), (3, 2));
        for y in 0..2 {
            for x in 0..3 {
                grid.set(
                    x,
                    y,
                    Some(SavedMv {
                        mv: mv(x as i32, y as i32),
                        ref_frame: REFS[(y * 3 + x) as usize],
                    }),
                );
            }
        }
        for y in 0..2 {
            for x in 0..3 {
                assert_eq!(
                    cell(&grid, x, y),
                    Some((x as i32, y as i32, REFS[(y * 3 + x) as usize]))
                );
            }
        }
        for (x, y) in [(3, 0), (3, 1), (0, 2), (2, 2), (u32::MAX, 0), (0, u32::MAX)] {
            assert_eq!(cell(&grid, x, y), None, "get ({x},{y})");
            grid.set(x, y, None);
        }
        // Writes outside changed nothing, and `None` clears a cell.
        assert_eq!(cell(&grid, 2, 1), Some((2, 1, RefFrame::AltRef2)));
        grid.set(2, 1, None);
        assert_eq!(cell(&grid, 2, 1), None);
        assert_eq!(cell(&grid, 1, 1), Some((1, 1, RefFrame::BwdRef)));
    }

    // ---- grid size ----

    #[test]
    fn the_grid_has_one_cell_per_eight_pixels_rounded_up_and_never_less_than_one() {
        for (w, h, cols, rows) in [
            (64, 64, 8, 8),
            (65, 64, 9, 8),
            (64, 65, 8, 9),
            (8, 8, 1, 1),
            (9, 1, 2, 1),
            (1, 9, 1, 2),
            (1, 1, 1, 1),
            (0, 0, 1, 1),
            (0, 16, 1, 2),
            (16, 0, 2, 1),
        ] {
            let grid = store_motion_field(&[], w, h, &ALL_SIGNS);
            assert_eq!((grid.cols_8x8, grid.rows_8x8), (cols, rows), "{w}x{h}");
            for y in 0..rows {
                for x in 0..cols {
                    assert_eq!(cell(&grid, x, y), None, "{w}x{h} ({x},{y}) with no units");
                }
            }
        }
    }

    // ---- which block names a cell ----

    #[test]
    fn a_cell_takes_the_bottom_right_4x4_of_its_8x8() {
        // Four 4x4 units in the first 8x8 cell with different vectors and references.
        let units = [
            inter(0, 0, 4, 4, RefFrame::Last, mv(1, 1)),
            inter(4, 0, 4, 4, RefFrame::Last2, mv(2, 2)),
            inter(0, 4, 4, 4, RefFrame::Last3, mv(3, 3)),
            inter(4, 4, 4, 4, RefFrame::Golden, mv(4, 4)),
        ];
        let grid = store_motion_field(&units, 8, 8, &ALL_SIGNS);
        assert_eq!(cell(&grid, 0, 0), Some((4, 4, RefFrame::Golden)));
    }

    #[test]
    fn each_cell_is_sampled_from_its_own_8x8_and_not_a_neighbours() {
        let units = [
            inter(0, 0, 8, 8, RefFrame::Last, mv(10, 0)),
            inter(8, 0, 8, 8, RefFrame::Last2, mv(20, 0)),
            inter(0, 8, 8, 8, RefFrame::Last3, mv(0, 30)),
            inter(8, 8, 8, 8, RefFrame::Golden, mv(0, 40)),
        ];
        let grid = store_motion_field(&units, 16, 16, &ALL_SIGNS);
        assert_eq!(cell(&grid, 0, 0), Some((10, 0, RefFrame::Last)));
        assert_eq!(cell(&grid, 1, 0), Some((20, 0, RefFrame::Last2)));
        assert_eq!(cell(&grid, 0, 1), Some((0, 30, RefFrame::Last3)));
        assert_eq!(cell(&grid, 1, 1), Some((0, 40, RefFrame::Golden)));
    }

    #[test]
    fn a_large_unit_names_every_cell_it_covers_and_a_narrower_frame_clamps_the_sample() {
        let grid = store_motion_field(
            &[inter(0, 0, 64, 64, RefFrame::Last, mv(7, -7))],
            64,
            64,
            &ALL_SIGNS,
        );
        for y in 0..8 {
            for x in 0..8 {
                assert_eq!(
                    cell(&grid, x, y),
                    Some((7, -7, RefFrame::Last)),
                    "({x},{y})"
                );
            }
        }

        // A 12x12 frame: the second cell column/row has only one 4x4 column/row, so the sample
        // position is clamped to it.
        let units = [
            inter(0, 0, 8, 8, RefFrame::Last, mv(1, 0)),
            inter(8, 0, 4, 8, RefFrame::Last2, mv(2, 0)),
            inter(0, 8, 8, 4, RefFrame::Last3, mv(3, 0)),
            inter(8, 8, 4, 4, RefFrame::Golden, mv(4, 0)),
        ];
        let grid = store_motion_field(&units, 12, 12, &ALL_SIGNS);
        assert_eq!((grid.cols_8x8, grid.rows_8x8), (2, 2));
        assert_eq!(cell(&grid, 0, 0), Some((1, 0, RefFrame::Last)));
        assert_eq!(cell(&grid, 1, 0), Some((2, 0, RefFrame::Last2)));
        assert_eq!(cell(&grid, 0, 1), Some((3, 0, RefFrame::Last3)));
        assert_eq!(cell(&grid, 1, 1), Some((4, 0, RefFrame::Golden)));

        // A 4x4 frame is one cell sampled at its only 4x4.
        let grid = store_motion_field(
            &[inter(0, 0, 4, 4, RefFrame::AltRef, mv(5, 5))],
            4,
            4,
            &ALL_SIGNS,
        );
        assert_eq!(cell(&grid, 0, 0), Some((5, 5, RefFrame::AltRef)));
    }

    /// A frame size that is not a multiple of 4 still has a (partial) last 4x4 column/row, and
    /// that is the one sampled.
    #[test]
    fn a_partial_last_4x4_column_or_row_is_the_one_sampled() {
        // 6 wide: 4x4 columns 0 (x 0..4) and 1 (x 4..6).
        let units = [
            inter(0, 0, 4, 8, RefFrame::Last, mv(1, 0)),
            inter(4, 0, 2, 8, RefFrame::Last2, mv(2, 0)),
        ];
        let grid = store_motion_field(&units, 6, 8, &ALL_SIGNS);
        assert_eq!(cell(&grid, 0, 0), Some((2, 0, RefFrame::Last2)));

        // 6 high.
        let units = [
            inter(0, 0, 8, 4, RefFrame::Last, mv(0, 1)),
            inter(0, 4, 8, 2, RefFrame::Last3, mv(0, 3)),
        ];
        let grid = store_motion_field(&units, 8, 6, &ALL_SIGNS);
        assert_eq!(cell(&grid, 0, 0), Some((0, 3, RefFrame::Last3)));
    }

    #[test]
    fn cells_no_unit_covers_are_empty() {
        let grid = store_motion_field(
            &[inter(8, 0, 8, 8, RefFrame::Last, mv(1, 1))],
            24,
            16,
            &ALL_SIGNS,
        );
        let filled: Vec<_> = (0..2)
            .flat_map(|y| (0..3).map(move |x| (x, y)))
            .filter(|&(x, y)| grid.get(x, y).is_some())
            .collect();
        assert_eq!(filled, vec![(1, 0)]);
    }

    #[test]
    fn a_unit_hanging_over_the_frame_edge_is_clamped_into_the_last_cell() {
        let grid = store_motion_field(
            &[inter(640, 480, 64, 64, RefFrame::Last, mv(6, 6))],
            16,
            16,
            &ALL_SIGNS,
        );
        assert_eq!(cell(&grid, 1, 1), Some((6, 6, RefFrame::Last)));
        assert_eq!(cell(&grid, 0, 0), None);
    }

    // ---- which blocks save a vector ----

    #[test]
    fn intra_blocks_save_nothing() {
        let grid = store_motion_field(&[CodingUnit::new(0, 0, 8, 8)], 8, 8, &ALL_SIGNS);
        assert_eq!(cell(&grid, 0, 0), None);
    }

    #[test]
    fn every_reference_is_gated_by_its_own_sign_entry() {
        for (i, &rf) in REFS.iter().enumerate() {
            let unit = [inter(0, 0, 8, 8, rf, mv(9, -9))];
            let mut only = [false; 7];
            only[i] = true;
            let grid = store_motion_field(&unit, 8, 8, &only);
            assert_eq!(cell(&grid, 0, 0), Some((9, -9, rf)), "{rf:?} enabled");

            let mut all_but = [true; 7];
            all_but[i] = false;
            let grid = store_motion_field(&unit, 8, 8, &all_but);
            assert_eq!(cell(&grid, 0, 0), None, "{rf:?} disabled");

            // No other entry can enable it.
            let mut others = [true; 7];
            others[i] = false;
            let grid = store_motion_field(&unit, 8, 8, &others);
            assert_eq!(cell(&grid, 0, 0), None, "{rf:?} with only the others on");
        }
    }

    #[test]
    fn a_vector_is_saved_only_below_4096_on_both_axes_and_both_signs() {
        for (x, y, saved) in [
            (4095, 0, true),
            (0, 4095, true),
            (-4095, 0, true),
            (0, -4095, true),
            (-4095, 4095, true),
            (4096, 0, false),
            (0, 4096, false),
            (-4096, 0, false),
            (0, -4096, false),
            (4096, 4096, false),
            (i32::MIN, 0, false),
            (0, i32::MAX, false),
        ] {
            let grid = store_motion_field(
                &[inter(0, 0, 8, 8, RefFrame::Last, mv(x, y))],
                8,
                8,
                &ALL_SIGNS,
            );
            assert_eq!(cell(&grid, 0, 0).is_some(), saved, "mv ({x},{y})");
        }
    }

    // ---- compound blocks ----

    #[test]
    fn a_compound_block_saves_its_second_reference_when_that_one_qualifies() {
        let unit = compound(
            0,
            0,
            8,
            8,
            [RefFrame::Last, RefFrame::AltRef],
            [mv(1, 1), mv(2, 2)],
        );
        let grid = store_motion_field(&[unit], 8, 8, &ALL_SIGNS);
        assert_eq!(cell(&grid, 0, 0), Some((2, 2, RefFrame::AltRef)));
    }

    #[test]
    fn a_compound_block_falls_back_to_its_first_reference_when_the_second_does_not_qualify() {
        let unit = |second_mv| {
            compound(
                0,
                0,
                8,
                8,
                [RefFrame::Last, RefFrame::AltRef],
                [mv(1, 1), second_mv],
            )
        };
        // Second reference's sign entry is off.
        let mut signs = ALL_SIGNS;
        signs[6] = false;
        let grid = store_motion_field(&[unit(mv(2, 2))], 8, 8, &signs);
        assert_eq!(cell(&grid, 0, 0), Some((1, 1, RefFrame::Last)));
        // Second vector too large.
        let grid = store_motion_field(&[unit(mv(4096, 0))], 8, 8, &ALL_SIGNS);
        assert_eq!(cell(&grid, 0, 0), Some((1, 1, RefFrame::Last)));
        // A single-reference block (second reference is intra) uses the first.
        let single = inter(0, 0, 8, 8, RefFrame::Golden, mv(3, 3));
        let grid = store_motion_field(&[single], 8, 8, &ALL_SIGNS);
        assert_eq!(cell(&grid, 0, 0), Some((3, 3, RefFrame::Golden)));
    }

    #[test]
    fn a_compound_block_with_neither_reference_qualifying_saves_nothing() {
        let unit = compound(
            0,
            0,
            8,
            8,
            [RefFrame::Last, RefFrame::AltRef],
            [mv(5000, 0), mv(0, 5000)],
        );
        let grid = store_motion_field(&[unit], 8, 8, &ALL_SIGNS);
        assert_eq!(cell(&grid, 0, 0), None);

        let unit = compound(
            0,
            0,
            8,
            8,
            [RefFrame::Last, RefFrame::AltRef],
            [mv(1, 1), mv(2, 2)],
        );
        let grid = store_motion_field(&[unit], 8, 8, &[false; 7]);
        assert_eq!(cell(&grid, 0, 0), None);
    }

    #[test]
    fn a_block_whose_first_reference_is_intra_but_second_is_not_saves_the_second() {
        // Not produced by the parser, but the rule is "second first, then first".
        let unit = compound(
            0,
            0,
            8,
            8,
            [RefFrame::Intra, RefFrame::Last2],
            [mv(0, 0), mv(8, 8)],
        );
        let grid = store_motion_field(&[unit], 8, 8, &ALL_SIGNS);
        assert_eq!(cell(&grid, 0, 0), Some((8, 8, RefFrame::Last2)));
    }

    // ---- overlap ----

    #[test]
    fn when_units_overlap_the_first_in_decode_order_names_the_cell() {
        let units = [
            inter(0, 0, 8, 8, RefFrame::Last, mv(1, 0)),
            inter(0, 0, 8, 8, RefFrame::Last2, mv(2, 0)),
        ];
        let grid = store_motion_field(&units, 8, 8, &ALL_SIGNS);
        assert_eq!(cell(&grid, 0, 0), Some((1, 0, RefFrame::Last)));
    }

    #[test]
    fn a_covering_unit_that_saves_nothing_blocks_a_later_one() {
        // The intra unit names the cell first; it has nothing to save, so the cell stays empty
        // even though a later inter unit overlaps it.
        let units = [
            CodingUnit::new(0, 0, 8, 8),
            inter(0, 0, 8, 8, RefFrame::Last, mv(1, 0)),
        ];
        let grid = store_motion_field(&units, 8, 8, &ALL_SIGNS);
        assert_eq!(cell(&grid, 0, 0), None);
    }

    #[test]
    fn a_rectangular_frame_keeps_rows_and_columns_apart() {
        // 24 wide x 8 high: three cells in a row; vectors identify the column.
        let units: Vec<_> = (0..3)
            .map(|i| inter(i * 8, 0, 8, 8, RefFrame::Last, mv(10 * (i as i32 + 1), 0)))
            .collect();
        let grid = store_motion_field(&units, 24, 8, &ALL_SIGNS);
        assert_eq!((grid.cols_8x8, grid.rows_8x8), (3, 1));
        assert_eq!(cell(&grid, 0, 0).map(|c| c.0), Some(10));
        assert_eq!(cell(&grid, 1, 0).map(|c| c.0), Some(20));
        assert_eq!(cell(&grid, 2, 0).map(|c| c.0), Some(30));

        // 8 wide x 24 high.
        let units: Vec<_> = (0..3)
            .map(|i| inter(0, i * 8, 8, 8, RefFrame::Last, mv(0, 10 * (i as i32 + 1))))
            .collect();
        let grid = store_motion_field(&units, 8, 24, &ALL_SIGNS);
        assert_eq!((grid.cols_8x8, grid.rows_8x8), (1, 3));
        assert_eq!(cell(&grid, 0, 0).map(|c| c.1), Some(10));
        assert_eq!(cell(&grid, 0, 1).map(|c| c.1), Some(20));
        assert_eq!(cell(&grid, 0, 2).map(|c| c.1), Some(30));
    }
}
