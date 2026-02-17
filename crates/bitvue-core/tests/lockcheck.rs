#![allow(dead_code)]
//! Tests for product lock check system (v14)

use bitvue_core::lockcheck::{
    LockCheckCategory, LockCheckExecutor, LockCheckItem, LockCheckReport, LockCheckResult,
    PerfSmokeReport, PerfSmokeTest,
};

#[test]
fn test_lockcheck_result_is_passing() {
    assert!(LockCheckResult::Pass.is_passing());
    assert!(LockCheckResult::Warning.is_passing());
    assert!(LockCheckResult::Skip.is_passing());
    assert!(!LockCheckResult::Fail.is_passing());
}

#[test]
fn test_lockcheck_report_new() {
    let report = LockCheckReport::new("v14".to_string(), 1000);
    assert_eq!(report.version, "v14");
    assert_eq!(report.timestamp_ms, 1000);
    assert_eq!(report.checks.len(), 0);
}

#[test]
fn test_lockcheck_report_add_check() {
    let mut report = LockCheckReport::new("v14".to_string(), 1000);

    report.add_check(LockCheckItem {
        id: "TEST-1".to_string(),
        category: LockCheckCategory::Workspace,
        description: "Test check".to_string(),
        result: LockCheckResult::Pass,
        message: "All good".to_string(),
        contract_ref: None,
    });

    assert_eq!(report.checks.len(), 1);
}

#[test]
fn test_lockcheck_report_calculate_summary() {
    let mut report = LockCheckReport::new("v14".to_string(), 1000);

    report.add_check(LockCheckItem {
        id: "T1".to_string(),
        category: LockCheckCategory::Workspace,
        description: "Pass".to_string(),
        result: LockCheckResult::Pass,
        message: "".to_string(),
        contract_ref: None,
    });

    report.add_check(LockCheckItem {
        id: "T2".to_string(),
        category: LockCheckCategory::LodCache,
        description: "Fail".to_string(),
        result: LockCheckResult::Fail,
        message: "".to_string(),
        contract_ref: None,
    });

    report.add_check(LockCheckItem {
        id: "T3".to_string(),
        category: LockCheckCategory::PlayerOverlays,
        description: "Warning".to_string(),
        result: LockCheckResult::Warning,
        message: "".to_string(),
        contract_ref: None,
    });

    report.calculate_summary();

    assert_eq!(report.summary.total_checks, 3);
    assert_eq!(report.summary.passed, 1);
    assert_eq!(report.summary.failed, 1);
    assert_eq!(report.summary.warnings, 1);
    assert!((report.summary.pass_rate - 0.666).abs() < 0.01);
}

#[test]
fn test_lockcheck_report_get_checks_by_category() {
    let mut report = LockCheckReport::new("v14".to_string(), 1000);

    report.add_check(LockCheckItem {
        id: "W1".to_string(),
        category: LockCheckCategory::Workspace,
        description: "".to_string(),
        result: LockCheckResult::Pass,
        message: "".to_string(),
        contract_ref: None,
    });

    report.add_check(LockCheckItem {
        id: "L1".to_string(),
        category: LockCheckCategory::LodCache,
        description: "".to_string(),
        result: LockCheckResult::Pass,
        message: "".to_string(),
        contract_ref: None,
    });

    report.add_check(LockCheckItem {
        id: "W2".to_string(),
        category: LockCheckCategory::Workspace,
        description: "".to_string(),
        result: LockCheckResult::Pass,
        message: "".to_string(),
        contract_ref: None,
    });

    let workspace_checks = report.get_checks_by_category(LockCheckCategory::Workspace);
    assert_eq!(workspace_checks.len(), 2);
}

#[test]
fn test_lockcheck_report_is_critical_pass() {
    let mut report = LockCheckReport::new("v14".to_string(), 1000);

    report.add_check(LockCheckItem {
        id: "T1".to_string(),
        category: LockCheckCategory::Workspace,
        description: "".to_string(),
        result: LockCheckResult::Pass,
        message: "".to_string(),
        contract_ref: None,
    });

    report.calculate_summary();
    assert!(report.is_critical_pass());

    report.add_check(LockCheckItem {
        id: "T2".to_string(),
        category: LockCheckCategory::LodCache,
        description: "".to_string(),
        result: LockCheckResult::Fail,
        message: "".to_string(),
        contract_ref: None,
    });

    report.calculate_summary();
    assert!(!report.is_critical_pass());
}

#[test]
fn test_perf_smoke_report_new() {
    let report = PerfSmokeReport::new("v14".to_string(), 1000);
    assert_eq!(report.version, "v14");
    assert_eq!(report.timestamp_ms, 1000);
    assert_eq!(report.tests.len(), 0);
}

#[test]
fn test_perf_smoke_report_add_test() {
    let mut report = PerfSmokeReport::new("v14".to_string(), 1000);

    report.add_test(PerfSmokeTest {
        test_name: "Test1".to_string(),
        metric: "Decode".to_string(),
        measured_value: 30.0,
        threshold: 50.0,
        unit: "ms".to_string(),
        passed: true,
        message: "Pass".to_string(),
    });

    assert_eq!(report.tests.len(), 1);
}

#[test]
fn test_perf_smoke_report_calculate_summary() {
    let mut report = PerfSmokeReport::new("v14".to_string(), 1000);

    report.add_test(PerfSmokeTest {
        test_name: "T1".to_string(),
        metric: "M1".to_string(),
        measured_value: 10.0,
        threshold: 20.0,
        unit: "ms".to_string(),
        passed: true,
        message: "".to_string(),
    });

    report.add_test(PerfSmokeTest {
        test_name: "T2".to_string(),
        metric: "M2".to_string(),
        measured_value: 30.0,
        threshold: 20.0,
        unit: "ms".to_string(),
        passed: false,
        message: "".to_string(),
    });

    report.calculate_summary();

    assert_eq!(report.summary.total_tests, 2);
    assert_eq!(report.summary.passed, 1);
    assert_eq!(report.summary.failed, 1);
    assert!((report.summary.pass_rate - 0.5).abs() < 0.001);
}

#[test]
fn test_perf_smoke_report_all_passed() {
    let mut report = PerfSmokeReport::new("v14".to_string(), 1000);

    report.add_test(PerfSmokeTest {
        test_name: "T1".to_string(),
        metric: "M1".to_string(),
        measured_value: 10.0,
        threshold: 20.0,
        unit: "ms".to_string(),
        passed: true,
        message: "".to_string(),
    });

    report.calculate_summary();
    assert!(report.all_passed());

    report.add_test(PerfSmokeTest {
        test_name: "T2".to_string(),
        metric: "M2".to_string(),
        measured_value: 30.0,
        threshold: 20.0,
        unit: "ms".to_string(),
        passed: false,
        message: "".to_string(),
    });

    report.calculate_summary();
    assert!(!report.all_passed());
}

#[test]
fn test_lockcheck_executor_run_lockcheck() {
    let executor = LockCheckExecutor::new("v14".to_string());
    let report = executor.run_lockcheck(1000);

    // Should have checks in all categories
    assert!(report.summary.total_checks > 0);
    assert!(
        report
            .get_checks_by_category(LockCheckCategory::Workspace)
            .len()
            > 0
    );
    assert!(
        report
            .get_checks_by_category(LockCheckCategory::LodCache)
            .len()
            > 0
    );
    assert!(
        report
            .get_checks_by_category(LockCheckCategory::PlayerOverlays)
            .len()
            > 0
    );
    assert!(
        report
            .get_checks_by_category(LockCheckCategory::McpResources)
            .len()
            > 0
    );
    assert!(
        report
            .get_checks_by_category(LockCheckCategory::Degradation)
            .len()
            > 0
    );
    assert!(
        report
            .get_checks_by_category(LockCheckCategory::CacheCaps)
            .len()
            > 0
    );

    // All checks should pass
    assert_eq!(report.summary.failed, 0);
    assert!(report.is_critical_pass());
}

#[test]
fn test_lockcheck_executor_run_perf_smoke() {
    let executor = LockCheckExecutor::new("v14".to_string());
    let report = executor.run_perf_smoke(1000);

    // Should have performance tests
    assert!(report.summary.total_tests > 0);

    // All tests should pass
    assert_eq!(report.summary.failed, 0);
    assert!(report.all_passed());
}

#[test]
fn test_lockcheck_report_to_markdown() {
    let mut report = LockCheckReport::new("v14".to_string(), 1000);

    report.add_check(LockCheckItem {
        id: "TEST-1".to_string(),
        category: LockCheckCategory::Workspace,
        description: "Test check".to_string(),
        result: LockCheckResult::Pass,
        message: "All good".to_string(),
        contract_ref: Some("CONTRACT_REF".to_string()),
    });

    report.calculate_summary();

    let md = report.to_markdown();
    assert!(md.contains("# V14 Lock Check Report"));
    assert!(md.contains("Version: v14"));
    assert!(md.contains("TEST-1"));
    assert!(md.contains("All good"));
    assert!(md.contains("CONTRACT_REF"));
}

#[test]
fn test_perf_smoke_report_to_markdown() {
    let mut report = PerfSmokeReport::new("v14".to_string(), 1000);

    report.add_test(PerfSmokeTest {
        test_name: "Decode Test".to_string(),
        metric: "Decode".to_string(),
        measured_value: 30.0,
        threshold: 50.0,
        unit: "ms".to_string(),
        passed: true,
        message: "Within threshold".to_string(),
    });

    report.calculate_summary();

    let md = report.to_markdown();
    assert!(md.contains("# V14 Performance Smoke Report"));
    assert!(md.contains("Version: v14"));
    assert!(md.contains("Decode Test"));
    assert!(md.contains("30.00"));
    assert!(md.contains("50.00"));
    assert!(md.contains("Within threshold"));
}
