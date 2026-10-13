//! Real AV1 temporal MV candidates -- spec 7.9 (motion field motion vector storage process) and
//! 7.10 (motion field estimation process), used by [`crate::tile::context::SpatialRefContext::
//! single_ref_mv_stack`]'s temporal scan.
//!
//! Source: dav1d's own C reference implementation (`memorysafety`-adjacent, VideoLAN/Two Orioles,
//! BSD-2-Clause), `src/refmvs.c`/`refmvs.h` -- ported directly (this crate's established
//! "verify against reference source, don't reconstruct from memory" discipline), not from the
//! spec text alone, because the real algorithm has load-bearing implementation detail the spec's
//! prose alone under-specifies (the up-to-3-source priority selection, the chained
//! reference-of-reference projection). Every function below cites the exact `refmvs.c` lines it
//! ports.
//!
//! **Unit note**: MVs are in 1/8-luma-sample units (spec's native MV precision), same as dav1d's
//! `mv.x`/`mv.y`, so every constant below is dav1d's literal value.
//!
//! **Scope**: implemented for correctness only, exercised by a sequential full-fixture test
//! harness (`overlay_extraction::cu_parser`'s `temporal_mv` tests) -- not wired into
//! `bitvue-sidecar`'s per-request commands, which remain stateless/random-access (see this
//! session's `docs/DEVELOPMENT_PHASES.md` entry for the scope discussion).

mod project;
mod saved;
mod state;
mod temporal;

pub use project::{project_motion_field, ProjectedMotionField, ProjectedMv};
pub use saved::{store_motion_field, MotionFieldGrid, SavedMv};
pub use state::{select_motion_field_sources, MfmvSource, MotionFieldState};
pub use temporal::{
    add_temporal_candidates, add_temporal_compound_candidates, fix_mv_precision, TemporalBlock,
};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tile::coding_unit::{MotionVector, RefFrame};

    fn mv(x: i32, y: i32) -> MotionVector {
        MotionVector { x, y }
    }

    /// dav1d `fix_mv_precision` / `fix_int_mv_precision` (`src/env.h`), worked by hand:
    /// `(v - (v >> 15)) & ~1` without high precision, `(v - (v >> 15) + 3) & ~7` for integer MVs.
    #[test]
    fn mv_precision_rounds_like_dav1d() {
        assert_eq!(fix_mv_precision(mv(3, -3), true, false), mv(3, -3));
        assert_eq!(fix_mv_precision(mv(3, -3), false, false), mv(2, -2));
        assert_eq!(fix_mv_precision(mv(-4, 5), false, false), mv(-4, 4));
        assert_eq!(fix_mv_precision(mv(5, -5), true, true), mv(8, -8));
        assert_eq!(fix_mv_precision(mv(4, -4), true, true), mv(0, 0));
    }

    fn projected_with(cells: &[(u32, u32, MotionVector)]) -> ProjectedMotionField {
        let mut field = ProjectedMotionField::empty(32, 32);
        for &(x, y, mv) in cells {
            field.set(x, y, ProjectedMv { mv, ref2ref: 1 });
        }
        field
    }

    /// A 2x2-unit (8x8 sample) block takes its own sample, then the cells below-left,
    /// below-right and right of it (`refmvs.c:432-451`), in that order.
    #[test]
    fn small_blocks_also_sample_the_cells_around_them() {
        let field = projected_with(&[
            (1, 1, mv(1, 0)),
            (0, 2, mv(2, 0)),
            (2, 2, mv(3, 0)),
            (2, 1, mv(4, 0)),
        ]);
        let block = TemporalBlock {
            bx4: 2,
            by4: 2,
            bw4: 2,
            bh4: 2,
            w4: 2,
            h4: 2,
            col_start: 0,
            col_end: 64,
            row_end: 64,
        };
        let got = add_temporal_candidates(&field, 1, block);
        assert_eq!(
            got,
            vec![
                (mv(1, 0), true),
                (mv(2, 0), false),
                (mv(3, 0), false),
                (mv(4, 0), false)
            ]
        );

        // 16 units wide is no longer "small": only the main grid is sampled.
        let wide = TemporalBlock {
            bw4: 16,
            w4: 16,
            ..block
        };
        assert!(add_temporal_candidates(&field, 1, wide)
            .iter()
            .all(|&(m, _)| m != mv(2, 0) && m != mv(3, 0)));
    }

    fn state_with_one_saved_mv(x: u32, y: u32, saved: MotionVector) -> MotionFieldState {
        let mut grid = MotionFieldGrid::empty(32, 32);
        grid.set(
            x,
            y,
            Some(SavedMv {
                mv: saved,
                ref_frame: RefFrame::Last,
            }),
        );
        let mut state = MotionFieldState::new();
        state.grids[0] = Some(std::sync::Arc::new(grid));
        state
    }

    /// An intra frame (without intra block copy) leaves no motion vectors in its slots, so the
    /// slot cannot be a temporal source -- even though its order hint says it lies in the future
    /// of a later frame (dav1d: `rp_ref[i] == NULL`).
    #[test]
    fn a_slot_refreshed_by_an_intra_frame_keeps_no_motion_field() {
        let mut state = MotionFieldState::new();
        state.update(
            &[0; 8],
            0b0000_0011,
            None,
            MotionFieldGrid::empty(4, 4),
            false,
        );
        assert!(state.grids[0].is_none() && state.grids[1].is_none());

        state.update(
            &[0; 8],
            0b0000_0010,
            None,
            MotionFieldGrid::empty(4, 4),
            true,
        );
        assert!(state.grids[0].is_none(), "slot 0 was not refreshed");
        assert!(state.grids[1].is_some());
    }

    fn source(forward: bool) -> MfmvSource {
        MfmvSource {
            slot: 0,
            ref2cur: 2,
            ref2ref: [2; 7],
            forward,
        }
    }

    fn only_projected_cell(field: &ProjectedMotionField) -> Option<(u32, u32)> {
        let mut found = None;
        for y in 0..32 {
            for x in 0..32 {
                if field.get(x, y).is_some() {
                    assert!(found.is_none(), "more than one projected cell");
                    found = Some((x, y));
                }
            }
        }
        found
    }

    /// With `ref2cur == ref2ref` the projected offset equals the saved MV, so 1280 (1/8 sample)
    /// is 20 cells. Backward sources move that way, forward sources the opposite way
    /// (`apply_sign(abs(offset) >> 6, offset ^ ref_sign)`).
    #[test]
    fn projection_direction_depends_on_the_source_side() {
        let state = state_with_one_saved_mv(10, 3, mv(128, 0)); // two cells
        let backward = project_motion_field(&[source(false)], &state, 32, 32);
        assert_eq!(only_projected_cell(&backward), Some((12, 3)));
        let forward = project_motion_field(&[source(true)], &state, 32, 32);
        assert_eq!(only_projected_cell(&forward), Some((8, 3)));
    }

    /// dav1d projects one superblock row at a time: a cell lands at most 8 cells left and 16
    /// cells right of its own 64-sample column, and never leaves its 8-cell row (`load_tmvs_c`).
    #[test]
    fn projection_stays_inside_the_source_superblock_window() {
        // Column 0 moving 20 cells right would land at 20 >= 16: dropped.
        let far = state_with_one_saved_mv(0, 3, mv(1280, 0));
        assert_eq!(
            only_projected_cell(&project_motion_field(&[source(false)], &far, 32, 32)),
            None
        );
        // 15 cells right is the last column inside the window.
        let near = state_with_one_saved_mv(0, 3, mv(960, 0));
        assert_eq!(
            only_projected_cell(&project_motion_field(&[source(false)], &near, 32, 32)),
            Some((15, 3))
        );
        // Row 7 moving one row down would leave the 8-cell row.
        let down = state_with_one_saved_mv(4, 7, mv(0, 64));
        assert_eq!(
            only_projected_cell(&project_motion_field(&[source(false)], &down, 32, 32)),
            None
        );
        // Row 8 starts the next 8-cell row: moving one row up would leave it too.
        let up = state_with_one_saved_mv(4, 8, mv(0, -64));
        assert_eq!(
            only_projected_cell(&project_motion_field(&[source(false)], &up, 32, 32)),
            None
        );
    }
}
