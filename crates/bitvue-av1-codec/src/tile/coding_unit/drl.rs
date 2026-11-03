//! `drl_idx` selection (spec 5.11.23), shared by the single-reference and compound blocks.

use crate::symbol::SymbolDecoder;
use bitvue_engine::Result;

/// The spec's DRL loop: from `start` (0 for the NEW modes, 1 for the NEAR modes) reads up to two
/// `drl_bit`s. A 1 moves on to the next stack entry; a 0, or a stack with no further entry
/// (`n_mvs`), ends the loop. `ctx(i)` is the context of position `i`.
pub(super) fn read_drl_idx(
    decoder: &mut SymbolDecoder,
    start: usize,
    n_mvs: usize,
    ctx: impl Fn(usize) -> u8,
) -> Result<usize> {
    let mut drl_idx = start;
    for idx in start..start + 2 {
        if n_mvs <= idx + 1 || !decoder.read_drl_bit(ctx(idx))? {
            break;
        }
        drl_idx = idx + 1;
    }
    Ok(drl_idx)
}
