//! The cached per-frame record and its small parts, plus the field accessors.

use crate::frame_header::TxfmMode;
use crate::ObuType;
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
    pub decoded: Option<crate::overlay_extraction::ParsedCodingUnits>,
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
    /// `gm_params` (spec 5.9.25) -- see `FrameHeader::gm_params`.
    pub gm_params: crate::frame_header_full::GmParams,
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
                gm_params: self.gm_params,
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
