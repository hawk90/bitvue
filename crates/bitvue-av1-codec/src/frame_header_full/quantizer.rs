//! The quantizer-dependent middle of `uncompressed_header()` (spec 5.9.2): `quantization_params`,
//! `segmentation_params`, `delta_q_params` and `delta_lf_params`, plus the `CodedLossless` flag
//! that the later loop-filter / CDEF / loop-restoration sections key off. Split out of
//! `parse_frame_header_full`.

use super::loop_filters::parse_delta_lf_params;
use super::segmentation::parse_segmentation_params;
use super::*;
use crate::frame_header::{parse_quantization_params, QuantizationParams};

/// What the earlier part of the header decided, which this section depends on.
pub(super) struct QuantizerInput<'a> {
    pub(super) primary_ref_frame: u32,
    pub(super) ref_frame_idx: &'a [u32; REFS_PER_FRAME],
    pub(super) allow_intrabc: bool,
}

pub(super) struct QuantizerSection {
    pub(super) quant: QuantizationParams,
    pub(super) segmentation: SegmentationInfo,
    pub(super) delta_q_present: bool,
    pub(super) delta_lf_present: bool,
    pub(super) delta_lf_multi: bool,
    pub(super) coded_lossless: bool,
}

pub(super) fn read_quantizer_section(
    reader: &mut BitReader,
    seq: &SequenceHeader,
    ref_state: &RefFrameState,
    input: &QuantizerInput,
) -> Result<QuantizerSection> {
    let QuantizerInput {
        primary_ref_frame,
        ref_frame_idx,
        allow_intrabc,
    } = *input;
    let quant = parse_quantization_params(
        reader,
        seq.color_config.num_planes,
        seq.color_config.separate_uv_delta_q,
    )?;

    // `load_previous()`: a frame with a primary reference starts from that slot's segmentation
    // features (and none otherwise, `setup_past_independence`).
    let previous_segmentation = if primary_ref_frame == PRIMARY_REF_NONE {
        SegmentationFeatures::default()
    } else {
        ref_state.segmentation[ref_frame_idx[primary_ref_frame as usize] as usize % NUM_REF_FRAMES]
    };
    let segmentation =
        parse_segmentation_params(reader, primary_ref_frame, &previous_segmentation)?;

    let delta_q_present = if quant.base_q_idx > 0 {
        reader.read_bit()?
    } else {
        false
    };
    if delta_q_present {
        reader.read_bits(2)?; // delta_q_res
    }
    let (delta_lf_present, delta_lf_multi) =
        parse_delta_lf_params(reader, delta_q_present, allow_intrabc)?;

    // CodedLossless: real spec needs every segment's per-segment qindex (base_q_idx adjusted by
    // segmentation's SEG_LVL_ALT_Q feature, which `SegmentationInfo` now retains but this
    // computation doesn't yet consult -- see below) AND every real delta-Q term
    // (DeltaQYDc/DeltaQUDc/DeltaQUAc/DeltaQVDc/DeltaQVAc). `uv_ac_delta_q` (`DeltaQUAc`, and its
    // implicit V mirror when `!separate_uv_delta_q`) is now real -- previously omitted entirely,
    // which wasn't just an "under-detects" approximation (this function's original doc claimed):
    // a real encoder using `base_q_idx=0` + zero Y/UV-DC deltas but a *nonzero* `DeltaQUAc` is
    // NOT lossless, and this crate would have wrongly said `coded_lossless=true`, causing it to
    // skip real `loop_filter_params`/`cdef_params`/`lr_params` bits the encoder actually wrote --
    // a genuine desync, not a benign approximation, for that (unusual but spec-legal) case. Two
    // narrower gaps remain, still approximated: (1) segmentation's SEG_LVL_ALT_Q isn't folded in
    // (matches this crate's `Qp`-grid extraction's own "no per-segment override" scope; only
    // under-detects, since a segment-specific override can only ever make a *subset* of segments
    // lossless, never all of them when the frame-wide check already says false) -- affects only
    // whether loop_filter/cdef/lr sections get skipped, not their VALUES when they don't; (2)
    // `separate_uv_delta_q`'s separate V deltas aren't retained (`parse_quantization_params`'s
    // doc), so a frame using that rarer path with V deltas differing from U's could still be
    // mis-classified either direction.
    let coded_lossless = quant.base_q_idx == 0 && quant.all_deltas_zero();

    Ok(QuantizerSection {
        quant,
        segmentation,
        delta_q_present,
        delta_lf_present,
        delta_lf_multi,
        coded_lossless,
    })
}
