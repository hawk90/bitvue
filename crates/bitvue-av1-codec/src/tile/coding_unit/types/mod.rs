//! Coding-unit data model: prediction modes, reference frames, motion vectors, transform sizes,
//! the `CodingUnit` record, and the frame-header flag bundles `parse_coding_unit` takes.

mod coding_unit;
mod frame_flags;
mod motion;
mod prediction_mode;
mod tx_size;

pub use coding_unit::{CodingUnit, TxBlock};
pub use frame_flags::{InterModeFlags, TxTypeFrameFlags};
pub use motion::{MotionVector, RefFrame};
pub use prediction_mode::{MvKind, PredictionMode};
pub use tx_size::TxSize;

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
