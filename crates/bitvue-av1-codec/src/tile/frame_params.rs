//! Frame-level, read-only inputs to coding-unit parsing.
//!
//! These come from the frame/sequence headers and are identical for every block of a frame, so
//! they travel together instead of as ~17 separate parameters through
//! `parse_superblock` -> `parse_coding_units_recursive` -> `parse_coding_unit`.
//! `ParsedFrame::coding_params` is the one place that derives them from a parsed frame.

use crate::frame_header_full::SegmentationInfo;
use crate::tile::{InterModeFlags, TxTypeFrameFlags};

/// See the module doc. Every field is `Copy`, so this is passed by shared reference and
/// destructured by the parser that needs the individual flags.
#[derive(Debug, Clone, Copy)]
pub struct FrameCodingParams {
    /// True if this is a KEY (or INTRA_ONLY) frame (INTRA only).
    pub is_key_frame: bool,
    /// True if delta Q is enabled for this frame.
    pub delta_q_enabled: bool,
    /// Frame header's `reference_select` flag (compound prediction enabled for this frame at
    /// all) -- see `ParsedFrame::reference_select`'s doc for how it's sourced.
    pub reference_select: bool,
    /// Frame header's `allow_intrabc` flag (only meaningful when `is_key_frame`).
    pub allow_intrabc: bool,
    /// Frame header's `allow_screen_content_tools` flag (spec 5.9.2, only meaningful when
    /// `is_key_frame` -- gates `palette_mode_info()`'s real eligibility independently of
    /// `allow_intrabc`, see `FrameHeader::allow_screen_content_tools`'s doc).
    pub allow_screen_content_tools: bool,
    /// Sequence header's `enable_filter_intra` flag (spec 5.5.1, gates
    /// `filter_intra_mode_info()`'s real eligibility).
    pub enable_filter_intra: bool,
    /// Frame header's `delta_lf_params()` flags (spec 5.9.14, see
    /// `FrameHeader::delta_lf_present`'s doc) -- gate the real `delta_lf` read alongside
    /// `delta_q_enabled`.
    pub delta_lf_present: bool,
    /// See `delta_lf_present`.
    pub delta_lf_multi: bool,
    /// Frame header's `use_ref_frame_mvs` flag, used as `inter_mode`'s `globalmv_ctx` (see
    /// `crate::tile::context::SpatialRefContext::inter_mode_context`'s doc).
    pub use_ref_frame_mvs: bool,
    /// Frame header's segmentation parameters.
    pub segmentation: SegmentationInfo,
    /// Frame header flags for `transform_type()` -- see `TxTypeFrameFlags`'s doc.
    pub tx_type_flags: TxTypeFrameFlags,
    /// Frame-level flags gating the real `motion_mode`/`interintra`/`compound_type`/`filter`
    /// reads -- see `InterModeFlags`'s doc.
    pub inter_mode_flags: InterModeFlags,
    /// Frame extent in AV1 "MI" (4x4) units (`tile::partition::mi_units` of the real frame pixel
    /// width/height) -- drives `parse_partition_recursive`'s real `hasRows`/`hasCols` frame-edge
    /// partition legality (spec 5.11.4). A frame smaller than the superblock loop's
    /// `sb_cols * sb_size`/`sb_rows * sb_size` extent needs the real values here; passing the
    /// superblock-rounded extent would make `has_rows`/`has_cols` always true.
    pub mi_rows: u32,
    /// See `mi_rows`.
    pub mi_cols: u32,
    /// `cdef_bits` from the frame header (width of the per-superblock `cdef_idx` read).
    pub cdef_bits: u8,
    /// `skip_mode_present` and the two `skip_mode_frame` references (spec 5.9.22).
    pub skip_mode_present: bool,
    /// See `skip_mode_present`.
    pub skip_mode_refs: [u8; 2],
}
