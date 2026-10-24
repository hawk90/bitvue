//! The header of a `show_existing_frame = 1` frame (spec 5.9.2), split out of
//! `parse_frame_header_full`.

use super::*;

/// Everything after `show_existing_frame = 1`: the slot to show, and a header that is a shown key
/// frame with every other field at its default. `reader` is positioned right after the flag.
pub(super) fn parse_show_existing_frame(
    reader: &mut BitReader,
    ref_state: &RefFrameState,
) -> Result<FrameHeader> {
    let frame_to_show_map_idx = reader.read_bits(3)?;
    // Real AV1 also refreshes RefOrderHint for a KEY-frame show_existing_frame using the
    // shown slot's own already-tracked order hint (RefOrderHint[idx] stays what it was) --
    // no new hint is introduced, so ref_state needs no update here. The shown frame's
    // OrderHint (spec 7.4) IS that same already-tracked slot value, though.
    let order_hint = ref_state.ref_order_hint[frame_to_show_map_idx as usize];
    Ok(FrameHeader {
        frame_type: FrameType::Key,
        show_frame: true,
        show_existing_frame: true,
        frame_to_show_map_idx: Some(frame_to_show_map_idx as u8),
        error_resilient_mode: false,
        base_q_idx: None,
        y_dc_delta_q: None,
        uv_dc_delta_q: None,
        delta_q_present: false,
        delta_q_residue: None,
        header_size_bytes: reader.byte_position(),
        header_size_bits: reader.position(),
        tiles: None,
        refresh_frame_flags: None,
        ref_frame_idx: None,
        order_hint,
        width: 0,
        height: 0,
        upscaled_width: 0,
        upscaled_height: 0,
        reference_select: false,
        allow_intrabc: false,
        allow_screen_content_tools: false,
        delta_lf_present: false,
        delta_lf_multi: false,
        skip_mode_present: false,
        skip_mode_refs: [0, 0],
        subpel_filter_switchable: false,
        switchable_motion_mode: false,
        allow_warped_motion: false,
        force_integer_mv: false,
        allow_high_precision_mv: false,
        gm_type: [0u8; 8],
        gm_params: [GM_IDENTITY_PARAMS; 8],
        ref_scaled: [false; 8],
        reduced_tx_set: false,
        txfm_mode: TxfmMode::Largest,
        use_ref_frame_mvs: false,
        primary_ref_frame: 7,
        disable_frame_end_update_cdf: true,
        segmentation: SegmentationInfo::default(),
        loop_filter: LoopFilterInfo::default(),
        cdef_damping: CdefInfo::default(),
        cdef_y_primary_strength: 0,
        cdef_y_secondary_strength: 0,
        cdef_uv_primary_strength: 0,
        cdef_uv_secondary_strength: 0,
        loop_restoration: LoopRestorationInfo::default(),
        // `load_grain_params(frame_to_show_map_idx)`.
        film_grain: ref_state.film_grain[frame_to_show_map_idx as usize].clone(),
        super_resolution: SuperResolutionInfo::default(),
    })
}
