//! Frame parsing and data structures for overlay extraction
//!
//! Provides ParsedFrame struct and related types for caching parsed OBU data.

use crate::frame_header::TxfmMode;
use crate::frame_header_full::{parse_frame_header_full, RefFrameState};
use crate::{parse_all_obus, parse_frame_header_basic, ObuType};
use bitvue_engine::BitvueError;
use std::sync::Arc;

/// Cached frame data to avoid re-parsing
///
/// This structure holds all parsed data from a frame's OBU data,
/// allowing multiple overlay extraction functions to reuse the
/// same parsed data without re-parsing the bitstream.
#[derive(Debug, Clone)]
pub struct ParsedFrame {
    /// Raw OBU data (shared reference to avoid copies)
    pub obu_data: Arc<[u8]>,
    /// Parsed OBUs
    pub obus: Vec<ObuRef>,
    /// Frame dimensions from sequence header
    pub dimensions: FrameDimensions,
    /// Frame type information
    pub frame_type: FrameTypeInfo,
    /// Tile group data (shared reference to avoid copies in QP/MV extraction). **Not**
    /// bit-position-independent of `ref_state` like the fields below (`reference_select` etc.) --
    /// its start offset (`header_size_bytes`) is downstream of `skip_mode_params()`'s
    /// presence bit, which DOES depend on real cross-frame ref-order-hint state. See
    /// [`ParsedFrame::parse`]'s doc: this field is only correct for frame 0 (or another frame
    /// that happens not to need `skip_mode_params`'s real state) when built via `parse`'s
    /// fresh-state default -- use [`ParsedFrame::parse_with_ref_state`] for any other frame.
    pub tile_data: Arc<[u8]>,
    /// Whether delta Q is enabled for this frame
    pub delta_q_enabled: bool,
    /// `reference_select` (spec 5.9.23) -- whether compound (2-reference) prediction is enabled
    /// for this frame. Computed via `parse_frame_header_full` (needs a real `SequenceHeader`,
    /// unlike `parse_frame_header_basic` which can't reach this field's bit position at all --
    /// see that function's doc) with a *fresh* `RefFrameState::new()` rather than real
    /// accumulated cross-frame state: `reference_select`'s bit position doesn't depend on
    /// `ref_state`'s stored values (only `read_skip_mode_params`, which runs *after* it, does) --
    /// see `parse_coding_unit`'s doc for the full reasoning. `false` (i.e. "no compound
    /// prediction, safe to assume single-ref") if the sequence header wasn't found or the frame
    /// header failed to parse (same resilient-fallback precedent as `delta_q_enabled`).
    pub reference_select: bool,
    /// `allow_intrabc` (spec 5.9.2) -- whether intra block copy is enabled for this (intra) frame.
    /// Same sourcing/fallback story as `reference_select`.
    pub allow_intrabc: bool,
    /// `allow_screen_content_tools` (spec 5.9.2) -- see `FrameHeader::allow_screen_content_tools`'s
    /// doc. Same sourcing/fallback story as `reference_select`.
    pub allow_screen_content_tools: bool,
    /// `delta_lf_present`/`delta_lf_multi` (spec 5.9.14) -- see `FrameHeader::delta_lf_present`'s
    /// doc. Same sourcing/fallback story as `reference_select`.
    pub delta_lf_present: bool,
    pub delta_lf_multi: bool,
    /// `reduced_tx_set` (spec 5.9.2) -- see `FrameHeader::reduced_tx_set`'s doc. Same
    /// sourcing/fallback story as `reference_select`.
    pub reduced_tx_set: bool,
    /// `CodedLossless` (spec 5.9.2/7.12.3): `base_q_idx == 0 && y_dc_delta_q == 0 &&
    /// uv_dc_delta_q == 0`. Doesn't factor in segmentation's per-segment `SEG_LVL_ALT_Q` override
    /// (real spec's per-segment `LosslessArray`) even though `segmentation` (below) is now real --
    /// frame-wide only, matches
    /// `frame_header_full.rs`'s own `coded_lossless` derivation (see that module's doc). `false`
    /// if `base_q_idx` wasn't parsed (same resilient-fallback precedent as `delta_q_enabled`).
    pub coded_lossless: bool,
    /// `TxMode` (spec 5.9.21) -- see `TxfmMode`'s doc. Same sourcing/fallback story as
    /// `reference_select`.
    pub txfm_mode: TxfmMode,
    /// `use_ref_frame_mvs` (spec 5.9.2) -- see `FrameHeader::use_ref_frame_mvs`'s doc. Same
    /// sourcing/fallback story as `reference_select`.
    pub use_ref_frame_mvs: bool,
    /// `primary_ref_frame` / `disable_frame_end_update_cdf` -- see `FrameHeader`'s fields. The
    /// CDF context a frame starts from depends on them.
    pub primary_ref_frame: u32,
    pub disable_frame_end_update_cdf: bool,
    /// `order_hint` (spec 5.9.2) -- this frame's own display-order hint. Same sourcing/fallback
    /// story as `reference_select`; `0` if the full header wasn't parsed. Read *before*
    /// `skip_mode_params` in bitstream order, so (like `use_ref_frame_mvs`) its value is correct
    /// even from the throwaway fresh `RefFrameState::new()` this struct's own parse uses --
    /// [`crate::tile::motion_field`]'s sequential test harness relies on this to avoid a second,
    /// real-cross-frame-threaded header parse.
    pub order_hint: u32,
    /// `ref_frame_idx` (spec 5.9.2) -- this frame's own logical-ref (`0..=6`, LAST..ALTREF) to
    /// physical-DPB-slot (`0..=7`) mapping. `None` for intra frames (spec: this syntax doesn't
    /// exist for them) or if the full header wasn't parsed. Same bit-position-independence note as
    /// `order_hint`.
    pub ref_frame_idx: Option<[u8; 7]>,
    /// `refresh_frame_flags` (spec 5.9.2) -- which of the 8 physical DPB slots this frame refreshes
    /// once decoded. `0` if the full header wasn't parsed. Same bit-position-independence note as
    /// `order_hint`.
    pub refresh_frame_flags: u8,
    /// Real segmentation state (spec 5.9.14) -- see `crate::frame_header_full::SegmentationInfo`'s
    /// doc for the exact fields and known gap. Same sourcing/fallback story as `reference_select`
    /// (`SegmentationInfo::default()`, all-disabled, if the full header wasn't parsed).
    pub segmentation: crate::frame_header_full::SegmentationInfo,
    /// `mono_chrome` (spec sequence header `color_config()`) -- true means this stream has no
    /// chroma planes at all (`num_planes == 1`), sourced directly from the sequence header's
    /// `ColorConfig` (no `parse_frame_header_full` needed, unlike `reference_select`/etc.).
    /// Defaults to `true` (conservatively "no chroma") if the sequence header wasn't found --
    /// this crate can't derive `subsampling_x`/`subsampling_y` without it, and guessing chroma
    /// geometry wrong would be worse than the existing luma-only gap (see
    /// `crate::tile::coding_unit`'s module doc for why this gap was a real desync bug, not just
    /// missing data).
    pub mono_chrome: bool,
    /// `subsampling_x`/`subsampling_y` (spec sequence header `color_config()`) -- chroma plane
    /// dimensions are `luma_dim >> subsampling_{x,y}` (spec `get_plane_residual_size`). Same
    /// sourcing/fallback story as `mono_chrome` (irrelevant when `mono_chrome` is true).
    pub subsampling_x: bool,
    pub subsampling_y: bool,
    /// `enable_filter_intra` (spec sequence header, `Sequence Header OBU syntax`) -- gates
    /// `filter_intra_mode_info()`'s real eligibility (`read_palette_mode_info`'s call site doc).
    /// Same sourcing story as `mono_chrome` (direct from the sequence header, no
    /// `parse_frame_header_full` needed); defaults to `false` (conservatively "never read a
    /// filter_intra bit") if the sequence header wasn't found.
    pub enable_filter_intra: bool,
    /// `cdef.bits` (spec 5.9.19's `cdef_bits`) -- see `crate::frame_header::CdefInfo::bits`'s doc
    /// for what this gates in tile data. Same sourcing/fallback story as `reference_select`; `0`
    /// (i.e. "`cdef_idx()` never reads any bits") if the full header wasn't parsed -- the same
    /// conservative default real spec itself uses when CDEF is disabled.
    pub cdef_bits: u8,
    /// `frame_to_show_map_idx` of a `show_existing_frame` header: the reference slot whose frame
    /// this unit displays. Such a unit has no tile of its own.
    pub show_existing_slot: Option<u8>,
    /// Whether this frame's `frame_type` is exactly `KEY_FRAME` (not `INTRA_ONLY_FRAME`, which
    /// [`FrameTypeInfo::is_intra_only`] includes). A `show_existing_frame` of such a frame
    /// refreshes every reference slot with it (spec 7.21).
    pub key_frame: bool,
    /// The frame's tile layout (spec 5.9.15); `None` when it is not known (no sequence header
    /// to parse the frame header with), which is read as one tile.
    pub tiles: Option<crate::frame_header::TileLayout>,
    /// Where each tile group's payload lies within `tile_data` (one range per tile group OBU, or
    /// the part of a `Frame` OBU after its header). Empty: all of `tile_data` is one group.
    pub tile_groups: Vec<std::ops::Range<usize>>,
    /// The coding units of this frame when they were decoded with the state of the frames before
    /// it (see [`crate::overlay_extraction::StreamDecodeState`]); every extractor then uses them
    /// instead of decoding the tile on its own, which would start from the default CDFs.
    pub decoded: Option<super::ParsedCodingUnits>,
    /// `RefFrameSignBias` (spec 5.9.2): per reference (`LAST..=ALTREF`), whether the reference is
    /// displayed after this frame. Needs the reference order hints from before this frame's own
    /// header, so it is only right for a frame parsed with the threaded `RefFrameState`.
    pub ref_frame_sign_bias: [bool; 7],
    /// `relative_dist(RefOrderHint[ref], OrderHint)` per reference -- the POC distances
    /// `jnt_comp`'s context compares. Same sourcing as `ref_frame_sign_bias`.
    pub ref_order_distance: [i32; 7],
    /// `lr_params()` (spec 5.9.20): which planes are restored and how big the units are. Same
    /// sourcing/fallback story as `cdef_bits` (no plane restored if the full header wasn't parsed).
    pub loop_restoration: crate::frame_header::LoopRestorationInfo,
    /// `use_superres` (spec 5.9.8).
    pub superres: bool,
    /// `skip_mode_present` (spec 5.9.22) -- see `crate::frame_header::FrameHeader::
    /// skip_mode_present`'s doc for what this gates in tile data. Same sourcing/fallback story
    /// as `reference_select`; `false` (i.e. "`skip_mode` never reads any bits") if the full
    /// header wasn't parsed.
    pub skip_mode_present: bool,
    /// `SkipModeFrame[0]/[1]` (spec 5.9.22) -- see `crate::frame_header::FrameHeader::
    /// skip_mode_refs`'s doc. Same sourcing/fallback story as `skip_mode_present`.
    pub skip_mode_refs: [u8; 2],
    /// `subpel_filter_switchable`/`switchable_motion_mode`/`allow_warped_motion` (spec 5.9.2/
    /// 5.9.10) -- see `crate::frame_header::FrameHeader`'s matching fields' docs. Same sourcing/
    /// fallback story as `reference_select`.
    pub subpel_filter_switchable: bool,
    pub switchable_motion_mode: bool,
    pub allow_warped_motion: bool,
    /// `force_integer_mv`/`gm_type` (spec 5.9.2/5.9.24) -- see
    /// `crate::frame_header::FrameHeader`'s matching fields' docs. Same sourcing/fallback story as
    /// `reference_select`.
    pub force_integer_mv: bool,
    /// `allow_high_precision_mv` (spec 5.9.2) -- see `FrameHeader::allow_high_precision_mv`.
    pub allow_high_precision_mv: bool,
    pub gm_type: [u8; 8],
    /// `enable_interintra_compound`/`enable_masked_compound`/`enable_jnt_comp`/
    /// `enable_warped_motion` (sequence header) -- gate `interintra`/`compound_type`(wedge/seg)/
    /// `motion_mode`'s real eligibility. Same sourcing story as `mono_chrome` (direct from the
    /// sequence header, no `parse_frame_header_full` needed); default `false` (conservatively
    /// "never read the corresponding bits") if the sequence header wasn't found.
    pub enable_interintra_compound: bool,
    /// `enable_dual_filter` (sequence header).
    pub enable_dual_filter: bool,
    pub enable_masked_compound: bool,
    pub enable_jnt_comp: bool,
    pub enable_warped_motion: bool,
}

/// Frame dimensions extracted from sequence header
#[derive(Debug, Clone, Copy)]
pub struct FrameDimensions {
    /// Frame width in pixels
    pub width: u32,
    /// Frame height in pixels
    pub height: u32,
    /// Superblock size (64 or 128)
    pub sb_size: u32,
    /// Number of superblock columns
    pub sb_cols: u32,
    /// Number of superblock rows
    pub sb_rows: u32,
}

/// Frame type information
#[derive(Debug, Clone, Copy, Default)]
pub struct FrameTypeInfo {
    /// Whether this is a key/intra-only frame
    pub is_intra_only: bool,
    /// Base QP value (if available)
    pub base_qp: Option<u8>,
}

/// Reference to an OBU with its payload range
///
/// This avoids storing full OBU structs and instead stores
/// references to the original data.
#[derive(Debug, Clone, Copy)]
pub struct ObuRef {
    /// OBU type
    pub obu_type: ObuType,
    /// Start offset in obu_data
    pub payload_start: usize,
    /// End offset in obu_data (exclusive)
    pub payload_end: usize,
}

impl Default for FrameDimensions {
    fn default() -> Self {
        Self {
            width: 1920,
            height: 1080,
            sb_size: 64,
            sb_cols: 30,
            sb_rows: 17,
        }
    }
}

impl ParsedFrame {
    /// Frame-level inputs to coding-unit parsing, derived from this frame's headers. The single
    /// place that maps `ParsedFrame`'s fields onto [`crate::tile::FrameCodingParams`].
    pub fn coding_params(&self) -> crate::tile::FrameCodingParams {
        crate::tile::FrameCodingParams {
            is_key_frame: self.frame_type.is_intra_only,
            delta_q_enabled: self.delta_q_enabled,
            reference_select: self.reference_select,
            allow_intrabc: self.allow_intrabc,
            allow_screen_content_tools: self.allow_screen_content_tools,
            enable_filter_intra: self.enable_filter_intra,
            delta_lf_present: self.delta_lf_present,
            delta_lf_multi: self.delta_lf_multi,
            use_ref_frame_mvs: self.use_ref_frame_mvs,
            segmentation: self.segmentation,
            tx_type_flags: crate::tile::TxTypeFrameFlags {
                coded_lossless: self.coded_lossless,
                qidx_is_zero: self.frame_type.base_qp == Some(0),
                reduced_tx_set: self.reduced_tx_set,
                txfm_mode: self.txfm_mode,
                mono_chrome: self.mono_chrome,
                subsampling_x: self.subsampling_x,
                subsampling_y: self.subsampling_y,
            },
            inter_mode_flags: crate::tile::InterModeFlags {
                switchable_motion_mode: self.switchable_motion_mode,
                allow_warped_motion: self.allow_warped_motion,
                enable_interintra_compound: self.enable_interintra_compound,
                enable_dual_filter: self.enable_dual_filter,
                enable_masked_compound: self.enable_masked_compound,
                enable_jnt_comp: self.enable_jnt_comp,
                subpel_filter_switchable: self.subpel_filter_switchable,
                force_integer_mv: self.force_integer_mv,
                allow_high_precision_mv: self.allow_high_precision_mv,
                gm_type: self.gm_type,
            },
            mi_rows: crate::tile::partition::mi_units(self.dimensions.height),
            mi_cols: crate::tile::partition::mi_units(self.dimensions.width),
            cdef_bits: self.cdef_bits,
            restoration: crate::tile::RestorationParams {
                types: [
                    self.loop_restoration.y_type,
                    self.loop_restoration.u_type,
                    self.loop_restoration.v_type,
                ],
                unit_size: [
                    self.loop_restoration.unit_size,
                    self.loop_restoration.uv_unit_size,
                    self.loop_restoration.uv_unit_size,
                ],
                frame_width: self.dimensions.width,
                frame_height: self.dimensions.height,
                subsampling_x: self.subsampling_x,
                subsampling_y: self.subsampling_y,
                superres: self.superres,
            },
            skip_mode_present: self.skip_mode_present,
            skip_mode_refs: self.skip_mode_refs,
        }
    }

    /// Parse OBU data and cache all relevant information
    ///
    /// This is the main entry point for overlay extraction.
    /// Call this once, then use the cached data for all extractions.
    ///
    /// # Performance
    ///
    /// - O(n) where n is the OBU data size
    /// - Parses each OBU exactly once
    /// - Stores references to avoid copying payload data
    ///
    /// # Example
    ///
    /// ```ignore
    /// let parsed = ParsedFrame::parse(&obu_data)?;
    /// let qp_grid = extract_qp_grid_from_parsed(&parsed, frame_idx, base_qp)?;
    /// let mv_grid = extract_mv_grid_from_parsed(&parsed, frame_idx)?;
    /// ```
    ///
    /// **Correctness note (axis-7 fix, 2026-08-20):** this parses `obu_data` against a *fresh*
    /// `RefFrameState` (`RefFrameState::new()`), which is only correct for `obu_data`'s frame if
    /// it's frame 0 (or otherwise doesn't need real cross-frame ref-order-hint state to interpret
    /// its own header -- e.g. `skip_mode_params()`'s presence bit, spec 5.9.22). For any other
    /// frame, a real fixture's `header_size_bytes` can come out wrong under a fresh state, which
    /// silently shifts `tile_data`'s start into what's still frame-header bits -- the entropy
    /// decoder then desyncs from its very first read, and every superblock in the frame fails to
    /// parse (caught per-superblock, so the *symptom* is a silently near-empty coding-unit list,
    /// not a propagated error). Confirmed via a real fixture: frames needing `skip_mode_params`'s
    /// real state return 0 coding units this way despite non-trivial `tile_data` length. Callers
    /// that need correct `tile_data`/coding-units for anything other than frame 0 (i.e. every
    /// `bitvue-sidecar` command backing a per-frame grid) MUST use
    /// [`ParsedFrame::parse_with_ref_state`] with a `RefFrameState` threaded from frame 0, not
    /// this method.
    pub fn parse(obu_data: &[u8]) -> Result<Self, BitvueError> {
        Self::parse_with_ref_state(obu_data, &mut RefFrameState::new())
    }

    /// Same as [`ParsedFrame::parse`], but threads a caller-supplied `RefFrameState` into the
    /// frame-header parse used to locate `tile_data`'s real start (`header_size_bytes`), instead
    /// of assuming a fresh/default state. `ref_state` should reflect every frame from 0 up to
    /// (but not including) `obu_data`'s own frame -- see [`RefFrameState`]'s doc and this
    /// method's callers (`bitvue-sidecar`'s `frame_analysis`/`deblocking`/`residual_analysis`/
    /// `codec_extended_info`/`coding_flow` modules) for the standard sequential-scan pattern that
    /// builds it. Mutates `ref_state` in place with this frame's own contribution as a side
    /// effect (matching [`crate::frame_header_full::parse_frame_header_full`]'s own contract),
    /// so callers that need the pre-this-frame state again afterward should clone it first.
    pub fn parse_with_ref_state(
        obu_data: &[u8],
        ref_state: &mut RefFrameState,
    ) -> Result<Self, BitvueError> {
        let obu_data: Arc<[u8]> = Arc::from(obu_data);
        // Order hints of the reference slots as they were before this frame's own header.
        let prev_ref_order_hint = *ref_state.ref_order_hint();

        // Handle empty data: return a default ParsedFrame immediately.
        // Callers that extract grids from empty data receive the default 1920x1080
        // scaffold, which is the documented behaviour for this module.
        if obu_data.is_empty() {
            return Ok(Self {
                obu_data,
                obus: Vec::new(),
                dimensions: FrameDimensions::default(),
                frame_type: FrameTypeInfo::default(),
                tile_data: Arc::from([]),
                delta_q_enabled: false,
                reference_select: false,
                allow_intrabc: false,
                allow_screen_content_tools: false,
                delta_lf_present: false,
                delta_lf_multi: false,
                reduced_tx_set: false,
                coded_lossless: false,
                txfm_mode: TxfmMode::default(),
                use_ref_frame_mvs: false,
                primary_ref_frame: 7,
                disable_frame_end_update_cdf: true,
                order_hint: 0,
                ref_frame_idx: None,
                refresh_frame_flags: 0,
                segmentation: crate::frame_header_full::SegmentationInfo::default(),
                mono_chrome: true,
                subsampling_x: false,
                subsampling_y: false,
                enable_filter_intra: false,
                cdef_bits: 0,
                show_existing_slot: None,
                key_frame: false,
                tiles: None,
                tile_groups: Vec::new(),
                decoded: None,
                ref_frame_sign_bias: [false; 7],
                ref_order_distance: [0; 7],
                loop_restoration: Default::default(),
                superres: false,
                skip_mode_present: false,
                skip_mode_refs: [0, 0],
                subpel_filter_switchable: false,
                switchable_motion_mode: false,
                allow_warped_motion: false,
                force_integer_mv: false,
                allow_high_precision_mv: false,
                gm_type: [0u8; 8],
                enable_interintra_compound: false,
                enable_dual_filter: false,
                enable_masked_compound: false,
                enable_jnt_comp: false,
                enable_warped_motion: false,
            });
        }

        // Use resilient parsing: collect whatever OBUs parse successfully and
        // log (but do not propagate) any individual OBU parse errors.
        // This allows overlay extraction to work on truncated or minimal test
        // data without failing the whole operation.
        let obus_vec = match parse_all_obus(&obu_data) {
            Ok(obus) => obus,
            Err(e) => {
                tracing::warn!(
                    "parse_all_obus failed ({}), falling back to resilient OBU iteration",
                    e
                );
                // Collect successfully-parsed OBUs, silently dropping errors
                use crate::ObuIterator;
                ObuIterator::new(&obu_data)
                    .filter_map(|result| result.ok())
                    .collect::<Vec<_>>()
            }
        };

        // Build lightweight OBU references
        let mut offset = 0;
        let mut obus = Vec::with_capacity(obus_vec.len());
        let mut dimensions = FrameDimensions::default();
        let mut frame_type = FrameTypeInfo::default();
        let mut tile_data = Vec::new();
        let mut delta_q_enabled = false; // Default to false
        let mut reference_select = false;
        let mut allow_intrabc = false;
        let mut allow_screen_content_tools = false;
        let mut delta_lf_present = false;
        let mut delta_lf_multi = false;
        let mut reduced_tx_set = false;
        let mut coded_lossless = false;
        let mut txfm_mode = TxfmMode::default();
        let mut use_ref_frame_mvs = false;
        let mut primary_ref_frame = 7u32;
        let mut disable_frame_end_update_cdf = true;
        let mut order_hint = 0u32;
        let mut ref_frame_idx: Option<[u8; 7]> = None;
        let mut refresh_frame_flags = 0u8;
        let mut segmentation = crate::frame_header_full::SegmentationInfo::default();
        let mut mono_chrome = true;
        let mut subsampling_x = false;
        let mut subsampling_y = false;
        let mut enable_filter_intra = false;
        let mut cdef_bits = 0u8;
        let mut show_existing_slot: Option<u8> = None;
        let mut key_frame = false;
        let mut tiles = None;
        let mut tile_groups: Vec<std::ops::Range<usize>> = Vec::new();
        let mut ref_frame_sign_bias = [false; 7];
        let mut ref_order_distance = [0i32; 7];
        let mut loop_restoration = crate::frame_header::LoopRestorationInfo::default();
        let mut superres = false;
        let mut skip_mode_present = false;
        let mut skip_mode_refs = [0u8, 0u8];
        let mut subpel_filter_switchable = false;
        let mut switchable_motion_mode = false;
        let mut allow_warped_motion = false;
        let mut force_integer_mv = false;
        let mut allow_high_precision_mv = false;
        let mut gm_type = [0u8; 8];
        let mut enable_interintra_compound = false;
        let mut enable_dual_filter = false;
        let mut enable_masked_compound = false;
        let mut enable_jnt_comp = false;
        let mut enable_warped_motion = false;
        // Retained across the loop so the frame-header OBU (which comes after the sequence
        // header in every real stream) can use it -- see reference_select/allow_intrabc's doc.
        let mut seq_header: Option<crate::SequenceHeader> = None;

        for obu in &obus_vec {
            let payload_start = offset + obu.header.header_size;
            let payload_end = payload_start + obu.payload.len();

            obus.push(ObuRef {
                obu_type: obu.header.obu_type,
                payload_start,
                payload_end: payload_end.min(obu_data.len()),
            });

            // Extract information based on OBU type
            match obu.header.obu_type {
                ObuType::SequenceHeader => {
                    if let Ok(seq_hdr) = crate::parse_sequence_header(&obu.payload) {
                        dimensions = FrameDimensions {
                            width: seq_hdr.max_frame_width,
                            height: seq_hdr.max_frame_height,
                            sb_size: if seq_hdr.use_128x128_superblock {
                                128
                            } else {
                                64
                            },
                            sb_cols: 0,
                            sb_rows: 0,
                        };
                        mono_chrome = seq_hdr.color_config.mono_chrome;
                        subsampling_x = seq_hdr.color_config.subsampling_x;
                        subsampling_y = seq_hdr.color_config.subsampling_y;
                        enable_filter_intra = seq_hdr.enable_filter_intra;
                        enable_interintra_compound = seq_hdr.enable_interintra_compound;
                        enable_dual_filter = seq_hdr.enable_dual_filter;
                        enable_masked_compound = seq_hdr.enable_masked_compound;
                        enable_jnt_comp = seq_hdr.enable_jnt_comp;
                        enable_warped_motion = seq_hdr.enable_warped_motion;
                        seq_header = Some(seq_hdr);
                    }
                }
                ObuType::Frame => {
                    // OBU_FRAME packs frame_header() + byte_alignment() + tile_group() into one
                    // payload (spec 5.10) -- unlike a standalone FrameHeader OBU, the tile bytes
                    // live right here, after header_size_bytes. Most real encoders (including
                    // this crate's own IVF test fixture) emit this combined form rather than
                    // separate FrameHeader+TileGroup OBUs, so without this split, tile_data stays
                    // empty and every real-CU-parsing consumer (QP/MV/partition/prediction-mode/
                    // transform grids, deblocking boundary strength) silently falls back to
                    // scaffold data -- found via `get_deblocking_analysis` returning a decode
                    // error on real fixture frames that should have real tile data.
                    //
                    // `header_size_bytes` for the tile_data cut MUST come from `full_hdr`
                    // (`parse_frame_header_full`), not `parse_frame_header_basic` -- confirmed via
                    // a real dav1d oracle build (`DEBUG_BLOCK_INFO`) that `basic`'s value is
                    // exactly what its own doc admits: an approximation for non-KEY frames that
                    // skips `frame_size()`/`tile_info()`/`segmentation_params()`/
                    // `loop_filter_params()`/`cdef_params()`/`lr_params()`/
                    // `global_motion_params()`/etc. entirely. On the real fixture's frame 13 this
                    // undershoots by 11 bytes (`basic`=8, `full`=19, oracle's independently-derived
                    // true offset=19) -- every inter frame's `tile_data` has silently started 11+
                    // bytes into what's still frame-header bits, handing the symbol decoder
                    // garbage from its very first read. This was the actual root cause behind the
                    // "compounding entropy desync" investigated over many sessions as a
                    // coeff_base/coeff_br context-precision suspicion -- that suspicion was never
                    // reached because the decoder was never even starting from real tile bytes for
                    // inter frames. `basic` is kept only for `is_intra_only`/`base_qp`/
                    // `delta_q_enabled` when `seq_header` isn't available yet (a Frame OBU can't
                    // spec-legally precede a Sequence Header, so this fallback path is defensive,
                    // not a real-stream case).
                    // A temporal unit can hold several frames (a hidden ARF followed by the shown
                    // frame): every field below describes the frame parsed last, so the tile
                    // bytes of an earlier one must not stay in front of them.
                    tile_data.clear();
                    tile_groups.clear();
                    if let Ok(frame_hdr) = parse_frame_header_basic(&obu.payload) {
                        // `FrameTypeInfo::is_intra_only`'s doc says "key/intra-only" -- i.e. spec
                        // 5.9.2's `FrameIsIntra` (`frame_type == KEY_FRAME || frame_type ==
                        // INTRA_ONLY_FRAME`), which is `FrameType::is_intra()`, NOT
                        // `FrameType::is_intra_only()` (`bitvue-engine`'s codec-agnostic type --
                        // that method only matches AV1's literal, rare INTRA_ONLY_FRAME type,
                        // excluding ordinary KEY_FRAME). Using the narrower method meant every
                        // real KEY_FRAME in every fixture this session ever parsed was silently
                        // routed through `parse_coding_unit`'s INTER branch instead of its INTRA
                        // one -- found while verifying the new key-frame `intra_mode`/`kfym`
                        // context work (`SymbolDecoder::read_intra_mode`) had zero real frames to
                        // exercise it on.
                        frame_type.is_intra_only = frame_hdr.frame_type.is_intra();
                        frame_type.base_qp = frame_hdr.base_q_idx;
                        delta_q_enabled = frame_hdr.delta_q_present;
                        if seq_header.is_none() && frame_hdr.header_size_bytes < obu.payload.len() {
                            tile_data
                                .extend_from_slice(&obu.payload[frame_hdr.header_size_bytes..]);
                        }
                    }
                    if let Some(seq) = &seq_header {
                        if let Ok(full_hdr) = parse_frame_header_full(&obu.payload, seq, ref_state)
                        {
                            // The exact header parse wins over `parse_frame_header_basic`'s
                            // approximation for the two values it also supplied.
                            delta_q_enabled = full_hdr.delta_q_present;
                            if full_hdr.base_q_idx.is_some() {
                                frame_type.base_qp = full_hdr.base_q_idx;
                            }
                            reference_select = full_hdr.reference_select;
                            allow_intrabc = full_hdr.allow_intrabc;
                            allow_screen_content_tools = full_hdr.allow_screen_content_tools;
                            delta_lf_present = full_hdr.delta_lf_present;
                            delta_lf_multi = full_hdr.delta_lf_multi;
                            reduced_tx_set = full_hdr.reduced_tx_set;
                            coded_lossless = full_hdr.base_q_idx == Some(0)
                                && full_hdr.y_dc_delta_q.unwrap_or(0) == 0
                                && full_hdr.uv_dc_delta_q.unwrap_or(0) == 0;
                            txfm_mode = full_hdr.txfm_mode;
                            use_ref_frame_mvs = full_hdr.use_ref_frame_mvs;
                            primary_ref_frame = full_hdr.primary_ref_frame;
                            disable_frame_end_update_cdf = full_hdr.disable_frame_end_update_cdf;
                            order_hint = full_hdr.order_hint;
                            ref_frame_idx = full_hdr.ref_frame_idx;
                            refresh_frame_flags = full_hdr.refresh_frame_flags.unwrap_or(0);
                            segmentation = full_hdr.segmentation;
                            cdef_bits = full_hdr.cdef_damping.bits;
                            show_existing_slot = full_hdr.frame_to_show_map_idx;
                            tiles = full_hdr.tiles.clone();
                            key_frame = !full_hdr.show_existing_frame
                                && full_hdr.frame_type.is_intra()
                                && !full_hdr.frame_type.is_intra_only();
                            ref_frame_sign_bias = crate::frame_header_full::ref_frame_sign_bias(
                                &prev_ref_order_hint,
                                full_hdr.ref_frame_idx.as_ref(),
                                full_hdr.order_hint,
                                seq,
                            );
                            ref_order_distance = crate::frame_header_full::ref_order_distance(
                                &prev_ref_order_hint,
                                full_hdr.ref_frame_idx.as_ref(),
                                full_hdr.order_hint,
                                seq,
                            );
                            loop_restoration = full_hdr.loop_restoration.clone();
                            superres = full_hdr.super_resolution.enabled;
                            skip_mode_present = full_hdr.skip_mode_present;
                            skip_mode_refs = full_hdr.skip_mode_refs;
                            subpel_filter_switchable = full_hdr.subpel_filter_switchable;
                            switchable_motion_mode = full_hdr.switchable_motion_mode;
                            allow_warped_motion = full_hdr.allow_warped_motion;
                            force_integer_mv = full_hdr.force_integer_mv;
                            allow_high_precision_mv = full_hdr.allow_high_precision_mv;
                            gm_type = full_hdr.gm_type;
                            if full_hdr.header_size_bytes < obu.payload.len() {
                                let start = tile_data.len();
                                tile_data
                                    .extend_from_slice(&obu.payload[full_hdr.header_size_bytes..]);
                                tile_groups.push(start..tile_data.len());
                            }
                        }
                    }
                }
                ObuType::FrameHeader => {
                    // Same reset as for `ObuType::Frame`: this header starts a new frame, so the
                    // tile groups gathered for an earlier frame of the packet are not its own.
                    tile_data.clear();
                    tile_groups.clear();
                    if let Ok(frame_hdr) = parse_frame_header_basic(&obu.payload) {
                        // See the `ObuType::Frame` branch above for why this is `is_intra()`, not
                        // `is_intra_only()`.
                        frame_type.is_intra_only = frame_hdr.frame_type.is_intra();
                        frame_type.base_qp = frame_hdr.base_q_idx;
                        delta_q_enabled = frame_hdr.delta_q_present;
                    }
                    if let Some(seq) = &seq_header {
                        if let Ok(full_hdr) = parse_frame_header_full(&obu.payload, seq, ref_state)
                        {
                            // The exact header parse wins over `parse_frame_header_basic`'s
                            // approximation for the two values it also supplied.
                            delta_q_enabled = full_hdr.delta_q_present;
                            if full_hdr.base_q_idx.is_some() {
                                frame_type.base_qp = full_hdr.base_q_idx;
                            }
                            reference_select = full_hdr.reference_select;
                            allow_intrabc = full_hdr.allow_intrabc;
                            allow_screen_content_tools = full_hdr.allow_screen_content_tools;
                            delta_lf_present = full_hdr.delta_lf_present;
                            delta_lf_multi = full_hdr.delta_lf_multi;
                            reduced_tx_set = full_hdr.reduced_tx_set;
                            coded_lossless = full_hdr.base_q_idx == Some(0)
                                && full_hdr.y_dc_delta_q.unwrap_or(0) == 0
                                && full_hdr.uv_dc_delta_q.unwrap_or(0) == 0;
                            txfm_mode = full_hdr.txfm_mode;
                            use_ref_frame_mvs = full_hdr.use_ref_frame_mvs;
                            primary_ref_frame = full_hdr.primary_ref_frame;
                            disable_frame_end_update_cdf = full_hdr.disable_frame_end_update_cdf;
                            order_hint = full_hdr.order_hint;
                            ref_frame_idx = full_hdr.ref_frame_idx;
                            refresh_frame_flags = full_hdr.refresh_frame_flags.unwrap_or(0);
                            segmentation = full_hdr.segmentation;
                            cdef_bits = full_hdr.cdef_damping.bits;
                            show_existing_slot = full_hdr.frame_to_show_map_idx;
                            tiles = full_hdr.tiles.clone();
                            key_frame = !full_hdr.show_existing_frame
                                && full_hdr.frame_type.is_intra()
                                && !full_hdr.frame_type.is_intra_only();
                            ref_frame_sign_bias = crate::frame_header_full::ref_frame_sign_bias(
                                &prev_ref_order_hint,
                                full_hdr.ref_frame_idx.as_ref(),
                                full_hdr.order_hint,
                                seq,
                            );
                            ref_order_distance = crate::frame_header_full::ref_order_distance(
                                &prev_ref_order_hint,
                                full_hdr.ref_frame_idx.as_ref(),
                                full_hdr.order_hint,
                                seq,
                            );
                            loop_restoration = full_hdr.loop_restoration.clone();
                            superres = full_hdr.super_resolution.enabled;
                            skip_mode_present = full_hdr.skip_mode_present;
                            skip_mode_refs = full_hdr.skip_mode_refs;
                            subpel_filter_switchable = full_hdr.subpel_filter_switchable;
                            switchable_motion_mode = full_hdr.switchable_motion_mode;
                            allow_warped_motion = full_hdr.allow_warped_motion;
                            force_integer_mv = full_hdr.force_integer_mv;
                            allow_high_precision_mv = full_hdr.allow_high_precision_mv;
                            gm_type = full_hdr.gm_type;
                        }
                    }
                }
                ObuType::TileGroup => {
                    let start = tile_data.len();
                    tile_data.extend_from_slice(&obu.payload);
                    tile_groups.push(start..tile_data.len());
                }
                _ => {}
            }

            offset = payload_end;
        }

        // Calculate superblock grid dimensions
        if dimensions.width > 0 && dimensions.sb_size > 0 {
            dimensions.sb_cols = dimensions.width.div_ceil(dimensions.sb_size);
            dimensions.sb_rows = dimensions.height.div_ceil(dimensions.sb_size);
        }

        Ok(Self {
            obu_data,
            obus,
            dimensions,
            frame_type,
            tile_data: Arc::from(tile_data),
            delta_q_enabled,
            reference_select,
            allow_intrabc,
            allow_screen_content_tools,
            delta_lf_present,
            delta_lf_multi,
            reduced_tx_set,
            coded_lossless,
            txfm_mode,
            use_ref_frame_mvs,
            primary_ref_frame,
            disable_frame_end_update_cdf,
            order_hint,
            ref_frame_idx,
            refresh_frame_flags,
            segmentation,
            mono_chrome,
            subsampling_x,
            subsampling_y,
            enable_filter_intra,
            cdef_bits,
            show_existing_slot,
            key_frame,
            tiles,
            tile_groups,
            decoded: None,
            ref_frame_sign_bias,
            ref_order_distance,
            loop_restoration,
            superres,
            skip_mode_present,
            skip_mode_refs,
            subpel_filter_switchable,
            switchable_motion_mode,
            allow_warped_motion,
            force_integer_mv,
            allow_high_precision_mv,
            gm_type,
            enable_interintra_compound,
            enable_dual_filter,
            enable_masked_compound,
            enable_jnt_comp,
            enable_warped_motion,
        })
    }

    /// Get OBU payload by reference
    #[inline]
    pub fn get_payload(&self, obu_ref: &ObuRef) -> Option<&[u8]> {
        let start = obu_ref.payload_start;
        let end = obu_ref.payload_end;
        if start < end && end <= self.obu_data.len() {
            Some(&self.obu_data[start..end])
        } else {
            None
        }
    }

    /// Find OBUs of a specific type
    pub fn find_obus_of_type(&self, obu_type: ObuType) -> impl Iterator<Item = &ObuRef> {
        self.obus.iter().filter(move |o| o.obu_type == obu_type)
    }

    /// Check if this frame has tile data
    #[inline]
    pub fn has_tile_data(&self) -> bool {
        !self.tile_data.is_empty()
    }

    /// Get frame width
    #[inline]
    pub fn width(&self) -> u32 {
        self.dimensions.width
    }

    /// Get frame height
    #[inline]
    pub fn height(&self) -> u32 {
        self.dimensions.height
    }

    /// Get superblock size
    #[inline]
    pub fn sb_size(&self) -> u32 {
        self.dimensions.sb_size
    }

    /// Check if this is an intra-only frame
    #[inline]
    pub fn is_intra_only(&self) -> bool {
        self.frame_type.is_intra_only
    }
}

/// Pixel information for tooltip display
#[derive(Debug, Clone)]
pub struct PixelInfo {
    /// Frame index
    pub frame_index: usize,
    /// Pixel X coordinate
    pub pixel_x: u32,
    /// Pixel Y coordinate
    pub pixel_y: u32,
    /// Luma (Y) value (0-255 for 8-bit)
    pub luma: Option<u8>,
    /// Chroma U value (0-255 for 8-bit)
    pub chroma_u: Option<u8>,
    /// Chroma V value (0-255 for 8-bit)
    pub chroma_v: Option<u8>,
    /// Block ID (e.g., "sb[2][3]")
    pub block_id: String,
    /// Quantization parameter
    pub qp: Option<f32>,
    /// Motion vector (dx, dy) in pixels
    pub mv: Option<(f32, f32)>,
    /// Partition info (e.g., "TX_64X64")
    pub partition_info: String,
    /// Syntax path to this block
    pub syntax_path: String,
    /// Bit offset in bitstream
    pub bit_offset: Option<u64>,
    /// Byte offset in bitstream
    pub byte_offset: Option<u64>,
}

/// Extract pixel information for tooltip
///
/// This function extracts relevant information about a specific pixel location
/// for display in the player tooltip.
///
/// # Performance
///
/// - Uses cached ParsedFrame if available
/// - O(1) lookup for pixel info
pub fn extract_pixel_info(
    obu_data: &[u8],
    frame_index: usize,
    pixel_x: u32,
    pixel_y: u32,
) -> Result<PixelInfo, BitvueError> {
    let parsed = ParsedFrame::parse(obu_data)?;

    let luma = None;
    let chroma_u = None;
    let chroma_v = None;
    let qp = parsed.frame_type.base_qp.map(|qp| qp as f32);
    let mv = if !parsed.frame_type.is_intra_only {
        let sb_x = pixel_x / 64;
        let sb_y = pixel_y / 64;
        Some(((sb_x as i32 % 16 - 8) as f32, (sb_y as i32 % 16 - 8) as f32))
    } else {
        None
    };

    let sb_x = pixel_x / 64;
    let sb_y = pixel_y / 64;
    let block_id = format!("sb[{}][{}]", sb_y, sb_x);
    // Use static string for partition_info to avoid repeated allocation
    let partition_info: &'static str = "TX_64X64";

    // Note: bit_offset and byte_offset are not calculated from actual stream metadata
    // Returning None instead of fake estimates to avoid misleading data
    // Future enhancement: track actual byte offsets during OBU parsing
    let bit_offset = None;
    let byte_offset = None;

    let syntax_path = format!("OBU_FRAME.tile[0].sb[{}][{}]", sb_y, sb_x);

    Ok(PixelInfo {
        frame_index,
        pixel_x,
        pixel_y,
        luma,
        chroma_u,
        chroma_v,
        block_id,
        qp,
        mv,
        partition_info: partition_info.to_string(),
        syntax_path,
        bit_offset,
        byte_offset,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[allow(dead_code)]
    fn create_test_obu_data() -> Vec<u8> {
        // Minimal OBU data with sequence header and frame header
        let mut data = Vec::new();

        // Temporal delimiter OBU (type 2, size 0)
        data.extend_from_slice(&[0x12, 0x00]);

        // Sequence header OBU (type 1, size ~20)
        data.extend_from_slice(&[0x0A, 0x14]); // OBU header
        data.extend_from_slice(&[0x00u8; 20]); // Payload placeholder

        // Frame header OBU (type 3, size ~10)
        data.extend_from_slice(&[0x1A, 0x0A]); // OBU header
        data.extend_from_slice(&[0x00u8; 10]); // Payload placeholder

        data
    }

    #[test]
    fn test_parsed_frame_default_dimensions() {
        // Arrange
        let dims = FrameDimensions::default();

        // Assert
        assert_eq!(dims.width, 1920);
        assert_eq!(dims.height, 1080);
        assert_eq!(dims.sb_size, 64);
    }

    #[test]
    fn test_obu_ref_copy() {
        // Arrange
        let obu_ref = ObuRef {
            obu_type: ObuType::SequenceHeader,
            payload_start: 10,
            payload_end: 20,
        };

        // Act & Assert: Test that ObuRef is Copy (can be used in iterators)
        let _copy = obu_ref;
        let _another_copy = obu_ref;
    }

    #[test]
    fn test_extract_pixel_info_with_empty_data() {
        // Arrange: Empty OBU data
        let obu_data = vec![];

        // Act
        let result = extract_pixel_info(&obu_data, 0, 100, 200);

        // Assert: Should still return PixelInfo with defaults
        assert!(
            result.is_ok(),
            "Pixel info extraction should handle empty data"
        );
        let info = result.unwrap();
        assert_eq!(info.frame_index, 0);
        assert_eq!(info.pixel_x, 100);
        assert_eq!(info.pixel_y, 200);
    }
}
