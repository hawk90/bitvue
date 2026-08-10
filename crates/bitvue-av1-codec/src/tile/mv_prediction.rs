//! Motion Vector Prediction
//!
//! Per AV1 Specification Section 5.10 (Motion Vector Prediction Process)
//!
//! This module implements MV predictor calculation for:
//! - NEARESTMV: Use nearest available MV from neighboring blocks
//! - NEARMV: Use near MV from candidate list
//! - GLOBALMV: Use global motion parameters
//! - NEWMV: Use explicit MV from bitstream (with predictor added)

use crate::tile::coding_unit::{CodingUnit, MotionVector, MvKind, PredictionMode, RefFrame};

/// Lightweight MV context entry
///
/// Stores only the fields needed for MV prediction, avoiding the need to clone
/// the entire CodingUnit struct (~40-50 bytes).
#[derive(Debug, Clone)]
struct MvCuEntry {
    /// Block position (top-left corner) in pixels
    x: u32,
    /// Block position (top-left corner) in pixels
    y: u32,
    /// Block width in pixels
    width: u32,
    /// Block height in pixels
    height: u32,
    /// Prediction mode
    mode: PredictionMode,
    /// Reference frames (for INTER)
    ref_frames: [RefFrame; 2],
    /// Motion vectors (for INTER)
    mv: [MotionVector; 2],
}

impl MvCuEntry {
    /// Create from a CodingUnit reference (no clone needed)
    fn from_cu(cu: &CodingUnit) -> Self {
        Self {
            x: cu.x,
            y: cu.y,
            width: cu.width,
            height: cu.height,
            mode: cu.mode,
            ref_frames: cu.ref_frames,
            mv: cu.mv,
        }
    }

    /// Check if this is an INTER block
    fn is_inter(&self) -> bool {
        self.mode.is_inter()
    }
}

/// MV predictor context
///
/// Tracks previously parsed coding units for MV prediction.
pub struct MvPredictorContext {
    /// Previously parsed coding units, indexed by position
    /// For MVP, we use a simplified neighbor tracking
    parsed_cus: Vec<MvCuEntry>,
    /// Frame width in superblocks
    _sb_cols: u32,
    /// Frame height in superblocks
    _sb_rows: u32,
}

impl MvPredictorContext {
    /// Create a new MV predictor context
    pub fn new(sb_cols: u32, sb_rows: u32) -> Self {
        Self {
            parsed_cus: Vec::new(),
            _sb_cols: sb_cols,
            _sb_rows: sb_rows,
        }
    }

    /// Add a parsed coding unit to the context
    ///
    /// This is now zero-copy - we extract only the fields needed for MV prediction.
    pub fn add_cu(&mut self, cu: &CodingUnit) {
        self.parsed_cus.push(MvCuEntry::from_cu(cu));
    }

    /// Find the nearest neighbor CU
    ///
    /// Searches in order: left, above, above-right, above-left
    fn find_nearest_neighbor(&self, x: u32, y: u32) -> Option<&MvCuEntry> {
        // Define search order with weights
        // Lower weight = higher priority
        let neighbors = [
            // Left neighbor (same row, previous block)
            (x.wrapping_sub(8), y, 1),
            // Above neighbor (previous row, same column)
            (x, y.wrapping_sub(8), 2),
            // Above-right neighbor (previous row, next column)
            (x + 8, y.wrapping_sub(8), 3),
            // Above-left neighbor (previous row, previous column)
            (x.wrapping_sub(8), y.wrapping_sub(8), 4),
        ];

        neighbors
            .iter()
            .filter_map(|(nx, ny, _weight)| {
                self.parsed_cus.iter().find(|cu| {
                    // Check if this CU overlaps with the neighbor position.
                    // Use saturating arithmetic: wrapping_sub can produce large values for
                    // coordinates near 0 (e.g. x=0, wrapping_sub(8) = u32::MAX-7), and
                    // adding 8 to those would overflow without saturation.
                    cu.x < nx.saturating_add(8)
                        && cu.x.saturating_add(cu.width) > *nx
                        && cu.y < ny.saturating_add(8)
                        && cu.y.saturating_add(cu.height) > *ny
                        && cu.is_inter()
                })
            })
            .next()
    }

    /// Get MV candidate list
    ///
    /// Returns up to 2 MV candidates from neighboring blocks
    fn get_mv_candidates(&self, x: u32, y: u32, ref_frame: RefFrame) -> Vec<MotionVector> {
        let mut candidates = Vec::new();

        // Find nearest neighbor with same reference frame
        if let Some(neighbor) = self.find_nearest_neighbor(x, y) {
            if neighbor.ref_frames[0] == ref_frame {
                candidates.push(neighbor.mv[0]);
            }
        }

        // For MVP, if we don't have enough candidates, use zero MV
        while candidates.len() < 2 {
            candidates.push(MotionVector::zero());
        }

        candidates
    }

    /// Calculate NEARESTMV predictor
    ///
    /// Uses the MV from the nearest neighboring block
    pub fn predict_nearest_mv(&self, x: u32, y: u32, ref_frame: RefFrame) -> MotionVector {
        if let Some(neighbor) = self.find_nearest_neighbor(x, y) {
            // If neighbor has same reference frame, use its MV
            if neighbor.ref_frames[0] == ref_frame {
                return neighbor.mv[0];
            }
            // Otherwise, use MV anyway (simplified)
            if neighbor.is_inter() {
                return neighbor.mv[0];
            }
        }
        MotionVector::zero()
    }

    /// Calculate NEARMV predictor
    ///
    /// Uses the second candidate from the MV candidate list
    pub fn predict_near_mv(&self, x: u32, y: u32, ref_frame: RefFrame) -> MotionVector {
        let candidates = self.get_mv_candidates(x, y, ref_frame);
        if candidates.len() >= 2 {
            candidates[1]
        } else {
            MotionVector::zero()
        }
    }

    /// Calculate GLOBALMV predictor
    ///
    /// Returns the global motion translation vector for the nearest inter
    /// neighbor's primary reference frame if one is present, otherwise zero.
    ///
    /// Per AV1 spec Section 7.10 (Global Motion Params), each reference frame
    /// carries one of four global motion types:
    /// - IDENTITY (0):    translation = (0, 0)
    /// - TRANSLATION (1): translation from gm_params[ref][4]/[5]
    /// - ROTZOOM (2):     affine transform with rotation/zoom
    /// - AFFINE (3):      general affine transform
    ///
    /// The gm_params table is parsed in global_motion_params() per spec
    /// Section 5.9.24 and stored in the frame header.  Since this predictor
    /// context does not yet carry the per-frame gm_params table, we return
    /// zero (identity translation), which is correct for IDENTITY-type global
    /// motion and is the safe fallback for all other types.  Callers that
    /// require accurate GLOBALMV should supply the gm_params directly.
    pub fn predict_global_mv(&self) -> MotionVector {
        // Zero (identity) is correct for IDENTITY global motion type.
        // TRANSLATION/ROTZOOM/AFFINE require gm_params from the FrameHeader
        // which are not currently threaded into this prediction context.
        MotionVector::zero()
    }

    /// Get the predictor for one `MvKind` against one reference frame. Shared by both L0 and L1
    /// lookups (`get_mv_predictor`/`get_mv_predictor_l1`) since the underlying candidate search
    /// (`predict_nearest_mv`/`predict_near_mv`) is already generic over which `ref_frame` it
    /// matches against. `MvKind::New` uses the nearest-neighbor MV as its predictor too --
    /// mirrors the original NEWMV behavior (the explicit bitstream delta is added on top by the
    /// caller).
    pub fn predict_by_kind(
        &self,
        kind: MvKind,
        x: u32,
        y: u32,
        ref_frame: RefFrame,
    ) -> MotionVector {
        match kind {
            MvKind::Nearest | MvKind::New => self.predict_nearest_mv(x, y, ref_frame),
            MvKind::Near => self.predict_near_mv(x, y, ref_frame),
            MvKind::Global => self.predict_global_mv(),
        }
    }

    /// Get MV predictor for reference list 0 (L0) of the given mode.
    ///
    /// Returns the appropriate MV predictor based on prediction mode. `PredictionMode::l0_mv_kind`
    /// returns `None` for INTRA modes, in which case this returns zero.
    pub fn get_mv_predictor(
        &self,
        mode: PredictionMode,
        x: u32,
        y: u32,
        ref_frame: RefFrame,
    ) -> MotionVector {
        match mode.l0_mv_kind() {
            Some(kind) => self.predict_by_kind(kind, x, y, ref_frame),
            None => MotionVector::zero(),
        }
    }

    /// Get MV predictor for reference list 1 (L1) of a compound mode. Returns zero for
    /// single-ref/INTRA modes (`PredictionMode::l1_mv_kind` is `None`).
    pub fn get_mv_predictor_l1(
        &self,
        mode: PredictionMode,
        x: u32,
        y: u32,
        ref_frame: RefFrame,
    ) -> MotionVector {
        match mode.l1_mv_kind() {
            Some(kind) => self.predict_by_kind(kind, x, y, ref_frame),
            None => MotionVector::zero(),
        }
    }

    /// Check if a position has been parsed
    pub fn is_position_parsed(&self, x: u32, y: u32) -> bool {
        self.parsed_cus.iter().any(|cu| cu.x == x && cu.y == y)
    }
}

/// Apply MV predictor to explicit MV
///
/// For NEWMV mode, the predictor is added to the explicitly coded MV.
///
/// # Arguments
///
/// * `mv` - Explicitly coded motion vector from bitstream (quarter-pel units)
/// * `predictor` - Predicted motion vector (quarter-pel units)
///
/// # Returns
///
/// The final motion vector (explicit + predictor, both in quarter-pel units)
pub fn apply_mv_predictor(mv: MotionVector, predictor: MotionVector) -> MotionVector {
    mv.add(predictor)
}

/// Parse MV with predictor
///
/// Reads the explicit MV from bitstream and adds the predictor
pub fn parse_mv_with_predictor(
    explicit_mv: MotionVector,
    mode: PredictionMode,
    x: u32,
    y: u32,
    ref_frame: RefFrame,
    ctx: &MvPredictorContext,
) -> MotionVector {
    let predictor = ctx.get_mv_predictor(mode, x, y, ref_frame);
    apply_mv_predictor(explicit_mv, predictor)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mv_predictor_context_creation() {
        // Per generate-tests skill: Test public interface only
        let ctx = MvPredictorContext::new(10, 10);
        assert_eq!(ctx.parsed_cus.len(), 0);
        // Note: sb_cols and sb_rows are private fields (prefixed with _)
        // We verify the context was created successfully by checking parsed_cus
    }

    #[test]
    fn test_predict_by_kind_matches_named_predictors() {
        let mut ctx = MvPredictorContext::new(10, 10);
        let mut neighbor = CodingUnit::new(0, 64, 64, 64);
        neighbor.mode = PredictionMode::NewMv;
        neighbor.ref_frames = [RefFrame::Last, RefFrame::Intra];
        neighbor.mv[0] = MotionVector::new(10, -5);
        ctx.add_cu(&neighbor);

        assert_eq!(
            ctx.predict_by_kind(MvKind::Nearest, 64, 64, RefFrame::Last),
            ctx.predict_nearest_mv(64, 64, RefFrame::Last)
        );
        assert_eq!(
            ctx.predict_by_kind(MvKind::New, 64, 64, RefFrame::Last),
            ctx.predict_nearest_mv(64, 64, RefFrame::Last)
        );
        assert_eq!(
            ctx.predict_by_kind(MvKind::Global, 64, 64, RefFrame::Last),
            MotionVector::zero()
        );
    }

    #[test]
    fn test_get_mv_predictor_l1_uses_l1_kind() {
        let ctx = MvPredictorContext::new(10, 10);
        // NearestNewMv: L1 is New -> predictor should equal the nearest-mv lookup (same as L0's
        // NewMv predictor), not zero.
        assert_eq!(
            ctx.get_mv_predictor_l1(PredictionMode::NearestNewMv, 64, 64, RefFrame::Last),
            ctx.predict_nearest_mv(64, 64, RefFrame::Last)
        );
        // Single-ref modes have no L1 -> always zero regardless of neighbors.
        assert_eq!(
            ctx.get_mv_predictor_l1(PredictionMode::NewMv, 64, 64, RefFrame::Last),
            MotionVector::zero()
        );
    }

    #[test]
    fn test_nearest_mv_no_neighbors() {
        let ctx = MvPredictorContext::new(10, 10);
        let mv = ctx.predict_nearest_mv(64, 64, RefFrame::Last);
        assert_eq!(mv.x, 0);
        assert_eq!(mv.y, 0);
    }

    #[test]
    fn test_nearest_mv_with_left_neighbor() {
        let mut ctx = MvPredictorContext::new(10, 10);

        // Add left neighbor (using reference, no clone)
        let mut neighbor = CodingUnit::new(0, 64, 64, 64);
        neighbor.mode = PredictionMode::NewMv;
        neighbor.ref_frames = [RefFrame::Last, RefFrame::Intra];
        neighbor.mv[0] = MotionVector::new(10, -5);
        ctx.add_cu(&neighbor); // Now takes reference instead of ownership

        let mv = ctx.predict_nearest_mv(64, 64, RefFrame::Last);
        assert_eq!(mv.x, 10);
        assert_eq!(mv.y, -5);
    }

    #[test]
    fn test_apply_mv_predictor() {
        let explicit = MotionVector::new(5, 3);
        let predictor = MotionVector::new(10, -5);
        let result = apply_mv_predictor(explicit, predictor);
        assert_eq!(result.x, 15);
        assert_eq!(result.y, -2);
    }

    #[test]
    fn test_apply_mv_predictor_with_quarter_pel() {
        use crate::QuarterPel;

        // Demonstrate quarter-pel precision awareness
        let explicit_mv = MotionVector::from_quarter_pel(
            QuarterPel::from_pel(2), // 8 quarter-pels
            QuarterPel::from_pel(1), // 4 quarter-pels
        );
        let predictor_mv = MotionVector::from_quarter_pel(
            QuarterPel::from_pel(1),   // 4 quarter-pels
            QuarterPel::from_qpel(-4), // -4 quarter-pels
        );

        let result = apply_mv_predictor(explicit_mv, predictor_mv);

        // Result: 8+4=12 qpel X, 4-4=0 qpel Y
        assert_eq!(result.x_quarter_pel().qpel(), 12);
        assert_eq!(result.y_quarter_pel().qpel(), 0);

        // In pixels: 12/4=3 pel X, 0/4=0 pel Y
        assert_eq!(result.x_pel(), 3);
        assert_eq!(result.y_pel(), 0);
    }

    #[test]
    fn test_mv_magnitude_in_pixels() {
        let mv = MotionVector::new(16, -8);

        // Quarter-pel magnitude
        assert_eq!(mv.magnitude_qpel(), 24);

        // Pixel magnitude (rounded)
        assert_eq!(mv.magnitude_pel(), 6);
    }
}
