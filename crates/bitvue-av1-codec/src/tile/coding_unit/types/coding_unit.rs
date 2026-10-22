//! The `CodingUnit` record one parsed block produces.

use super::super::palette::PaletteInfo;
use super::{MotionVector, PredictionMode, RefFrame, TxSize};
use crate::symbol::ResidualBlockStats;
use serde::{Deserialize, Serialize};

/// Coding Unit information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodingUnit {
    /// Block position (top-left corner) in pixels
    pub x: u32,
    pub y: u32,
    /// Block width in pixels
    pub width: u32,
    /// Block height in pixels
    pub height: u32,

    /// Skip flag (true = skip encoding, use prediction only)
    pub skip: bool,
    /// `skip_mode` (spec 5.11.5) -- true when this CU used implicit compound prediction with no
    /// explicit ref_frame/mode/MV/residual signaling at all (forced `skip = true`). Always
    /// `false` for key frames. See `SymbolDecoder::read_skip_mode`'s doc.
    pub skip_mode: bool,

    /// Real `segment_id` (spec 5.11.9/5.11.10), `0` when segmentation is disabled/inactive for
    /// this CU or this crate's known gaps apply -- see `parse_coding_unit`'s segment_id wiring
    /// and `crate::frame_header_full::SegmentationInfo`'s doc for the exact scope (spatial context
    /// real; temporal prediction and `!update_map` both fall back to `0`, a documented
    /// approximation that doesn't affect bitstream position).
    pub segment_id: u8,

    /// Prediction mode
    pub mode: PredictionMode,

    /// Reference frames (for INTER)
    /// AV1 supports compound prediction (2 references)
    pub ref_frames: [RefFrame; 2],

    /// `use_intrabc` (spec 5.11.6) -- true if this intra-frame block uses intra block copy
    /// (screen-content-coding: motion-compensation-style copy from already-decoded pixels in the
    /// *current* frame, rather than spatial intra prediction). Always `false` for inter blocks;
    /// only ever `true` when the frame header's `allow_intrabc` was set.
    pub use_intrabc: bool,

    /// Motion vectors (for INTER)
    /// L0 = forward reference, L1 = backward reference
    pub mv: [MotionVector; 2],

    /// Transform size (for residual coding). For CUs with a real `tx_blocks` var-tx breakdown,
    /// this is just the block's own starting/largest class (`Max_Tx_Size_Rect`) -- see
    /// `tx_blocks`'s doc for the real per-leaf sizes.
    pub tx_size: TxSize,

    /// Real per-leaf transform block breakdown from `read_var_tx_size` (spec 5.11.17/18), when
    /// available -- genuinely rectangular leaves supported (`TxBlock`'s doc), not just square.
    /// `Some` for non-`skip` INTER coding units *and* IntraBC coding units (real spec routes both
    /// through the same recursive `read_var_tx_size()` tree, see `compute_inter_tx_blocks`'s doc)
    /// within its width/height range. `None` elsewhere (regular intra, skip, or oversized CUs):
    /// those still use the older uniform `tx_size`-tiled grid (`width.div_ceil(tx_size.size())`
    /// etc, see `parse_coding_unit`'s residual loop) -- a heuristic, not a real bitstream read,
    /// for exactly those CUs.
    pub tx_blocks: Option<Vec<TxBlock>>,

    /// QP value (quantization parameter)
    /// None for blocks that don't have QP (e.g., skip blocks)
    pub qp: Option<i16>,

    /// Aggregate residual coefficient statistics, summed across every transform block tiling
    /// this coding unit. `None` for skipped CUs (no residual read at all -- not the same as a
    /// non-skip CU whose transform blocks all happened to signal `all_zero`, which is `Some` with
    /// zero counts). See `SymbolDecoder::read_residual_block`'s doc for what this does and
    /// doesn't capture.
    pub residual: Option<ResidualBlockStats>,

    /// Real `palette_mode_info()` result (spec 5.11.46) -- `y_size`/`uv_size` both `0` (the
    /// default/common case) when this CU doesn't use palette mode for that plane. See
    /// `read_palette_mode_info`'s doc; per-pixel color-index maps (`read_palette_tokens`) are read
    /// for real bitstream sync but not retained here (matches `residual`'s aggregate-not-raw
    /// precedent -- no per-pixel/per-coefficient data is exposed on `CodingUnit` elsewhere either).
    pub palette: PaletteInfo,
}

/// One leaf transform block from a real `read_var_tx_size` walk (spec 5.11.17/18), in absolute
/// 4x4 ("MI") units for position, real pixel dimensions for size -- see `CodingUnit::tx_blocks`'s
/// doc. `width_px`/`height_px` (rather than a single square `TxSize`, this crate's original
/// square-only leaf representation) support genuinely rectangular leaves from non-square starting
/// blocks (see `read_var_tx_size`'s doc) -- for a square leaf these are simply equal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TxBlock {
    pub x4: u32,
    pub y4: u32,
    pub width_px: u32,
    pub height_px: u32,
}

impl CodingUnit {
    /// Create new coding unit (default INTRA)
    pub fn new(x: u32, y: u32, width: u32, height: u32) -> Self {
        let tx_size = TxSize::from_dimensions(width, height);
        Self {
            x,
            y,
            width,
            height,
            skip: false,
            skip_mode: false,
            segment_id: 0,
            mode: PredictionMode::DcPred,
            ref_frames: [RefFrame::Intra, RefFrame::Intra],
            use_intrabc: false,
            mv: [MotionVector::zero(), MotionVector::zero()],
            tx_size,
            tx_blocks: None,
            qp: None,
            residual: None,
            palette: PaletteInfo::default(),
        }
    }

    /// Check if this is an INTRA block
    pub fn is_intra(&self) -> bool {
        self.ref_frames[0] == RefFrame::Intra
    }

    /// Check if this is an INTER block
    pub fn is_inter(&self) -> bool {
        !self.is_intra()
    }

    /// Get effective QP value
    /// Returns base_qp if this block doesn't have a specific QP
    pub fn effective_qp(&self, base_qp: i16) -> i16 {
        self.qp.unwrap_or(base_qp)
    }
}
