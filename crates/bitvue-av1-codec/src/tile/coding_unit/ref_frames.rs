//! `ref_frame()` (spec 5.11.25): which reference frame(s) an inter block predicts from, plus the
//! above/left reference context update that later blocks' contexts read.

use super::types::{CodingUnit, RefFrame};
use crate::frame_header_full::{
    SegmentationInfo, SEG_LVL_GLOBALMV, SEG_LVL_REF_FRAME, SEG_LVL_SKIP,
};
use crate::symbol::SymbolDecoder;
use crate::tile::{BlockRect, FrameCodingParams, MiRect, TileContext};
use bitvue_engine::Result;

/// Reads (or derives) the reference pair; the caller stores it. This does NOT touch the above/left
/// reference contexts: later syntax of the same block (motion mode, filter, compound type) still
/// reads the *neighbours'* values from them, so the block's own refs are recorded only once all of
/// its mode info is parsed (see `parse_coding_unit`). Real per-context CDF + adaptation, see `SymbolDecoder::read_ref_frames`.
///
/// Spec priority order (dav1d `decode.c:1401` vs `1424`):
/// 1. `skip_mode` beats everything, forcing the refs from `skip_mode_refs` (the spec's
///    `SkipModeFrame[0]/[1]`) with no bits read -- always a genuine compound pair.
/// 2. `SEG_LVL_REF_FRAME` forces `RefFrame[0]` from its feature data (`RefFrame[1] = Intra`, never
///    compound).
/// 3. `SEG_LVL_SKIP` or `SEG_LVL_GLOBALMV` (either) forces `RefFrame[0] = Last`,
///    `RefFrame[1] = Intra`.
/// 4. Otherwise the symbols are read.
pub(super) fn read_ref_frames(
    decoder: &mut SymbolDecoder,
    tile_ctx: &mut TileContext,
    rect: BlockRect,
    mi: MiRect,
    frame: &FrameCodingParams,
    cu: &CodingUnit,
) -> Result<[RefFrame; 2]> {
    let ref_frames = match forced_ref_frames(
        cu.skip_mode,
        frame.skip_mode_refs,
        &frame.segmentation,
        cu.segment_id,
    ) {
        Some(forced) => forced,
        None => decoder.read_ref_frames(
            tile_ctx,
            mi.x4,
            mi.y4,
            frame.reference_select,
            rect.width.min(rect.height),
        )?,
    };
    Ok(ref_frames)
}

/// The reference pair when it is dictated without reading any bits (priority 1-3 above).
fn forced_ref_frames(
    skip_mode: bool,
    skip_mode_refs: [u8; 2],
    segmentation: &SegmentationInfo,
    segment_id: u8,
) -> Option<[RefFrame; 2]> {
    if skip_mode {
        Some([
            RefFrame::from_u8(skip_mode_refs[0]).unwrap_or(RefFrame::Last),
            RefFrame::from_u8(skip_mode_refs[1]).unwrap_or(RefFrame::Intra),
        ])
    } else if segmentation.seg_feature_active(segment_id, SEG_LVL_REF_FRAME) {
        let raw = segmentation.seg_feature_data(segment_id, SEG_LVL_REF_FRAME);
        Some([
            RefFrame::from_u8(raw.clamp(0, 7) as u8).unwrap_or(RefFrame::Last),
            RefFrame::Intra,
        ])
    } else if segmentation.seg_feature_active(segment_id, SEG_LVL_SKIP)
        || segmentation.seg_feature_active(segment_id, SEG_LVL_GLOBALMV)
    {
        Some([RefFrame::Last, RefFrame::Intra])
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn seg_with(feature: usize, data: i16) -> SegmentationInfo {
        let mut s = SegmentationInfo {
            enabled: true,
            ..Default::default()
        };
        s.feature_enabled[1][feature] = true;
        s.feature_data[1][feature] = data;
        s
    }

    #[test]
    fn segment_ref_frame_feature_reaches_every_reference_including_the_last() {
        for raw in 0..=7i16 {
            let seg = seg_with(SEG_LVL_REF_FRAME, raw);
            let got = forced_ref_frames(false, [0, 0], &seg, 1).unwrap();
            assert_eq!(got[0], RefFrame::from_u8(raw as u8).unwrap(), "raw {raw}");
            assert_eq!(got[1], RefFrame::Intra);
        }
    }

    #[test]
    fn skip_mode_wins_over_segmentation_and_skip_or_globalmv_force_last() {
        let seg = seg_with(SEG_LVL_REF_FRAME, 5);
        let got = forced_ref_frames(true, [1, 2], &seg, 1).unwrap();
        assert_eq!(
            got,
            [RefFrame::from_u8(1).unwrap(), RefFrame::from_u8(2).unwrap()]
        );
        for f in [SEG_LVL_SKIP, SEG_LVL_GLOBALMV] {
            let got = forced_ref_frames(false, [0, 0], &seg_with(f, 0), 1).unwrap();
            assert_eq!(got, [RefFrame::Last, RefFrame::Intra]);
        }
        assert_eq!(
            forced_ref_frames(false, [0, 0], &seg_with(SEG_LVL_SKIP, 0), 2),
            None
        );
    }
}

#[cfg(test)]
mod context_order_tests {
    use super::*;

    /// A block's own reference must not be visible in the above/left reference context while the
    /// rest of its mode info is still being parsed: `motion_mode` asks "does a neighbour use my
    /// reference?" and would otherwise find the block itself.
    #[test]
    fn reading_ref_frames_leaves_the_neighbours_values_in_the_context() {
        let mut tile_ctx = TileContext::new(16, 16);
        // Left neighbour at x4 0..4 uses reference index 3 (rav1d numbering).
        tile_ctx.set_ref_frames(0, 0, 4, 4, false, false, 3, -1);

        let mut cu = CodingUnit::new(16, 0, 16, 16);
        cu.skip_mode = true; // forces [Last, Last2] with no bits read
        let mut frame = FrameCodingParams::for_tests();
        frame.skip_mode_refs = [1, 2];
        let data = [0u8; 16];
        let mut decoder = SymbolDecoder::new(&data).unwrap();
        let rect = BlockRect {
            x: 16,
            y: 0,
            width: 16,
            height: 16,
        };
        let mi = MiRect {
            x4: 4,
            y4: 0,
            width: 4,
            height: 4,
        };
        let refs = read_ref_frames(&mut decoder, &mut tile_ctx, rect, mi, &frame, &cu).unwrap();
        assert_eq!(refs, [RefFrame::Last, RefFrame::Last2]);
        // The block uses rav1d reference 0; the neighbour's 3 must still be what a lookup finds.
        assert!(!tile_ctx.has_matching_single_ref(4, 0, false, true, 0));
        assert!(tile_ctx.has_matching_single_ref(4, 0, false, true, 3));
    }
}
