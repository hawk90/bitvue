// Lock Check module tests
use super::*;

// ============================================================================
// Fixtures
// ============================================================================
fn create_test_result() -> LockCheckResult {
    LockCheckResult::Pass
}

fn create_test_item() -> LockCheckItem {
    LockCheckItem {
        id: "TEST-1".to_string(),
        category: LockCheckCategory::Workspace,
        description: "Test check".to_string(),
        result: LockCheckResult::Pass,
        message: "Passed".to_string(),
        contract_ref: Some("TEST_CONTRACT".to_string()),
    }
}

fn create_test_report() -> LockCheckReport {
    LockCheckReport::new("v14".to_string(), 1000)
}

fn create_test_executor() -> LockCheckExecutor {
    LockCheckExecutor::new("v14".to_string())
}

// ============================================================================
// LockCheckResult Tests
// ============================================================================
#[cfg(test)]
mod result_tests {
    use super::*;

    #[test]
    fn test_is_passing() {
        assert!(LockCheckResult::Pass.is_passing());
        assert!(LockCheckResult::Warning.is_passing());
        assert!(LockCheckResult::Skip.is_passing());
        assert!(!LockCheckResult::Fail.is_passing());
    }

    #[test]
    fn test_display_symbol() {
        assert_eq!(LockCheckResult::Pass.display_symbol(), "✓");
        assert_eq!(LockCheckResult::Fail.display_symbol(), "✗");
        assert_eq!(LockCheckResult::Warning.display_symbol(), "⚠");
        assert_eq!(LockCheckResult::Skip.display_symbol(), "○");
    }
}

// ============================================================================
// LockCheckCategory Tests
// ============================================================================
#[cfg(test)]
mod category_tests {
    use super::*;

    #[test]
    fn test_display_name() {
        assert_eq!(LockCheckCategory::Workspace.display_name(), "Workspace Architecture");
        assert_eq!(LockCheckCategory::LodCache.display_name(), "LOD & Cache Keys");
    }
}

// ============================================================================
// LockCheckReport Tests
// ============================================================================
#[cfg(test)]
mod report_tests {
    use super::*;

    #[test]
    fn test_new_creates_report() {
        let report = LockCheckReport::new("v14".to_string(), 1000);
        assert_eq!(report.version, "v14");
        assert_eq!(report.timestamp_ms, 1000);
    }

    #[test]
    fn test_add_check() {
        let mut report = create_test_report();
        report.add_check(create_test_item());
        assert_eq!(report.checks.len(), 1);
    }

    #[test]
    fn test_calculate_summary() {
        let mut report = create_test_report();
        report.add_check(LockCheckItem {
            id: "1".to_string(),
            category: LockCheckCategory::Workspace,
            description: "x".to_string(),
            result: LockCheckResult::Pass,
            message: "x".to_string(),
            contract_ref: None,
        });
        report.calculate_summary();
        assert_eq!(report.summary.total_checks, 1);
        assert_eq!(report.summary.passed, 1);
    }

    #[test]
    fn test_get_checks_by_category() {
        let mut report = create_test_report();
        report.add_check(LockCheckItem {
            id: "1".to_string(),
            category: LockCheckCategory::Workspace,
            description: "x".to_string(),
            result: LockCheckResult::Pass,
            message: "x".to_string(),
            contract_ref: None,
        });
        let checks = report.get_checks_by_category(LockCheckCategory::Workspace);
        assert_eq!(checks.len(), 1);
    }

    #[test]
    fn test_is_critical_pass() {
        let mut report = create_test_report();
        report.add_check(LockCheckItem {
            id: "1".to_string(),
            category: LockCheckCategory::Workspace,
            description: "x".to_string(),
            result: LockCheckResult::Pass,
            message: "x".to_string(),
            contract_ref: None,
        });
        report.calculate_summary();
        assert!(report.is_critical_pass());
    }

    #[test]
    fn test_to_markdown() {
        let report = create_test_report();
        let md = report.to_markdown();
        assert!(md.contains("# V14 Lock Check Report"));
    }
}

// ============================================================================
// PerfSmokeReport Tests
// ============================================================================
#[cfg(test)]
mod perf_report_tests {
    use super::*;

    #[test]
    fn test_new_creates_perf_report() {
        let report = PerfSmokeReport::new("v14".to_string(), 1000);
        assert_eq!(report.version, "v14");
    }

    #[test]
    fn test_add_test() {
        let mut report = PerfSmokeReport::new("v14".to_string(), 1000);
        report.add_test(PerfSmokeTest {
            test_name: "Test".to_string(),
            metric: "Metric".to_string(),
            measured_value: 100.0,
            threshold: 200.0,
            unit: "ms".to_string(),
            passed: true,
            message: "OK".to_string(),
        });
        assert_eq!(report.tests.len(), 1);
    }

    #[test]
    fn test_calculate_summary() {
        let mut report = PerfSmokeReport::new("v14".to_string(), 1000);
        report.add_test(PerfSmokeTest {
            test_name: "x".to_string(),
            metric: "x".to_string(),
            measured_value: 100.0,
            threshold: 200.0,
            unit: "x".to_string(),
            passed: true,
            message: "x".to_string(),
        });
        report.calculate_summary();
        assert_eq!(report.summary.total_tests, 1);
    }
}

// ============================================================================
// LockCheckExecutor Tests
// ============================================================================
#[cfg(test)]
mod executor_tests {
    use super::*;

    #[test]
    fn test_new_creates_executor() {
        let executor = create_test_executor();
        assert_eq!(executor.version, "v14");
    }

    #[test]
    fn test_run_lockcheck() {
        let executor = create_test_executor();
        let report = executor.run_lockcheck(1000);
        assert!(!report.checks.is_empty());
    }

    #[test]
    fn test_run_perf_smoke() {
        let executor = create_test_executor();
        let report = executor.run_perf_smoke(1000);
        assert!(!report.tests.is_empty());
    }
}
