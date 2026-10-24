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
//! The global-motion parameters are coded relative to `PrevGmParams` (the primary reference
//! frame's saved `gm_params`), so `RefFrameState` keeps those per slot as well, next to the
//! segmentation features. This is NOT a general decoder simulation: it does not need
//! probability/CDF state or tile/pixel data.
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

mod assemble;
mod coding_tools;
mod film_grain;
mod frame_prelude;
mod frame_size;
mod global_motion;
mod loop_filters;
mod quantizer;
mod segmentation;
mod show_existing;
mod size_and_refs;
mod skip_mode;
#[cfg(test)]
mod tests;
mod tile_info;

#[cfg(test)]
use self::global_motion::{classify_gm_type, GM_TYPE_AFFINE};
pub use self::global_motion::{GmParams, GM_IDENTITY_PARAMS};
pub(crate) use self::global_motion::{GM_TYPE_IDENTITY, GM_TYPE_ROTZOOM, GM_TYPE_TRANSLATION};
#[cfg(test)]
use self::loop_filters::parse_lr_params;
pub use self::segmentation::SegmentationInfo;
#[cfg(test)]
use self::segmentation::{parse_segmentation_params, MAX_SEGMENTS, SEG_LVL_MAX};
pub(crate) use self::segmentation::{SEG_LVL_GLOBALMV, SEG_LVL_REF_FRAME, SEG_LVL_SKIP};
pub use self::skip_mode::relative_dist;
use self::{
    assemble::{assemble_frame_header, ref_scaled_flags, refresh_ref_slots, HeaderSections},
    coding_tools::{read_coding_tools, CodingToolsInput},
    frame_prelude::read_frame_prelude,
    global_motion::parse_global_motion_params,
    quantizer::{read_quantizer_section, QuantizerInput},
    segmentation::SegmentationFeatures,
    show_existing::parse_show_existing_frame,
    size_and_refs::{read_size_and_refs, SizeAndRefsInput},
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
#[derive(Debug, Clone)]
pub struct RefFrameState {
    ref_order_hint: [u32; NUM_REF_FRAMES],
    /// `gm_params` each slot was last refreshed with (spec `SavedGmParams`).
    gm_params: [GmParams; NUM_REF_FRAMES],
    /// `(RefUpscaledWidth, RefFrameHeight)` each slot was last refreshed with; `(0, 0)` for a
    /// slot no parsed frame has refreshed yet.
    size: [(u32, u32); NUM_REF_FRAMES],
    /// Segmentation feature data each slot was last refreshed with (spec
    /// `save_segmentation_params`).
    segmentation: [SegmentationFeatures; NUM_REF_FRAMES],
    /// Film grain parameters each slot was last refreshed with (spec `save_grain_params`).
    film_grain: [FilmGrainInfo; NUM_REF_FRAMES],
}

impl Default for RefFrameState {
    fn default() -> Self {
        Self {
            ref_order_hint: [0; NUM_REF_FRAMES],
            gm_params: [[GM_IDENTITY_PARAMS; 8]; NUM_REF_FRAMES],
            size: [(0, 0); NUM_REF_FRAMES],
            segmentation: Default::default(),
            film_grain: Default::default(),
        }
    }
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
        self.gm_params = [self.gm_params[from]; NUM_REF_FRAMES];
        self.size = [self.size[from]; NUM_REF_FRAMES];
        self.ref_order_hint = [hint; NUM_REF_FRAMES];
        self.segmentation = [segmentation; NUM_REF_FRAMES];
        self.film_grain = std::array::from_fn(|_| self.film_grain[from].clone());
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
    let show_existing_frame = if seq.reduced_still_picture_header {
        false
    } else {
        reader.read_bit()?
    };
    if show_existing_frame {
        return parse_show_existing_frame(&mut reader, ref_state);
    }

    let prelude = read_frame_prelude(&mut reader, seq)?;
    let refs = read_size_and_refs(
        &mut reader,
        seq,
        &SizeAndRefsInput {
            frame_is_intra: prelude.frame_is_intra,
            frame_size_override_flag: prelude.frame_size_override_flag,
            allow_screen_content_tools: prelude.allow_screen_content_tools,
            force_integer_mv: prelude.force_integer_mv,
            error_resilient_mode: prelude.error_resilient_mode,
        },
    )?;

    let disable_frame_end_update_cdf =
        if seq.reduced_still_picture_header || prelude.disable_cdf_update {
            true
        } else {
            reader.read_bit()?
        };

    let tiles = read_tile_info(&mut reader, seq, refs.size.width, refs.size.height)?;

    let quantizer = read_quantizer_section(
        &mut reader,
        seq,
        ref_state,
        &QuantizerInput {
            primary_ref_frame: prelude.primary_ref_frame,
            ref_frame_idx: &refs.ref_frame_idx,
            allow_intrabc: refs.allow_intrabc,
        },
    )?;
    let all_lossless = quantizer.coded_lossless && refs.size.width == refs.size.upscaled_width;

    let tools = read_coding_tools(
        &mut reader,
        seq,
        ref_state,
        &CodingToolsInput {
            frame_type: prelude.frame_type,
            frame_is_intra: prelude.frame_is_intra,
            show_frame: prelude.show_frame,
            showable_frame: prelude.showable_frame,
            error_resilient_mode: prelude.error_resilient_mode,
            allow_intrabc: refs.allow_intrabc,
            allow_high_precision_mv: refs.allow_high_precision_mv,
            order_hint: prelude.order_hint,
            ref_frame_idx: &refs.ref_frame_idx,
            coded_lossless: quantizer.coded_lossless,
            all_lossless,
            primary_ref_frame: prelude.primary_ref_frame,
        },
    )?;

    let ref_scaled = ref_scaled_flags(ref_state, &refs);

    refresh_ref_slots(
        ref_state,
        prelude.refresh_frame_flags,
        prelude.order_hint,
        &quantizer.segmentation,
        &tools.gm_params,
        (refs.size.upscaled_width, refs.size.height),
        &tools.film_grain,
    );

    Ok(assemble_frame_header(HeaderSections {
        prelude,
        refs,
        disable_frame_end_update_cdf,
        tiles,
        quantizer,
        tools,
        ref_scaled,
        header_size_bits: reader.position(),
        header_size_bytes: reader
            .byte_position()
            .saturating_add(usize::from(!reader.position().is_multiple_of(8))),
    }))
}
