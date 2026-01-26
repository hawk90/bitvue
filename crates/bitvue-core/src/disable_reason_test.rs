// Disable Reason Matrix module tests
use super::*;

// ============================================================================
// Fixtures
// ============================================================================
fn create_test_reason() -> DisableReason {
    DisableReason::MissingDependency("decoder".to_string())
}

fn create_test_feature_id() -> FeatureId {
    FeatureId::TimelinePanel
}

fn create_test_matrix() -> DisableReasonMatrix {
    DisableReasonMatrix::new()
}

// ============================================================================
// DisableReason Tests
// ============================================================================
#[cfg(test)]
mod disable_reason_tests {
    use super::*;

    #[test]
    fn test_missing_dependency_description() {
        let reason = DisableReason::MissingDependency("libavcodec".to_string());
        assert!(reason.description().contains("Missing dependency"));
        assert!(reason.description().contains("libavcodec"));
    }

    #[test]
    fn test_invalid_stream_description() {
        let reason = DisableReason::InvalidStream("no frames".to_string());
        assert!(reason.description().contains("Invalid stream"));
    }

    #[test]
    fn test_label() {
        assert_eq!(DisableReason::MissingDependency("x".to_string()).label(), "Missing Dependency");
        assert_eq!(DisableReason::InvalidStream("x".to_string()).label(), "Invalid Stream");
    }
}

// ============================================================================
// FeatureId Tests
// ============================================================================
#[cfg(test)]
mod feature_id_tests {
    use super::*;

    #[test]
    fn test_feature_id_names() {
        assert_eq!(FeatureId::TimelinePanel.name(), "Timeline Panel");
        assert_eq!(FeatureId::PlayerOverlayQp.name(), "QP Heatmap Overlay");
        assert_eq!(FeatureId::Custom("Custom Feature".to_string()).name(), "Custom Feature");
    }
}

// ============================================================================
// DisableReasonMatrix Tests
// ============================================================================
#[cfg(test)]
mod matrix_tests {
    use super::*;

    #[test]
    fn test_new_all_enabled() {
        let matrix = create_test_matrix();
        assert!(matrix.is_enabled(&FeatureId::TimelinePanel));
        assert!(matrix.is_enabled(&FeatureId::PlayerPanel));
    }

    #[test]
    fn test_disable_feature() {
        let mut matrix = create_test_matrix();
        matrix.disable(FeatureId::TimelinePanel, create_test_reason());
        assert!(matrix.is_disabled(&FeatureId::TimelinePanel));
    }

    #[test]
    fn test_enable_feature() {
        let mut matrix = create_test_matrix();
        matrix.disable(FeatureId::TimelinePanel, create_test_reason());
        matrix.enable(FeatureId::TimelinePanel);
        assert!(matrix.is_enabled(&FeatureId::TimelinePanel));
    }

    #[test]
    fn test_disabled_features() {
        let mut matrix = create_test_matrix();
        matrix.disable(FeatureId::TimelinePanel, create_test_reason());
        matrix.disable(FeatureId::PlayerPanel, DisableReason::InvalidStream("x".to_string()));
        assert_eq!(matrix.disabled_features().len(), 2);
    }

    #[test]
    fn test_enabled_features() {
        let mut matrix = create_test_matrix();
        matrix.disable(FeatureId::TimelinePanel, create_test_reason());
        assert_eq!(matrix.enabled_features().len(), 0); // Only TimelinePanel disabled
    }

    #[test]
    fn test_clear() {
        let mut matrix = create_test_matrix();
        matrix.disable(FeatureId::TimelinePanel, create_test_reason());
        matrix.clear();
        assert!(matrix.is_enabled(&FeatureId::TimelinePanel));
    }

    #[test]
    fn test_disabled_count() {
        let mut matrix = create_test_matrix();
        assert_eq!(matrix.disabled_count(), 0);
        matrix.disable(FeatureId::TimelinePanel, create_test_reason());
        matrix.disable(FeatureId::PlayerPanel, create_test_reason());
        assert_eq!(matrix.disabled_count(), 2);
    }

    #[test]
    fn test_enabled_count() {
        let mut matrix = create_test_matrix();
        assert_eq!(matrix.enabled_count(), 0); // No features registered
    }
}
