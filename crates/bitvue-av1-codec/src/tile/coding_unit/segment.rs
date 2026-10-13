//! `segment_id()` (spec 5.11.9/5.11.10): the one syntax element that can sit on either side of
//! `skip` in a block's header, so it has two entry points -- [`read_pre_skip`] and
//! [`read_post_skip`] -- over a shared core.

use crate::frame_header_full::SegmentationInfo;
use crate::symbol::SymbolDecoder;
use crate::tile::{MiRect, TileContext};
use bitvue_engine::Result;

/// Pre-skip position -- ported from dav1d's `decode_b` (`src/decode.c`) call-site structure, not
/// the spec pseudocode alone, to get the `update_map`/`seg_id_pre_skip` branching exactly right.
/// Real spec unifies `!update_map` (pulls from the previous frame's segment map, no bits read)
/// and the `seg_id_pre_skip` real read into one `if/else if` here; the remaining case
/// (`update_map && !seg_id_pre_skip`) is deferred to [`read_post_skip`].
///
/// `None` = segment id not resolved at this position (caller keeps its current value).
pub(super) fn read_pre_skip(
    decoder: &mut SymbolDecoder,
    tile_ctx: &mut TileContext,
    mi: MiRect,
    segmentation: SegmentationInfo,
) -> Result<Option<u8>> {
    if !segmentation.enabled {
        return Ok(None);
    }
    if !segmentation.update_map {
        // No bits read either way -- see `SegmentationInfo`'s doc for why this crate reports
        // `0` (no cross-frame segment-map state) rather than the real previous-frame value.
        tile_ctx.set_segment_id(mi.x4, mi.y4, mi.width, mi.height, 0);
        Ok(Some(0))
    } else if segmentation.seg_id_pre_skip {
        read_segment_id(decoder, tile_ctx, mi, segmentation, None).map(Some)
    } else {
        Ok(None)
    }
}

/// Post-skip position -- the remaining `update_map && !seg_id_pre_skip` case (see
/// [`read_pre_skip`]); `skip` is known here, so a skipped block takes the predicted segment id
/// directly with no further bits (`read_segment_id`'s doc).
pub(super) fn read_post_skip(
    decoder: &mut SymbolDecoder,
    tile_ctx: &mut TileContext,
    mi: MiRect,
    segmentation: SegmentationInfo,
    skip: bool,
) -> Result<Option<u8>> {
    if segmentation.enabled && segmentation.update_map && !segmentation.seg_id_pre_skip {
        read_segment_id(decoder, tile_ctx, mi, segmentation, Some(skip)).map(Some)
    } else {
        Ok(None)
    }
}

/// Real `segment_id()` (spec 5.11.9/5.11.10) -- shared core for both the pre-skip and post-skip
/// call sites in `parse_coding_unit`, which differ only in whether `skip` is already known.
/// Ported from dav1d's `decode_b` (`src/decode.c`), not reconstructed from the spec pseudocode
/// alone, to get the skip/temporal interactions exactly right.
///
/// `skip_already_known`: `None` at the pre-skip call site (real spec: `skip` isn't read yet, so
/// no shortcut is available -- the non-temporal-predicted branch always does a real read).
/// `Some(skip)` at the post-skip call site (`skip == true` shortcuts straight to the predicted
/// segment id, no bits read -- matches dav1d's `if (b->skip) { b->seg_id = pred_seg_id; }`) and
/// also gates whether the temporal `seg_pred` bit itself gets read (`!skip && temporal_update`).
fn read_segment_id(
    decoder: &mut SymbolDecoder,
    tile_ctx: &mut TileContext,
    mi: MiRect,
    segmentation: SegmentationInfo,
    skip_already_known: Option<bool>,
) -> Result<u8> {
    let MiRect {
        x4,
        y4,
        width: width_4x4,
        height: height_4x4,
    } = mi;
    let temporal_eligible = segmentation.temporal_update && skip_already_known != Some(true);
    let seg_pred = if temporal_eligible {
        let ctx = tile_ctx.seg_pred_context(x4, y4);
        decoder.read_seg_pred(ctx)?
    } else {
        false
    };
    tile_ctx.set_seg_pred(x4, y4, width_4x4, height_4x4, seg_pred);

    let segment_id = if seg_pred {
        // Temporal prediction: real spec pulls this from the previous frame's segment map. Real
        // bits (`seg_pred` above) are already consumed correctly regardless -- no further bits
        // are read here, so reporting `0` (no cross-frame segment-map state, see
        // `SegmentationInfo`'s doc) doesn't risk desync, only this one CU's reported value.
        0
    } else {
        let (ctx, pred) = tile_ctx.segment_id_context(x4, y4);
        match skip_already_known {
            Some(true) => pred,
            _ => {
                let diff = decoder.read_segment_id_diff(ctx)?;
                let max = segmentation.last_active_seg_id as i32 + 1;
                let decoded = neg_deinterleave(diff as i32, pred as i32, max);
                if !(0..=segmentation.last_active_seg_id as i32).contains(&decoded) {
                    0
                } else {
                    decoded as u8
                }
            }
        }
    };
    tile_ctx.set_segment_id(x4, y4, width_4x4, height_4x4, segment_id);
    Ok(segment_id)
}

/// Decode a `neg_deinterleave`-encoded diff back into a real value (spec 5.11.9/5.11.10's
/// `segment_id()`, also used elsewhere in real AV1 for similarly-encoded values this crate
/// doesn't read) -- ported index-for-index from dav1d's `neg_deinterleave` (`src/decode.c`,
/// `memorysafety/rav1d`/`videolan/dav1d`, BSD-2-Clause), not reimplemented from a description, to
/// avoid an off-by-one in the branch math. `ref_val`/`max` name the spec's `ref`/`max` params
/// (`ref` avoided as a Rust keyword).
fn neg_deinterleave(diff: i32, ref_val: i32, max: i32) -> i32 {
    if ref_val == 0 {
        return diff;
    }
    if ref_val >= max - 1 {
        return max - diff - 1;
    }
    if 2 * ref_val < max {
        if diff <= 2 * ref_val {
            if diff & 1 != 0 {
                ref_val + ((diff + 1) >> 1)
            } else {
                ref_val - (diff >> 1)
            }
        } else {
            diff
        }
    } else if diff <= 2 * (max - ref_val - 1) {
        if diff & 1 != 0 {
            ref_val + ((diff + 1) >> 1)
        } else {
            ref_val - (diff >> 1)
        }
    } else {
        max - (diff + 1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_neg_deinterleave_ref_zero_returns_diff_directly() {
        assert_eq!(neg_deinterleave(3, 0, 8), 3);
    }

    #[test]
    fn test_neg_deinterleave_ref_at_max_boundary() {
        // ref_val=7 >= max-1=7 -> max - diff - 1.
        assert_eq!(neg_deinterleave(2, 7, 8), 5);
    }

    #[test]
    fn test_neg_deinterleave_low_ref_branch() {
        // ref_val=2, max=8 (2*ref_val=4 < max).
        assert_eq!(neg_deinterleave(3, 2, 8), 4); // diff<=4, odd: ref + (diff+1)/2
        assert_eq!(neg_deinterleave(4, 2, 8), 0); // diff<=4, even: ref - diff/2
        assert_eq!(neg_deinterleave(5, 2, 8), 5); // diff>4: diff unchanged
    }

    #[test]
    fn test_neg_deinterleave_high_ref_branch() {
        // ref_val=5, max=8 (2*ref_val=10 >= max, ref_val=5 < max-1=7).
        assert_eq!(neg_deinterleave(3, 5, 8), 7); // diff<=4, odd: ref + (diff+1)/2
        assert_eq!(neg_deinterleave(4, 5, 8), 3); // diff<=4, even: ref - diff/2
        assert_eq!(neg_deinterleave(5, 5, 8), 2); // diff>4: max - (diff+1)
    }
}
