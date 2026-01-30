//! Motion Vector Prediction
//!
//! Per AV1 Specification Section 5.10 (Motion Vector Prediction Process)
//!
//! This module implements MV predictor calculation for:
//! - NEARESTMV: Use nearest available MV from neighboring blocks
//! - NEARMV: Use near MV from candidate list
//! - GLOBALMV: Use global motion parameters
//! - NEWMV: Use explicit MV from bitstream (with predictor added)

use crate::tile::coding_unit::{CodingUnit, MotionVector, PredictionMode, RefFrame};

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
                    // Check if this CU overlaps with the neighbor position
                    cu.x < nx + 8
                        && cu.x + cu.width > *nx
                        && cu.y < ny + 8
                        && cu.y + cu.height > *ny
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
    /// For MVP, we use zero as global motion predictor
    /// Full implementation would parse global motion parameters from frame header
    pub fn predict_global_mv(&self) -> MotionVector {
        // TODO: Parse actual global motion parameters from frame header
        // Global motion is specified per reference frame (LAST, GOLDEN, ALTREF)
        // For MVP, we use zero
        MotionVector::zero()
    }

    /// Get MV predictor for the given mode
    ///
    /// Returns the appropriate MV predictor based on prediction mode
    pub fn get_mv_predictor(
        &self,
        mode: PredictionMode,
        x: u32,
        y: u32,
        ref_frame: RefFrame,
    ) -> MotionVector {
        match mode {
            PredictionMode::NearestMv => self.predict_nearest_mv(x, y, ref_frame),
            PredictionMode::NearMv => self.predict_near_mv(x, y, ref_frame),
            PredictionMode::GlobalMv => self.predict_global_mv(),
            PredictionMode::NewMv => {
                // For NEWMV, we still need a predictor to add to the explicit MV
                // The predictor is typically the nearest MV
                self.predict_nearest_mv(x, y, ref_frame)
            }
            _ => MotionVector::zero(),
        }
    }

    /// Check if a position has been parsed
    pub fn is_position_parsed(&self, x: u32, y: u32) -> bool {
        self.parsed_cus.iter().any(|cu| cu.x == x && cu.y == y)
    }
}

/// Apply MV predictor to explicit MV
///
/// For NEWMV mode, the predictor is added to the explicitly coded MV
pub fn apply_mv_predictor(mv: MotionVector, predictor: MotionVector) -> MotionVector {
    MotionVector::new(mv.x + predictor.x, mv.y + predictor.y)
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
        ctx.add_cu(&neighbor);  // Now takes reference instead of ownership

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
}
