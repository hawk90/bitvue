//! The data a parsed AV1 frame header is made of: `FrameHeader` and the per-tool records it holds.

use super::FrameType;

/// Loop restoration type of a plane (spec `FrameRestorationType`, 5.9.20). `Switchable` means
/// each restoration unit picks NONE/WIENER/SGRPROJ itself, with a symbol in the tile data.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoopRestorationType {
    None = 0,
    Wiener = 1,
    SgrProj = 2,
    Switchable = 3,
}

impl LoopRestorationType {
    pub fn from_bits(bits: u32) -> Self {
        match bits & 0x3 {
            0 => Self::None,
            1 => Self::Wiener,
            2 => Self::SgrProj,
            3 => Self::Switchable,
            _ => Self::None,
        }
    }
}

/// Loop filter (deblocking) parameters parsed from the frame header, per AV1 spec Section 5.9.11
/// (`loop_filter_params()`).
#[derive(Debug, Clone, Default)]
pub struct LoopFilterInfo {
    /// Per-plane filter levels: `[0]`=vertical luma edges, `[1]`=horizontal luma edges,
    /// `[2]`=U, `[3]`=V (chroma levels only set when `num_planes > 1`).
    pub level: [u8; 4],
    pub sharpness: u8,
    pub delta_enabled: bool,
    /// Per-reference-frame delta (indexed by `RefFrame as usize`, `NUM_REF_FRAMES` = 8), only
    /// meaningful when `delta_enabled`.
    pub ref_deltas: [i8; 8],
    /// Per-mode delta (`[0]` = ZEROMV, `[1]` = other inter modes), only meaningful when
    /// `delta_enabled`.
    pub mode_deltas: [i8; 2],
}

/// CDEF damping info parsed from the frame header
#[derive(Debug, Clone, Default)]
pub struct CdefInfo {
    pub enabled: bool,
    pub damping: u8,
    pub y_primary_strength: u8,
    pub y_secondary_strength: u8,
    pub uv_primary_strength: u8,
    pub uv_secondary_strength: u8,
    /// `cdef_bits` (spec 5.9.19) -- the raw bit-width of each tile-data `cdef_idx()` symbol (spec
    /// 5.11.56). `0` when CDEF is disabled for this frame (`!enabled`), matching real spec's own
    /// `cdef_bits = 0` default for that case (`cdef_idx()` becomes a 0-bit no-op, not skipped
    /// entirely -- see `tile::coding_unit::parse_coding_unit`'s doc for why this crate previously
    /// never read `cdef_idx()` at all, a real per-superblock desync).
    pub bits: u8,
}

/// Loop restoration info parsed from the frame header
#[derive(Debug, Clone)]
pub struct LoopRestorationInfo {
    pub enabled: bool,
    /// Luma restoration unit size in pixels (64, 128, or 256)
    pub unit_size: u32,
    /// Chroma restoration unit size in pixels (`lr_uv_shift` halves it for 4:2:0)
    pub uv_unit_size: u32,
    pub y_type: LoopRestorationType,
    pub u_type: LoopRestorationType,
    pub v_type: LoopRestorationType,
}

impl Default for LoopRestorationInfo {
    fn default() -> Self {
        Self {
            enabled: false,
            unit_size: 64,
            uv_unit_size: 64,
            y_type: LoopRestorationType::None,
            u_type: LoopRestorationType::None,
            v_type: LoopRestorationType::None,
        }
    }
}

/// Film grain synthesis parameters
#[derive(Debug, Clone, Default)]
pub struct FilmGrainInfo {
    pub enabled: bool,
    pub update_offset: u8,
    pub seed: u64,
    pub scaling_shift: u8,
    pub ar_coeff_lag: u8,
    pub ar_coeffs_y: Vec<i8>,
    pub ar_coeffs_uv: Vec<i8>,
    pub ar_coeff_shift: u8,
    pub grain_scale_shift: u8,
    pub chroma_scaling_from_luma: bool,
    pub overlap: bool,
    pub clip_to_restricted_range: bool,
}

/// Super resolution parameters
#[derive(Debug, Clone, Default)]
pub struct SuperResolutionInfo {
    pub enabled: bool,
    pub scale_denominator: u8,
}

/// `TxMode` (spec 5.9.21 `read_tx_mode`) -- selects how a coding block's transform size(s) are
/// determined. `Only4x4` and `Largest` both read zero bits (a mechanical table lookup only);
/// `Switchable` is the only mode where `tx_size()`/`read_var_tx_size()` (spec 5.11.15-18) read
/// real per-block symbols. Default `Largest` matches the real spec's fallback for OBUs this
/// crate can't parse (same resilient-fallback precedent as `reference_select`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum TxfmMode {
    /// `CodedLossless == 1` forces this -- every transform is 4x4, unconditionally.
    Only4x4,
    /// No `tx_size()` bits read -- every transform is the largest that fits the coding block.
    #[default]
    Largest,
    /// Real per-block `tx_size()`/`read_var_tx_size()` reads determine the actual transform
    /// size(s), which can be smaller than the coding block's own largest-fitting size.
    Switchable,
}

/// The tile layout of a frame (spec 5.9.15 `tile_info()`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TileLayout {
    /// First superblock column of every tile column, then the superblock column count.
    pub col_starts_sb: Vec<u32>,
    /// First superblock row of every tile row, then the superblock row count.
    pub row_starts_sb: Vec<u32>,
    /// `TileColsLog2` / `TileRowsLog2`: the bit counts of a tile group's first/last tile numbers
    /// are their sum (not derived from the tile counts: a uniform layout can have fewer tiles
    /// than `1 << log2`).
    pub tile_cols_log2: u32,
    pub tile_rows_log2: u32,
    /// `context_update_tile_id`: the tile whose final CDFs the frame saves.
    pub context_update_tile_id: u32,
    /// `TileSizeBytes`: the width of the size field in front of every tile but the last of a
    /// tile group.
    pub tile_size_bytes: u8,
}

impl TileLayout {
    pub fn tile_cols(&self) -> u32 {
        self.col_starts_sb.len().saturating_sub(1) as u32
    }

    pub fn tile_rows(&self) -> u32 {
        self.row_starts_sb.len().saturating_sub(1) as u32
    }

    pub fn tile_count(&self) -> u32 {
        self.tile_cols() * self.tile_rows()
    }
}

/// Minimal frame header information
#[derive(Debug, Clone)]
pub struct FrameHeader {
    /// Frame type
    pub frame_type: FrameType,
    /// Show frame flag
    pub show_frame: bool,
    /// Show existing frame flag
    pub show_existing_frame: bool,
    /// Frame to show (if show_existing_frame is true)
    pub frame_to_show_map_idx: Option<u8>,
    /// Error resilient mode
    pub error_resilient_mode: bool,
    /// Base quantization index (0-255)
    pub base_q_idx: Option<u8>,
    /// Y DC delta Q
    pub y_dc_delta_q: Option<i8>,
    /// UV DC delta Q
    pub uv_dc_delta_q: Option<i8>,
    /// Delta Q params (whether delta Q is enabled for this frame)
    /// Per AV1 Spec Section 5.9.17 (Delta Q Params Syntax)
    pub delta_q_present: bool,
    /// Delta Q residue (1 bit if delta_q_residue_enabled in sequence header)
    pub delta_q_residue: Option<bool>,
    /// Uncompressed header size in bytes (for finding tile data start)
    pub header_size_bytes: usize,
    /// Exact length of the uncompressed header in bits, before its byte alignment (or trailing
    /// bits, for a `FrameHeader` OBU).
    pub header_size_bits: u64,
    /// The frame's tile layout (spec 5.9.15); `None` when the header was not parsed that far
    /// (`show_existing_frame`, or the approximate parser), meaning one tile.
    pub tiles: Option<TileLayout>,
    /// Refresh frame flags (8 bits) - which reference slots to refresh
    pub refresh_frame_flags: Option<u8>,
    /// Reference frame indices, one per `ref_frame_sign_bias` slot
    /// [LAST, LAST2, LAST3, GOLDEN, BWDREF, ALTREF2, ALTREF] (3 bits each, `REFS_PER_FRAME`=7).
    pub ref_frame_idx: Option<[u8; 7]>,
    /// `OrderHint` for this frame (spec 7.4) -- 0 when not parsed (e.g. via
    /// `parse_frame_header_basic`, which deliberately doesn't track it).
    pub order_hint: u32,
    /// Frame width in pixels (0 if not parsed)
    pub width: u32,
    /// Frame height in pixels (0 if not parsed)
    pub height: u32,
    /// Upscaled width (super-resolution)
    pub upscaled_width: u32,
    /// Upscaled height (super-resolution)
    pub upscaled_height: u32,
    /// `reference_select` (spec 5.9.23 `frame_reference_mode()`) -- whether compound (2-reference)
    /// prediction is enabled for this frame. Always `false` from `parse_frame_header_basic` (it
    /// doesn't reach this field's bit position -- see that function's doc); real value only from
    /// `parse_frame_header_full`.
    pub reference_select: bool,
    /// `allow_intrabc` (spec 5.9.2) -- whether intra block copy is enabled for this (intra) frame.
    /// Same basic-vs-full caveat as `reference_select`.
    pub allow_intrabc: bool,
    /// `allow_screen_content_tools` (spec 5.9.2) -- gates `palette_mode_info()`'s real eligibility
    /// (`crate::tile::coding_unit::read_palette_mode_info`'s doc) independently of `allow_intrabc`
    /// (real spec: `allow_intrabc` implies this, but this can be `true` while `allow_intrabc` is
    /// `false` -- palette-only screen content). Same basic-vs-full caveat as `reference_select`;
    /// `parse_frame_header_basic` conservatively reports `false` (never attempts palette), which is
    /// always bitstream-sync-safe here since nothing reads a `has_palette_y`/`_uv` bit unless this
    /// is `true`.
    pub allow_screen_content_tools: bool,
    /// `delta_lf_present`/`delta_lf_multi` (spec 5.9.14 `delta_lf_params()`) -- gate
    /// `parse_coding_unit`'s real `delta_lf` read (only reachable at all when `delta_q_present` is
    /// also true -- real spec nests `delta_lf_params()`'s bits inside `delta_q_params()`'s). Same
    /// basic-vs-full caveat as `reference_select`; both default `false` from
    /// `parse_frame_header_basic` (never attempts a `delta_lf` read), which is bitstream-sync-safe
    /// for the same reason `allow_screen_content_tools`'s doc gives.
    pub delta_lf_present: bool,
    pub delta_lf_multi: bool,
    /// `skip_mode_present` (spec 5.9.22 `skip_mode_params()`) -- gates the real per-CU
    /// `skip_mode` read (spec 5.11.5). Same basic-vs-full caveat as `reference_select`; always
    /// `false` from `parse_frame_header_basic` (matches real spec's own `skip_mode_present = 0`
    /// default whenever `skip_mode_params()`'s derivation conditions aren't met, so treating an
    /// un-derivable value as "absent" is bitstream-sync-safe, not just a placeholder).
    pub skip_mode_present: bool,
    /// `SkipModeFrame[0]/[1]` (spec 5.9.22) -- the two real `RefFrame` values a `skip_mode` CU's
    /// `ref_frame()`/`mv[]` are forced from (spec 5.11.25, no bits read). Only meaningful when
    /// `skip_mode_present`; `[0, 0]` (both `RefFrame::Intra`-numbered, never a real ref pair)
    /// otherwise -- same "un-derivable means absent" convention as `skip_mode_present` itself.
    pub skip_mode_refs: [u8; 2],
    /// `is_filter_switchable` (spec 5.9.10 `interpolation_filter()`) -- gates the real per-CU
    /// `filter[]` (subpel interpolation filter) read (spec 5.11.30). Same basic-vs-full caveat
    /// as `reference_select`.
    pub subpel_filter_switchable: bool,
    /// `is_motion_mode_switchable` (spec 5.9.2) -- gates the real per-CU `motion_mode` read (spec
    /// 5.11.27, `read_motion_mode`'s doc). Same basic-vs-full caveat as `reference_select`.
    pub switchable_motion_mode: bool,
    /// `allow_warped_motion` (spec 5.9.2) -- one of `motion_mode`'s eligibility conditions (the
    /// other being a real above/left matching-reference scan, `find_matching_ref`'s doc). Same
    /// basic-vs-full caveat as `reference_select`.
    pub allow_warped_motion: bool,
    /// `force_integer_mv` (spec 5.9.2) -- when true, `motion_mode`'s real spec-5.11.27
    /// `GmType[RefFrame[0]] > TRANSLATION` exclusion (see `gm_type`'s doc) never applies (the
    /// spec gates that whole check on `!force_integer_mv`). Always `false` from
    /// `parse_frame_header_basic` (same basic-vs-full caveat as `reference_select` -- `false` is
    /// the common case anyway, since it requires `allow_screen_content_tools`).
    pub force_integer_mv: bool,
    /// `allow_high_precision_mv` (spec 5.9.2) -- whether MV differences carry the eighth-sample
    /// bit. Always `false` from `parse_frame_header_basic`; real value from
    /// `frame_header_full`.
    pub allow_high_precision_mv: bool,
    /// `GmType[ref]` (spec 5.9.24 `global_motion_params()`) for each of the 8 `RefFrame` values
    /// (index 0/`RefFrame::Intra` unused, stays `IDENTITY`) -- `0`=IDENTITY, `1`=TRANSLATION,
    /// `2`=ROTZOOM, `3`=AFFINE. Closes `read_motion_mode`'s documented gap: real spec forces
    /// `motion_mode = SIMPLE` (no bits read) for a `GLOBALMV`/`GLOBAL_GLOBALMV` block whose
    /// `GmType[RefFrame[0]]` is more complex than TRANSLATION, which this crate previously never
    /// modeled (`global_motion_params()` bits were read for sync but the classification was
    /// discarded). All-`IDENTITY` (`[0; 8]`) whenever `global_motion_params()` isn't parsed at all
    /// (intra frames, or `parse_frame_header_basic`'s basic-vs-full caveat) -- correct, since real
    /// spec's own default for an unparsed/intra frame is all-IDENTITY too.
    pub gm_type: [u8; 8],
    /// `gm_params` (spec 5.9.25) per `RefFrame`; the identity wherever `gm_type` is IDENTITY.
    pub gm_params: crate::frame_header_full::GmParams,
    /// `is_scaled(refFrame)` (spec 5.11.27) per `RefFrame`: the reference's upscaled size differs
    /// from this frame's coded size, which is every reference of a superres frame. A scaled
    /// reference cannot be warped, so `motion_mode` is then never read. Unknown sizes (a slot no
    /// parsed frame refreshed) count as unscaled.
    pub ref_scaled: [bool; 8],
    /// `reduced_tx_set` (spec 5.9.2) -- restricts `transform_type()` (5.11.47) to a smaller
    /// symbol alphabet when set. Always `false` from `parse_frame_header_basic` (same
    /// basic-vs-full caveat as `reference_select`); real value only from
    /// `parse_frame_header_full`.
    pub reduced_tx_set: bool,
    /// `TxMode` (spec 5.9.21) -- see `TxfmMode`'s doc. Always `TxfmMode::Largest` from
    /// `parse_frame_header_basic` (same basic-vs-full caveat as `reference_select`).
    pub txfm_mode: TxfmMode,
    /// `use_ref_frame_mvs` (spec 5.9.2) -- whether this frame's `inter_mode` context may draw on
    /// temporal (cross-frame) motion-field candidates (spec 7.9's `motion_field_estimation`).
    /// This crate never saves a per-frame motion field (no reconstruction/motion compensation),
    /// so `crate::tile::context::SpatialRefContext::inter_mode_context` uses this flag directly as
    /// `globalmv_ctx`'s value -- rav1d's own `globalmv_ctx` starts at exactly this flag and is
    /// only ever overridden away from it when a real temporal candidate is found (`src/refmvs.rs`
    /// `rav1d_refmvs_find`/`add_temporal_candidate`), so this matches rav1d's *initial* value
    /// unconditionally and its *final* value whenever no temporal candidate happens to override
    /// it -- a strictly closer approximation than a hardcoded `false`, not a full implementation
    /// of the temporal subsystem itself. Same basic-vs-full caveat as `reference_select`.
    pub use_ref_frame_mvs: bool,
    /// `primary_ref_frame` (spec 5.9.2): which reference's saved CDFs this frame starts from, or
    /// `7` (`PRIMARY_REF_NONE`) for the defaults. Only the full parser reads it.
    pub primary_ref_frame: u32,
    /// `disable_frame_end_update_cdf` (spec 5.9.2): when set, the CDFs this frame saves for its
    /// refreshed slots are the ones it started with, not the adapted ones.
    pub disable_frame_end_update_cdf: bool,
    /// Real segmentation state (spec 5.9.14) -- see `crate::frame_header_full::SegmentationInfo`'s
    /// doc for the exact fields and known gap. Always `SegmentationInfo::default()` (all-disabled)
    /// from `parse_frame_header_basic` (same basic-vs-full caveat as `reference_select`).
    pub segmentation: crate::frame_header_full::SegmentationInfo,
    /// Loop filter (deblocking) parameters
    pub loop_filter: LoopFilterInfo,
    /// CDEF parameters
    pub cdef_damping: CdefInfo,
    /// Convenience alias: y primary CDEF strength
    pub cdef_y_primary_strength: u8,
    /// Convenience alias: y secondary CDEF strength
    pub cdef_y_secondary_strength: u8,
    /// Convenience alias: uv primary CDEF strength
    pub cdef_uv_primary_strength: u8,
    /// Convenience alias: uv secondary CDEF strength
    pub cdef_uv_secondary_strength: u8,
    /// Loop restoration parameters
    pub loop_restoration: LoopRestorationInfo,
    /// Film grain synthesis parameters
    pub film_grain: FilmGrainInfo,
    /// Super resolution parameters
    pub super_resolution: SuperResolutionInfo,
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------
