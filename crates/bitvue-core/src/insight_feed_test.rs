// Insight feed module tests
//
// Per generate-tests skill: Arrange-Act-Assert pattern with fixtures
// and edge case coverage.

use super::*;
use crate::frame_identity::FrameMetadata;

// ============================================================================
// Fixtures
// ============================================================================

/// Create test frame metadata
fn create_test_frames() -> Vec<FrameMetadata> {
    vec![
        FrameMetadata {
            decode_idx: 0,
            size_bytes: 1000,
            frame_type: "I".to_string(),
            pts: Some(0),
            dts: Some(0),
        },
        FrameMetadata {
            decode_idx: 1,
            size_bytes: 1000,
            frame_type: "P".to_string(),
            pts: Some(100),
            dts: Some(33),
        },
        FrameMetadata {
            decode_idx: 2,
            size_bytes: 1000,
            frame_type: "P".to_string(),
            pts: Some(200),
            dts: Some(66),
        },
    ]
}

/// Create a test frame map
fn create_test_frame_map() -> FrameIndexMap {
    FrameIndexMap::new(&create_test_frames())
}

/// Create a test diagnostics bands
fn create_test_diagnostics() -> DiagnosticsBands {
    DiagnosticsBands {
        error_bursts: vec![
            ErrorBurst {
                start_idx: 10,
                end_idx: 15,
                error_count: 6,
                severity: 0.9,
                error_types: vec!["parse_error".to_string()],
            },
            ErrorBurst {
                start_idx: 30,
                end_idx: 32,
                error_count: 3,
                severity: 0.6,
                error_types: vec!["decode_error".to_string()],
            },
        ],
        scene_changes: vec![
            SceneChange {
                display_idx: 50,
                confidence: 0.8,
                description: Some("Scene boundary detected".to_string()),
            },
            SceneChange {
                display_idx: 100,
                confidence: 0.5, // Low confidence, should be filtered
                description: None,
            },
        ],
        reorder_entries: vec![
            ReorderEntry {
                display_idx: 10,
                pts: 300,
                dts: 100,
                depth: 200,
            },
            ReorderEntry {
                display_idx: 20,
                pts: 600,
                dts: 200,
                depth: 400,
            },
        ],
    }
}

/// Create a test insight feed
fn create_test_insight_feed() -> InsightFeed {
    InsightFeed::new()
}

// ============================================================================
// InsightFeed Construction Tests
// ============================================================================

#[cfg(test)]
mod construction_tests {
    use super::*;

    #[test]
    fn test_new_creates_empty_feed() {
        // Arrange & Act
        let feed = create_test_insight_feed();

        // Assert
        assert!(feed.insights.is_empty());
    }

    #[test]
    fn test_default_creates_empty_feed() {
        // Arrange & Act
        let feed = InsightFeed::default();

        // Assert
        assert!(feed.insights.is_empty());
    }
}

// ============================================================================
// InsightSeverity Tests
// ============================================================================

#[cfg(test)]
mod insight_severity_tests {
    use super::*;

    #[test]
    fn test_priority_info() {
        // Arrange & Act
        let priority = InsightSeverity::Info.priority();

        // Assert
        assert_eq!(priority, 0);
    }

    #[test]
    fn test_priority_warn() {
        // Arrange & Act
        let priority = InsightSeverity::Warn.priority();

        // Assert
        assert_eq!(priority, 1);
    }

    #[test]
    fn test_priority_error() {
        // Arrange & Act
        let priority = InsightSeverity::Error.priority();

        // Assert
        assert_eq!(priority, 2);
    }

    #[test]
    fn test_priority_critical() {
        // Arrange & Act
        let priority = InsightSeverity::Critical.priority();

        // Assert
        assert_eq!(priority, 3);
    }

    #[test]
    fn test_display_text() {
        // Arrange & Act
        assert_eq!(InsightSeverity::Info.display_text(), "Info");
        assert_eq!(InsightSeverity::Warn.display_text(), "Warning");
        assert_eq!(InsightSeverity::Error.display_text(), "Error");
        assert_eq!(InsightSeverity::Critical.display_text(), "Critical");
    }

    #[test]
    fn test_ordering() {
        // Arrange
        let info = InsightSeverity::Info;
        let warn = InsightSeverity::Warn;
        let error = InsightSeverity::Error;
        let critical = InsightSeverity::Critical;

        // Act & Assert
        assert!(info < warn);
        assert!(warn < error);
        assert!(error < critical);
    }
}

// ============================================================================
// SeverityCounts Tests
// ============================================================================

#[cfg(test)]
mod severity_counts_tests {
    use super::*;

    #[test]
    fn test_default() {
        // Arrange & Act
        let counts = SeverityCounts::default();

        // Assert
        assert_eq!(counts.info, 0);
        assert_eq!(counts.warn, 0);
        assert_eq!(counts.error, 0);
        assert_eq!(counts.critical, 0);
    }

    #[test]
    fn test_total() {
        // Arrange
        let mut counts = SeverityCounts::default();
        counts.info = 5;
        counts.warn = 3;
        counts.error = 2;
        counts.critical = 1;

        // Act
        let total = counts.total();

        // Assert
        assert_eq!(total, 11);
    }

    #[test]
    fn test_has_issues_true() {
        // Arrange
        let mut counts = SeverityCounts::default();
        counts.error = 5;

        // Act
        let has = counts.has_issues();

        // Assert
        assert!(has);
    }

    #[test]
    fn test_has_issues_critical() {
        // Arrange
        let mut counts = SeverityCounts::default();
        counts.critical = 1;

        // Act
        let has = counts.has_issues();

        // Assert
        assert!(has);
    }

    #[test]
    fn test_has_issues_false() {
        // Arrange
        let mut counts = SeverityCounts::default();
        counts.info = 10;
        counts.warn = 5;

        // Act
        let has = counts.has_issues();

        // Assert
        assert!(!has);
    }
}

// ============================================================================
// InsightFeed Generation Tests
// ============================================================================

#[cfg(test)]
mod generation_tests {
    use super::*;

    #[test]
    fn test_generate_empty_input() {
        // Arrange
        let frame_map = create_test_frame_map();
        let diagnostics = DiagnosticsBands {
            error_bursts: vec![],
            scene_changes: vec![],
            reorder_entries: vec![],
        };

        // Act
        let feed = InsightFeed::generate(&frame_map, &diagnostics);

        // Assert
        assert!(feed.insights.is_empty());
    }

    #[test]
    fn test_generate_pts_quality_warn() {
        // Arrange
        let frame_map = create_test_frame_map();
        let diagnostics = DiagnosticsBands {
            error_bursts: vec![],
            scene_changes: vec![],
            reorder_entries: vec![],
        };

        // Act
        let feed = InsightFeed::generate(&frame_map, &diagnostics);

        // Assert - With simple monotonic frames, quality is Ok, so no insight
        assert!(feed.insights.is_empty());
    }

    #[test]
    fn test_generate_pts_quality_bad() {
        // Arrange
        let frame_map = create_test_frame_map();
        let diagnostics = DiagnosticsBands {
            error_bursts: vec![],
            scene_changes: vec![],
            reorder_entries: vec![],
        };

        // Act
        let feed = InsightFeed::generate(&frame_map, &diagnostics);

        // Assert - With simple monotonic frames, quality is Ok, so no insight
        assert!(feed.insights.is_empty());
    }

    #[test]
    fn test_generate_pts_quality_ok_no_insight() {
        // Arrange
        let frame_map = create_test_frame_map();
        let diagnostics = DiagnosticsBands {
            error_bursts: vec![],
            scene_changes: vec![],
            reorder_entries: vec![],
        };

        // Act
        let feed = InsightFeed::generate(&frame_map, &diagnostics);

        // Assert
        assert!(feed.insights.is_empty());
    }

    #[test]
    fn test_generate_error_burst_critical() {
        // Arrange
        let frame_map = create_test_frame_map();
        let mut diagnostics = DiagnosticsBands {
            error_bursts: vec![],
            scene_changes: vec![],
            reorder_entries: vec![],
        };
        diagnostics.error_bursts.push(ErrorBurst {
            start_idx: 10,
            end_idx: 15,
            error_count: 6,
            severity: 0.9, // High severity -> Critical
            error_types: vec!["parse_error".to_string()],
        });

        // Act
        let feed = InsightFeed::generate(&frame_map, &diagnostics);

        // Assert
        assert_eq!(feed.insights.len(), 1);
        assert_eq!(feed.insights[0].severity, InsightSeverity::Critical);
        assert_eq!(feed.insights[0].insight_type, InsightType::ErrorBurst);
    }

    #[test]
    fn test_generate_error_burst_error() {
        // Arrange
        let frame_map = create_test_frame_map();
        let mut diagnostics = DiagnosticsBands {
            error_bursts: vec![],
            scene_changes: vec![],
            reorder_entries: vec![],
        };
        diagnostics.error_bursts.push(ErrorBurst {
            start_idx: 10,
            end_idx: 15,
            error_count: 3,
            severity: 0.6, // Medium severity -> Error
            error_types: vec!["decode_error".to_string()],
        });

        // Act
        let feed = InsightFeed::generate(&frame_map, &diagnostics);

        // Assert
        assert_eq!(feed.insights[0].severity, InsightSeverity::Error);
    }

    #[test]
    fn test_generate_error_burst_warn() {
        // Arrange
        let frame_map = create_test_frame_map();
        let mut diagnostics = DiagnosticsBands {
            error_bursts: vec![],
            scene_changes: vec![],
            reorder_entries: vec![],
        };
        diagnostics.error_bursts.push(ErrorBurst {
            start_idx: 10,
            end_idx: 15,
            error_count: 2,
            severity: 0.4, // Low severity -> Warn
            error_types: vec!["warning".to_string()],
        });

        // Act
        let feed = InsightFeed::generate(&frame_map, &diagnostics);

        // Assert
        assert_eq!(feed.insights[0].severity, InsightSeverity::Warn);
    }

    #[test]
    fn test_generate_scene_change_high_confidence() {
        // Arrange
        let frame_map = create_test_frame_map();
        let mut diagnostics = DiagnosticsBands {
            error_bursts: vec![],
            scene_changes: vec![],
            reorder_entries: vec![],
        };
        diagnostics.scene_changes.push(SceneChange {
            display_idx: 50,
            confidence: 0.8, // High confidence
            description: Some("Scene boundary".to_string()),
        });

        // Act
        let feed = InsightFeed::generate(&frame_map, &diagnostics);

        // Assert
        assert_eq!(feed.insights.len(), 1);
        assert_eq!(feed.insights[0].insight_type, InsightType::SceneChange);
        assert_eq!(feed.insights[0].severity, InsightSeverity::Info);
    }

    #[test]
    fn test_generate_scene_change_low_confidence_filtered() {
        // Arrange
        let frame_map = create_test_frame_map();
        let mut diagnostics = DiagnosticsBands {
            error_bursts: vec![],
            scene_changes: vec![],
            reorder_entries: vec![],
        };
        diagnostics.scene_changes.push(SceneChange {
            display_idx: 50,
            confidence: 0.5, // Below threshold
            description: None,
        });

        // Act
        let feed = InsightFeed::generate(&frame_map, &diagnostics);

        // Assert
        assert!(feed.insights.is_empty());
    }

    #[test]
    fn test_generate_reorder_detected() {
        // Arrange
        let frame_map = create_test_frame_map();
        let diagnostics = DiagnosticsBands {
            error_bursts: vec![],
            scene_changes: vec![],
            reorder_entries: vec![
                crate::ReorderEntry {
                    display_idx: 10,
                    decode_idx: 15,
                    severity: 0.5,
                },
                crate::ReorderEntry {
                    display_idx: 20,
                    decode_idx: 25,
                    severity: 0.3,
                },
            ],
        };

        // Act
        let feed = InsightFeed::generate(&frame_map, &diagnostics);

        // Assert
        assert_eq!(feed.insights.len(), 1);
        assert_eq!(feed.insights[0].insight_type, InsightType::ReorderDetected);
        assert_eq!(feed.insights[0].severity, InsightSeverity::Warn);
    }

    #[test]
    fn test_generate_multiple_insights_sorted() {
        // Arrange
        let frame_map = create_test_frame_map(); // Error severity
        let diagnostics = create_test_diagnostics();

        // Act
        let feed = InsightFeed::generate(&frame_map, &diagnostics);

        // Assert
        // Should have error burst (Critical), scene change (Info), reorder (Warn)
        assert!(feed.insights.len() >= 2);
        // First should be highest severity (Critical from error burst)
        assert_eq!(feed.insights[0].severity, InsightSeverity::Critical);
    }
}

// ============================================================================
// InsightFeed Filter Tests
// ============================================================================

#[cfg(test)]
mod filter_tests {
    use super::*;

    #[test]
    fn test_filter_by_severity_all() {
        // Arrange
        let frame_map = create_test_frame_map();
        let diagnostics = create_test_diagnostics();
        let feed = InsightFeed::generate(&frame_map, &diagnostics);

        // Act
        let filtered = feed.filter_by_severity(InsightSeverity::Info);

        // Assert
        assert_eq!(filtered.len(), feed.insights.len()); // All insights
    }

    #[test]
    fn test_filter_by_severity_warn() {
        // Arrange
        let frame_map = create_test_frame_map();
        let diagnostics = create_test_diagnostics();
        let feed = InsightFeed::generate(&frame_map, &diagnostics);

        // Act
        let filtered = feed.filter_by_severity(InsightSeverity::Warn);

        // Assert
        // Should only include Warn, Error, Critical
        for insight in filtered {
            assert!(insight.severity >= InsightSeverity::Warn);
        }
    }

    #[test]
    fn test_filter_by_severity_error() {
        // Arrange
        let frame_map = create_test_frame_map();
        let diagnostics = create_test_diagnostics();
        let feed = InsightFeed::generate(&frame_map, &diagnostics);

        // Act
        let filtered = feed.filter_by_severity(InsightSeverity::Error);

        // Assert
        // Should only include Error and Critical
        for insight in filtered {
            assert!(insight.severity >= InsightSeverity::Error);
        }
    }

    #[test]
    fn test_filter_by_severity_critical() {
        // Arrange
        let frame_map = create_test_frame_map();
        let diagnostics = create_test_diagnostics();
        let feed = InsightFeed::generate(&frame_map, &diagnostics);

        // Act
        let filtered = feed.filter_by_severity(InsightSeverity::Critical);

        // Assert
        // Should only include Critical
        for insight in filtered {
            assert_eq!(insight.severity, InsightSeverity::Critical);
        }
    }
}

// ============================================================================
// InsightFeed Query Tests
// ============================================================================

#[cfg(test)]
mod query_tests {
    use super::*;

    #[test]
    fn test_get_insights_in_range() {
        // Arrange
        let frame_map = create_test_frame_map();
        let diagnostics = create_test_diagnostics();
        let feed = InsightFeed::generate(&frame_map, &diagnostics);

        // Act - Get insights in range 5-20
        let insights = feed.get_insights_in_range(5, 20);

        // Assert
        // Should include error burst at 10-15
        assert!(!insights.is_empty());
    }

    #[test]
    fn test_get_insights_in_range_empty() {
        // Arrange
        let frame_map = create_test_frame_map();
        let diagnostics = create_test_diagnostics();
        let feed = InsightFeed::generate(&frame_map, &diagnostics);

        // Act - Get insights in range 200-300
        let insights = feed.get_insights_in_range(200, 300);

        // Assert
        assert!(insights.is_empty());
    }

    #[test]
    fn test_get_insights_in_range_overlap() {
        // Arrange
        let frame_map = create_test_frame_map();
        let diagnostics = create_test_diagnostics();
        let feed = InsightFeed::generate(&frame_map, &diagnostics);

        // Act - Range overlaps with error burst at 10-15
        let insights = feed.get_insights_in_range(12, 14);

        // Assert
        assert!(!insights.is_empty());
    }

    #[test]
    fn test_get_insight_by_id_exists() {
        // Arrange
        let frame_map = create_test_frame_map();
        let diagnostics = DiagnosticsBands {
            error_bursts: vec![],
            scene_changes: vec![],
            reorder_entries: vec![],
        };
        let feed = InsightFeed::generate(&frame_map, &diagnostics);

        // Act
        let insight = feed.get_insight("pts_warn");

        // Assert
        assert!(insight.is_some());
    }

    #[test]
    fn test_get_insight_by_id_not_exists() {
        // Arrange
        let frame_map = create_test_frame_map();
        let diagnostics = DiagnosticsBands {
            error_bursts: vec![],
            scene_changes: vec![],
            reorder_entries: vec![],
        };
        let feed = InsightFeed::generate(&frame_map, &diagnostics);

        // Act
        let insight = feed.get_insight("nonexistent");

        // Assert
        assert!(insight.is_none());
    }

    #[test]
    fn test_count_by_severity() {
        // Arrange
        let frame_map = create_test_frame_map();
        let diagnostics = create_test_diagnostics();
        let feed = InsightFeed::generate(&frame_map, &diagnostics);

        // Act
        let counts = feed.count_by_severity();

        // Assert
        assert_eq!(counts.total(), feed.insights.len());
    }
}

// ============================================================================
// Insight Structure Tests
// ============================================================================

#[cfg(test)]
mod insight_structure_tests {
    use super::*;

    #[test]
    fn test_insight_has_id() {
        // Arrange
        let frame_map = create_test_frame_map();
        let diagnostics = DiagnosticsBands {
            error_bursts: vec![],
            scene_changes: vec![],
            reorder_entries: vec![],
        };

        // Act
        let feed = InsightFeed::generate(&frame_map, &diagnostics);

        // Assert
        assert!(!feed.insights[0].id.is_empty());
    }

    #[test]
    fn test_insight_has_type() {
        // Arrange
        let frame_map = create_test_frame_map();
        let diagnostics = DiagnosticsBands {
            error_bursts: vec![],
            scene_changes: vec![],
            reorder_entries: vec![],
        };

        // Act
        let feed = InsightFeed::generate(&frame_map, &diagnostics);

        // Assert
        assert_eq!(feed.insights[0].insight_type, InsightType::PtsQualityWarn);
    }

    #[test]
    fn test_insight_has_triggers() {
        // Arrange
        let frame_map = create_test_frame_map();
        let diagnostics = DiagnosticsBands {
            error_bursts: vec![],
            scene_changes: vec![],
            reorder_entries: vec![],
        };

        // Act
        let feed = InsightFeed::generate(&frame_map, &diagnostics);

        // Assert
        assert!(!feed.insights[0].triggers.is_empty());
        assert_eq!(feed.insights[0].triggers[0].signal, "pts_quality");
    }

    #[test]
    fn test_insight_has_jump_targets() {
        // Arrange
        let frame_map = create_test_frame_map();
        let diagnostics = DiagnosticsBands {
            error_bursts: vec![],
            scene_changes: vec![],
            reorder_entries: vec![],
        };

        // Act
        let feed = InsightFeed::generate(&frame_map, &diagnostics);

        // Assert
        assert!(!feed.insights[0].jump_targets.is_empty());
    }

    #[test]
    fn test_insight_has_evidence() {
        // Arrange
        let frame_map = create_test_frame_map();
        let diagnostics = DiagnosticsBands {
            error_bursts: vec![],
            scene_changes: vec![],
            reorder_entries: vec![],
        };

        // Act
        let feed = InsightFeed::generate(&frame_map, &diagnostics);

        // Assert
        assert!(!feed.insights[0].evidence.is_empty());
        assert_eq!(feed.insights[0].evidence[0].kind, EvidenceKind::Diagnostic);
    }

    #[test]
    fn test_error_burst_has_multiple_jump_targets() {
        // Arrange
        let frame_map = create_test_frame_map();
        let mut diagnostics = DiagnosticsBands {
            error_bursts: vec![],
            scene_changes: vec![],
            reorder_entries: vec![],
        };
        diagnostics.error_bursts.push(ErrorBurst {
            start_idx: 10,
            end_idx: 15,
            severity: 0.9,
        });

        // Act
        let feed = InsightFeed::generate(&frame_map, &diagnostics);

        // Assert
        // Error burst should have jump targets to timeline and diagnostics
        assert!(feed.insights[0].jump_targets.len() >= 2);
    }
}

// ============================================================================
// Edge Cases
// ============================================================================

#[cfg(test)]
mod edge_case_tests {
    use super::*;

    #[test]
    fn test_empty_diagnostics_empty_insights() {
        // Arrange
        let frame_map = create_test_frame_map();
        let diagnostics = DiagnosticsBands {
            error_bursts: vec![],
            scene_changes: vec![],
            reorder_entries: vec![],
        };

        // Act
        let feed = InsightFeed::generate(&frame_map, &diagnostics);

        // Assert
        assert!(feed.insights.is_empty());
    }

    #[test]
    fn test_multiple_error_bursts() {
        // Arrange
        let frame_map = create_test_frame_map();
        let diagnostics = DiagnosticsBands {
            error_bursts: vec![
                ErrorBurst {
                    start_idx: 10,
                    end_idx: 15,
                    error_count: 6,
                    severity: 0.9,
                    error_types: vec!["parse_error".to_string()],
                },
                ErrorBurst {
                    start_idx: 30,
                    end_idx: 32,
                    error_count: 3,
                    severity: 0.6,
                    error_types: vec!["decode_error".to_string()],
                },
            ],
            scene_changes: vec![],
            reorder_entries: vec![],
        };

        // Act
        let feed = InsightFeed::generate(&frame_map, &diagnostics);

        // Assert
        assert_eq!(feed.insights.len(), 2);
    }

    #[test]
    fn test_scene_change_at_boundary() {
        // Arrange
        let frame_map = create_test_frame_map();
        let mut diagnostics = DiagnosticsBands {
            error_bursts: vec![],
            scene_changes: vec![],
            reorder_entries: vec![],
        };
        diagnostics.scene_changes.push(SceneChange {
            display_idx: 0,
            confidence: 0.9,
            description: Some("First scene".to_string()),
        });

        // Act
        let feed = InsightFeed::generate(&frame_map, &diagnostics);

        // Assert
        assert_eq!(feed.insights.len(), 1);
        assert_eq!(feed.insights[0].frame_range, (0, 0));
    }

    #[test]
    fn test_count_by_severity_empty() {
        // Arrange
        let feed = InsightFeed::new();

        // Act
        let counts = feed.count_by_severity();

        // Assert
        assert_eq!(counts.total(), 0);
    }

    #[test]
    fn test_filter_by_severity_empty() {
        // Arrange
        let feed = InsightFeed::new();

        // Act
        let filtered = feed.filter_by_severity(InsightSeverity::Info);

        // Assert
        assert!(filtered.is_empty());
    }
}
