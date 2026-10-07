//! `skip_mode` and `skip` (spec 5.11.5), the two block-level flags read right after the
//! (pre-skip) `segment_id`. Both update the tile's above/left entropy contexts as they go.

use crate::frame_header_full::{SegmentationInfo, SEG_LVL_SKIP};
use crate::symbol::SymbolDecoder;
use crate::tile::{MiRect, TileContext};
use bitvue_engine::Result;

/// `skip_mode` -- real per-context CDF + adaptation, read BEFORE `skip` (dav1d's `decode_b`
/// order: skip_mode -> skip). `allowed` is "non-key frame with `skip_mode_present`"
/// (`skip_mode_present` is always `false` from a real intra-only frame's header, per spec's own
/// `skip_mode_params()` derivation); on top of that only blocks with `min(bw4, bh4) > 1` (never
/// 4-wide-or-tall) read it. Previously never read at all -- the desync this closed is described
/// in `parse_coding_unit`'s doc.
pub(super) fn read_skip_mode(
    decoder: &mut SymbolDecoder,
    tile_ctx: &mut TileContext,
    mi: MiRect,
    allowed: bool,
) -> Result<bool> {
    let min_dim4 = mi.width.min(mi.height);
    let skip_mode = if allowed && min_dim4 > 1 {
        let ctx = tile_ctx.skip_mode_context(mi.x4, mi.y4);
        decoder.read_skip_mode(ctx)?
    } else {
        false
    };
    tile_ctx.set_skip_mode(mi.x4, mi.y4, mi.width, mi.height, skip_mode);
    Ok(skip_mode)
}

/// `skip` -- real per-context CDF + adaptation, see `SymbolDecoder::read_skip`'s doc.
/// `skip_mode` forces `skip = true` with NO bit read (spec: a skip_mode block has nothing to
/// signal), real dav1d `if (b->skip_mode || (seg && seg->skip)) { b->skip = 1; } else { read }`.
/// `segment_id` is safe to use here even though the general `update_map && !seg_id_pre_skip` case
/// resolves it AFTER this point: `SEG_LVL_SKIP` is index `SEG_LVL_REF_FRAME..` (`>= 5`), so if it
/// is active for ANY segment this frame, `segmentation.seg_id_pre_skip` is unconditionally `true`
/// too (`SegmentationInfo`'s doc) -- meaning the id was already resolved by the pre-skip read
/// whenever this check could possibly fire.
pub(super) fn read_skip(
    decoder: &mut SymbolDecoder,
    tile_ctx: &mut TileContext,
    mi: MiRect,
    skip_mode: bool,
    segmentation: SegmentationInfo,
    segment_id: u8,
) -> Result<bool> {
    let skip = if skip_mode || segmentation.seg_feature_active(segment_id, SEG_LVL_SKIP) {
        true
    } else {
        let ctx = tile_ctx.skip_context(mi.x4, mi.y4);
        decoder.read_skip(ctx)?
    };
    tile_ctx.set_skip(mi.x4, mi.y4, mi.width, mi.height, skip);
    Ok(skip)
}
