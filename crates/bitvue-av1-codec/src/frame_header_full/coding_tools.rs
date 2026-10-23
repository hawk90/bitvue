//! The coding-tool parameters at the end of `uncompressed_header()` (spec 5.9.2): loop filter,
//! CDEF, loop restoration, transform mode, reference mode, skip mode, warped motion, reduced
//! transform set, global motion and film grain. Split out of `parse_frame_header_full`.

use super::film_grain::parse_film_grain_params;
use super::loop_filters::{parse_cdef_params, parse_loop_filter_params, parse_lr_params};
use super::*;

/// What the earlier part of the header decided, which these parameters depend on.
pub(super) struct CodingToolsInput<'a> {
    pub(super) frame_type: FrameType,
    pub(super) frame_is_intra: bool,
    pub(super) show_frame: bool,
    pub(super) showable_frame: bool,
    pub(super) error_resilient_mode: bool,
    pub(super) allow_intrabc: bool,
    pub(super) allow_high_precision_mv: bool,
    pub(super) order_hint: u32,
    pub(super) ref_frame_idx: &'a [u32; REFS_PER_FRAME],
    pub(super) coded_lossless: bool,
    pub(super) all_lossless: bool,
    pub(super) primary_ref_frame: u32,
}

pub(super) struct CodingTools {
    pub(super) loop_filter: LoopFilterInfo,
    pub(super) cdef: CdefInfo,
    pub(super) loop_restoration: LoopRestorationInfo,
    pub(super) txfm_mode: TxfmMode,
    pub(super) reference_select: bool,
    pub(super) skip_mode_present: bool,
    pub(super) skip_mode_refs: [u8; 2],
    pub(super) allow_warped_motion: bool,
    pub(super) reduced_tx_set: bool,
    pub(super) gm_type: [u8; 8],
    pub(super) gm_params: GmParams,
    pub(super) film_grain: FilmGrainInfo,
}

pub(super) fn read_coding_tools(
    reader: &mut BitReader,
    seq: &SequenceHeader,
    ref_state: &RefFrameState,
    input: &CodingToolsInput,
) -> Result<CodingTools> {
    let CodingToolsInput {
        frame_type,
        frame_is_intra,
        show_frame,
        showable_frame,
        error_resilient_mode,
        allow_intrabc,
        allow_high_precision_mv,
        order_hint,
        ref_frame_idx,
        coded_lossless,
        all_lossless,
        primary_ref_frame,
    } = *input;
    let loop_filter = parse_loop_filter_params(
        reader,
        seq.color_config.num_planes,
        coded_lossless,
        allow_intrabc,
    )?;
    let cdef = parse_cdef_params(reader, seq, coded_lossless, allow_intrabc)?;
    let loop_restoration = parse_lr_params(reader, seq, all_lossless, allow_intrabc)?;

    let txfm_mode = read_tx_mode(reader, coded_lossless)?;
    let reference_select = read_frame_reference_mode(reader, frame_is_intra)?;
    let (skip_mode_present, skip_mode_refs) = read_skip_mode_params(
        reader,
        frame_is_intra,
        reference_select,
        seq,
        ref_state,
        ref_frame_idx,
        order_hint,
    )?;
    let skip_mode_refs = skip_mode_refs.unwrap_or([0, 0]);

    let allow_warped_motion = if frame_is_intra || error_resilient_mode || !seq.enable_warped_motion
    {
        false
    } else {
        reader.read_bit()?
    };
    let reduced_tx_set = reader.read_bit()?;

    // spec 5.9.2: `global_motion_params()` is only called when `!FrameIsIntra` -- a KEY_FRAME/
    // INTRA_ONLY_FRAME has no reference frames to hold global motion against, so the encoder
    // never emits this syntax for one at all. Previously called unconditionally here, which for
    // a real key frame spuriously read `REFS_PER_FRAME` (7) `is_global` bits (plus whatever
    // `is_rot_zoom`/`is_translation`/param bits those happened to gate on) that don't exist in
    // the bitstream at that position -- found via a real dav1d oracle build (`DEBUG_BLOCK_INFO`)
    // showing this crate's key-frame `header_size_bytes` (25) didn't match the oracle's
    // independently-derived true tile-data offset (17) for the real fixture's frame 0, an 8-byte
    // (64-bit) overshoot consistent with this exact miscount.
    let (gm_type, gm_params) = if !frame_is_intra {
        // `PrevGmParams` (spec `load_previous` / `setup_past_independence`).
        let prev = if primary_ref_frame == PRIMARY_REF_NONE {
            [GM_IDENTITY_PARAMS; 8]
        } else {
            ref_state.gm_params[ref_frame_idx[primary_ref_frame as usize] as usize % NUM_REF_FRAMES]
        };
        parse_global_motion_params(reader, allow_high_precision_mv, &prev)?
    } else {
        ([GM_TYPE_IDENTITY; 8], [GM_IDENTITY_PARAMS; 8])
    };

    let film_grain = parse_film_grain_params(reader, seq, show_frame, showable_frame, frame_type)?;

    Ok(CodingTools {
        loop_filter,
        cdef,
        loop_restoration,
        txfm_mode,
        reference_select,
        skip_mode_present,
        skip_mode_refs,
        allow_warped_motion,
        reduced_tx_set,
        gm_type,
        gm_params,
        film_grain,
    })
}
