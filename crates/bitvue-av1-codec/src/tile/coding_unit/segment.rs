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

    // ---- the three entry points ---------------------------------------------------------

    fn noise(seed: u64, len: usize) -> Vec<u8> {
        let mut x = seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1;
        (0..len)
            .map(|_| {
                x ^= x << 13;
                x ^= x >> 7;
                x ^= x << 17;
                (x >> 24) as u8
            })
            .collect()
    }

    fn seg(update_map: bool, pre_skip: bool, temporal: bool, last_active: u8) -> SegmentationInfo {
        SegmentationInfo {
            enabled: true,
            update_map,
            temporal_update: temporal,
            seg_id_pre_skip: pre_skip,
            last_active_seg_id: last_active,
            ..SegmentationInfo::default()
        }
    }

    fn mi(x4: u32, y4: u32) -> MiRect {
        MiRect {
            x4,
            y4,
            width: 2,
            height: 2,
        }
    }

    fn bits(decoder: &mut SymbolDecoder) -> u32 {
        decoder.read_bools_n(24).unwrap()
    }

    #[test]
    fn disabled_segmentation_reads_nothing_at_either_position() {
        let data = noise(1, 32);
        let untouched = bits(&mut SymbolDecoder::new(&data).unwrap());
        let off = SegmentationInfo::default();
        for skip in [false, true] {
            let mut d = SymbolDecoder::new(&data).unwrap();
            let mut ctx = TileContext::new(16, 16);
            assert_eq!(
                read_pre_skip(&mut d, &mut ctx, mi(0, 0), off).unwrap(),
                None
            );
            assert_eq!(
                read_post_skip(&mut d, &mut ctx, mi(0, 0), off, skip).unwrap(),
                None
            );
            assert_eq!(bits(&mut d), untouched);
        }
    }

    #[test]
    fn without_update_map_the_id_is_zero_and_nothing_is_read() {
        let data = noise(2, 32);
        let untouched = bits(&mut SymbolDecoder::new(&data).unwrap());
        let s = seg(false, true, true, 7);
        let mut d = SymbolDecoder::new(&data).unwrap();
        let mut ctx = TileContext::new(16, 16);
        // Pre-skip reports 0 and records it; post-skip leaves the id alone.
        assert_eq!(
            read_pre_skip(&mut d, &mut ctx, mi(4, 4), s).unwrap(),
            Some(0)
        );
        assert_eq!(
            read_post_skip(&mut d, &mut ctx, mi(4, 4), s, false).unwrap(),
            None
        );
        // The 0 is recorded: with the left and above-left cells written and the cell above not,
        // exactly one pair agrees (context 1); with nothing recorded all three would agree (2).
        assert_eq!(ctx.segment_id_context(6, 5).0, 1);
        assert_eq!(bits(&mut d), untouched);
        assert_eq!(ctx.segment_id_context(5, 4).1, 0);
    }

    #[test]
    fn the_read_happens_before_skip_only_when_seg_id_pre_skip_is_set() {
        let data = noise(3, 32);
        let untouched = bits(&mut SymbolDecoder::new(&data).unwrap());

        // seg_id_pre_skip: pre-skip reads, post-skip does not.
        let s = seg(true, true, false, 7);
        let mut d = SymbolDecoder::new(&data).unwrap();
        let mut ctx = TileContext::new(16, 16);
        assert!(read_pre_skip(&mut d, &mut ctx, mi(0, 0), s)
            .unwrap()
            .is_some());
        let after_read = bits(&mut d);
        assert_ne!(after_read, untouched, "a symbol was consumed");
        let mut d = SymbolDecoder::new(&data).unwrap();
        assert_eq!(
            read_post_skip(&mut d, &mut ctx, mi(0, 0), s, false).unwrap(),
            None
        );
        assert_eq!(bits(&mut d), untouched);

        // !seg_id_pre_skip: the other way round.
        let s = seg(true, false, false, 7);
        let mut d = SymbolDecoder::new(&data).unwrap();
        let mut ctx = TileContext::new(16, 16);
        assert_eq!(read_pre_skip(&mut d, &mut ctx, mi(0, 0), s).unwrap(), None);
        assert_eq!(bits(&mut d), untouched);
        let mut d = SymbolDecoder::new(&data).unwrap();
        assert!(read_post_skip(&mut d, &mut ctx, mi(0, 0), s, false)
            .unwrap()
            .is_some());
        assert_eq!(
            bits(&mut d),
            after_read,
            "same symbol, same position in the stream"
        );
    }

    #[test]
    fn a_decoded_id_is_the_deinterleaved_difference_against_the_neighbour_prediction() {
        let s = seg(true, true, false, 7);
        let mut seen = std::collections::BTreeSet::new();
        for seed in 1..=64u64 {
            let data = noise(seed, 48);
            let (ctx0, pred) = TileContext::new(16, 16).segment_id_context(0, 0);
            assert_eq!((ctx0, pred), (0, 0), "no neighbours");
            let diff = SymbolDecoder::new(&data)
                .unwrap()
                .read_segment_id_diff(0)
                .unwrap();
            let expected = neg_deinterleave(diff as i32, 0, 8);

            let mut d = SymbolDecoder::new(&data).unwrap();
            let mut ctx = TileContext::new(16, 16);
            let id = read_pre_skip(&mut d, &mut ctx, mi(0, 0), s)
                .unwrap()
                .unwrap();
            assert_eq!(id as i32, expected, "seed {seed}");
            // The id is recorded for the neighbours' prediction.
            assert_eq!(ctx.segment_id_context(2, 0).1, id, "seed {seed}");
            seen.insert(id);
        }
        assert!(seen.len() >= 3, "seeds should decode several ids: {seen:?}");
    }

    #[test]
    fn an_id_beyond_last_active_seg_id_is_reported_as_zero() {
        // last_active_seg_id = 0: only id 0 exists, any non-zero difference decodes out of range.
        let s = seg(true, true, false, 0);
        let mut out_of_range = 0;
        for seed in 1..=64u64 {
            let data = noise(seed, 48);
            let diff = SymbolDecoder::new(&data)
                .unwrap()
                .read_segment_id_diff(0)
                .unwrap();
            let mut d = SymbolDecoder::new(&data).unwrap();
            let mut ctx = TileContext::new(16, 16);
            let id = read_pre_skip(&mut d, &mut ctx, mi(0, 0), s)
                .unwrap()
                .unwrap();
            assert_eq!(id, 0, "seed {seed} diff {diff}");
            // The id is still consumed from the stream and recorded as 0.
            assert_eq!(ctx.segment_id_context(2, 0).1, 0);
            out_of_range += usize::from(diff != 0);
        }
        assert!(
            out_of_range > 0,
            "some seed must decode a non-zero difference"
        );

        // And with last_active_seg_id = 2 the largest valid id is 2, never 3..=7.
        let s = seg(true, true, false, 2);
        for seed in 1..=64u64 {
            let data = noise(seed, 48);
            let mut d = SymbolDecoder::new(&data).unwrap();
            let mut ctx = TileContext::new(16, 16);
            let id = read_pre_skip(&mut d, &mut ctx, mi(0, 0), s)
                .unwrap()
                .unwrap();
            assert!(id <= 2, "seed {seed}: {id}");
        }
    }

    #[test]
    fn a_skipped_block_takes_the_predicted_id_without_reading_the_difference() {
        let s = seg(true, false, false, 7);
        for seed in 1..=16u64 {
            let data = noise(seed, 48);
            let untouched = bits(&mut SymbolDecoder::new(&data).unwrap());
            let mut ctx = TileContext::new(16, 16);
            // Give the left neighbour id 3 so the prediction is not trivially 0.
            ctx.set_segment_id(0, 0, 2, 2, 3);
            let (_, pred) = ctx.segment_id_context(2, 0);
            assert_eq!(pred, 3);
            let mut d = SymbolDecoder::new(&data).unwrap();
            let id = read_post_skip(&mut d, &mut ctx, mi(2, 0), s, true)
                .unwrap()
                .unwrap();
            assert_eq!(id, 3);
            assert_eq!(bits(&mut d), untouched, "skip: no segment_id_diff symbol");
        }
    }

    #[test]
    fn temporal_update_reads_seg_pred_and_a_predicted_block_reports_zero() {
        let s = seg(true, true, true, 7);
        let (mut predicted, mut explicit) = (0, 0);
        for seed in 1..=64u64 {
            let data = noise(seed, 48);
            let mut r = SymbolDecoder::new(&data).unwrap();
            let seg_pred = r.read_seg_pred(0).unwrap();

            let mut d = SymbolDecoder::new(&data).unwrap();
            let mut ctx = TileContext::new(16, 16);
            let id = read_pre_skip(&mut d, &mut ctx, mi(0, 0), s)
                .unwrap()
                .unwrap();
            if seg_pred {
                predicted += 1;
                assert_eq!(id, 0, "seed {seed}: temporal prediction reports 0");
                assert_eq!(
                    bits(&mut d),
                    bits(&mut r),
                    "seed {seed}: no difference symbol"
                );
            } else {
                explicit += 1;
                let diff = r.read_segment_id_diff(0).unwrap();
                assert_eq!(
                    id as i32,
                    neg_deinterleave(diff as i32, 0, 8),
                    "seed {seed}"
                );
            }
        }
        assert!(
            predicted > 0 && explicit > 0,
            "{predicted} predicted / {explicit} explicit"
        );

        // A skipped block never reads seg_pred: post-skip with skip == true consumes nothing.
        let data = noise(7, 48);
        let untouched = bits(&mut SymbolDecoder::new(&data).unwrap());
        let s = seg(true, false, true, 7);
        let mut d = SymbolDecoder::new(&data).unwrap();
        let mut ctx = TileContext::new(16, 16);
        read_post_skip(&mut d, &mut ctx, mi(0, 0), s, true).unwrap();
        assert_eq!(bits(&mut d), untouched);
    }
}
