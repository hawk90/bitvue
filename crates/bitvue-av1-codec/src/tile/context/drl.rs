//! `drl_bit`'s context and the stack entries it reads.

/// One candidate in `SpatialRefContext::single_ref_mv_stack`'s real weighted DRL stack.
#[derive(Debug, Clone, Copy, Default)]
pub struct MvStackEntry {
    pub mv: crate::tile::coding_unit::MotionVector,
    pub(in crate::tile::context) weight: i32,
}

/// One candidate in `SpatialRefContext::compound_mv_stack`'s real weighted DRL stack -- same
/// shape as `MvStackEntry`, but a joint L0/L1 pair (real spec's compound candidates are pairs
/// from one unified search, not two independent single-ref ones -- `compound_mv_stack`'s doc).
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct CompoundMvStackEntry {
    pub mv: [crate::tile::coding_unit::MotionVector; 2],
    pub(in crate::tile::context) weight: i32,
}

/// `drl_bit`'s real context (0..=2) for a compound candidate stack -- identical logic to
/// `get_drl_context`, duplicated rather than made generic since it only ever reads `.weight`
/// (confirmed by that function's body) and `CompoundMvStackEntry`/`MvStackEntry` are otherwise
/// unrelated types.
pub fn get_compound_drl_context(stack: &[CompoundMvStackEntry; 8], idx: usize) -> u8 {
    let w0 = stack[idx.min(7)].weight;
    let w1 = stack[(idx + 1).min(7)].weight;
    if w0 >= 640 {
        u8::from(w1 < 640)
    } else if w1 < 640 {
        2
    } else {
        0
    }
}

/// `drl_bit`'s real context (0..=2), spec 7.10.2.10's `DrlCtxStack` comparison -- source: rav1d
/// `get_drl_context` (`memorysafety/rav1d`, BSD-2-Clause, `src/env.rs`). `idx`: the DRL position
/// being decided between (`0` when choosing NEAREST-vs-NEARER, `1` for NEARER-vs-NEAR, `2` for
/// NEAR-vs-NEARISH) -- compares `stack[idx]`'s weight against `stack[idx+1]`'s.
pub fn get_drl_context(stack: &[MvStackEntry; 8], idx: usize) -> u8 {
    let w0 = stack[idx.min(7)].weight;
    let w1 = stack[(idx + 1).min(7)].weight;
    if w0 >= 640 {
        u8::from(w1 < 640)
    } else if w1 < 640 {
        2
    } else {
        0
    }
}
