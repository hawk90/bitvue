// Diagnostics module tests
#[allow(unused_imports)]
use super::*;

// ============================================================================
// Fixtures
// ============================================================================
#[allow(dead_code)]
fn create_test_diagnostic() -> Diagnostic {
    Diagnostic::new(
        1,
        DiagnosticSeverity::Error,
        crate::StreamId::A,
        "Test error".to_string(),
        DiagnosticCategory::Bitstream,
        1000,
    )
}

#[allow(dead_code)]
fn create_test_manager() -> DiagnosticsManager {
    DiagnosticsManager::new()
}

#[allow(dead_code)]
fn create_test_filter() -> DiagnosticsFilter {
    DiagnosticsFilter::default()
}

#[allow(dead_code)]
fn create_test_panel() -> DiagnosticsPanel {
    DiagnosticsPanel::new()
}

// ============================================================================
// DiagnosticSeverity Tests
// ============================================================================
#[cfg(test)]
mod severity_tests {
#[allow(unused_imports)]
    use super::*;

    #[test]
    fn test_display_text() {
        assert_eq!(DiagnosticSeverity::Info.display_text(), "INFO");
        assert_eq!(DiagnosticSeverity::Error.display_text(), "ERROR");
    }

    #[test]
    fn test_short_code() {
        assert_eq!(DiagnosticSeverity::Info.short_code(), "I");
        assert_eq!(DiagnosticSeverity::Fatal.short_code(), "F");
    }

    #[test]
    fn test_is_actionable() {
        assert!(!DiagnosticSeverity::Info.is_actionable());
        assert!(DiagnosticSeverity::Warn.is_actionable());
        assert!(DiagnosticSeverity::Error.is_actionable());
        assert!(DiagnosticSeverity::Fatal.is_actionable());
    }
}

// ============================================================================
// Diagnostic Tests
// ============================================================================
#[cfg(test)]
mod diagnostic_tests {
#[allow(unused_imports)]
    use super::*;

    #[test]
    fn test_new_creates_diagnostic() {
        let diag = create_test_diagnostic();
        assert_eq!(diag.id, 1);
        assert_eq!(diag.severity, DiagnosticSeverity::Error);
        assert_eq!(diag.message, "Test error");
    }

    #[test]
    fn test_with_bit_range() {
        let diag = create_test_diagnostic().with_bit_range(100, 200);
        assert_eq!(diag.bit_range, Some((100, 200)));
    }

    #[test]
    fn test_short_summary() {
        let diag = create_test_diagnostic();
        let summary = diag.short_summary();
        assert!(summary.contains("[E]"));
        assert!(summary.contains("Test error"));
    }

    #[test]
    fn test_full_message() {
        let diag = create_test_diagnostic();
        let msg = diag.full_message();
        assert!(msg.contains("ERROR"));
        assert!(msg.contains("Offset:"));
    }
}

// ============================================================================
// DiagnosticsManager Tests
// ============================================================================
#[cfg(test)]
mod manager_tests {
#[allow(unused_imports)]
    use super::*;

    #[test]
    fn test_new_creates_manager() {
        let manager = create_test_manager();
        assert_eq!(manager.diagnostics.len(), 0);
        assert_eq!(manager.next_id, 1);
    }

    #[test]
    fn test_add_returns_id() {
        let mut manager = create_test_manager();
        let id = manager.add(create_test_diagnostic());
        assert_eq!(id, 1);
    }

    #[test]
    fn test_add_adds_to_list() {
        let mut manager = create_test_manager();
        manager.add(create_test_diagnostic());
        assert_eq!(manager.diagnostics.len(), 1);
    }

    #[test]
    fn test_get_returns_diagnostic() {
        let mut manager = create_test_manager();
        let id = manager.add(create_test_diagnostic());
        let diag = manager.get(id);
        assert!(diag.is_some());
    }

    #[test]
    fn test_filter_by_severity() {
        let mut manager = create_test_manager();
        manager.add(Diagnostic::new(
            1,
            DiagnosticSeverity::Info,
            crate::StreamId::A,
            "x".to_string(),
            DiagnosticCategory::Bitstream,
            100,
        ));
        manager.add(Diagnostic::new(
            2,
            DiagnosticSeverity::Error,
            crate::StreamId::A,
            "x".to_string(),
            DiagnosticCategory::Bitstream,
            200,
        ));
        let filtered = manager.filter_by_severity(DiagnosticSeverity::Warn);
        assert_eq!(filtered.len(), 1); // Only ERROR
    }

    #[test]
    fn test_count_by_severity() {
        let mut manager = create_test_manager();
        manager.add(Diagnostic::new(
            1,
            DiagnosticSeverity::Info,
            crate::StreamId::A,
            "x".to_string(),
            DiagnosticCategory::Bitstream,
            100,
        ));
        manager.add(Diagnostic::new(
            2,
            DiagnosticSeverity::Error,
            crate::StreamId::A,
            "x".to_string(),
            DiagnosticCategory::Bitstream,
            200,
        ));
        let counts = manager.count_by_severity();
        assert_eq!(counts.info, 1);
        assert_eq!(counts.error, 1);
    }

    #[test]
    fn test_clear() {
        let mut manager = create_test_manager();
        manager.add(create_test_diagnostic());
        manager.clear();
        assert_eq!(manager.diagnostics.len(), 0);
    }
}

// ============================================================================
// SeverityCounts Tests
// ============================================================================
#[cfg(test)]
mod severity_counts_tests {
#[allow(unused_imports)]
    use super::*;

    #[test]
    fn test_total() {
        let counts = SeverityCounts {
            info: 5,
            warn: 3,
            error: 2,
            fatal: 1,
        };
        assert_eq!(counts.total(), 11);
    }

    #[test]
    fn test_has_issues() {
        let counts = SeverityCounts {
            info: 5,
            warn: 3,
            error: 2,
            fatal: 1,
        };
        assert!(counts.has_issues());
    }

    #[test]
    fn test_status_bar_text() {
        let counts = SeverityCounts {
            info: 5,
            warn: 3,
            error: 2,
            fatal: 1,
        };
        let text = counts.status_bar_text();
        assert_eq!(text, "W:3 E:2 F:1");
    }
}

// ============================================================================
// DiagnosticsPanel Tests
// ============================================================================
#[cfg(test)]
mod panel_tests {
#[allow(unused_imports)]
    use super::*;

    #[test]
    fn test_toggle_sort() {
        let mut panel = create_test_panel();
        panel.toggle_sort(DiagnosticsSortColumn::Severity);
        assert_eq!(panel.sort_column, DiagnosticsSortColumn::Severity);
    }

    #[test]
    fn test_next_prev_page() {
        let mut panel = create_test_panel();
        panel.next_page(&create_test_manager());
        assert_eq!(panel.current_page, 0); // No pages
    }

    #[test]
    fn test_select_diagnostic() {
        let mut panel = create_test_panel();
        let mut manager = create_test_manager();
        let diag = create_test_diagnostic()
            .with_frame(crate::FrameKey { frame_index: 5, stream: crate::StreamId::A, pts: None });
        let id = manager.add(diag);
        let result = panel.select_diagnostic(&manager, id);
        assert!(result.is_some());
    }
}

// ============================================================================
// FeatureDegradeState Tests
// ============================================================================
#[cfg(test)]
mod degrade_state_tests {
#[allow(unused_imports)]
    use super::*;

    #[test]
    fn test_available() {
        let state = FeatureDegradeState::available("Test Feature");
        assert_eq!(state.mode, DegradeMode::Available);
    }

    #[test]
    fn test_degraded() {
        let state = FeatureDegradeState::degraded("Test Feature", "No data");
        assert_eq!(state.mode, DegradeMode::Degraded);
    }

    #[test]
    fn test_unavailable() {
        let state = FeatureDegradeState::unavailable("Test Feature", "Missing codec");
        assert_eq!(state.mode, DegradeMode::Unavailable);
    }
}
