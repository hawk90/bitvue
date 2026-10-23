//! The end of `parse_frame_header_full`: put the parsed sections together into a [`FrameHeader`]
//! and record what this frame refreshes in the reference-slot state.

use super::coding_tools::CodingTools;
use super::frame_prelude::FramePrelude;
use super::frame_size::FrameSize;
use super::quantizer::QuantizerSection;
use super::size_and_refs::SizeAndRefs;
use super::*;
use crate::frame_header::TileLayout;

/// Every section of the header, as the earlier steps returned them.
pub(super) struct HeaderSections {
    pub(super) prelude: FramePrelude,
    pub(super) refs: SizeAndRefs,
    pub(super) disable_frame_end_update_cdf: bool,
    pub(super) tiles: TileLayout,
    pub(super) quantizer: QuantizerSection,
    pub(super) tools: CodingTools,
    pub(super) header_size_bits: u64,
    pub(super) header_size_bytes: usize,
}

/// Update the reference-slot state for whichever slots this frame refreshes, so a later
/// `parse_frame_header_full` call (a later frame in the same sequential scan) sees this frame's
/// order hint when computing `skip_mode_params`. Must run after every early-return path of the
/// parse (`show_existing_frame` is handled separately; none of the `Err` short-circuits reach here).
pub(super) fn refresh_ref_slots(
    ref_state: &mut RefFrameState,
    refresh_frame_flags: u32,
    order_hint: u32,
    segmentation: &SegmentationInfo,
    gm_params: &GmParams,
) {
    for i in 0..NUM_REF_FRAMES {
        if (refresh_frame_flags >> i) & 1 == 1 {
            ref_state.ref_order_hint[i] = order_hint;
            ref_state.segmentation[i] = SegmentationFeatures::from(segmentation);
            ref_state.gm_params[i] = *gm_params;
        }
    }
}

pub(super) fn assemble_frame_header(sections: HeaderSections) -> FrameHeader {
    let HeaderSections {
        prelude,
        refs,
        disable_frame_end_update_cdf,
        tiles,
        quantizer,
        tools,
        header_size_bits,
        header_size_bytes,
    } = sections;
    let FrameSize {
        width,
        height,
        upscaled_width,
        upscaled_height,
        use_superres,
        superres_denom,
    } = refs.size;
    FrameHeader {
        frame_type: prelude.frame_type,
        show_frame: prelude.show_frame,
        show_existing_frame: false,
        frame_to_show_map_idx: None,
        error_resilient_mode: prelude.error_resilient_mode,
        base_q_idx: Some(quantizer.quant.base_q_idx),
        y_dc_delta_q: Some(quantizer.quant.y_dc),
        uv_dc_delta_q: Some(quantizer.quant.u_dc),
        delta_q_present: quantizer.delta_q_present,
        delta_q_residue: None,
        header_size_bytes,
        header_size_bits,
        tiles: Some(tiles),
        refresh_frame_flags: Some(prelude.refresh_frame_flags as u8),
        ref_frame_idx: if prelude.frame_is_intra {
            None
        } else {
            Some(std::array::from_fn(|i| refs.ref_frame_idx[i] as u8))
        },
        order_hint: prelude.order_hint,
        width,
        height,
        upscaled_width,
        upscaled_height,
        reference_select: tools.reference_select,
        allow_intrabc: refs.allow_intrabc,
        allow_screen_content_tools: prelude.allow_screen_content_tools,
        delta_lf_present: quantizer.delta_lf_present,
        delta_lf_multi: quantizer.delta_lf_multi,
        skip_mode_present: tools.skip_mode_present,
        skip_mode_refs: tools.skip_mode_refs,
        subpel_filter_switchable: refs.subpel_filter_switchable,
        switchable_motion_mode: refs.switchable_motion_mode,
        allow_warped_motion: tools.allow_warped_motion,
        force_integer_mv: prelude.force_integer_mv,
        allow_high_precision_mv: refs.allow_high_precision_mv,
        gm_type: tools.gm_type,
        gm_params: tools.gm_params,
        reduced_tx_set: tools.reduced_tx_set,
        txfm_mode: tools.txfm_mode,
        use_ref_frame_mvs: refs.use_ref_frame_mvs,
        primary_ref_frame: prelude.primary_ref_frame,
        disable_frame_end_update_cdf,
        segmentation: quantizer.segmentation,
        loop_filter: tools.loop_filter,
        cdef_damping: tools.cdef.clone(),
        cdef_y_primary_strength: tools.cdef.y_primary_strength,
        cdef_y_secondary_strength: tools.cdef.y_secondary_strength,
        cdef_uv_primary_strength: tools.cdef.uv_primary_strength,
        cdef_uv_secondary_strength: tools.cdef.uv_secondary_strength,
        loop_restoration: tools.loop_restoration,
        film_grain: tools.film_grain,
        super_resolution: SuperResolutionInfo {
            enabled: use_superres,
            scale_denominator: superres_denom as u8,
        },
    }
}
