//! `interp_filter` (spec 5.11.30): the subpel interpolation filter, one symbol per axis.

use super::contexts::needs_interp_filter;
use super::types::{CodingUnit, RefFrame};
use crate::symbol::SymbolDecoder;
use crate::tile::{FrameCodingParams, MiRect, TileContext};
use bitvue_engine::Result;

pub(super) fn read_interp_filter(
    decoder: &mut SymbolDecoder,
    tile_ctx: &mut TileContext,
    mi: MiRect,
    frame: &FrameCodingParams,
    cu: &CodingUnit,
    is_warp: bool,
) -> Result<()> {
    let MiRect {
        x4,
        y4,
        width: width_4x4,
        height: height_4x4,
    } = mi;
    let inter_mode_flags = frame.inter_mode_flags;
    let is_compound = cu.ref_frames[1] != RefFrame::Intra;
    let rav1d_ref0 = cu.ref_frames[0] as i8 - 1;

    // filter (spec 5.11.30, subpel interpolation filter -- one symbol per axis) -- real
    // per-`(dir, ctx)` CDF + adaptation, see `SymbolDecoder::read_filter`'s doc for the
    // desync this closes (previously never read at all). Real spec's `needs_interp_filter()`
    // exclusion is modeled for both real cases now: `skip_mode` forces `false`
    // unconditionally (dav1d `decode.c:1407`, `has_subpel_filter = 0`, checked first since
    // `cu.mode` is always `NearestNearestMv` there -- `needs_interp_filter`'s own `_ => true`
    // catch-all would otherwise wrongly read bits for it), and the `GmType`-dependent
    // GLOBALMV/GLOBAL_GLOBALMV case (verified against dav1d's `decode.c` `has_subpel_filter`
    // computation, source-only re-clone): unconditionally read for NEARESTMV/NEARMV/NEWMV
    // and any compound mode other than GLOBAL_GLOBALMV; for GLOBALMV/GLOBAL_GLOBALMV, only
    // read when the block is minimal size (`min(width_4x4, height_4x4) == 1`) or the
    // relevant ref's `GmType` is exactly TRANSLATION (not `>` -- IDENTITY/ROTZOOM/AFFINE all
    // suppress the read).
    // A WARPED_CAUSAL block (`motion_mode == 2`) reads no filter either (dav1d: `has_subpel_filter
    // = 0` when `b->motion_mode == MM_WARP`).
    let has_subpel_filter = !cu.skip_mode
        && !is_warp
        && needs_interp_filter(
            cu.mode,
            width_4x4,
            height_4x4,
            &inter_mode_flags.gm_type,
            cu.ref_frames[0],
            cu.ref_frames[1],
        );
    if inter_mode_flags.subpel_filter_switchable {
        let is_comp = is_compound;
        // Without `enable_dual_filter` only the first axis is coded; the second takes its value
        // (dav1d: `filter[1] = filter[0]`), and both are recorded for the neighbours.
        let mut first = 0;
        for dir in 0..2u8 {
            // Real dav1d always records the resulting filter into neighbor context
            // regardless of whether it was actually read -- `0` (`EIGHTTAP_REGULAR`) is the
            // real default `read_filter` never returns via the CDF path (its symbols start
            // at the crate's own regular-tap index), matching dav1d's own
            // `filter[i] = DAV1D_FILTER_8TAP_REGULAR` fallback.
            let filter = if has_subpel_filter && dir == 1 && !inter_mode_flags.enable_dual_filter {
                first
            } else if has_subpel_filter {
                let fctx = tile_ctx.filter_context(x4, y4, is_comp, dir as usize, rav1d_ref0);
                decoder.read_filter(dir, fctx)?
            } else {
                0
            };
            if dir == 0 {
                first = filter;
            }
            tile_ctx.set_filter(x4, y4, width_4x4, height_4x4, dir as usize, filter);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tile::coding_unit::PredictionMode;

    /// How many symbols `read_interp_filter` reads for an 8x8 single-reference NEARESTMV block.
    fn symbols_read(enable_dual_filter: bool) -> usize {
        let mut frame = FrameCodingParams::for_tests();
        frame.inter_mode_flags.subpel_filter_switchable = true;
        frame.inter_mode_flags.enable_dual_filter = enable_dual_filter;
        let mut cu = CodingUnit::new(0, 0, 8, 8);
        cu.mode = PredictionMode::NearestMv;
        cu.ref_frames = [RefFrame::Last, RefFrame::Intra];
        let mi = MiRect {
            x4: 0,
            y4: 0,
            width: 2,
            height: 2,
        };
        let mut tile_ctx = TileContext::new(16, 16);
        let mut decoder = SymbolDecoder::new_with_qcat(&[0x5a; 64], 0).unwrap();
        decoder.decoder.range_trace = Some(Vec::new());

        read_interp_filter(&mut decoder, &mut tile_ctx, mi, &frame, &cu, false).unwrap();

        decoder.decoder.range_trace.take().unwrap().len()
    }

    /// Without `enable_dual_filter` one filter is coded and shared by both axes (dav1d logs a
    /// single `Post-subpel_filter`); with it there is one symbol per axis.
    #[test]
    fn one_filter_symbol_without_dual_filter_two_with_it() {
        assert_eq!(symbols_read(false), 1);
        assert_eq!(symbols_read(true), 2);
    }
}
