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

use crate::symbol::SymbolDecoder;
use bitvue_core::{BitvueError, Result};
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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

    /// Check if this mode requires reading motion vectors
    pub fn needs_mv(&self) -> bool {
        matches!(self, PredictionMode::NewMv)
    }
}

/// Reference frame type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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

    /// Motion vectors (for INTER)
    /// L0 = forward reference, L1 = backward reference
    pub mv: [MotionVector; 2],

    /// Transform size (for residual coding)
    pub tx_size: TxSize,

    /// QP value (quantization parameter)
    /// None for blocks that don't have QP (e.g., skip blocks)
    pub qp: Option<i16>,
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
            mv: [MotionVector::zero(), MotionVector::zero()],
            tx_size,
            qp: None,
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
///
/// # Returns
///
/// Parsed coding unit with prediction info, motion vectors (if INTER), and QP value
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
) -> Result<(CodingUnit, i16)> {
    let mut cu = CodingUnit::new(x, y, width, height);

    // Read skip flag
    cu.skip = decoder.read_skip()?;

    // TODO: Read segment ID (if segmentation enabled)

    // Determine if INTRA or INTER
    if is_key_frame {
        // KEY frames are always INTRA
        cu.ref_frames = [RefFrame::Intra, RefFrame::Intra];

        // Read INTRA prediction mode
        let mode_symbol = decoder.read_intra_mode()?;
        cu.mode = intra_mode_from_symbol(mode_symbol)?;
    } else {
        // INTER frame - read prediction mode
        let mode_symbol = decoder.read_inter_mode()?;
        cu.mode = inter_mode_from_symbol(mode_symbol)?;

        // TODO: Read reference frames
        // For MVP, use LAST frame as default
        cu.ref_frames = [RefFrame::Last, RefFrame::Intra];

        // If NEWMV, read motion vectors
        if cu.mode == PredictionMode::NewMv {
            // Read MV for L0 (forward reference)
            let mv_x = decoder.read_mv_component()?;
            let mv_y = decoder.read_mv_component()?;
            let explicit_mv = MotionVector::new(mv_x, mv_y);

            // Get MV predictor and add to explicit MV
            let predictor = mv_ctx.get_mv_predictor(cu.mode, x, y, cu.ref_frames[0]);
            cu.mv[0] = MotionVector::new(explicit_mv.x + predictor.x, explicit_mv.y + predictor.y);

            // TODO: If compound prediction (2 references), read L1 MV
            // For MVP, single reference only
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
    }

    // Add this CU to the MV predictor context for future blocks
    mv_ctx.add_cu(cu.clone());

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

    Ok((cu, new_qp))
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
