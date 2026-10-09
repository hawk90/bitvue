//! Coding-unit data model: prediction modes, reference frames, motion vectors, transform sizes,
//! the `CodingUnit` record, and the frame-header flag bundles `parse_coding_unit` takes.

use super::palette::PaletteInfo;
use crate::symbol::ResidualBlockStats;
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

/// Frame-level flags gating the real `motion_mode`/`interintra`/`compound_type`(wedge/seg)/
/// `filter` reads (spec 5.11.27-30) -- see `SymbolDecoder::read_motion_mode`/`read_interintra`/
/// `read_mask_comp`/`read_filter`'s docs for what each gates. Bundled the same way
/// `TxTypeFrameFlags` bundles its own frame-level gates, to avoid a further parameter-list
/// explosion on `parse_coding_unit`.
#[derive(Debug, Clone, Copy)]
pub struct InterModeFlags {
    pub switchable_motion_mode: bool,
    pub allow_warped_motion: bool,
    pub enable_interintra_compound: bool,
    /// Sequence header's `enable_dual_filter`: whether `interp_filter` codes one filter per axis
    /// (two symbols) or a single one shared by both.
    pub enable_dual_filter: bool,
    pub enable_masked_compound: bool,
    pub enable_jnt_comp: bool,
    pub subpel_filter_switchable: bool,
    /// `force_integer_mv`/`gm_type` (spec 5.9.2/5.9.24) -- gate `read_motion_mode`'s real
    /// `GmType[RefFrame[0]] > TRANSLATION` exclusion for `GLOBALMV`/`GLOBAL_GLOBALMV` blocks (see
    /// the call site's doc). `gm_type` indexed by `RefFrame as usize` (0=Intra unused).
    pub force_integer_mv: bool,
    /// `allow_high_precision_mv` (spec 5.9.2): with `force_integer_mv`, decides how many
    /// fractional MV bits are coded -- see [`InterModeFlags::mv_precision`].
    pub allow_high_precision_mv: bool,
    pub gm_type: [u8; 8],
}

impl InterModeFlags {
    /// dav1d's `mv_prec` (`hp - force_integer_mv`): `-1` integer MVs, `0` quarter-sample, `1`
    /// eighth-sample.
    pub fn mv_precision(&self) -> i8 {
        i8::from(self.allow_high_precision_mv) - i8::from(self.force_integer_mv)
    }
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

/// Motion vector in AV1's native 1/8-sample units (spec `Mv`, dav1d `mv.x`/`mv.y`). The overlay
/// grids use quarter-sample units; `overlay_extraction::mv_extractor` converts.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MotionVector {
    /// Horizontal component (1/8-sample units)
    pub x: i32,
    /// Vertical component (1/8-sample units)
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
