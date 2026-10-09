//! Full `uncompressed_header()` parsing (AV1 spec Section 5.9.2), extending
//! [`crate::frame_header::parse_frame_header_basic`] through the sections that basic parsing
//! deliberately skips: `segmentation_params`, `loop_filter_params`, `cdef_params`, `lr_params`,
//! `read_tx_mode`, `frame_reference_mode`, `skip_mode_params`, `global_motion_params`, and
//! `film_grain_params`. Exists to feed [`crate::advanced_features`]'s CDEF/loop-restoration/
//! film-grain/super-resolution extractors real (non-default) `FrameHeader` fields -- those
//! extractors already existed and worked, but nothing in the codebase ever produced a
//! `FrameHeader` with `cdef_damping`/`loop_restoration`/`film_grain`/`super_resolution`
//! populated (`parse_frame_header_basic` always leaves them at `Default::default()`).
//!
//! # Why this needs real sequence-header context (and `parse_frame_header_basic` didn't)
//!
//! `parse_frame_header_basic` hardcodes several assumptions specifically so it can avoid needing
//! a `SequenceHeader` (see its own module doc). Reaching `cdef_params()`/`lr_params()`/
//! `film_grain_params()` correctly requires actually knowing `enable_cdef`/`enable_restoration`/
//! `enable_superres`/`enable_order_hint`/`use_128x128_superblock`/color config -- all real
//! sequence-header fields, not assumption placeholders. `parse_sequence_header` (this crate)
//! already parses all of these; this module is the first caller to actually need that context
//! for a *frame* header.
//!
//! # Why this needs cross-frame state (and `get_frame_analysis` didn't)
//!
//! `skip_mode_params()`'s bit *presence* (not just its value) depends on comparing the current
//! frame's reference frames' order hints against each other -- which requires knowing each
//! reference slot's order hint, which is only knowable by having processed every earlier frame in
//! decode order and tracked `RefOrderHint[8]` (updated via each frame's `refresh_frame_flags`).
//! `RefFrameState` is exactly that -- an 8-slot `u32` array, nothing more. This is NOT a general
//! decoder simulation: it does not need probability/CDF state, tile/pixel data, or
//! `PrevGmParams` (global motion's subexponential-Golomb code length is self-terminating and
//! independent of the reference value being decoded relative to -- verified against the spec's
//! `decode_subexp`, which never reads `r`; only the final `inverse_recenter` remap uses it, and
//! this module never needs the recentered *value*, only correct bit consumption past it).
//!
//! Callers must parse every frame from index 0 up to and including the target frame, in decode
//! order, threading the same `RefFrameState` through each call -- the same "start from the
//! beginning" pattern `debug_yuv::find_first_diff` already uses for a different reason
//! (single-pass streaming decode instead of redecode-per-call).
//!
//! # Known unsupported case: short reference-frame signaling
//!
//! `frame_refs_short_signaling` (an inter-frame flag, present only when `enable_order_hint`) can
//! select "short" ref-frame signaling, which requires `set_frame_refs()` -- a real reference-slot
//! selection *algorithm* (nearest/furthest-by-order-hint search across all 8 ref slots), not just
//! more bits to read. This module does not implement it; a frame using short signaling returns
//! `Err`. This mirrors `parse_frame_header_basic`'s own precedent of documented, narrower scope
//! rather than full generality (see its module doc's own assumption list).
//!
//! No independent decoder oracle validates this beyond the real AV1 test fixture parsing without
//! error end-to-end and producing internally-consistent (non-default, in-range) values -- the
//! `dav1d` Rust crate (used elsewhere in this workspace for pixel decode) does not expose header
//! introspection, so there is no independent ground truth to diff against, unlike e.g.
//! `decode_bridge`'s pixel-byte tests pinned against known-correct output.

mod film_grain;
mod frame_size;
mod global_motion;
mod loop_filters;
mod segmentation;
mod skip_mode;
#[cfg(test)]
mod tests;
mod tile_info;

#[cfg(test)]
pub(crate) use self::global_motion::GM_TYPE_ROTZOOM;
#[cfg(test)]
use self::global_motion::{classify_gm_type, GM_TYPE_AFFINE};
pub(crate) use self::global_motion::{GM_TYPE_IDENTITY, GM_TYPE_TRANSLATION};
pub use self::segmentation::SegmentationInfo;
#[cfg(test)]
use self::segmentation::{MAX_SEGMENTS, SEG_LVL_MAX};
pub(crate) use self::segmentation::{SEG_LVL_GLOBALMV, SEG_LVL_REF_FRAME, SEG_LVL_SKIP};
pub use self::skip_mode::relative_dist;
use self::{
    film_grain::parse_film_grain_params,
    frame_size::{read_frame_size, read_interpolation_filter, read_render_size},
    global_motion::parse_global_motion_params,
    loop_filters::{
        parse_cdef_params, parse_delta_lf_params, parse_loop_filter_params, parse_lr_params,
    },
    segmentation::{parse_segmentation_params, SegmentationFeatures},
    skip_mode::read_skip_mode_params,
    tile_info::read_tile_info,
};

use crate::bitreader::BitReader;
use crate::frame_header::{
    CdefInfo, FilmGrainInfo, FrameHeader, FrameType, LoopFilterInfo, LoopRestorationInfo,
    LoopRestorationType, SuperResolutionInfo, TxfmMode,
};
use crate::sequence::SequenceHeader;
use bitvue_engine::{BitvueError, Result};

const NUM_REF_FRAMES: usize = 8;
const REFS_PER_FRAME: usize = 7;
const PRIMARY_REF_NONE: u32 = 7;
const SELECT_SCREEN_CONTENT_TOOLS: u32 = 2;
const SELECT_INTEGER_MV: u32 = 2;
/// Per-reference-slot order-hint state, threaded through sequential
/// [`parse_frame_header_full`] calls (frame 0 first) -- see module doc for why this (and only
/// this) piece of cross-frame state is needed.
#[derive(Debug, Clone, Default)]
pub struct RefFrameState {
    ref_order_hint: [u32; NUM_REF_FRAMES],
    /// Segmentation feature data each slot was last refreshed with (spec
    /// `save_segmentation_params`).
    segmentation: [SegmentationFeatures; NUM_REF_FRAMES],
}

impl RefFrameState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Read-only access to the per-slot order hints -- used by
    /// [`crate::tile::motion_field`]'s sequential test harness to snapshot this state *before*
    /// applying [`RefFrameState::apply_refresh`] for the frame just parsed (see
    /// `MotionFieldState::update`'s doc for why the snapshot must be taken beforehand).
    pub fn ref_order_hint(&self) -> &[u32; NUM_REF_FRAMES] {
        &self.ref_order_hint
    }

    /// Apply one frame's `refresh_frame_flags`/`order_hint` update directly, without re-running
    /// `parse_frame_header_full` -- mirrors this module's own internal per-frame update
    /// (`ref_state.ref_order_hint[i] = order_hint` for each refreshed slot, see this function's
    /// call site in `parse_frame_header_full`). [`ParsedFrame`](crate::overlay_extraction::
    /// ParsedFrame)'s `order_hint`/`refresh_frame_flags` fields are sourced from the same
    /// bit-position-independent header fields this update reads, so a caller with access to a
    /// `ParsedFrame` never needs a second header parse just to keep this state current.
    /// Copies slot `from` into every other slot: the state a `show_existing_frame` of a key frame
    /// leaves behind (spec 7.21, `refresh_frame_flags = allFrames`).
    pub fn copy_slot_to_all(&mut self, from: usize) {
        let from = from % NUM_REF_FRAMES;
        let (hint, segmentation) = (self.ref_order_hint[from], self.segmentation[from]);
        self.ref_order_hint = [hint; NUM_REF_FRAMES];
        self.segmentation = [segmentation; NUM_REF_FRAMES];
    }

    pub fn apply_refresh(&mut self, refresh_frame_flags: u8, order_hint: u32) {
        for i in 0..NUM_REF_FRAMES {
            if (refresh_frame_flags >> i) & 1 == 1 {
                self.ref_order_hint[i] = order_hint;
            }
        }
    }
}

/// `relative_dist(RefOrderHint[ref], OrderHint)` for `LAST..=ALTREF` (spec 5.9.2's `OrderHints`
/// against the current frame): positive for a reference displayed after the frame.
/// `prev_ref_order_hint` is [`RefFrameState`]'s order hints from before this frame's own header;
/// intra frames have no references, so all zero.
pub fn ref_order_distance(
    prev_ref_order_hint: &[u32; NUM_REF_FRAMES],
    ref_frame_idx: Option<&[u8; REFS_PER_FRAME]>,
    order_hint: u32,
    seq: &SequenceHeader,
) -> [i32; REFS_PER_FRAME] {
    let Some(ref_frame_idx) = ref_frame_idx else {
        return [0; REFS_PER_FRAME];
    };
    let bits = seq.order_hint_bits_minus_1.map_or(0, |v| u32::from(v) + 1);
    std::array::from_fn(|i| {
        let hint = prev_ref_order_hint[ref_frame_idx[i] as usize % NUM_REF_FRAMES];
        relative_dist(hint, order_hint, seq.enable_order_hint, bits) as i32
    })
}

/// `RefFrameSignBias[LAST..=ALTREF]` (spec 5.9.2): true when a reference is displayed after the
/// current frame -- see [`ref_order_distance`].
pub fn ref_frame_sign_bias(
    prev_ref_order_hint: &[u32; NUM_REF_FRAMES],
    ref_frame_idx: Option<&[u8; REFS_PER_FRAME]>,
    order_hint: u32,
    seq: &SequenceHeader,
) -> [bool; REFS_PER_FRAME] {
    ref_order_distance(prev_ref_order_hint, ref_frame_idx, order_hint, seq).map(|d| d > 0)
}

fn err(msg: impl Into<String>) -> BitvueError {
    BitvueError::Decode(msg.into())
}

/// `ns(n)` -- non-symmetric unsigned encoding (AV1 spec 4.10.7). Reads `ceil(log2(n))` or
/// `floor(log2(n))+1` bits depending on where the value falls, so it isn't a fixed-width read.
fn read_ns(reader: &mut BitReader, n: u32) -> Result<u32> {
    if n <= 1 {
        return Ok(0);
    }
    let w = 32 - (n - 1).leading_zeros(); // FloorLog2(n) + 1, for n > 1
    let m = (1u32 << w) - n;
    let v = reader.read_bits((w - 1) as u8)?;
    if v < m {
        return Ok(v);
    }
    let extra_bit = reader.read_bit()?;
    Ok((v << 1) - m + u32::from(extra_bit))
}

fn tile_log2(blk_size: u32, target: u32) -> u32 {
    let mut k = 0u32;
    while (blk_size << k) < target {
        k += 1;
    }
    k
}

fn read_tx_mode(reader: &mut BitReader, coded_lossless: bool) -> Result<TxfmMode> {
    if coded_lossless {
        return Ok(TxfmMode::Only4x4);
    }
    let tx_mode_select = reader.read_bit()?;
    Ok(if tx_mode_select {
        TxfmMode::Switchable
    } else {
        TxfmMode::Largest
    })
}

fn read_frame_reference_mode(reader: &mut BitReader, frame_is_intra: bool) -> Result<bool> {
    if frame_is_intra {
        Ok(false)
    } else {
        Ok(reader.read_bit()?)
    }
}

/// Finds the Frame/FrameHeader OBU's payload within one IVF chunk's OBU-container bytes -- an
/// IVF chunk is a Temporal Delimiter followed by the real Frame/FrameHeader OBU (and, for
/// `Frame` OBUs, tile group data appended after the header within the same OBU). Mirrors
/// `bitvue-indexer`'s private `find_frame_obu` (not reused directly -- that function returns the
/// whole `ObuWithOffset`, used there for `ref_frame_idx`/`base_q_idx` only; this crate's callers
/// need the raw payload slice to hand to [`parse_frame_header_full`]).
pub fn find_frame_header_payload(chunk_data: &[u8]) -> Option<std::sync::Arc<[u8]>> {
    find_frame_header_payloads(chunk_data).into_iter().next()
}

/// Every Frame/FrameHeader OBU payload of one IVF chunk, in decode order. A temporal unit can
/// carry several frames (a hidden ARF followed by the shown frame, as the real fixture's IVF
/// chunk 1 does); each one updates the reference slots, so state threading must visit all of
/// them, not only the first.
pub fn find_frame_header_payloads(chunk_data: &[u8]) -> Vec<std::sync::Arc<[u8]>> {
    let mut iter = crate::obu::ObuIterator::new(chunk_data);
    let mut payloads = Vec::new();
    while let Some(Ok(found)) = iter.next_obu_with_offset() {
        if matches!(
            found.obu.header.obu_type,
            crate::obu::ObuType::Frame | crate::obu::ObuType::FrameHeader
        ) {
            payloads.push(std::sync::Arc::clone(&found.obu.payload));
        }
    }
    payloads
}

/// Threads `RefFrameState` sequentially across frames `[0, frame_index)` -- NOT including
/// `frame_index` itself -- returning the state as it stood immediately before `frame_index`'s
/// own header would be parsed. This is the state every `bitvue-sidecar` per-frame command needs
/// to correctly reach `frame_index` via [`crate::overlay_extraction::ParsedFrame::
/// parse_with_ref_state`] (its `tile_data` slicing depends on `skip_mode_params()`'s presence
/// bit, spec 5.9.22, which in turn depends on real per-slot order hints -- see that method's
/// doc) or via a direct [`parse_frame_header_full`] call for `frame_index`'s own header fields
/// (the `av1_features`/`codec_extended_info`/`deblocking` sidecar modules' existing pattern).
/// Shared here instead of duplicated per-module since five sidecar command modules all need
/// exactly this recipe.
pub fn thread_ref_state_before(
    frames: &[crate::ivf::IvfFrame],
    seq: &SequenceHeader,
    frame_index: usize,
) -> std::result::Result<RefFrameState, BitvueError> {
    let mut ref_state = RefFrameState::new();
    for frame in frames.iter().take(frame_index) {
        let payloads = find_frame_header_payloads(&frame.data);
        if payloads.is_empty() {
            return Err(BitvueError::InvalidData(
                "frame has no Frame/FrameHeader OBU".to_string(),
            ));
        }
        for payload in payloads {
            parse_frame_header_full(&payload, seq, &mut ref_state)?;
        }
    }
    Ok(ref_state)
}

/// Parses one frame header OBU payload completely (through `film_grain_params()`), given real
/// sequence-header context and the reference-slot state accumulated from every earlier frame in
/// decode order. See module doc for the cross-frame state requirement and the short
/// reference-signaling limitation.
pub fn parse_frame_header_full(
    payload: &[u8],
    seq: &SequenceHeader,
    ref_state: &mut RefFrameState,
) -> std::result::Result<FrameHeader, BitvueError> {
    let mut reader = BitReader::new(payload);
    let order_hint_bits = seq
        .order_hint_bits_minus_1
        .map(|v| v as u32 + 1)
        .unwrap_or(0);

    let show_existing_frame = if seq.reduced_still_picture_header {
        false
    } else {
        reader.read_bit()?
    };
    if show_existing_frame {
        let frame_to_show_map_idx = reader.read_bits(3)?;
        // Real AV1 also refreshes RefOrderHint for a KEY-frame show_existing_frame using the
        // shown slot's own already-tracked order hint (RefOrderHint[idx] stays what it was) --
        // no new hint is introduced, so ref_state needs no update here. The shown frame's
        // OrderHint (spec 7.4) IS that same already-tracked slot value, though.
        let order_hint = ref_state.ref_order_hint[frame_to_show_map_idx as usize];
        return Ok(FrameHeader {
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
            film_grain: FilmGrainInfo::default(),
            super_resolution: SuperResolutionInfo::default(),
        });
    }

    let frame_type = if seq.reduced_still_picture_header {
        FrameType::Key
    } else {
        FrameType::from_av1_bits(reader.read_bits(2)?)
    };
    let frame_is_intra = frame_type.is_intra();
    let show_frame = if seq.reduced_still_picture_header {
        true
    } else {
        reader.read_bit()?
    };
    let showable_frame = if seq.reduced_still_picture_header {
        false
    } else if show_frame {
        frame_type != FrameType::Key
    } else {
        reader.read_bit()?
    };
    let error_resilient_mode = if seq.reduced_still_picture_header {
        false
    } else if frame_type == FrameType::Switch || (frame_type == FrameType::Key && show_frame) {
        true
    } else {
        reader.read_bit()?
    };

    let disable_cdf_update = reader.read_bit()?;
    let allow_screen_content_tools =
        if seq.seq_force_screen_content_tools == SELECT_SCREEN_CONTENT_TOOLS as u8 {
            reader.read_bit()?
        } else {
            seq.seq_force_screen_content_tools != 0
        };
    let force_integer_mv = if allow_screen_content_tools {
        if seq.seq_force_integer_mv == SELECT_INTEGER_MV as u8 {
            reader.read_bit()?
        } else {
            seq.seq_force_integer_mv != 0
        }
    } else {
        false
    } || frame_is_intra;

    if seq.frame_id_numbers_present {
        let id_len = seq.additional_frame_id_length_minus_1.unwrap_or(0) as u32
            + seq.delta_frame_id_length_minus_2.unwrap_or(0) as u32
            + 3;
        reader.read_bits_u64(id_len as u8)?; // current_frame_id
    }

    let frame_size_override_flag = if frame_type == FrameType::Switch {
        true
    } else if seq.reduced_still_picture_header {
        false
    } else {
        reader.read_bit()?
    };

    let order_hint = if order_hint_bits > 0 {
        reader.read_bits(order_hint_bits as u8)?
    } else {
        0
    };

    let primary_ref_frame = if frame_is_intra || error_resilient_mode {
        PRIMARY_REF_NONE
    } else {
        reader.read_bits(3)?
    };

    // decoder_model_info / buffer_removal_time: this crate's SequenceHeader carries
    // decoder_model_info as Option, but not per-operating-point decoder_model_present flags
    // needed to size buffer_removal_time's per-op loop -- streams using this (rare outside
    // low-delay broadcast profiles) aren't supported here. Real-world encoders overwhelmingly
    // don't set decoder_model_info_present_flag, matching parse_frame_header_basic's own
    // "assume absent" precedent for similarly rare fields.
    if seq.decoder_model_info.is_some() {
        return Err(err(
            "decoder_model_info_present streams are not supported by parse_frame_header_full",
        ));
    }

    let mut ref_frame_idx = [0u32; REFS_PER_FRAME];
    let refresh_frame_flags: u32 =
        if frame_type == FrameType::Switch || (frame_type == FrameType::Key && show_frame) {
            0xFF
        } else {
            reader.read_bits(8)?
        };
    if (!frame_is_intra || refresh_frame_flags != 0xFF)
        && error_resilient_mode
        && seq.enable_order_hint
    {
        for _ in 0..NUM_REF_FRAMES {
            reader.read_bits(order_hint_bits as u8)?; // ref_order_hint[i]
        }
    }

    let mut allow_intrabc = false;
    let (width, height, upscaled_width, upscaled_height, use_superres, superres_denom);
    let mut allow_high_precision_mv = false;
    let mut use_ref_frame_mvs = false;
    let mut subpel_filter_switchable = false;
    let mut switchable_motion_mode = false;
    if frame_is_intra {
        let size = read_frame_size(&mut reader, seq, frame_size_override_flag)?;
        read_render_size(&mut reader)?;
        if allow_screen_content_tools && size.upscaled_width == size.width {
            allow_intrabc = reader.read_bit()?;
        }
        width = size.width;
        height = size.height;
        upscaled_width = size.upscaled_width;
        upscaled_height = size.upscaled_height;
        use_superres = size.use_superres;
        superres_denom = size.superres_denom;
    } else {
        let frame_refs_short_signaling = if seq.enable_order_hint {
            reader.read_bit()?
        } else {
            false
        };
        if frame_refs_short_signaling {
            return Err(err(
                "frame_refs_short_signaling (set_frame_refs) is not supported by parse_frame_header_full",
            ));
        }
        for slot in ref_frame_idx.iter_mut() {
            *slot = reader.read_bits(3)?;
            if seq.frame_id_numbers_present {
                let delta_len = seq.delta_frame_id_length_minus_2.unwrap_or(0) as u32 + 2;
                reader.read_bits(delta_len as u8)?; // delta_frame_id_minus_1
            }
        }
        if frame_size_override_flag && !error_resilient_mode {
            // frame_size_with_refs(): if found_ref via any ref frame's stored size, reuse it;
            // otherwise frame_size()+render_size(). This parser doesn't track ref frame
            // dimensions (only order hints), so it can't take the found_ref=1 shortcut path --
            // always falls through to a real frame_size()+render_size() read below. This is only
            // wrong (reads bits that shouldn't be there) if a *real* encoder actually emits
            // found_ref=1, which requires explicit ref-size reuse signaling support the encoder
            // must opt into; flagged here as a real accuracy gap, not silently assumed away.
            for _ in 0..REFS_PER_FRAME {
                if reader.read_bit()? {
                    return Err(err(
                        "frame_size_with_refs' found_ref=1 path is not supported by parse_frame_header_full",
                    ));
                }
            }
        }
        let size = read_frame_size(&mut reader, seq, frame_size_override_flag)?;
        read_render_size(&mut reader)?;
        width = size.width;
        height = size.height;
        upscaled_width = size.upscaled_width;
        upscaled_height = size.upscaled_height;
        use_superres = size.use_superres;
        superres_denom = size.superres_denom;

        allow_high_precision_mv = if force_integer_mv {
            false
        } else {
            reader.read_bit()?
        };
        subpel_filter_switchable = read_interpolation_filter(&mut reader)?;
        switchable_motion_mode = reader.read_bit()?; // is_motion_mode_switchable
        use_ref_frame_mvs = if error_resilient_mode || !seq.enable_ref_frame_mvs {
            false
        } else {
            reader.read_bit()?
        };
    }

    let disable_frame_end_update_cdf = if seq.reduced_still_picture_header || disable_cdf_update {
        true
    } else {
        reader.read_bit()?
    };

    read_tile_info(&mut reader, seq, width, height)?;

    let quant = crate::frame_header::parse_quantization_params(
        &mut reader,
        seq.color_config.num_planes,
        seq.color_config.separate_uv_delta_q,
    )?;
    let base_q_idx = quant.base_q_idx;
    let (y_dc_delta_q, uv_dc_delta_q) = (Some(quant.y_dc), Some(quant.u_dc));

    // `load_previous()`: a frame with a primary reference starts from that slot's segmentation
    // features (and none otherwise, `setup_past_independence`).
    let previous_segmentation = if primary_ref_frame == PRIMARY_REF_NONE {
        SegmentationFeatures::default()
    } else {
        ref_state.segmentation[ref_frame_idx[primary_ref_frame as usize] as usize % NUM_REF_FRAMES]
    };
    let segmentation =
        parse_segmentation_params(&mut reader, primary_ref_frame, &previous_segmentation)?;

    let delta_q_present = if base_q_idx > 0 {
        reader.read_bit()?
    } else {
        false
    };
    if delta_q_present {
        reader.read_bits(2)?; // delta_q_res
    }
    let (delta_lf_present, delta_lf_multi) =
        parse_delta_lf_params(&mut reader, delta_q_present, allow_intrabc)?;

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
    let coded_lossless = base_q_idx == 0 && quant.all_deltas_zero();
    let all_lossless = coded_lossless && width == upscaled_width;

    let loop_filter = parse_loop_filter_params(
        &mut reader,
        seq.color_config.num_planes,
        coded_lossless,
        allow_intrabc,
    )?;
    let cdef = parse_cdef_params(&mut reader, seq, coded_lossless, allow_intrabc)?;
    let loop_restoration = parse_lr_params(&mut reader, seq, all_lossless, allow_intrabc)?;

    let txfm_mode = read_tx_mode(&mut reader, coded_lossless)?;
    let reference_select = read_frame_reference_mode(&mut reader, frame_is_intra)?;
    let (skip_mode_present, skip_mode_refs) = read_skip_mode_params(
        &mut reader,
        frame_is_intra,
        reference_select,
        seq,
        ref_state,
        &ref_frame_idx,
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
    let gm_type = if !frame_is_intra {
        parse_global_motion_params(&mut reader, allow_high_precision_mv)?
    } else {
        [GM_TYPE_IDENTITY; 8]
    };

    let film_grain =
        parse_film_grain_params(&mut reader, seq, show_frame, showable_frame, frame_type)?;

    // Update reference-slot order-hint state for whichever frames get refreshed, so a later
    // parse_frame_header_full call (a later frame in the same sequential scan) sees this frame's
    // order hint when computing skip_mode_params. Must happen after every early-return path above
    // (show_existing_frame handled separately; none of the Err(...) short-circuits reach here).
    for i in 0..NUM_REF_FRAMES {
        if (refresh_frame_flags >> i) & 1 == 1 {
            ref_state.ref_order_hint[i] = order_hint;
            ref_state.segmentation[i] = SegmentationFeatures::from(&segmentation);
        }
    }

    let super_resolution = SuperResolutionInfo {
        enabled: use_superres,
        scale_denominator: superres_denom as u8,
    };

    Ok(FrameHeader {
        frame_type,
        show_frame,
        show_existing_frame: false,
        frame_to_show_map_idx: None,
        error_resilient_mode,
        base_q_idx: Some(base_q_idx),
        y_dc_delta_q,
        uv_dc_delta_q,
        delta_q_present,
        delta_q_residue: None,
        header_size_bytes: reader
            .byte_position()
            .saturating_add(usize::from(!reader.position().is_multiple_of(8))),
        header_size_bits: reader.position(),
        refresh_frame_flags: Some(refresh_frame_flags as u8),
        ref_frame_idx: if frame_is_intra {
            None
        } else {
            Some(std::array::from_fn(|i| ref_frame_idx[i] as u8))
        },
        order_hint,
        width,
        height,
        upscaled_width,
        upscaled_height,
        reference_select,
        allow_intrabc,
        allow_screen_content_tools,
        delta_lf_present,
        delta_lf_multi,
        skip_mode_present,
        skip_mode_refs,
        subpel_filter_switchable,
        switchable_motion_mode,
        allow_warped_motion,
        force_integer_mv,
        allow_high_precision_mv,
        gm_type,
        reduced_tx_set,
        txfm_mode,
        use_ref_frame_mvs,
        primary_ref_frame,
        disable_frame_end_update_cdf,
        segmentation,
        loop_filter,
        cdef_damping: cdef.clone(),
        cdef_y_primary_strength: cdef.y_primary_strength,
        cdef_y_secondary_strength: cdef.y_secondary_strength,
        cdef_uv_primary_strength: cdef.uv_primary_strength,
        cdef_uv_secondary_strength: cdef.uv_secondary_strength,
        loop_restoration,
        film_grain,
        super_resolution,
    })
}
