//! Tests for insight feed generation

use bitvue_core::diagnostics_bands::{DiagnosticsBands, ErrorBurst, ReorderEntry, SceneChange};
use bitvue_core::frame_identity::FrameMetadata;
use bitvue_core::{FrameIndexMap, InsightFeed, InsightSeverity, InsightType};

fn create_test_frame_map(count: usize) -> FrameIndexMap {
    let frames: Vec<FrameMetadata> = (0..count)
        .map(|i| FrameMetadata {
            pts: Some((i * 1000) as u64),
            dts: Some((i * 1000) as u64),
        })
        .collect();
    FrameIndexMap::new(&frames)
}

#[test]
fn test_empty_insight_feed() {
    let feed = InsightFeed::new();
    assert_eq!(feed.insights.len(), 0);
}

#[test]
fn test_pts_quality_insight() {
    // Create frame map with duplicate PTS (BAD quality)
    let frames = vec![
        FrameMetadata {
            pts: Some(0),
            dts: Some(0),
        },
        FrameMetadata {
            pts: Some(1000),
            dts: Some(1000),
        },
        FrameMetadata {
            pts: Some(1000),
            dts: Some(2000),
        }, // Duplicate
    ];
    let frame_map = FrameIndexMap::new(&frames);
    let diagnostics = DiagnosticsBands::new();

    let feed = InsightFeed::generate(&frame_map, &diagnostics);

    assert!(feed.insights.len() > 0);
    let pts_insight = feed
        .insights
        .iter()
        .find(|i| matches!(i.insight_type, InsightType::PtsQualityBad));
    assert!(pts_insight.is_some());

    let insight = pts_insight.unwrap();
    assert_eq!(insight.severity, InsightSeverity::Error);
    assert_eq!(insight.triggers.len(), 1);
    assert_eq!(insight.triggers[0].signal, "pts_quality");
}

#[test]
fn test_error_burst_insight() {
    let frame_map = create_test_frame_map(100);
    let mut diagnostics = DiagnosticsBands::new();

    // Add error burst
    diagnostics.error_bursts.push(ErrorBurst {
        start_idx: 50,
        end_idx: 60,
        error_count: 10,
        severity: 0.9, // High severity
        error_types: vec!["decode_error".to_string()],
    });

    let feed = InsightFeed::generate(&frame_map, &diagnostics);

    let burst_insight = feed
        .insights
        .iter()
        .find(|i| matches!(i.insight_type, InsightType::ErrorBurst));
    assert!(burst_insight.is_some());

    let insight = burst_insight.unwrap();
    assert_eq!(insight.severity, InsightSeverity::Critical); // High severity
    assert_eq!(insight.frame_range, (50, 60));
    assert!(insight.jump_targets.len() > 0);
}

#[test]
fn test_scene_change_insight() {
    let frame_map = create_test_frame_map(100);
    let mut diagnostics = DiagnosticsBands::new();

    // Add significant scene change
    diagnostics.scene_changes.push(SceneChange {
        display_idx: 25,
        confidence: 0.85, // High confidence
        description: None,
    });

    // Add insignificant scene change (should be filtered)
    diagnostics.scene_changes.push(SceneChange {
        display_idx: 50,
        confidence: 0.3, // Low confidence
        description: None,
    });

    let feed = InsightFeed::generate(&frame_map, &diagnostics);

    let scene_insights: Vec<_> = feed
        .insights
        .iter()
        .filter(|i| matches!(i.insight_type, InsightType::SceneChange))
        .collect();

    assert_eq!(scene_insights.len(), 1); // Only high-score change
    assert_eq!(scene_insights[0].frame_range.0, 25);
    assert_eq!(scene_insights[0].severity, InsightSeverity::Info);
}

#[test]
fn test_reorder_insight() {
    let frame_map = create_test_frame_map(100);
    let mut diagnostics = DiagnosticsBands::new();

    // Add reorder entries
    diagnostics.reorder_entries.push(ReorderEntry {
        display_idx: 10,
        pts: 10000,
        dts: 12000,
        depth: 2000,
    });
    diagnostics.reorder_entries.push(ReorderEntry {
        display_idx: 11,
        pts: 11000,
        dts: 13000,
        depth: 2000,
    });

    let feed = InsightFeed::generate(&frame_map, &diagnostics);

    let reorder_insight = feed
        .insights
        .iter()
        .find(|i| matches!(i.insight_type, InsightType::ReorderDetected));
    assert!(reorder_insight.is_some());

    let insight = reorder_insight.unwrap();
    assert_eq!(insight.severity, InsightSeverity::Warn);
    assert_eq!(insight.frame_range, (10, 11));
}

#[test]
fn test_filter_by_severity() {
    let frame_map = create_test_frame_map(100);
    let mut diagnostics = DiagnosticsBands::new();

    // Add insights of different severities
    diagnostics.error_bursts.push(ErrorBurst {
        start_idx: 10,
        end_idx: 20,
        error_count: 5,
        severity: 0.9, // Critical
        error_types: vec!["decode_error".to_string()],
    });
    diagnostics.scene_changes.push(SceneChange {
        display_idx: 30,
        confidence: 0.8, // Info
        description: None,
    });

    let feed = InsightFeed::generate(&frame_map, &diagnostics);

    // Filter for Error and above
    let high_severity = feed.filter_by_severity(InsightSeverity::Error);
    assert!(high_severity.len() > 0);
    assert!(high_severity
        .iter()
        .all(|i| i.severity.priority() >= InsightSeverity::Error.priority()));
}

#[test]
fn test_insights_in_range() {
    let frame_map = create_test_frame_map(100);
    let mut diagnostics = DiagnosticsBands::new();

    diagnostics.error_bursts.push(ErrorBurst {
        start_idx: 10,
        end_idx: 20,
        error_count: 5,
        severity: 0.5,
        error_types: vec!["decode_error".to_string()],
    });
    diagnostics.error_bursts.push(ErrorBurst {
        start_idx: 50,
        end_idx: 60,
        error_count: 8,
        severity: 0.7,
        error_types: vec!["decode_error".to_string()],
    });

    let feed = InsightFeed::generate(&frame_map, &diagnostics);

    // Get insights in range [15, 55]
    let range_insights = feed.get_insights_in_range(15, 55);
    assert!(range_insights.len() >= 2); // Both bursts overlap
}

#[test]
fn test_severity_counts() {
    let frame_map = create_test_frame_map(100);
    let mut diagnostics = DiagnosticsBands::new();

    // Add various insights
    diagnostics.error_bursts.push(ErrorBurst {
        start_idx: 10,
        end_idx: 20,
        error_count: 10,
        severity: 0.9, // Critical
        error_types: vec!["decode_error".to_string()],
    });
    diagnostics.error_bursts.push(ErrorBurst {
        start_idx: 30,
        end_idx: 40,
        error_count: 5,
        severity: 0.6, // Error
        error_types: vec!["syntax_error".to_string()],
    });
    diagnostics.scene_changes.push(SceneChange {
        display_idx: 50,
        confidence: 0.8, // Info
        description: None,
    });

    let feed = InsightFeed::generate(&frame_map, &diagnostics);
    let counts = feed.count_by_severity();

    assert!(counts.total() >= 3);
    assert!(counts.has_issues()); // Has error or critical
    assert!(counts.critical > 0);
    assert!(counts.error > 0);
}

#[test]
fn test_insight_severity_priority() {
    assert!(InsightSeverity::Critical.priority() > InsightSeverity::Error.priority());
    assert!(InsightSeverity::Error.priority() > InsightSeverity::Warn.priority());
    assert!(InsightSeverity::Warn.priority() > InsightSeverity::Info.priority());
}

// UX InsightFeed evidence chain test - Task 14 (S.T4-1.ALL.UX.InsightFeed.impl.evidence_chain.001)

#[test]
fn test_ux_insight_feed_click_traces_to_evidence_chain() {
    // UX InsightFeed: User clicks on error burst insight to trace to bitstream
    let frame_map = create_test_frame_map(50);
    let mut diagnostics = DiagnosticsBands::new();

    // UX InsightFeed: System detects error burst at frames 10-15
    diagnostics.error_bursts.push(ErrorBurst {
        start_idx: 10,
        end_idx: 15,
        error_count: 6,
        severity: 0.85,
        error_types: vec!["decode_error".to_string(), "syntax_error".to_string()],
    });

    // UX InsightFeed: Generate insights
    let feed = InsightFeed::generate(&frame_map, &diagnostics);

    // UX InsightFeed: User views insight feed panel
    assert!(feed.insights.len() > 0);

    // UX InsightFeed: User clicks on error burst insight
    let burst_insight = feed
        .insights
        .iter()
        .find(|i| matches!(i.insight_type, InsightType::ErrorBurst))
        .unwrap();

    // UX InsightFeed: Insight shows severity and affected range
    assert_eq!(burst_insight.severity, InsightSeverity::Critical);
    assert_eq!(burst_insight.frame_range, (10, 15));

    // UX InsightFeed: Insight has jump targets for navigation
    assert!(burst_insight.jump_targets.len() > 0);
    let timeline_target = burst_insight
        .jump_targets
        .iter()
        .find(|t| t.panel == "timeline")
        .unwrap();

    // UX InsightFeed: Timeline jump target includes start frame
    assert!(timeline_target.payload["frame_idx"].is_number());
    assert_eq!(timeline_target.payload["frame_idx"], 10);

    // UX InsightFeed: Evidence pointers would link to evidence chain when populated
    // (Evidence pointers are optional and may not be populated for all insight types)

    // UX InsightFeed: Insight has triggers explaining detection
    assert_eq!(burst_insight.triggers.len(), 1);
    let trigger = &burst_insight.triggers[0];
    assert_eq!(trigger.signal, "error_density");
    assert!((trigger.value - 0.85).abs() < 0.01); // Severity value
    assert_eq!(trigger.method, "burst_detection");

    // UX InsightFeed: User can filter feed by severity
    let critical_insights = feed.filter_by_severity(InsightSeverity::Critical);
    assert!(critical_insights.len() > 0);
    assert!(critical_insights
        .iter()
        .all(|i| i.severity == InsightSeverity::Critical));

    // UX InsightFeed: User can query insights in frame range
    let range_insights = feed.get_insights_in_range(10, 20);
    assert!(range_insights.len() > 0);
    assert!(range_insights.iter().any(|i| i.frame_range.0 == 10));

    // UX InsightFeed: User views severity counts badge
    let counts = feed.count_by_severity();
    assert!(counts.critical > 0);
    assert!(counts.has_issues());

    // UX InsightFeed: User can get specific insight by ID
    let insight_by_id = feed.get_insight(&burst_insight.id);
    assert!(insight_by_id.is_some());
    assert_eq!(insight_by_id.unwrap().id, burst_insight.id);
}
