//! `skip_mode_params()` (spec 5.9.22) and order-hint distance.

use super::*;

/// `get_relative_dist` (AV1 spec 5.9.3) -- signed circular distance between two order hints.
/// Signed relative distance between two order hints per AV1 spec 7.9.2 `get_relative_dist` --
/// positive when `a` is "after" `b` in display order. `pub` so `bitvue-sidecar`'s
/// `codec_extended_info` can reuse it to classify references into L0 (forward/past) vs L1
/// (backward/future), the same comparison `skip_mode_params` uses internally.
pub fn relative_dist(a: u32, b: u32, enable_order_hint: bool, order_hint_bits: u32) -> i64 {
    if !enable_order_hint || order_hint_bits == 0 {
        return 0;
    }
    let diff = a.wrapping_sub(b) as i32;
    let m = 1i32 << (order_hint_bits - 1);
    ((diff & (m - 1)) - (diff & m)) as i64
}

#[allow(clippy::too_many_arguments)]
pub(super) fn read_skip_mode_params(
    reader: &mut BitReader,
    frame_is_intra: bool,
    reference_select: bool,
    seq: &SequenceHeader,
    ref_state: &RefFrameState,
    ref_frame_idx: &[u32; REFS_PER_FRAME],
    order_hint: u32,
) -> Result<(bool, Option<[u8; 2]>)> {
    let order_hint_bits = seq
        .order_hint_bits_minus_1
        .map(|v| v as u32 + 1)
        .unwrap_or(0);
    let enable_order_hint = seq.enable_order_hint;

    // `SkipModeFrame[0]/[1]` (spec 5.9.22) -- previously only a `skip_mode_allowed` bool was kept
    // (the derived ref indices discarded), leaving `skip_mode`'s real forced `ref_frame()`/`mv[]`
    // derivation (spec 5.11.25, dav1d `decode.c:1401-1423`) with nothing to force from. Real spec:
    // when both a forward and backward ref exist, use the closest of each (`min`/`max` of their
    // `ref_frame_idx[]` positions, `+1` for `LAST_FRAME`-relative `RefFrame` numbering); when only
    // a forward ref exists, fall back to the two closest forward refs instead (same "maximize
    // `relative_dist` against the already-found one" search pattern as the primary forward search,
    // just restricted to refs strictly closer than it).
    let skip_mode_refs = if frame_is_intra || !reference_select || !enable_order_hint {
        None
    } else {
        let mut forward_idx: Option<usize> = None;
        let mut forward_hint = 0u32;
        let mut backward_idx: Option<usize> = None;
        let mut backward_hint = 0u32;
        for (i, &idx) in ref_frame_idx.iter().enumerate() {
            let hint = ref_state.ref_order_hint[idx as usize];
            let d = relative_dist(hint, order_hint, enable_order_hint, order_hint_bits);
            if d < 0 {
                if forward_idx.is_none()
                    || relative_dist(hint, forward_hint, enable_order_hint, order_hint_bits) > 0
                {
                    forward_idx = Some(i);
                    forward_hint = hint;
                }
            } else if d > 0
                && (backward_idx.is_none()
                    || relative_dist(hint, backward_hint, enable_order_hint, order_hint_bits) < 0)
            {
                backward_idx = Some(i);
                backward_hint = hint;
            }
        }
        match (forward_idx, backward_idx) {
            (None, _) => None,
            (Some(fwd), Some(bwd)) => Some([fwd.min(bwd) as u8 + 1, fwd.max(bwd) as u8 + 1]),
            (Some(fwd), None) => {
                let mut second_forward_idx: Option<usize> = None;
                let mut second_forward_hint = 0u32;
                for (i, &idx) in ref_frame_idx.iter().enumerate() {
                    let hint = ref_state.ref_order_hint[idx as usize];
                    if relative_dist(hint, forward_hint, enable_order_hint, order_hint_bits) < 0
                        && (second_forward_idx.is_none()
                            || relative_dist(
                                hint,
                                second_forward_hint,
                                enable_order_hint,
                                order_hint_bits,
                            ) > 0)
                    {
                        second_forward_idx = Some(i);
                        second_forward_hint = hint;
                    }
                }
                second_forward_idx
                    .map(|second| [fwd.min(second) as u8 + 1, fwd.max(second) as u8 + 1])
            }
        }
    };
    let skip_mode_present = if skip_mode_refs.is_some() {
        reader.read_bit()?
    } else {
        false
    };
    Ok((skip_mode_present, skip_mode_refs))
}
