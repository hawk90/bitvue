//! Intra and inter prediction modes and the motion-vector kind each inter direction uses.

use serde::{Deserialize, Serialize};

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
