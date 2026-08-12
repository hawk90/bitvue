//! Coding Unit Parsing
//!
//! Per AV1 Specification Section 5.11 (Coding Block Syntax)
//!
//! A Coding Unit contains:
//! - Prediction information (INTRA/INTER mode)
//! - Motion vectors (for INTER blocks)
//! - Transform information
//! - Quantization parameters
//! - Residual data
//!
//! ## Implementation Strategy
//!
//! **Phase 1** (Current):
//! - Parse skip flag
//! - Parse prediction mode
//! - Parse reference frames (for INTER)
//!
//! **Phase 2**:
//! - Parse motion vectors
//! - Calculate MV predictors
//! - Reconstruct final MVs
//!
//! **Phase 3**:
//! - Parse transform sizes
//! - Parse quantization info
//! - Parse residuals (optional for visualization)
//!
//! ## Residual reading is not optional -- it was a real desync bug, not a scope choice
//!
//! `parse_coding_unit` used to return immediately after `delta_q`, never reading the AV1 spec's
//! `residual()` syntax element for non-skip coding units. Because a tile's coefficient data is
//! arithmetic-coded with no byte-aligned skip points, this silently desynced the shared
//! `SymbolDecoder`'s position from every subsequent syntax element in the tile as soon as any CU
//! had `skip == false` -- confirmed via a real crash (`get_codec_extended_info` on frame 100 of
//! the real fixture panicked in `ArithmeticDecoder::refill` once the drift exhausted the tile's
//! real bytes). `SymbolDecoder::read_residual_block` (see its own doc for the CDF/context
//! simplifications) now reads a real (if non-spec-exact) residual read sequence for every
//! non-skip CU's transform blocks, closing the gap that caused this.

use crate::symbol::cdf::tx_size_class;
use crate::symbol::{ResidualBlockStats, SymbolDecoder};
use bitvue_engine::{BitvueError, Result};
use serde::{Deserialize, Serialize};

/// Frame header flags `SymbolDecoder::read_transform_type_is_1d`/`read_tx_size` need, bundled to
/// avoid growing `parse_coding_unit`'s already-long parameter list further -- see `ParsedFrame`'s
/// doc for how `coded_lossless`/`reduced_tx_set`/`txfm_mode` are sourced, and
/// `read_transform_type_is_1d`'s doc for why `qidx_is_zero` is a distinct condition from
/// `coded_lossless` (the real spec shortcut checks `base_q_idx == 0` alone, without also
/// requiring zero delta-Q).
///
/// `mono_chrome`/`subsampling_x`/`subsampling_y` are threaded through here too (sourced from
/// `ParsedFrame`'s fields of the same name), gating `parse_coding_unit`'s chroma residual read
/// (`SymbolDecoder::read_chroma_residual_block`, see its doc for scope and the regression this
/// piece's first attempt hit before landing on real chroma-specific default CDF values).
#[derive(Debug, Clone, Copy)]
pub struct TxTypeFrameFlags {
    pub coded_lossless: bool,
    pub qidx_is_zero: bool,
    pub reduced_tx_set: bool,
    pub txfm_mode: crate::frame_header::TxfmMode,
    pub mono_chrome: bool,
    pub subsampling_x: bool,
    pub subsampling_y: bool,
}

/// Prediction mode for intra and inter prediction
///
/// # Intra Modes (DcPred through PaethPred)
/// Used for intra-frame prediction where pixels are predicted from previously
/// coded samples within the same frame. Each mode uses a specific directional
/// or DC-based prediction strategy.
///
/// # Inter Modes (NewMv through GlobalMv)
/// Used for inter-frame prediction where pixels are predicted from reference frames
/// using motion vectors. Each mode represents a different MV selection strategy.
///
/// # AV1 Specific Modes
/// - **DcPred**: DC prediction (average of above/left samples)
/// - **SmoothPred/SmoothVPred/SmoothHPred**: Smooth interpolation modes
/// - **PaethPred**: Paeth predictor (edge detection)
/// - **NewMv**: Create new motion vector
/// - **NearestMv**: Use nearest MV from neighboring blocks
/// - **NearMv**: Use near MV from neighboring blocks
/// - **GlobalMv**: Use global motion vector
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PredictionMode {
    /// DC prediction (INTRA)
    DcPred,
    /// Vertical prediction (INTRA)
    VPred,
    /// Horizontal prediction (INTRA)
    HPred,
    /// Diagonal prediction (INTRA)
    D45Pred,
    /// Diagonal prediction (INTRA)
    D135Pred,
    /// Diagonal prediction (INTRA)
    D113Pred,
    /// Diagonal prediction (INTRA)
    D157Pred,
    /// Diagonal prediction (INTRA)
    D203Pred,
    /// Diagonal prediction (INTRA)
    D67Pred,
    /// Smooth prediction (INTRA)
    SmoothPred,
    /// Smooth vertical (INTRA)
    SmoothVPred,
    /// Smooth horizontal (INTRA)
    SmoothHPred,
    /// Paeth prediction (INTRA)
    PaethPred,

    /// INTER: Single reference, single MV
    NewMv,
    /// INTER: Use nearest MV from neighbors
    NearestMv,
    /// INTER: Use near MV from neighbors
    NearMv,
    /// INTER: Global motion
    GlobalMv,

    /// INTER (compound, spec 5.11.24 `compound_mode()`): L0=nearest, L1=nearest
    NearestNearestMv,
    /// INTER (compound): L0=near, L1=near
    NearNearMv,
    /// INTER (compound): L0=nearest, L1=new (explicit MV read for L1 only)
    NearestNewMv,
    /// INTER (compound): L0=new (explicit MV read for L0 only), L1=nearest
    NewNearestMv,
    /// INTER (compound): L0=near, L1=new (explicit MV read for L1 only)
    NearNewMv,
    /// INTER (compound): L0=new (explicit MV read for L0 only), L1=near
    NewNearMv,
    /// INTER (compound): L0=global, L1=global
    GlobalGlobalMv,
    /// INTER (compound): L0=new, L1=new (explicit MV read for both)
    NewNewMv,
}

/// Which MV-selection strategy applies to one reference-list slot (L0 or L1) of a prediction
/// mode. Single-ref modes only ever have an L0 component; compound modes (spec 5.11.24
/// `compound_mode()`) can combine two different kinds across L0/L1 -- e.g. `NearestNewMv` means
/// L0 uses the nearest-neighbor predictor while L1 reads an explicit MV from the bitstream.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MvKind {
    /// Use the nearest neighboring block's MV
    Nearest,
    /// Use the second-nearest candidate MV
    Near,
    /// Use the frame's global motion translation
    Global,
    /// Read an explicit MV delta from the bitstream (added to a nearest-MV predictor)
    New,
}

impl PredictionMode {
    /// Check if this is an INTRA mode
    pub fn is_intra(&self) -> bool {
        matches!(
            self,
            PredictionMode::DcPred
                | PredictionMode::VPred
                | PredictionMode::HPred
                | PredictionMode::D45Pred
                | PredictionMode::D135Pred
                | PredictionMode::D113Pred
                | PredictionMode::D157Pred
                | PredictionMode::D203Pred
                | PredictionMode::D67Pred
                | PredictionMode::SmoothPred
                | PredictionMode::SmoothVPred
                | PredictionMode::SmoothHPred
                | PredictionMode::PaethPred
        )
    }

    /// Check if this is an INTER mode
    pub fn is_inter(&self) -> bool {
        !self.is_intra()
    }

    /// Check if this mode requires reading at least one explicit MV component from the
    /// bitstream (single-ref `NewMv`, or any compound mode with a `New` component on either
    /// reference list).
    pub fn needs_mv(&self) -> bool {
        matches!(
            self,
            PredictionMode::NewMv
                | PredictionMode::NearestNewMv
                | PredictionMode::NewNearestMv
                | PredictionMode::NearNewMv
                | PredictionMode::NewNearMv
                | PredictionMode::NewNewMv
        )
    }

    /// True for any of the 8 compound (`compound_mode()`) modes.
    pub fn is_compound(&self) -> bool {
        self.l1_mv_kind().is_some()
    }

    /// MV-selection strategy for reference list 0 (L0). `None` for INTRA modes.
    pub fn l0_mv_kind(&self) -> Option<MvKind> {
        use PredictionMode::*;
        match self {
            NewMv | NewNearestMv | NewNearMv | NewNewMv => Some(MvKind::New),
            NearestMv | NearestNearestMv | NearestNewMv => Some(MvKind::Nearest),
            NearMv | NearNearMv | NearNewMv => Some(MvKind::Near),
            GlobalMv | GlobalGlobalMv => Some(MvKind::Global),
            _ => None,
        }
    }

    /// MV-selection strategy for reference list 1 (L1). `None` for single-ref and INTRA modes
    /// (no L1 exists).
    pub fn l1_mv_kind(&self) -> Option<MvKind> {
        use PredictionMode::*;
        match self {
            NearestNearestMv | NewNearestMv => Some(MvKind::Nearest),
            NearNearMv | NewNearMv => Some(MvKind::Near),
            NearestNewMv | NearNewMv | NewNewMv => Some(MvKind::New),
            GlobalGlobalMv => Some(MvKind::Global),
            _ => None,
        }
    }
}

/// Reference frame type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum RefFrame {
    /// No reference (INTRA)
    Intra = 0,
    /// Last frame
    Last = 1,
    /// Last2 frame
    Last2 = 2,
    /// Last3 frame
    Last3 = 3,
    /// Golden frame
    Golden = 4,
    /// BWD reference frame
    BwdRef = 5,
    /// ALT2 reference frame
    AltRef2 = 6,
    /// ALT reference frame
    AltRef = 7,
}

impl RefFrame {
    /// Parse from value (0-7)
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(RefFrame::Intra),
            1 => Some(RefFrame::Last),
            2 => Some(RefFrame::Last2),
            3 => Some(RefFrame::Last3),
            4 => Some(RefFrame::Golden),
            5 => Some(RefFrame::BwdRef),
            6 => Some(RefFrame::AltRef2),
            7 => Some(RefFrame::AltRef),
            _ => None,
        }
    }
}

/// Motion Vector (quarter-pel precision)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct MotionVector {
    /// Horizontal component (quarter-pel units)
    pub x: i32,
    /// Vertical component (quarter-pel units)
    pub y: i32,
}

impl MotionVector {
    /// Create new motion vector
    pub fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }

    /// Zero motion vector
    pub fn zero() -> Self {
        Self { x: 0, y: 0 }
    }

    /// Create from QuarterPel components
    ///
    /// Provides type-safe construction from QuarterPel newtype.
    #[inline]
    pub fn from_quarter_pel(x: crate::QuarterPel, y: crate::QuarterPel) -> Self {
        Self {
            x: x.qpel(),
            y: y.qpel(),
        }
    }

    /// Get X component as QuarterPel
    #[inline]
    #[must_use]
    pub const fn x_quarter_pel(&self) -> crate::QuarterPel {
        crate::QuarterPel::from_qpel(self.x)
    }

    /// Get Y component as QuarterPel
    #[inline]
    #[must_use]
    pub const fn y_quarter_pel(&self) -> crate::QuarterPel {
        crate::QuarterPel::from_qpel(self.y)
    }

    /// Get X component in pixels (integer-pel)
    #[inline]
    #[must_use]
    pub const fn x_pel(&self) -> i32 {
        self.x / 4
    }

    /// Get Y component in pixels (integer-pel)
    #[inline]
    #[must_use]
    pub const fn y_pel(&self) -> i32 {
        self.y / 4
    }

    /// Add motion vectors (saturating to prevent overflow with large predictor values)
    #[inline]
    #[must_use]
    pub fn add(&self, other: MotionVector) -> MotionVector {
        MotionVector::new(
            self.x.saturating_add(other.x),
            self.y.saturating_add(other.y),
        )
    }

    /// Subtract motion vectors (saturating to prevent overflow with large predictor values)
    #[inline]
    #[must_use]
    pub fn sub(&self, other: MotionVector) -> MotionVector {
        MotionVector::new(
            self.x.saturating_sub(other.x),
            self.y.saturating_sub(other.y),
        )
    }

    /// Get magnitude in quarter-pel units
    #[inline]
    #[must_use]
    pub fn magnitude_qpel(&self) -> i32 {
        // Approximate magnitude (avoiding sqrt for performance)
        self.x.abs() + self.y.abs()
    }

    /// Get magnitude in pixels (integer-pel, rounded)
    #[inline]
    #[must_use]
    pub fn magnitude_pel(&self) -> i32 {
        (self.magnitude_qpel() / 4).max(0)
    }
}

/// Transform size enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TxSize {
    Tx4x4 = 0,
    Tx8x8 = 1,
    Tx16x16 = 2,
    Tx32x32 = 3,
    Tx64x64 = 4,
}

impl TxSize {
    /// Get the size in pixels
    pub fn size(&self) -> u32 {
        match self {
            TxSize::Tx4x4 => 4,
            TxSize::Tx8x8 => 8,
            TxSize::Tx16x16 => 16,
            TxSize::Tx32x32 => 32,
            TxSize::Tx64x64 => 64,
        }
    }

    /// Get TxSize from block dimensions
    pub fn from_dimensions(width: u32, height: u32) -> Self {
        let size = width.max(height);
        match size {
            0..=4 => TxSize::Tx4x4,
            5..=8 => TxSize::Tx8x8,
            9..=16 => TxSize::Tx16x16,
            17..=32 => TxSize::Tx32x32,
            _ => TxSize::Tx64x64,
        }
    }

    /// Inverse of this enum's own discriminant order (0..=4) -- the size-*class* form
    /// `SymbolDecoder::read_tx_size`/`TileContext::tx_size_context` operate on. Clamps rather
    /// than erroring since callers only ever pass values already derived from a `TxSize`.
    pub fn from_class(class: u8) -> Self {
        match class {
            0 => TxSize::Tx4x4,
            1 => TxSize::Tx8x8,
            2 => TxSize::Tx16x16,
            3 => TxSize::Tx32x32,
            _ => TxSize::Tx64x64,
        }
    }
}

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
            mode: PredictionMode::DcPred,
            ref_frames: [RefFrame::Intra, RefFrame::Intra],
            use_intrabc: false,
            mv: [MotionVector::zero(), MotionVector::zero()],
            tx_size,
            tx_blocks: None,
            qp: None,
            residual: None,
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

/// Parse coding unit from symbol decoder
///
/// Reads block-level syntax elements from the bitstream.
///
/// # Arguments
///
/// * `decoder` - Symbol decoder for reading entropy-coded symbols
/// * `x`, `y` - Block position in pixels
/// * `width`, `height` - Block dimensions in pixels
/// * `is_key_frame` - True if this is a KEY frame (INTRA only)
/// * `current_qp` - Current quantization parameter value
/// * `delta_q_enabled` - True if delta Q is enabled for this frame
/// * `mv_ctx` - MV predictor context for calculating motion vector predictors
/// * `reference_select` - Frame header's `reference_select` flag (compound prediction enabled
///   for this frame at all) -- see `ParsedFrame::reference_select`'s doc for how it's sourced.
/// * `allow_intrabc` - Frame header's `allow_intrabc` flag (only meaningful when `is_key_frame`)
/// * `tile_ctx` - Above/left neighbor-state tracker for entropy context (currently only `skip`
///   uses it -- see `crate::tile::TileContext`'s doc)
/// * `tx_type_flags` - Frame header flags for `transform_type()` -- see `TxTypeFrameFlags`'s doc.
///
/// # Returns
///
/// Parsed coding unit with prediction info, motion vectors (if INTER), and QP value
#[allow(clippy::too_many_arguments)]
pub fn parse_coding_unit(
    decoder: &mut SymbolDecoder,
    x: u32,
    y: u32,
    width: u32,
    height: u32,
    is_key_frame: bool,
    current_qp: i16,
    delta_q_enabled: bool,
    mv_ctx: &mut crate::tile::MvPredictorContext,
    reference_select: bool,
    allow_intrabc: bool,
    use_ref_frame_mvs: bool,
    tile_ctx: &mut crate::tile::TileContext,
    tx_type_flags: TxTypeFrameFlags,
    mi_rows: u32,
    mi_cols: u32,
) -> Result<(CodingUnit, i16)> {
    let mut cu = CodingUnit::new(x, y, width, height);

    // Read skip flag -- real per-context CDF + adaptation, see `SymbolDecoder::read_skip`'s doc.
    let (x4, y4) = (x / 4, y / 4);
    let (width_4x4, height_4x4) = (width.div_ceil(4).max(1), height.div_ceil(4).max(1));
    let skip_ctx = tile_ctx.skip_context(x4, y4);
    cu.skip = decoder.read_skip(skip_ctx)?;
    tile_ctx.set_skip(x4, y4, width_4x4, height_4x4, cu.skip);

    // TODO: Read segment ID (if segmentation enabled)

    // Raw intra mode symbol (0..=12), captured below when `is_key_frame` -- only meaningful for
    // `SymbolDecoder::read_transform_type_is_1d`'s `y_mode_raw` param when `is_intra` (this
    // decoder never reads intra blocks within inter frames, so `is_key_frame` and "is this CU
    // intra" coincide -- see `read_ref_frames`'s wiring above/below for the same equivalence).
    let mut y_mode_raw: u8 = 0;

    // Determine if INTRA or INTER
    if is_key_frame {
        // KEY frames are always INTRA
        cu.ref_frames = [RefFrame::Intra, RefFrame::Intra];

        // use_intrabc (spec 5.11.6) -- rare (screen-content-coding), only read at all when the
        // frame header allows it.
        cu.use_intrabc = if allow_intrabc {
            decoder.read_use_intrabc()?
        } else {
            false
        };

        // Read INTRA prediction mode -- real per-context CDF + adaptation, see
        // `SymbolDecoder::read_intra_mode`'s doc.
        let (above_class, left_class) = tile_ctx.intra_mode_context(x4, y4);
        let mode_symbol = decoder.read_intra_mode(above_class, left_class)?;
        cu.mode = intra_mode_from_symbol(mode_symbol)?;
        tile_ctx.set_mode(x4, y4, width_4x4, height_4x4, mode_symbol);
        y_mode_raw = mode_symbol;

        // tx_size() (spec 5.11.15/16) -- real per-context CDF + adaptation, see
        // `SymbolDecoder::read_tx_size`'s doc. IntraBC is excluded from *this* single-size read:
        // real spec's `read_block_tx_size()` gates the recursive `read_var_tx_size()` tree on
        // `is_inter`, and dav1d's own block-mode dispatch (`b->intra = !intrabc_flag`, verified
        // directly against `src/decode.c`, not assumed) confirms IntraBC blocks are classified
        // `is_inter` for this purpose despite being coded within an intra frame -- real
        // `read_vartx_tree` is called for them identically to real inter blocks (`compute_inter_
        // tx_blocks`, below), not this heuristic-single-size path.
        if !cu.use_intrabc {
            let max_tx_class = cu.tx_size as u8; // from_dimensions's heuristic starting point
            let resolved_class = if tx_type_flags.coded_lossless {
                0
            } else {
                match tx_type_flags.txfm_mode {
                    crate::frame_header::TxfmMode::Only4x4 => 0,
                    crate::frame_header::TxfmMode::Largest => max_tx_class,
                    crate::frame_header::TxfmMode::Switchable => {
                        let ctx = tile_ctx.tx_size_context(x4, y4, max_tx_class);
                        decoder.read_tx_size(max_tx_class, ctx)?
                    }
                }
            };
            cu.tx_size = TxSize::from_class(resolved_class);
            tile_ctx.set_tx_class(x4, y4, width_4x4, height_4x4, resolved_class);
        } else {
            // Verification caveat (unlike every other real-context piece landed this session):
            // this crate's only real fixture (`test_data/av1_test.ivf`) has zero `use_intrabc`
            // CUs (0/1676, confirmed) -- `allow_screen_content_tools` is a rare screen-content
            // flag this clip never sets. This branch is spec/rav1d-verified (see above) but
            // *not* real-fixture-verified like the rest of this crate's entropy-decode work; it
            // reuses `compute_inter_tx_blocks` exactly as written for real inter CUs (already
            // real-fixture-verified there), so the risk is narrower than a from-scratch parser,
            // but genuinely exercising it needs a screen-content test clip this session doesn't
            // have (same class of gap as segment_id/palette, see `DEVELOPMENT_PHASES.md`).
            cu.tx_blocks = compute_inter_tx_blocks(
                decoder,
                tile_ctx,
                x,
                y,
                width,
                height,
                cu.skip,
                tx_type_flags.coded_lossless,
                tx_type_flags.txfm_mode,
                mi_rows,
                mi_cols,
            )?;
        }
    } else {
        // ref_frame() (spec 5.11.25) -- real per-context CDF + adaptation, see
        // `SymbolDecoder::read_ref_frames`'s doc.
        cu.ref_frames =
            decoder.read_ref_frames(tile_ctx, x4, y4, reference_select, width.min(height))?;
        let is_compound = cu.ref_frames[1] != RefFrame::Intra;
        tile_ctx.set_ref_frames(
            x4,
            y4,
            width_4x4,
            height_4x4,
            false, // never intra -- this decoder doesn't read intra blocks within inter frames
            is_compound,
            cu.ref_frames[0] as i8 - 1,
            if is_compound {
                cu.ref_frames[1] as i8 - 1
            } else {
                -1
            },
        );
        let rav1d_ref0 = cu.ref_frames[0] as i8 - 1;
        let rav1d_ref1 = if is_compound {
            cu.ref_frames[1] as i8 - 1
        } else {
            -1
        };

        if is_compound {
            // compound_mode() (spec 5.11.24) -- a distinct 8-symbol alphabet from the single-ref
            // 4-way `inter_mode`, see `SymbolDecoder::read_compound_mode`'s doc.
            let ctx = tile_ctx
                .compound_mode_context(x4, y4, width_4x4, height_4x4, rav1d_ref0, rav1d_ref1);
            let mode_symbol = decoder.read_compound_mode(ctx)?;
            cu.mode = compound_mode_from_symbol(mode_symbol)?;

            cu.mv[0] = match cu.mode.l0_mv_kind() {
                Some(MvKind::New) => {
                    let explicit_mv = read_explicit_mv(decoder)?;
                    let predictor = mv_ctx.get_mv_predictor(cu.mode, x, y, cu.ref_frames[0]);
                    explicit_mv.add(predictor)
                }
                Some(kind) => mv_ctx.predict_by_kind(kind, x, y, cu.ref_frames[0]),
                None => MotionVector::zero(),
            };
            cu.mv[1] = match cu.mode.l1_mv_kind() {
                Some(MvKind::New) => {
                    let explicit_mv = read_explicit_mv(decoder)?;
                    let predictor = mv_ctx.get_mv_predictor_l1(cu.mode, x, y, cu.ref_frames[1]);
                    explicit_mv.add(predictor)
                }
                Some(kind) => mv_ctx.predict_by_kind(kind, x, y, cu.ref_frames[1]),
                None => MotionVector::zero(),
            };

            tile_ctx.set_spatial_ref_block(
                x4,
                y4,
                width_4x4,
                height_4x4,
                rav1d_ref0,
                rav1d_ref1,
                cu.mode.l0_mv_kind() == Some(MvKind::New)
                    || cu.mode.l1_mv_kind() == Some(MvKind::New),
            );

            tracing::debug!(
                "Compound mode {:?} at ({}, {}): mv0={:?}, mv1={:?}",
                cu.mode,
                x,
                y,
                cu.mv[0],
                cu.mv[1]
            );
        } else {
            // INTER frame - read prediction mode
            let ctx = tile_ctx.inter_mode_context(
                x4,
                y4,
                width_4x4,
                height_4x4,
                rav1d_ref0,
                use_ref_frame_mvs,
            );
            let mode_symbol = decoder.read_inter_mode(ctx)?;
            cu.mode = inter_mode_from_symbol(mode_symbol)?;

            // If NEWMV, read motion vectors
            if cu.mode == PredictionMode::NewMv {
                // Read MV for L0 (forward reference)
                let explicit_mv = read_explicit_mv(decoder)?;

                // Get MV predictor and add to explicit MV
                let predictor = mv_ctx.get_mv_predictor(cu.mode, x, y, cu.ref_frames[0]);
                cu.mv[0] =
                    MotionVector::new(explicit_mv.x + predictor.x, explicit_mv.y + predictor.y);
                cu.mv[1] = MotionVector::zero();

                tracing::debug!(
                    "NEWMV at ({}, {}): explicit=({:?}), predictor=({:?}), final=({:?})",
                    x,
                    y,
                    explicit_mv,
                    predictor,
                    cu.mv[0]
                );
            } else {
                // For NEARESTMV, NEARMV, GLOBALMV: use predictor directly
                let predictor = mv_ctx.get_mv_predictor(cu.mode, x, y, cu.ref_frames[0]);
                cu.mv = [predictor, MotionVector::zero()];

                tracing::debug!(
                    "Mode {:?} at ({}, {}): using predictor {:?}",
                    cu.mode,
                    x,
                    y,
                    cu.mv[0]
                );
            }

            tile_ctx.set_spatial_ref_block(
                x4,
                y4,
                width_4x4,
                height_4x4,
                rav1d_ref0,
                rav1d_ref1,
                cu.mode == PredictionMode::NewMv,
            );
        }

        // read_block_tx_size() (spec 5.11.16/17/18) for INTER blocks -- real recursive var-tx
        // read, see `read_var_tx_size`'s doc for the exact scope (square CUs only) and why.
        cu.tx_blocks = compute_inter_tx_blocks(
            decoder,
            tile_ctx,
            x,
            y,
            width,
            height,
            cu.skip,
            tx_type_flags.coded_lossless,
            tx_type_flags.txfm_mode,
            mi_rows,
            mi_cols,
        )?;
    }

    // Add this CU to the MV predictor context for future blocks
    // Now uses zero-copy reference instead of cloning the entire CU
    mv_ctx.add_cu(&cu);

    // Read delta Q if enabled
    // Per AV1 Spec Section 5.11.38 (Quantization Parameter Delta)
    let new_qp = if delta_q_enabled {
        match decoder.read_delta_q() {
            Ok(delta_q) => {
                // Apply delta Q to current QP
                // Clamp to valid range [0, 255]
                let qp = (current_qp + delta_q).clamp(0, 255);
                tracing::debug!(
                    "Delta Q applied at ({}, {}): {} + {} = {}",
                    x,
                    y,
                    current_qp,
                    delta_q,
                    qp
                );
                cu.qp = Some(qp);
                qp
            }
            Err(e) => {
                tracing::warn!(
                    "Failed to read delta Q at ({}, {}): {}, using current QP",
                    x,
                    y,
                    e
                );
                cu.qp = Some(current_qp);
                current_qp
            }
        }
    } else {
        // Delta Q not enabled, use current QP
        cu.qp = Some(current_qp);
        current_qp
    };

    // Read residual() for every transform block tiling this CU -- required for correct bitstream
    // alignment whenever skip == false, not just for producing residual statistics. See this
    // module's doc and `SymbolDecoder::read_residual_block`'s doc.
    if !cu.skip {
        // Real `tx_blocks` (var-tx, inter only -- see `compute_inter_tx_blocks`'s doc) gives the
        // true per-leaf positions/sizes directly, including genuinely rectangular leaves;
        // everything else still tiles uniformly at `cu.tx_size` (a heuristic for those CUs, not a
        // real bitstream-derived size, still square-only).
        let tx_positions: Vec<(u32, u32, u32, u32)> = if let Some(blocks) = &cu.tx_blocks {
            blocks
                .iter()
                .map(|b| (b.x4, b.y4, b.width_px, b.height_px))
                .collect()
        } else {
            let tx_px = cu.tx_size.size();
            let tx_cols = width.div_ceil(tx_px).max(1);
            let tx_rows = height.div_ceil(tx_px).max(1);
            let tx_wh4 = tx_px / 4;
            (0..tx_rows)
                .flat_map(|tx_row| {
                    (0..tx_cols).map(move |tx_col| {
                        (x4 + tx_col * tx_wh4, y4 + tx_row * tx_wh4, tx_px, tx_px)
                    })
                })
                .collect()
        };
        // Real `txb_skip`/`dc_sign` neighbor context is only trustworthy where transform-block
        // boundaries are real (regular key-frame intra via `tx_size()`, or inter/IntraBC via real
        // `tx_blocks` -- both real bitstream-derived boundaries); other CUs keep the
        // fixed-context-0 fallback and never touch `tile_ctx`'s residual arrays, matching
        // `SymbolDecoder::read_residual_block`'s doc.
        let use_real_residual_ctx = (is_key_frame && !cu.use_intrabc) || cu.tx_blocks.is_some();
        let is_single_tx_block = tx_positions.len() == 1;
        let mut summary = ResidualBlockStats::default();
        for (tx_x4, tx_y4, tx_w_px, tx_h_px) in tx_positions {
            let (tx_w4, tx_h4) = (tx_w_px / 4, tx_h_px / 4);

            // transform_type() (spec 5.11.47) precedes coeffs() for every transform block -- see
            // `SymbolDecoder::read_transform_type_is_1d`'s doc. Its own `tx_size_px` param is the
            // real spec square-up size (larger dimension), matching `read_residual_block`'s own
            // `tx_class` derivation.
            let tx_class_1d = decoder.read_transform_type_is_1d(
                is_key_frame,
                tx_type_flags.coded_lossless,
                tx_type_flags.qidx_is_zero,
                tx_type_flags.reduced_tx_set,
                tx_w_px.max(tx_h_px),
                y_mode_raw,
            )?;

            let (txb_skip_ctx, dc_sign_ctx) = if use_real_residual_ctx {
                (
                    tile_ctx.txb_skip_context(tx_x4, tx_y4, tx_w4, tx_h4, is_single_tx_block),
                    tile_ctx.dc_sign_context(tx_x4, tx_y4, tx_w4, tx_h4),
                )
            } else {
                (0, 0)
            };

            let block = decoder.read_residual_block(
                tx_w_px,
                tx_h_px,
                tx_class_1d,
                txb_skip_ctx,
                dc_sign_ctx,
            )?;

            if use_real_residual_ctx {
                let cul_level = block.sum_abs_level.min(63) as u8;
                tile_ctx.set_residual_ctx(
                    tx_x4,
                    tx_y4,
                    tx_w4,
                    tx_h4,
                    cul_level,
                    block.dc_sign_value,
                );
            }

            summary.nonzero_count += block.nonzero_count;
            summary.sum_abs_level += block.sum_abs_level;
            summary.max_level = summary.max_level.max(block.max_level);
        }

        // Chroma (U/V) residual -- required for bitstream sync (spec 5.11.34's `residual()`
        // reads luma, then U, then V for every `HasChroma` block). Restricted to non-IntraBC luma
        // coding blocks 8x8 through 128x128 in either dimension (real rectangular chroma tiles
        // supported, since the luma CU itself can be non-square -- see `SymbolDecoder::
        // read_chroma_residual_block`'s doc for the desync bug this closed: every non-square
        // `HasChroma` block's chroma bits were previously never read at all once non-square inter
        // var-tx made non-square CUs common). Chroma's real max transform size caps each axis at
        // 32 independently -- spec `Max_Tx_Size_Rect`, confirmed against rav1d's
        // `DAV1D_MAX_TXFM_SIZE_FOR_BS` table -- regardless of luma size, in a 4:2:0 stream. Not
        // restricted to key frames: real fixture-verified on inter frames too (key-frame content
        // here happens to only ever use unpartitioned 128x128 blocks, so an earlier
        // key-frame-only version of this gate was accidentally *never exercised* by this fixture
        // at all -- see `real_fixture_square_chroma_eligible_blocks_exist_and_parse_cleanly`).
        //
        // Position tracking: chroma tile positions are tracked at the luma CU's `x4/2`/`y4/2`
        // origin (a coordinate-scale approximation, not a truly independent chroma-plane grid --
        // see `TileContext`'s chroma field doc) since only above/left *adjacency* matters for
        // context selection here, not absolute physical distance.
        if !cu.use_intrabc
            && !tx_type_flags.mono_chrome
            && tx_type_flags.subsampling_x
            && tx_type_flags.subsampling_y
            && (8..=128).contains(&width)
            && (8..=128).contains(&height)
        {
            let (chroma_w, chroma_h) = (width / 2, height / 2);
            let (chroma_tx_w, chroma_tx_h) = (chroma_w.min(32), chroma_h.min(32));
            let (chroma_tx_w4, chroma_tx_h4) = (chroma_tx_w / 4, chroma_tx_h / 4);
            let chroma_tiles_x = chroma_w.div_ceil(chroma_tx_w).max(1);
            let chroma_tiles_y = chroma_h.div_ceil(chroma_tx_h).max(1);
            let not_one_blk = chroma_tiles_x * chroma_tiles_y > 1;
            let (cx4_base, cy4_base) = (x4 / 2, y4 / 2);
            for plane in 0..2usize {
                for tile_row in 0..chroma_tiles_y {
                    for tile_col in 0..chroma_tiles_x {
                        let cx4 = cx4_base + tile_col * chroma_tx_w4;
                        let cy4 = cy4_base + tile_row * chroma_tx_h4;
                        let txb_skip_ctx = tile_ctx.txb_skip_context_chroma(
                            plane,
                            cx4,
                            cy4,
                            chroma_tx_w4,
                            chroma_tx_h4,
                            not_one_blk,
                        );
                        let dc_sign_ctx = tile_ctx.dc_sign_context_chroma(
                            plane,
                            cx4,
                            cy4,
                            chroma_tx_w4,
                            chroma_tx_h4,
                        );
                        let block = decoder.read_chroma_residual_block(
                            chroma_tx_w,
                            chroma_tx_h,
                            txb_skip_ctx,
                            dc_sign_ctx,
                        )?;
                        let cul_level = block.sum_abs_level.min(63) as u8;
                        tile_ctx.set_residual_ctx_chroma(
                            plane,
                            cx4,
                            cy4,
                            chroma_tx_w4,
                            chroma_tx_h4,
                            cul_level,
                            block.dc_sign_value,
                        );
                    }
                }
            }
        }

        cu.residual = Some(summary);
    } else {
        cu.residual = None;
    }

    Ok((cu, new_qp))
}

/// Compute the real (or, for non-`Switchable` `TxMode`s, deterministic-no-read) transform block
/// breakdown for one INTER **or IntraBC** coding unit -- spec 5.11.16's `read_block_tx_size()`
/// (real spec gates the recursive var-tx tree on `is_inter`, and IntraBC blocks are classified
/// `is_inter` for this purpose despite being coded within an intra frame -- see this function's
/// call sites' docs). Supports genuinely rectangular coding units (real `Max_Tx_Size_Rect`, not
/// this crate's older square-only `TxSize::from_dimensions` heuristic) -- verified against
/// rav1d's `dav1d_max_txfm_size_for_bs` table (`src/tables.c`) directly: for every real AV1 block
/// size up to 64 in each axis, the natural starting max transform size is simply the block's own
/// size (real var-tx recursion, not this table, is what performs any further splitting); only
/// block sizes wider or taller than 64 (128-wide/tall) cap that axis at 64 (spec: no transform
/// exceeds 64x64). Hence `max_ytx = (width.min(64), height.min(64))` -- no lookup table needed,
/// unlike what an earlier pass expected.
///
/// Mirrors rav1d's `read_vartx_tree` (`src/decode.c`, `memorysafety/rav1d`/`videolan/dav1d`,
/// BSD-2-Clause) dispatch order:
/// 1. `skip` (spec: no bits read regardless of `TxMode` -- but `Switchable` still needs the
///    block's natural max size written into `var_tx_context`'s neighbor arrays for later blocks'
///    context, even though this CU's own leaf list is moot since `residual()` never runs for a
///    skipped CU). Returns `None` (caller's `!cu.skip` gate already skips the residual loop).
/// 2. `coded_lossless` (this crate's frame-wide approximation of spec's per-segment
///    `LosslessArray`) forces uniform 4x4 tiling, no bits read, regardless of `TxMode` -- checked
///    before `TxMode` since lossless overrides even `Switchable`.
/// 3. `TxfmMode::Only4x4`/`Largest`: deterministic uniform tiling (4x4, or the CU's own natural
///    max size), no bits read -- `TxfmMode::Switchable` is the only case needing a real read.
/// 4. `TxfmMode::Switchable`: real recursive `read_var_tx_size` walk.
///
/// For CUs bigger than one max-size transform tile in either axis (>64 wide and/or tall), tiles
/// the walk across each max-size block -- matches rav1d's own `for y_off in 0..bh4/h { for x_off
/// in 0..bw4/w { read_tx_tree(...) } }`.
#[allow(clippy::too_many_arguments)]
fn compute_inter_tx_blocks(
    decoder: &mut SymbolDecoder,
    tile_ctx: &mut crate::tile::TileContext,
    x: u32,
    y: u32,
    width: u32,
    height: u32,
    skip: bool,
    coded_lossless: bool,
    txfm_mode: crate::frame_header::TxfmMode,
    mi_rows: u32,
    mi_cols: u32,
) -> Result<Option<Vec<TxBlock>>> {
    if !(4..=128).contains(&width) || !(4..=128).contains(&height) {
        return Ok(None);
    }
    let (x4, y4) = (x / 4, y / 4);
    let (width_4x4, height_4x4) = (width / 4, height / 4);
    let (max_ytx_w, max_ytx_h) = (width.min(64), height.min(64));

    if skip {
        tile_ctx.set_var_tx_class(
            x4,
            y4,
            width_4x4,
            height_4x4,
            tx_size_class(max_ytx_w) as u8,
            tx_size_class(max_ytx_h) as u8,
        );
        return Ok(None);
    }

    let uniform_size = if coded_lossless {
        Some((4, 4))
    } else {
        match txfm_mode {
            crate::frame_header::TxfmMode::Only4x4 => Some((4, 4)),
            crate::frame_header::TxfmMode::Largest => Some((max_ytx_w, max_ytx_h)),
            crate::frame_header::TxfmMode::Switchable => None,
        }
    };

    let mut leaves = Vec::new();
    if let Some((uw, uh)) = uniform_size {
        let (uw4, uh4) = (uw / 4, uh / 4);
        let (uw_class, uh_class) = (tx_size_class(uw) as u8, tx_size_class(uh) as u8);
        let mut ly = y4;
        while ly < y4 + height_4x4 {
            let mut lx = x4;
            while lx < x4 + width_4x4 {
                leaves.push(TxBlock {
                    x4: lx,
                    y4: ly,
                    width_px: uw,
                    height_px: uh,
                });
                tile_ctx.set_var_tx_class(lx, ly, uw4, uh4, uw_class, uh_class);
                lx += uw4;
            }
            ly += uh4;
        }
    } else {
        let (tile_w4, tile_h4) = (max_ytx_w / 4, max_ytx_h / 4);
        let mut ty = y4;
        while ty < y4 + height_4x4 {
            let mut tx = x4;
            while tx < x4 + width_4x4 {
                read_var_tx_size(
                    decoder,
                    tile_ctx,
                    tx,
                    ty,
                    max_ytx_w,
                    max_ytx_h,
                    0,
                    mi_rows,
                    mi_cols,
                    &mut leaves,
                )?;
                tx += tile_w4;
            }
            ty += tile_h4;
        }
    }
    Ok(Some(leaves))
}

/// Recursively read `read_var_tx_size()` (spec 5.11.17/18) for one max-size transform tile,
/// genuinely rectangular starting sizes supported -- see `compute_inter_tx_blocks`'s doc. Ports
/// rav1d's `read_tx_tree` (`src/decode.c`, `memorysafety/rav1d`/`videolan/dav1d`, BSD-2-Clause)
/// index-for-index, including its real asymmetric-split branching (verified against the C
/// directly, not assumed -- a naive square-only 4-way quad-split, this crate's original
/// implementation, is provably wrong for non-square starting sizes):
///
/// - Reads `txfm_split` only when `depth < 2 && (from_w, from_h) != (4, 4)` (spec: recursion caps
///   at 2 levels below the tile's own starting size, and 4x4 is always terminal) -- `cat =
///   2*(4-max_class)-depth` selects the CDF row (`SymbolDecoder::read_txfm_split`'s doc, `
///   max_class` = the square-up class of the *larger* dimension, matching real `t_dim->max`),
///   context from `TileContext::var_tx_context` (real per-axis width/height classes, not one
///   shared class).
/// - If split and `max_class > 1` (bigger than an 8x8-equivalent): recurse into 1, 2, or 4
///   children at `sub` -- real spec's `sub` always halves only the *larger* dimension (or both,
///   for a square starting size); the *count* of children read is asymmetric too: child `(0,0)`
///   always, `(1,0)` only when `from_w >= from_h`, `(0,1)` only when `from_h >= from_w`, and
///   `(1,1)` only when *both* hold (i.e. only ever for a square starting size) -- so a wide
///   starting size (`from_w > from_h`) reads exactly 2 children side by side, a tall one reads 2
///   stacked, and only a square one reads all 4. Skips (early return, no read, no leaves) any
///   child whose origin is `>= mi_rows`/`mi_cols` -- spec: transform blocks entirely outside the
///   frame aren't separately coded (the same shape as, but distinct from,
///   `tile::partition::parse_partition_recursive`'s own frame-edge check).
/// - If split and `max_class <= 1` (an 8x8-or-smaller-max-class starting size, e.g. an 8x8, 4x8,
///   or 8x4): no further symbol is read (spec-deterministic, always all-4x4) -- the leaf loop
///   below naturally produces the right leaf count since it always walks `from`'s full footprint
///   at 4x4 granularity in that case.
/// - Otherwise (not split, or `depth`/`from` already forced no-read): `(from_w, from_h)` itself is
///   the one leaf covering this node's whole footprint.
///
/// Every leaf updates `TileContext::set_var_tx_class` across its own footprint before returning,
/// matching rav1d's `case.set_disjoint(&dir.tx, tx)`.
#[allow(clippy::too_many_arguments)]
fn read_var_tx_size(
    decoder: &mut SymbolDecoder,
    tile_ctx: &mut crate::tile::TileContext,
    x4: u32,
    y4: u32,
    from_w: u32,
    from_h: u32,
    depth: u8,
    mi_rows: u32,
    mi_cols: u32,
    out: &mut Vec<TxBlock>,
) -> Result<()> {
    if x4 >= mi_cols || y4 >= mi_rows {
        return Ok(());
    }
    let from_w_class = tx_size_class(from_w) as u8;
    let from_h_class = tx_size_class(from_h) as u8;
    let max_class = from_w_class.max(from_h_class);
    let is_4x4 = from_w == 4 && from_h == 4;
    let is_split = if depth < 2 && !is_4x4 {
        let cat = 2 * (4 - max_class) - depth;
        let (a, l) = tile_ctx.var_tx_context(x4, y4, from_w_class, from_h_class);
        decoder.read_txfm_split(cat, a + l)?
    } else {
        false
    };

    if is_split && max_class > 1 {
        let (sub_w, sub_h) = match from_w.cmp(&from_h) {
            std::cmp::Ordering::Greater => (from_w / 2, from_h),
            std::cmp::Ordering::Less => (from_w, from_h / 2),
            std::cmp::Ordering::Equal => (from_w / 2, from_h / 2),
        };
        let (half_w4, half_h4) = (sub_w / 4, sub_h / 4);
        read_var_tx_size(
            decoder,
            tile_ctx,
            x4,
            y4,
            sub_w,
            sub_h,
            depth + 1,
            mi_rows,
            mi_cols,
            out,
        )?;
        if from_w >= from_h {
            read_var_tx_size(
                decoder,
                tile_ctx,
                x4 + half_w4,
                y4,
                sub_w,
                sub_h,
                depth + 1,
                mi_rows,
                mi_cols,
                out,
            )?;
        }
        if from_h >= from_w {
            read_var_tx_size(
                decoder,
                tile_ctx,
                x4,
                y4 + half_h4,
                sub_w,
                sub_h,
                depth + 1,
                mi_rows,
                mi_cols,
                out,
            )?;
            if from_w >= from_h {
                read_var_tx_size(
                    decoder,
                    tile_ctx,
                    x4 + half_w4,
                    y4 + half_h4,
                    sub_w,
                    sub_h,
                    depth + 1,
                    mi_rows,
                    mi_cols,
                    out,
                )?;
            }
        }
        return Ok(());
    }

    let (leaf_w, leaf_h) = if is_split { (4, 4) } else { (from_w, from_h) };
    let (leaf_w4, leaf_h4) = (leaf_w / 4, leaf_h / 4);
    let (w4, h4) = (from_w / 4, from_h / 4);
    let (leaf_w_class, leaf_h_class) = (tx_size_class(leaf_w) as u8, tx_size_class(leaf_h) as u8);
    let mut ly = y4;
    while ly < y4 + h4 {
        let mut lx = x4;
        while lx < x4 + w4 {
            out.push(TxBlock {
                x4: lx,
                y4: ly,
                width_px: leaf_w,
                height_px: leaf_h,
            });
            tile_ctx.set_var_tx_class(lx, ly, leaf_w4, leaf_h4, leaf_w_class, leaf_h_class);
            lx += leaf_w4;
        }
        ly += leaf_h4;
    }
    Ok(())
}

/// Read one explicit MV delta (horizontal + vertical component) from the bitstream, per AV1 spec
/// 5.11.32 `read_mv(ref)`. Used for every `MvKind::New` reference-list slot -- single-ref
/// `NewMv`'s L0, and compound modes' L0 and/or L1 (spec 5.11.26 `assign_mv()`).
///
/// The `mv_joint` symbol gates which axis actually has a coded component -- an axis mv_joint
/// marks "zero" is NOT read from the bitstream at all (it's implicitly 0), it doesn't just
/// happen to decode to a small value. The previous implementation unconditionally read both
/// components for every MV, which desynced the shared `SymbolDecoder` against any real
/// bitstream whenever mv_joint indicated a zero axis -- the same "syntax element not read at
/// all" pattern as this session's earlier residual()/ref_frame() bugs, just not crash-visible
/// here since a `SymbolDecoder` never panics on merely-wrong-but-in-range values.
fn read_explicit_mv(decoder: &mut SymbolDecoder) -> Result<MotionVector> {
    let joint = decoder.read_mv_joint()?;
    // MV_JOINT_HZVNZ(2)/MV_JOINT_HNZVNZ(3): vertical component is non-zero, read it (spec reads
    // diffMv[0], the row/vertical component, first).
    let mv_y = if matches!(joint, 2 | 3) {
        decoder.read_mv_component()?
    } else {
        0
    };
    // MV_JOINT_HNZVZ(1)/MV_JOINT_HNZVNZ(3): horizontal component is non-zero, read it.
    let mv_x = if matches!(joint, 1 | 3) {
        decoder.read_mv_component()?
    } else {
        0
    };
    Ok(MotionVector::new(mv_x, mv_y))
}

/// Convert compound_mode() symbol (spec 5.11.24) to PredictionMode. Symbol ordering matches
/// libaom's `COMPOUND_TYPES`/`compound_mode` enum (`NEAREST_NEARESTMV`=0 .. `NEW_NEWMV`=7).
fn compound_mode_from_symbol(symbol: u8) -> Result<PredictionMode> {
    match symbol {
        0 => Ok(PredictionMode::NearestNearestMv),
        1 => Ok(PredictionMode::NearNearMv),
        2 => Ok(PredictionMode::NearestNewMv),
        3 => Ok(PredictionMode::NewNearestMv),
        4 => Ok(PredictionMode::NearNewMv),
        5 => Ok(PredictionMode::NewNearMv),
        6 => Ok(PredictionMode::GlobalGlobalMv),
        7 => Ok(PredictionMode::NewNewMv),
        _ => Err(BitvueError::InvalidData(format!(
            "Invalid compound mode symbol: {}",
            symbol
        ))),
    }
}

/// Convert INTRA mode symbol to PredictionMode
fn intra_mode_from_symbol(symbol: u8) -> Result<PredictionMode> {
    match symbol {
        0 => Ok(PredictionMode::DcPred),
        1 => Ok(PredictionMode::VPred),
        2 => Ok(PredictionMode::HPred),
        3 => Ok(PredictionMode::D45Pred),
        4 => Ok(PredictionMode::D135Pred),
        5 => Ok(PredictionMode::D113Pred),
        6 => Ok(PredictionMode::D157Pred),
        7 => Ok(PredictionMode::D203Pred),
        8 => Ok(PredictionMode::D67Pred),
        9 => Ok(PredictionMode::SmoothPred),
        10 => Ok(PredictionMode::SmoothVPred),
        11 => Ok(PredictionMode::SmoothHPred),
        12 => Ok(PredictionMode::PaethPred),
        _ => Err(BitvueError::InvalidData(format!(
            "Invalid INTRA mode symbol: {}",
            symbol
        ))),
    }
}

/// Convert INTER mode symbol to PredictionMode
fn inter_mode_from_symbol(symbol: u8) -> Result<PredictionMode> {
    match symbol {
        0 => Ok(PredictionMode::NewMv),
        1 => Ok(PredictionMode::NearestMv),
        2 => Ok(PredictionMode::NearMv),
        3 => Ok(PredictionMode::GlobalMv),
        _ => Err(BitvueError::InvalidData(format!(
            "Invalid INTER mode symbol: {}",
            symbol
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prediction_mode_is_intra() {
        assert!(PredictionMode::DcPred.is_intra());
        assert!(PredictionMode::VPred.is_intra());
        assert!(!PredictionMode::NewMv.is_intra());
    }

    #[test]
    fn test_prediction_mode_is_inter() {
        assert!(PredictionMode::NewMv.is_inter());
        assert!(PredictionMode::NearestMv.is_inter());
        assert!(!PredictionMode::DcPred.is_inter());
    }

    #[test]
    fn test_prediction_mode_needs_mv() {
        assert!(PredictionMode::NewMv.needs_mv());
        assert!(!PredictionMode::NearestMv.needs_mv()); // Uses neighbor MV
        assert!(!PredictionMode::DcPred.needs_mv());
        assert!(PredictionMode::NewNewMv.needs_mv());
        assert!(PredictionMode::NearestNewMv.needs_mv()); // L1 is New
        assert!(PredictionMode::NewNearestMv.needs_mv()); // L0 is New
        assert!(!PredictionMode::NearestNearestMv.needs_mv()); // no New component
        assert!(!PredictionMode::GlobalGlobalMv.needs_mv());
    }

    #[test]
    fn test_compound_mode_from_symbol_round_trips_all_8() {
        let expected = [
            PredictionMode::NearestNearestMv,
            PredictionMode::NearNearMv,
            PredictionMode::NearestNewMv,
            PredictionMode::NewNearestMv,
            PredictionMode::NearNewMv,
            PredictionMode::NewNearMv,
            PredictionMode::GlobalGlobalMv,
            PredictionMode::NewNewMv,
        ];
        for (symbol, mode) in expected.iter().enumerate() {
            assert_eq!(compound_mode_from_symbol(symbol as u8).unwrap(), *mode);
        }
        assert!(compound_mode_from_symbol(8).is_err());
    }

    #[test]
    fn test_compound_mode_l0_l1_mv_kind() {
        // L0=nearest, L1=new
        assert_eq!(
            PredictionMode::NearestNewMv.l0_mv_kind(),
            Some(MvKind::Nearest)
        );
        assert_eq!(PredictionMode::NearestNewMv.l1_mv_kind(), Some(MvKind::New));
        // L0=new, L1=near
        assert_eq!(PredictionMode::NewNearMv.l0_mv_kind(), Some(MvKind::New));
        assert_eq!(PredictionMode::NewNearMv.l1_mv_kind(), Some(MvKind::Near));
        // Single-ref modes have no L1
        assert_eq!(PredictionMode::NewMv.l1_mv_kind(), None);
        assert_eq!(PredictionMode::NewMv.l0_mv_kind(), Some(MvKind::New));
        // INTRA modes have neither
        assert_eq!(PredictionMode::DcPred.l0_mv_kind(), None);
        assert_eq!(PredictionMode::DcPred.l1_mv_kind(), None);
    }

    #[test]
    fn test_prediction_mode_is_compound() {
        assert!(PredictionMode::NewNewMv.is_compound());
        assert!(PredictionMode::GlobalGlobalMv.is_compound());
        assert!(!PredictionMode::NewMv.is_compound());
        assert!(!PredictionMode::DcPred.is_compound());
    }

    #[test]
    fn test_ref_frame_from_u8() {
        assert_eq!(RefFrame::from_u8(0), Some(RefFrame::Intra));
        assert_eq!(RefFrame::from_u8(1), Some(RefFrame::Last));
        assert_eq!(RefFrame::from_u8(7), Some(RefFrame::AltRef));
        assert_eq!(RefFrame::from_u8(8), None);
    }

    #[test]
    fn test_motion_vector_zero() {
        let mv = MotionVector::zero();
        assert_eq!(mv.x, 0);
        assert_eq!(mv.y, 0);
    }

    #[test]
    #[allow(unused_imports)]
    fn test_motion_vector_quarter_pel_accessors() {
        use crate::QuarterPel;

        let mv = MotionVector::new(8, 12);
        assert_eq!(mv.x_quarter_pel().qpel(), 8);
        assert_eq!(mv.y_quarter_pel().qpel(), 12);
    }

    #[test]
    fn test_motion_vector_from_quarter_pel() {
        use crate::QuarterPel;

        let x = QuarterPel::from_pel(2); // 8 quarter-pels
        let y = QuarterPel::from_pel(3); // 12 quarter-pels
        let mv = MotionVector::from_quarter_pel(x, y);

        assert_eq!(mv.x, 8);
        assert_eq!(mv.y, 12);
    }

    #[test]
    fn test_motion_vector_pel_accessors() {
        let mv = MotionVector::new(16, -8);
        assert_eq!(mv.x_pel(), 4); // 16/4 = 4 pixels
        assert_eq!(mv.y_pel(), -2); // -8/4 = -2 pixels
    }

    #[test]
    fn test_motion_vector_arithmetic() {
        let mv1 = MotionVector::new(10, 5);
        let mv2 = MotionVector::new(3, 2);

        let sum = mv1.add(mv2);
        assert_eq!(sum.x, 13);
        assert_eq!(sum.y, 7);

        let diff = mv1.sub(mv2);
        assert_eq!(diff.x, 7);
        assert_eq!(diff.y, 3);
    }

    #[test]
    fn test_motion_vector_magnitude() {
        let mv = MotionVector::new(8, 4);
        assert_eq!(mv.magnitude_qpel(), 12); // |8| + |4|
        assert_eq!(mv.magnitude_pel(), 3); // 12/4 = 3
    }

    #[test]
    fn test_motion_vector_magnitude_rounding() {
        let mv = MotionVector::new(7, 7);
        assert_eq!(mv.magnitude_qpel(), 14); // |7| + |7|
        assert_eq!(mv.magnitude_pel(), 3); // 14/4 = 3.5 -> 3 (rounded down)
    }

    #[test]
    fn test_coding_unit_new() {
        let cu = CodingUnit::new(0, 0, 16, 16);
        assert_eq!(cu.x, 0);
        assert_eq!(cu.y, 0);
        assert_eq!(cu.width, 16);
        assert_eq!(cu.height, 16);
        assert!(!cu.skip);
        assert!(cu.is_intra());
        assert!(!cu.is_inter());
    }
}
