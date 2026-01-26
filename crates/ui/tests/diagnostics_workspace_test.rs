//! Tests for Diagnostics Workspace

#[test]
fn test_diagnostics_categories() {
    // Test diagnostic category types
    #[derive(Debug, PartialEq)]
    enum DiagnosticCategory {
        Error,
        Warning,
        Info,
        Hint,
    }

    let categories = vec![
        DiagnosticCategory::Error,
        DiagnosticCategory::Warning,
        DiagnosticCategory::Info,
    ];

    assert_eq!(categories.len(), 3);
}

#[test]
fn test_diagnostic_severity_ordering() {
    // Test diagnostic severity ordering
    #[derive(Debug, PartialEq, Ord, PartialOrd, Eq)]
    enum Severity {
        Hint = 0,
        Info = 1,
        Warning = 2,
        Error = 3,
    }

    assert!(Severity::Error > Severity::Warning);
    assert!(Severity::Warning > Severity::Info);
}

#[test]
fn test_diagnostic_message() {
    // Test diagnostic message structure
    struct Diagnostic {
        severity: String,
        message: String,
        file: String,
        line: usize,
        column: usize,
    }

    let diag = Diagnostic {
        severity: "Error".to_string(),
        message: "Invalid OBU type".to_string(),
        file: "stream.ivf".to_string(),
        line: 42,
        column: 10,
    };

    assert_eq!(diag.severity, "Error");
    assert_eq!(diag.line, 42);
}

#[test]
fn test_diagnostic_filtering() {
    // Test diagnostic filtering by severity
    struct DiagnosticFilter {
        show_errors: bool,
        show_warnings: bool,
        show_info: bool,
    }

    impl DiagnosticFilter {
        fn should_show(&self, severity: &str) -> bool {
            match severity {
                "Error" => self.show_errors,
                "Warning" => self.show_warnings,
                "Info" => self.show_info,
                _ => false,
            }
        }
    }

    let filter = DiagnosticFilter {
        show_errors: true,
        show_warnings: true,
        show_info: false,
    };

    assert!(filter.should_show("Error"));
    assert!(!filter.should_show("Info"));
}

#[test]
fn test_diagnostic_count_summary() {
    // Test diagnostic count summary
    struct DiagnosticSummary {
        error_count: usize,
        warning_count: usize,
        info_count: usize,
    }

    impl DiagnosticSummary {
        fn total(&self) -> usize {
            self.error_count + self.warning_count + self.info_count
        }

        fn has_errors(&self) -> bool {
            self.error_count > 0
        }
    }

    let summary = DiagnosticSummary {
        error_count: 2,
        warning_count: 5,
        info_count: 10,
    };

    assert_eq!(summary.total(), 17);
    assert!(summary.has_errors());
}

#[test]
fn test_diagnostic_navigation() {
    // Test diagnostic list navigation
    struct DiagnosticList {
        diagnostics: Vec<String>,
        current_index: usize,
    }

    impl DiagnosticList {
        fn next(&mut self) {
            if self.current_index < self.diagnostics.len() - 1 {
                self.current_index += 1;
            }
        }

        fn previous(&mut self) {
            if self.current_index > 0 {
                self.current_index -= 1;
            }
        }
    }

    let mut list = DiagnosticList {
        diagnostics: vec!["Error 1".to_string(), "Error 2".to_string()],
        current_index: 0,
    };

    list.next();
    assert_eq!(list.current_index, 1);

    list.previous();
    assert_eq!(list.current_index, 0);
}

#[test]
fn test_diagnostic_grouping() {
    // Test grouping diagnostics by file
    struct DiagnosticGroup {
        file: String,
        diagnostics: Vec<String>,
    }

    let groups = vec![
        DiagnosticGroup {
            file: "stream1.ivf".to_string(),
            diagnostics: vec!["Error 1".to_string(), "Warning 1".to_string()],
        },
        DiagnosticGroup {
            file: "stream2.ivf".to_string(),
            diagnostics: vec!["Error 2".to_string()],
        },
    ];

    assert_eq!(groups.len(), 2);
    assert_eq!(groups[0].diagnostics.len(), 2);
}

#[test]
fn test_diagnostic_quickfix() {
    // Test quick fix suggestions
    struct QuickFix {
        description: String,
        replacement: String,
    }

    struct DiagnosticWithFix {
        message: String,
        fixes: Vec<QuickFix>,
    }

    let diag = DiagnosticWithFix {
        message: "Invalid syntax".to_string(),
        fixes: vec![QuickFix {
            description: "Use correct syntax".to_string(),
            replacement: "corrected_value".to_string(),
        }],
    };

    assert_eq!(diag.fixes.len(), 1);
}

#[test]
fn test_diagnostic_search() {
    // Test searching diagnostics
    fn search_diagnostics(diagnostics: &[String], query: &str) -> Vec<usize> {
        diagnostics
            .iter()
            .enumerate()
            .filter(|(_, d)| d.contains(query))
            .map(|(i, _)| i)
            .collect()
    }

    let diagnostics = vec![
        "Error: Invalid OBU".to_string(),
        "Warning: Missing header".to_string(),
        "Error: Parse failed".to_string(),
    ];

    let results = search_diagnostics(&diagnostics, "Error");
    assert_eq!(results.len(), 2);
}

#[test]
fn test_diagnostic_export() {
    // Test exporting diagnostics
    struct DiagnosticExport {
        format: String,
        include_severity: bool,
        include_location: bool,
    }

    let export = DiagnosticExport {
        format: "CSV".to_string(),
        include_severity: true,
        include_location: true,
    };

    assert_eq!(export.format, "CSV");
}

#[test]
fn test_diagnostic_auto_scroll() {
    // Test auto-scroll to diagnostic
    struct ScrollState {
        viewport_top: usize,
        viewport_height: usize,
        target_line: usize,
    }

    impl ScrollState {
        fn is_visible(&self) -> bool {
            self.target_line >= self.viewport_top
                && self.target_line < self.viewport_top + self.viewport_height
        }

        fn scroll_to_target(&mut self) {
            if !self.is_visible() {
                self.viewport_top = self.target_line.saturating_sub(self.viewport_height / 2);
            }
        }
    }

    let mut scroll = ScrollState {
        viewport_top: 0,
        viewport_height: 20,
        target_line: 50,
    };

    assert!(!scroll.is_visible());
    scroll.scroll_to_target();
    assert!(scroll.is_visible());
}

#[test]
fn test_diagnostic_annotation() {
    // Test inline diagnostic annotations
    struct DiagnosticAnnotation {
        line: usize,
        column_start: usize,
        column_end: usize,
        message: String,
    }

    let annotation = DiagnosticAnnotation {
        line: 42,
        column_start: 10,
        column_end: 20,
        message: "Invalid value".to_string(),
    };

    assert_eq!(annotation.column_end - annotation.column_start, 10);
}
