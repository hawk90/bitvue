//! Global motion in the candidate stack: the vector `setup_global_mv` computes for a block, and
//! where the stack uses it (substituting `GLOBALMV` neighbours, padding missing candidates).

use super::*;
use crate::frame_header_full::{GmParams, GM_IDENTITY_PARAMS};
use crate::tile::coding_unit::MotionVector;

/// 0-based `ref0` of ALTREF, whose warp is the one set up below.
const ALTREF: i8 = 6;

/// The warp of the global-motion clip's first global-motion frame (ALTREF, ROTZOOM).
const ROTZOOM_PARAMS: [i32; 6] = [145408, 126976, 64378, -40, 40, 64378];

fn mv(x: i32, y: i32) -> MotionVector {
    MotionVector::new(x, y)
}

fn params_for_altref(p: [i32; 6]) -> GmParams {
    let mut params = [GM_IDENTITY_PARAMS; 8];
    params[ALTREF as usize + 1] = p;
    params
}

fn grid_with(gm_type: u8, p: [i32; 6], allow_high_precision_mv: bool) -> SpatialRefContext {
    let mut ctx = SpatialRefContext::new(32, 32);
    let mut types = [0u8; 8];
    types[ALTREF as usize + 1] = gm_type;
    ctx.set_global_motion(types, params_for_altref(p), allow_high_precision_mv, false);
    ctx
}

#[test]
fn an_identity_warp_has_a_zero_global_mv() {
    let ctx = SpatialRefContext::new(32, 32);
    assert_eq!(ctx.global_mv(ALTREF, 4, 4, 2, 2), MotionVector::zero());
    assert_eq!(ctx.global_candidate(ALTREF, 4, 4, 2, 2), None);
}

#[test]
fn no_reference_has_a_zero_global_mv() {
    let ctx = grid_with(2, ROTZOOM_PARAMS, true);
    assert_eq!(ctx.global_mv(-1, 4, 4, 2, 2), MotionVector::zero());
}

#[test]
fn a_translation_takes_row_from_parameter_0_and_column_from_parameter_1() {
    let mut p = GM_IDENTITY_PARAMS;
    p[0] = 3 << 13;
    p[1] = -5 << 13;
    let ctx = grid_with(1, p, true);
    assert_eq!(ctx.global_mv(ALTREF, 4, 4, 2, 2), mv(-5, 3));
    // A translation is not a candidate substitute: only warps beyond it are.
    assert_eq!(ctx.global_candidate(ALTREF, 4, 4, 2, 2), None);
}

#[test]
fn a_rotzoom_warp_is_evaluated_at_the_centre_of_the_block() {
    // 8x8 block at the origin: centre (3, 3). xc = (64378 - 65536) * 3 - 40 * 3 + 145408 = 141814,
    // yc = 40 * 3 + (64378 - 65536) * 3 + 126976 = 123622.
    let ctx = grid_with(2, ROTZOOM_PARAMS, true);
    assert_eq!(ctx.global_mv(ALTREF, 0, 0, 2, 2), mv(17, 15));
    assert_eq!(ctx.global_candidate(ALTREF, 0, 0, 2, 2), Some(mv(17, 15)));
}

#[test]
fn without_high_precision_the_global_mv_is_even() {
    let ctx = grid_with(2, ROTZOOM_PARAMS, false);
    assert_eq!(ctx.global_mv(ALTREF, 0, 0, 2, 2), mv(18, 16));
}

#[test]
fn the_warp_differs_across_the_frame() {
    let ctx = grid_with(2, ROTZOOM_PARAMS, true);
    assert_ne!(
        ctx.global_mv(ALTREF, 0, 0, 2, 2),
        ctx.global_mv(ALTREF, 24, 24, 2, 2)
    );
}

fn stack_of(ctx: &SpatialRefContext, x4: u32, y4: u32) -> ([MvStackEntry; 8], usize) {
    ctx.single_ref_mv_stack(x4, y4, 2, 2, ALTREF, false)
}

#[test]
fn missing_candidates_read_as_the_global_mv() {
    let ctx = grid_with(2, ROTZOOM_PARAMS, true);
    let (stack, cnt) = stack_of(&ctx, 8, 8);
    assert_eq!(cnt, 0);
    let global = ctx.global_mv(ALTREF, 8, 8, 2, 2);
    assert_ne!(global, MotionVector::zero());
    assert_eq!([stack[0].mv, stack[1].mv], [global, global]);
}

#[test]
fn a_globalmv_neighbour_contributes_the_global_mv_of_the_block_not_its_own() {
    let mut ctx = grid_with(2, ROTZOOM_PARAMS, true);
    // An 8x8 GLOBALMV block above, stored with the vector it had at its own position.
    ctx.set_block(
        8,
        6,
        2,
        2,
        ALTREF,
        -1,
        false,
        mv(99, 99),
        MotionVector::zero(),
    );
    ctx.mark_global_mode(8, 6, 2, 2);
    let (stack, cnt) = stack_of(&ctx, 8, 8);
    assert_eq!(stack[0].mv, ctx.global_mv(ALTREF, 8, 8, 2, 2));
    assert_eq!(stack[0].weight, 640 + 4);
    // With fewer than two candidates the neighbour is also added as an "other reference"
    // candidate, which uses the vector it stored (spec 7.10.2.12 has no global-motion case).
    assert_eq!(cnt, 2);
    assert_eq!(stack[1].mv, mv(99, 99));
}

#[test]
fn a_neighbour_that_is_not_a_globalmv_block_keeps_its_own_vector() {
    let mut ctx = grid_with(2, ROTZOOM_PARAMS, true);
    ctx.set_block(
        8,
        6,
        2,
        2,
        ALTREF,
        -1,
        false,
        mv(98, 98),
        MotionVector::zero(),
    );
    let (stack, cnt) = stack_of(&ctx, 8, 8);
    assert_eq!(cnt, 1);
    assert_eq!(stack[0].mv, mv(98, 98));
}

#[test]
fn a_globalmv_block_smaller_than_8x8_keeps_its_own_vector() {
    let mut ctx = grid_with(2, ROTZOOM_PARAMS, true);
    ctx.set_block(
        8,
        7,
        2,
        1,
        ALTREF,
        -1,
        false,
        mv(98, 98),
        MotionVector::zero(),
    );
    ctx.mark_global_mode(8, 7, 2, 1);
    let (stack, cnt) = stack_of(&ctx, 8, 8);
    assert_eq!(cnt, 1);
    assert_eq!(stack[0].mv, mv(98, 98));
}

#[test]
fn a_translation_warp_does_not_substitute_globalmv_neighbours() {
    let mut p = GM_IDENTITY_PARAMS;
    p[0] = 3 << 13;
    let mut ctx = grid_with(1, p, true);
    ctx.set_block(
        8,
        6,
        2,
        2,
        ALTREF,
        -1,
        false,
        mv(99, 99),
        MotionVector::zero(),
    );
    ctx.mark_global_mode(8, 6, 2, 2);
    let (stack, _) = stack_of(&ctx, 8, 8);
    assert_eq!(stack[0].mv, mv(99, 99));
}
