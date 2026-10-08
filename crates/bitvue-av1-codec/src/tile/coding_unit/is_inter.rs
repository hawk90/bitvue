//! `is_inter` (spec 5.11.5): the per-block intra/inter dispatch, read right after the
//! `delta_q`/`delta_lf` side information. Also records the result in the tile's above/left
//! "is intra" context.

use crate::frame_header_full::{SegmentationInfo, SEG_LVL_GLOBALMV, SEG_LVL_REF_FRAME};
use crate::symbol::SymbolDecoder;
use crate::tile::{MiRect, RefFrame, TileContext};
use bitvue_engine::Result;

/// See `SymbolDecoder::read_is_inter` for the desync this closes (this crate previously treated
/// every non-key-frame block as unconditionally inter and never read the bit -- a real intra block
/// inside an inter frame, e.g. scene-change intra refresh, is legal and common).
///
/// Priority order, matching the spec:
/// 1. Key frames are always intra, no bit read (`IS_INTER_OR_SWITCH` is false for an intra-only
///    frame, so this branch of `decode_b` never runs).
/// 2. `skip_mode` forces inter, no bit read.
/// 3. `SEG_LVL_REF_FRAME` forces it from the feature data (an actual `RefFrame`; `!= Intra` means
///    inter).
/// 4. `SEG_LVL_GLOBALMV` (only checked when `SEG_LVL_REF_FRAME` isn't active) forces inter.
/// 5. Otherwise the bit is read with the above/left intra context.
pub(super) fn read_is_inter(
    decoder: &mut SymbolDecoder,
    tile_ctx: &mut TileContext,
    mi: MiRect,
    is_key_frame: bool,
    skip_mode: bool,
    segmentation: SegmentationInfo,
    segment_id: u8,
) -> Result<bool> {
    let is_inter = if is_key_frame {
        false
    } else if skip_mode {
        true
    } else if segmentation.seg_feature_active(segment_id, SEG_LVL_REF_FRAME) {
        segmentation.seg_feature_data(segment_id, SEG_LVL_REF_FRAME) != RefFrame::Intra as i16
    } else if segmentation.seg_feature_active(segment_id, SEG_LVL_GLOBALMV) {
        true
    } else {
        let ictx = tile_ctx.intra_ctx(mi.x4, mi.y4);
        decoder.read_is_inter(ictx)?
    };
    Ok(is_inter)
}

#[cfg(test)]
mod context_order_tests {
    use super::*;

    /// Same rule as for `ref_frames`: `motion_mode` asks whether the neighbours are intra, and
    /// must not find the block's own (inter) flag there.
    #[test]
    fn reading_is_inter_leaves_the_neighbours_flag_in_the_context() {
        let mut tile_ctx = TileContext::new(16, 16);
        tile_ctx.set_intra_flag(0, 0, 4, 4, true); // intra neighbour on the left
        let data = [0u8; 16];
        let mut decoder = SymbolDecoder::new(&data).unwrap();
        let mi = MiRect {
            x4: 4,
            y4: 0,
            width: 4,
            height: 4,
        };
        let is_inter = read_is_inter(
            &mut decoder,
            &mut tile_ctx,
            mi,
            false,
            true, // skip_mode: inter, no bits read
            SegmentationInfo::default(),
            0,
        )
        .unwrap();
        assert!(is_inter);
        assert!(
            tile_ctx.left_is_intra(0),
            "the neighbour's intra flag was overwritten"
        );
    }
}
