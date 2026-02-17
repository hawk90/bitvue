#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unused_mut)]
#![allow(unused_comparisons)]
#![allow(unused_doc_comments)]
//! Tests for disable_reason module

use bitvue_core::{DisableReason, DisableReasonMatrix, FeatureId};

#[test]
fn test_feature_enabled_by_default() {
    let matrix = DisableReasonMatrix::new();
    assert!(matrix.is_enabled(&FeatureId::TimelinePanel));
    assert!(matrix.is_enabled(&FeatureId::PlayerPanel));
}

#[test]
fn test_disable_feature() {
    let mut matrix = DisableReasonMatrix::new();

    matrix.disable(
        FeatureId::PlayerOverlayMv,
        DisableReason::MissingDependency("Motion vector data not available".to_string()),
    );

    assert!(matrix.is_disabled(&FeatureId::PlayerOverlayMv));
    assert!(!matrix.is_enabled(&FeatureId::PlayerOverlayMv));

    let reason = matrix
        .get_disable_reason(&FeatureId::PlayerOverlayMv)
        .unwrap();
    assert_eq!(reason.label(), "Missing Dependency");
}

#[test]
fn test_enable_feature() {
    let mut matrix = DisableReasonMatrix::new();

    // Disable then enable
    matrix.disable(
        FeatureId::MetricsVmaf,
        DisableReason::UnsupportedCodecFeature("VMAF not supported".to_string()),
    );
    assert!(matrix.is_disabled(&FeatureId::MetricsVmaf));

    matrix.enable(FeatureId::MetricsVmaf);
    assert!(matrix.is_enabled(&FeatureId::MetricsVmaf));
}

#[test]
fn test_disabled_features_list() {
    let mut matrix = DisableReasonMatrix::new();

    matrix.disable(
        FeatureId::ComparePanel,
        DisableReason::InsufficientData("Need at least 2 streams".to_string()),
    );
    matrix.disable(
        FeatureId::MetricsPsnr,
        DisableReason::MissingDependency("Reference not loaded".to_string()),
    );

    let disabled = matrix.disabled_features();
    assert_eq!(disabled.len(), 2);
    assert_eq!(matrix.disabled_count(), 2);
}

#[test]
fn test_reason_descriptions() {
    let reasons = vec![
        DisableReason::MissingDependency("libdav1d".to_string()),
        DisableReason::InvalidStream("No frames loaded".to_string()),
        DisableReason::UnsupportedCodecFeature("Film grain".to_string()),
    ];

    for reason in &reasons {
        let desc = reason.description();
        assert!(!desc.is_empty());
        // Description should contain the specific detail from the variant
        match reason {
            DisableReason::MissingDependency(dep) => assert!(desc.contains(dep)),
            DisableReason::InvalidStream(r) => assert!(desc.contains(r)),
            DisableReason::UnsupportedCodecFeature(f) => assert!(desc.contains(f)),
            _ => {}
        }
    }
}
