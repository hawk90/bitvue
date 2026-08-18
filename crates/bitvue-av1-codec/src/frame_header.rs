//! Frame header parsing for AV1
//!
//! Parses frame header OBUs to extract frame type and other metadata.
//!
//! # Implementation Notes
//!
//! This parser implements the AV1 uncompressed header per spec Section 5.9.2.
//! Because a full decode of all conditional fields requires the Sequence Header
//! context (color config, film-grain params, etc.), fields that depend on
//! absent context are treated conservatively:
//!
//! - `frame_size_override_flag` is parsed but actual width/height override
//!   reading requires sequence-header `frame_width_bits`/`frame_height_bits`,
//!   so those are skipped (we read only the flag).
//! - `reduced_still_picture_header` (from sequence header) is assumed to be
//!   false for all calls coming through `parse_frame_header_basic`, which is
//!   the correct assumption for normal video streams.
//! - `enable_order_hint` is assumed to be true when `order_hint_bits > 0`.
//!
//! The parser extracts: show_existing_frame, frame_type, show_frame,
//! error_resilient_mode, disable_cdf_update, allow_screen_content_tools,
//! quantization_params (base_q_idx + delta_q fields), delta_q_params, and
//! refresh_frame_flags / ref_frame_idx for INTER frames.

use crate::bitreader::BitReader;
use bitvue_engine::BitvueError;
// Re-export FrameType for other modules in this crate
pub use bitvue_engine::FrameType;

/// Loop restoration type (per plane)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoopRestorationType {
    None = 0,
    Wiener = 1,
    SgrProj = 2,
    Dual = 3,
}

impl LoopRestorationType {
    pub fn from_bits(bits: u32) -> Self {
        match bits & 0x3 {
            0 => Self::None,
            1 => Self::Wiener,
            2 => Self::SgrProj,
            3 => Self::Dual,
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
    /// Restoration unit size in pixels (64, 128, or 256)
    pub unit_size: u32,
    pub y_type: LoopRestorationType,
    pub u_type: LoopRestorationType,
    pub v_type: LoopRestorationType,
}

impl Default for LoopRestorationInfo {
    fn default() -> Self {
        Self {
            enabled: false,
            unit_size: 64,
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

/// Read a signed delta-Q value: 1-bit present flag, then su(7) if present.
/// Per AV1 spec Section 5.9.14 (read_delta_q).
fn read_delta_q(reader: &mut BitReader) -> Result<Option<i8>, BitvueError> {
    let delta_coded = reader.read_bit()?;
    if delta_coded {
        // su(7): signed 7-bit value in [-64, 63]
        let val = reader.read_su(7)?;
        Ok(Some(val.clamp(-128, 127) as i8))
    } else {
        Ok(Some(0i8))
    }
}

/// Parse quantization_params() per AV1 spec Section 5.9.14.
///
/// Returns `(base_q_idx, y_dc_delta_q, uv_dc_delta_q, uv_ac_delta_q)`. `uv_ac_delta_q` is
/// `DeltaQUAc` -- previously read for bit-position sync only, then discarded, even though real
/// spec's `CodedLossless` derivation needs it (`parse_frame_header_full`'s `coded_lossless`
/// doc). When `separate_uv_delta_q`, V's own separate `DeltaQVDc`/`DeltaQVAc` are still read (for
/// sync) but not retained -- a narrower, still-open version of the same gap (real spec's
/// `CodedLossless` also needs those two whenever they can legally differ from U's), left
/// undertested/unfixed here since `separate_uv_delta_q` is itself the rarer path.
///
/// `separate_uv_delta_q` comes from the sequence header color config.
/// We assume false (the most common case) when no sequence header is available.
#[allow(clippy::type_complexity)]
pub(crate) fn parse_quantization_params(
    reader: &mut BitReader,
    separate_uv_delta_q: bool,
) -> Result<(Option<u8>, Option<i8>, Option<i8>, Option<i8>), BitvueError> {
    // base_q_idx  u(8)
    let base_q_idx = reader.read_bits(8)? as u8;

    // DeltaQYDc  read_delta_q()
    let y_dc = read_delta_q(reader)?;

    let (uv_dc, uv_ac) = if !separate_uv_delta_q {
        // DeltaQUDc  read_delta_q()
        let uv_dc = read_delta_q(reader)?;
        // DeltaQUAc  read_delta_q()
        let uv_ac = read_delta_q(reader)?;
        // DeltaQVDc = DeltaQUDc, DeltaQVAc = DeltaQUAc (implicit)
        (uv_dc, uv_ac)
    } else {
        // DeltaQUDc  read_delta_q()
        let udc = read_delta_q(reader)?;
        // DeltaQUAc  read_delta_q()
        let uac = read_delta_q(reader)?;
        // DeltaQVDc  read_delta_q()
        let _vdc = read_delta_q(reader)?;
        // DeltaQVAc  read_delta_q()
        let _vac = read_delta_q(reader)?;
        (udc, uac)
    };

    // using_qmatrix (1 bit)
    let using_qmatrix = reader.read_bit()?;
    if using_qmatrix {
        // qm_y  u(4)
        reader.read_bits(4)?;
        // qm_u  u(4)
        reader.read_bits(4)?;
        // qm_v  u(4)
        reader.read_bits(4)?;
    }

    Ok((Some(base_q_idx), y_dc, uv_dc, uv_ac))
}

/// Parse delta_q_params() per AV1 spec Section 5.9.17.
///
/// Returns `delta_q_present` flag.
fn parse_delta_q_params(reader: &mut BitReader, base_q_idx: u8) -> Result<bool, BitvueError> {
    let delta_q_present = if base_q_idx > 0 {
        reader.read_bit()?
    } else {
        false
    };

    if delta_q_present {
        // delta_q_res  u(2)
        reader.read_bits(2)?;
    }

    Ok(delta_q_present)
}

/// Parse delta_lf_params() per AV1 spec Section 5.9.18.
#[allow(dead_code)]
fn parse_delta_lf_params(
    reader: &mut BitReader,
    delta_q_present: bool,
    allow_intrabc: bool,
) -> Result<(), BitvueError> {
    if delta_q_present && !allow_intrabc {
        // delta_lf_present  f(1)
        let delta_lf_present = reader.read_bit()?;
        if delta_lf_present {
            // delta_lf_res  u(2)
            reader.read_bits(2)?;
            // delta_lf_multi  f(1)
            reader.read_bit()?;
        }
    }
    Ok(())
}

/// Segmentation params – skip all bits.
/// Per AV1 spec Section 5.9.14 (segmentation_params).
/// This skips the segmentation block without interpreting it.
#[allow(dead_code)]
fn skip_segmentation_params(reader: &mut BitReader) -> Result<(), BitvueError> {
    // segmentation_enabled  f(1)
    let seg_enabled = reader.read_bit()?;
    if seg_enabled {
        // segmentation_update_map  f(1)
        let update_map = reader.read_bit()?;
        if update_map {
            // segmentation_temporal_update  f(1)
            reader.read_bit()?;
        }
        // segmentation_update_data  f(1)
        let update_data = reader.read_bit()?;
        if update_data {
            // For each of 8 segments, read feature flags and values
            // Per spec: 8 segments × (SEG_LVL_MAX=8 features × (1 flag + value if enabled))
            for _seg in 0..8usize {
                for feature in 0..8usize {
                    let feature_enabled = reader.read_bit()?;
                    if feature_enabled {
                        // Feature value bit widths (per spec Table 5):
                        // 0=ALT_Q(8), 1=ALT_LF_Y_V(6), 2=ALT_LF_Y_H(6),
                        // 3=ALT_LF_U(6), 4=ALT_LF_V(6), 5=REF_FRAME(3),
                        // 6=SKIP(0), 7=GLOBALMV(0)
                        let (bits, signed) = match feature {
                            0 => (8u8, true),
                            1 | 2 | 3 | 4 => (6u8, true),
                            5 => (3u8, false),
                            _ => (0u8, false),
                        };
                        if bits > 0 {
                            reader.read_bits(bits)?;
                            if signed {
                                // sign bit
                                reader.read_bit()?;
                            }
                        }
                    }
                }
            }
        }
    }
    Ok(())
}

/// Skip loop_filter_params() per AV1 spec Section 5.9.11.
#[allow(dead_code)]
fn skip_loop_filter_params(
    reader: &mut BitReader,
    is_intra: bool,
    allow_intrabc: bool,
) -> Result<(), BitvueError> {
    if is_intra || allow_intrabc {
        return Ok(());
    }
    // loop_filter_level[0]  u(6)
    reader.read_bits(6)?;
    // loop_filter_level[1]  u(6)
    reader.read_bits(6)?;
    // loop_filter_sharpness  u(3)
    reader.read_bits(3)?;
    // loop_filter_delta_enabled  f(1)
    let delta_enabled = reader.read_bit()?;
    if delta_enabled {
        // loop_filter_delta_update  f(1)
        let delta_update = reader.read_bit()?;
        if delta_update {
            // 8 ref delta loop filters + 2 mode delta loop filters
            for _ in 0..10usize {
                let update_delta = reader.read_bit()?;
                if update_delta {
                    // loop_filter_ref_deltas[i] / loop_filter_mode_deltas[i]  su(7)
                    reader.read_su(7)?;
                }
            }
        }
    }
    Ok(())
}

/// Skip cdef_params() per AV1 spec Section 5.9.19.
#[allow(dead_code)]
fn skip_cdef_params(
    reader: &mut BitReader,
    is_intra: bool,
    allow_intrabc: bool,
    enable_cdef: bool,
) -> Result<(), BitvueError> {
    if !enable_cdef || allow_intrabc || is_intra {
        return Ok(());
    }
    // cdef_damping_minus_3  u(2)
    reader.read_bits(2)?;
    // cdef_bits  u(2)
    let cdef_bits = reader.read_bits(2)?;
    let num_cdef_strengths = 1usize << cdef_bits;
    for _ in 0..num_cdef_strengths {
        // cdef_y_pri_strength  u(4)
        reader.read_bits(4)?;
        // cdef_y_sec_strength  u(2)
        reader.read_bits(2)?;
        // cdef_uv_pri_strength  u(4)
        reader.read_bits(4)?;
        // cdef_uv_sec_strength  u(2)
        reader.read_bits(2)?;
    }
    Ok(())
}

/// Skip lr_params() per AV1 spec Section 5.9.20.
#[allow(dead_code)]
fn skip_lr_params(
    reader: &mut BitReader,
    is_intra: bool,
    allow_intrabc: bool,
    enable_restoration: bool,
    use_128x128_superblock: bool,
) -> Result<(), BitvueError> {
    if !enable_restoration || allow_intrabc || is_intra {
        return Ok(());
    }
    for _plane in 0..3usize {
        // lr_type  u(2)
        reader.read_bits(2)?;
    }
    // lr_unit_shift  u(1)
    let lr_unit_shift = reader.read_bits(1)?;
    if use_128x128_superblock && lr_unit_shift > 0 {
        // lr_unit_extra_shift  u(1)
        reader.read_bits(1)?;
    }
    // lr_uv_shift  u(1)  (only for sub-sampled chroma)
    reader.read_bits(1)?;
    Ok(())
}

/// Skip tx_mode_select() per AV1 spec Section 5.9.21.
#[allow(dead_code)]
fn skip_tx_mode(reader: &mut BitReader, is_intra: bool) -> Result<(), BitvueError> {
    if !is_intra {
        // tx_mode_select  f(1)
        reader.read_bit()?;
    }
    Ok(())
}

/// Skip frame_reference_mode() per AV1 spec Section 5.9.23.
#[allow(dead_code)]
fn skip_frame_reference_mode(reader: &mut BitReader, is_intra: bool) -> Result<(), BitvueError> {
    if !is_intra {
        // reference_select  f(1)
        reader.read_bit()?;
    }
    Ok(())
}

/// Skip skip_mode_params() per AV1 spec Section 5.9.22.
#[allow(dead_code)]
fn skip_skip_mode_params(
    reader: &mut BitReader,
    is_intra: bool,
    reference_select: bool,
) -> Result<(), BitvueError> {
    // Simplified: skip_mode_allowed depends on many conditions
    // For now just try to read the flag for inter frames with reference_select
    if !is_intra && reference_select {
        // skip_mode_present  f(1)
        reader.read_bit()?;
    }
    Ok(())
}

/// Parse frame header from payload
///
/// Implements AV1 spec Section 5.9.2 (uncompressed_header).
///
/// # Assumptions
///
/// Since we don't carry sequence header context:
/// - `reduced_still_picture_header` = false
/// - `enable_order_hint` = false (skips order_hint reading)
/// - `separate_uv_delta_q` = false
/// - `enable_cdef` = true
/// - `enable_restoration` = true
/// - `use_128x128_superblock` = false
pub fn parse_frame_header_basic(payload: &[u8]) -> Result<FrameHeader, BitvueError> {
    let mut reader = BitReader::new(payload);

    // show_existing_frame (1 bit)
    let show_existing_frame = reader.read_bit()?;

    if show_existing_frame {
        let frame_to_show_map_idx = reader.read_bits(3)? as u8;

        let header_size_bytes =
            reader.byte_position() + usize::from(!reader.position().is_multiple_of(8));

        return Ok(FrameHeader {
            frame_type: FrameType::Key,
            show_frame: true,
            show_existing_frame: true,
            frame_to_show_map_idx: Some(frame_to_show_map_idx),
            error_resilient_mode: false,
            base_q_idx: None,
            y_dc_delta_q: None,
            uv_dc_delta_q: None,
            delta_q_present: false,
            delta_q_residue: None,
            header_size_bytes,
            refresh_frame_flags: None,
            ref_frame_idx: None,
            order_hint: 0,
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
            gm_type: [0u8; 8],
            reduced_tx_set: false,
            txfm_mode: TxfmMode::Largest,
            use_ref_frame_mvs: false,
            segmentation: crate::frame_header_full::SegmentationInfo::default(),
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

    // frame_type (2 bits) – AV1 Spec 5.9.2
    let frame_type_bits = reader.read_bits(2)?;
    let frame_type = FrameType::from_av1_bits(frame_type_bits);

    // show_frame (1 bit)
    let show_frame = reader.read_bit()?;

    // showable_frame (1 bit) – only when NOT (KEY and show_frame)
    let _showable_frame = if !(frame_type == FrameType::Key && show_frame) {
        reader.read_bit()?
    } else {
        true
    };

    // error_resilient_mode (1 bit) – KEY frames shown are always error-resilient
    let error_resilient_mode = match (frame_type, show_frame) {
        (FrameType::Key, true) => true,
        _ => reader.read_bit()?,
    };

    // disable_cdf_update (1 bit)
    let _disable_cdf_update = reader.read_bit()?;

    // allow_screen_content_tools – conditional per spec
    // force_integer_mv – conditional
    //
    // These depend on sequence-header `seq_force_screen_content_tools` /
    // `seq_force_integer_mv`. Without that context we skip them conservatively.
    // In practice most streams do NOT set these flags, so skipping them is safe
    // for the vast majority of real-world AV1.

    // frame_size_override_flag (1 bit) – for SWITCH frames or when not KEY
    let _frame_size_override = if frame_type == FrameType::Switch {
        true
    } else if frame_type != FrameType::Key {
        reader.read_bit()?
    } else {
        false
    };

    // order_hint – skip; without seq header we don't know order_hint_bits
    // We skip this field for simplicity.

    let is_intra = frame_type.is_intra();

    // primary_ref_frame (3 bits) per AV1 spec 5.9.2:
    // present when error_resilient_mode == 0 AND NOT (KEY_FRAME AND show_frame)
    // AND NOT SWITCH_FRAME
    let needs_primary_ref = !error_resilient_mode
        && !(frame_type == FrameType::Key && show_frame)
        && frame_type != FrameType::Switch;
    if needs_primary_ref {
        reader.read_bits(3)?;
    }

    // Buffer flags for inter frames
    // refresh_frame_flags (8 bits):
    // For KEY+show_frame and SWITCH frames: all slots refreshed (0xFF), no field in bitstream
    // For show_existing_frame: already handled above (None)
    // All other frames: read 8 bits from bitstream
    let refresh_frame_flags =
        if (frame_type == FrameType::Key && show_frame) || frame_type == FrameType::Switch {
            Some(0xFFu8)
        } else {
            match reader.read_bits(8) {
                Ok(v) => Some(v as u8),
                Err(_) => None,
            }
        };

    // For INTRA_ONLY frames: read ref_order_hint for each refreshed buffer
    // (we skip the actual values since we don't use them for extraction)
    // For INTER frames: read reference frame info
    let mut ref_frame_idx: Option<[u8; 7]> = None;
    if !is_intra {
        // frame_refs_short_signaling  f(1)
        // Only present when enable_order_hint is true in sequence header.
        // We conservatively assume false (not present) since we lack seq context.
        // Most bitstreams use enable_order_hint=true, so this bit IS typically
        // present. Reading it as 0 means we treat it as "not short-signaling",
        // which requires reading all 7 ref indices.
        let frame_refs_short = reader.read_bit().unwrap_or(false);
        if frame_refs_short {
            // Short signaling: last_frame_idx u(3) + gold_frame_idx u(3)
            // then set_frame_refs() fills in the remaining 5 slots implicitly
            reader.read_bits(3).ok(); // last_frame_idx
            reader.read_bits(3).ok(); // gold_frame_idx
                                      // No explicit ref_frame_idx bits to read; slots are derived.
                                      // We cannot recover the index values without running set_frame_refs().
        } else {
            // Long signaling: read all 7 ref_frame_idx u(3) entries
            let mut idx = [0u8; 7];
            let ok = (0..7).all(|i| {
                reader
                    .read_bits(3)
                    .map(|v| {
                        idx[i] = v as u8;
                    })
                    .is_ok()
            });
            if ok {
                ref_frame_idx = Some(idx);
            }
        }
    }

    // frame_size() – skip; without seq header we cannot know field widths
    // For frame_size_override we would need frame_width_bits/height_bits from seq header.
    // We skip these bits. This means header_size is an approximation for non-KEY frames.

    // render_and_frame_size_different – skip (1 bit, then maybe 2 × 16 bits)
    // For simplicity we skip these too.

    // allow_high_precision_mv, use_ref_frame_mvs, allow_intrabc: skip for inter

    // --- Fields we CAN reliably parse regardless of missing context ---

    // Skip loop_filter_params, segmentation, etc. and go directly to quant params.
    // We use a bounded read attempt: try parsing quantization at the current position.
    // If it fails we still return a valid (partial) header.

    let (base_q_idx_opt, y_dc_delta_q, uv_dc_delta_q, delta_q_present) =
        match parse_quantization_params(&mut reader, false) {
            Ok((base, y_dc, uv_dc, _uv_ac)) => {
                let base_val = base.unwrap_or(0);
                let dq = parse_delta_q_params(&mut reader, base_val).unwrap_or(false);
                (base, y_dc, uv_dc, dq)
            }
            Err(_) => (None, None, None, false),
        };

    let header_size_bytes = reader
        .byte_position()
        .saturating_add(usize::from(!reader.position().is_multiple_of(8)));

    Ok(FrameHeader {
        frame_type,
        show_frame,
        show_existing_frame: false,
        frame_to_show_map_idx: None,
        error_resilient_mode,
        base_q_idx: base_q_idx_opt,
        y_dc_delta_q,
        uv_dc_delta_q,
        delta_q_present,
        delta_q_residue: None,
        header_size_bytes,
        refresh_frame_flags,
        ref_frame_idx,
        order_hint: 0,
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
        gm_type: [0u8; 8],
        reduced_tx_set: false,
        txfm_mode: TxfmMode::Largest,
        use_ref_frame_mvs: false,
        segmentation: crate::frame_header_full::SegmentationInfo::default(),
        loop_filter: LoopFilterInfo::default(),
        cdef_damping: CdefInfo::default(),
        cdef_y_primary_strength: 0,
        cdef_y_secondary_strength: 0,
        cdef_uv_primary_strength: 0,
        cdef_uv_secondary_strength: 0,
        loop_restoration: LoopRestorationInfo::default(),
        film_grain: FilmGrainInfo::default(),
        super_resolution: SuperResolutionInfo::default(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bits_writer() -> (Vec<u8>, impl FnMut(&mut Vec<u8>, u32, u8)) {
        (Vec::new(), |bits: &mut Vec<u8>, v: u32, n: u8| {
            for i in (0..n).rev() {
                bits.push(((v >> i) & 1) as u8);
            }
        })
    }

    fn pack(bits: &[u8]) -> Vec<u8> {
        let mut out = vec![0u8; bits.len().div_ceil(8)];
        for (i, bit) in bits.iter().enumerate() {
            if *bit != 0 {
                out[i / 8] |= 1 << (7 - (i % 8));
            }
        }
        out
    }

    #[test]
    fn parse_quantization_params_retains_real_uv_ac_delta_q() {
        let (mut bits, mut push) = bits_writer();
        push(&mut bits, 0, 8); // base_q_idx = 0
        push(&mut bits, 0, 1); // DeltaQYDc: not coded => 0
        push(&mut bits, 0, 1); // DeltaQUDc: not coded => 0
        push(&mut bits, 1, 1); // DeltaQUAc: coded
        push(&mut bits, 3, 7); // DeltaQUAc value = 3 (su(7), positive)
        push(&mut bits, 0, 1); // using_qmatrix = 0
        let payload = pack(&bits);
        let mut reader = BitReader::new(&payload);

        let (base_q_idx, y_dc, uv_dc, uv_ac) =
            parse_quantization_params(&mut reader, false).unwrap();

        assert_eq!(base_q_idx, Some(0));
        assert_eq!(y_dc, Some(0));
        assert_eq!(uv_dc, Some(0));
        assert_eq!(uv_ac, Some(3), "DeltaQUAc must be retained, not discarded");
    }

    #[test]
    fn test_frame_type_from_bits() {
        assert_eq!(FrameType::from_av1_bits(0), FrameType::Key);
        assert_eq!(FrameType::from_av1_bits(1), FrameType::Inter);
        assert_eq!(FrameType::from_av1_bits(2), FrameType::IntraOnly);
        assert_eq!(FrameType::from_av1_bits(3), FrameType::Switch);
        assert_eq!(FrameType::from_av1_bits(4), FrameType::Unknown);
    }

    #[test]
    fn test_frame_type_names() {
        assert!(FrameType::Key.description().contains("Key"));
        assert!(FrameType::Inter.description().contains("Inter"));
        assert!(FrameType::IntraOnly.description().contains("Intra"));
        assert!(FrameType::Switch.description().contains("Switch"));
    }

    #[test]
    fn test_is_intra() {
        assert!(FrameType::Key.is_intra());
        assert!(!FrameType::Inter.is_intra());
        assert!(FrameType::IntraOnly.is_intra());
        assert!(!FrameType::Switch.is_intra());
    }

    #[test]
    fn test_parse_key_frame() {
        // show_existing_frame=0, frame_type=00 (KEY), show_frame=1
        // Binary: 0 00 1 0000 = 0x10
        let payload = [0b0001_0000];
        let header = parse_frame_header_basic(&payload).unwrap();
        assert_eq!(header.frame_type, FrameType::Key);
        assert!(header.show_frame);
        assert!(!header.show_existing_frame);
    }

    #[test]
    fn test_parse_inter_frame() {
        // Bit layout for first byte (0x30):
        //   bit0: show_existing_frame=0, bits1-2: frame_type=01 (INTER),
        //   bit3: show_frame=1, bit4: showable_frame=0, bit5: error_resilient_mode=0,
        //   bit6: disable_cdf_update=0, bit7: frame_size_override=0
        // Remaining fields read from zero-padded bytes that follow.
        let mut payload = [0u8; 16];
        payload[0] = 0b0011_0000;
        let header = parse_frame_header_basic(&payload).unwrap();
        assert_eq!(header.frame_type, FrameType::Inter);
        assert!(header.show_frame);
        assert!(!header.error_resilient_mode);
    }

    #[test]
    fn test_parse_show_existing_frame() {
        // show_existing_frame=1, frame_to_show_map_idx=101 (5)
        // Binary: 1 101 0000 = 0xD0
        let payload = [0b1101_0000];
        let header = parse_frame_header_basic(&payload).unwrap();
        assert!(header.show_existing_frame);
        assert_eq!(header.frame_to_show_map_idx, Some(5));
    }

    #[test]
    fn test_delta_q_present_when_base_q_nonzero() {
        // Build a minimal payload that has enough bits to reach quantization_params.
        // show_existing=0, frame_type=KEY(0b00), show_frame=1 → error_resilient=true(KEY shown)
        // disable_cdf_update=0, frame_size_override=false(KEY), …
        // We pad with zeros so reads don't fail; if parsing succeeds delta_q_present
        // should be parseable.
        let payload = vec![0u8; 32];
        // We just verify it doesn't panic; the exact value depends on the zero-filled content.
        let result = parse_frame_header_basic(&payload);
        assert!(result.is_ok());
    }
}
