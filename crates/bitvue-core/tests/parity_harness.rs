//! Tests for the Parity Harness system
//!
//! These tests validate the parity matrix parsing, scoring, gates, and report generation.

use bitvue_core::parity_harness::{
    calculate_parity_score, compare_evidence_bundles, compare_render_snapshots, evaluate_guard,
    get_full_parity_matrix, parse_and_validate_parity_matrix, DiffSeverity, EntityRef,
    EvidenceBundleManifest, EvidenceDiffConfig, GuardContext, HardFailGate, HardFailKind,
    OrderType, ParityCategory, ParityGate, ParityHarness, ParityHarnessConfig, ParityResult,
    PerfEventType, PerfGate, PerfTelemetryEvent, RenderSnapshot, ScoringWeights, SelectionSnapshot,
    Severity, SnapshotTolerances, ViewportState,
};
use std::collections::HashMap;

const SEED_JSON: &str = r#"{
  "meta": {
    "version": "v14",
    "generated_at": "2026-01-15T12:24:35.270519Z",
    "scope": "UI/UX parity vs competitor references (behavioral/intent parity; original visual design).",
    "competitors": ["vq_analyzer", "elecard_streameye", "intel_vpa"]
  },
  "items": [
    {
      "id": "IA_TRISYNC_PANELS_PRESENT",
      "category": "IA",
      "severity": "P0",
      "question": "Are the core panels required for tri-sync present and reachable without detours?",
      "competitor_refs": [{"target_id": "vq_analyzer", "evidence": ["02-UI-Components/*"]}],
      "our_refs": ["ui_state_space.json"],
      "verification": {
        "method": "manual_review",
        "oracle": "ui_acceptance_oracle.json:CORE_TRISYNC_CLICK_PLAYER_TO_HEX",
        "pass_criteria": "All nodes exist; selection propagation works."
      }
    },
    {
      "id": "CONTRACT_DISPLAY_DECODE_SEPARATION",
      "category": "Contract",
      "severity": "P0",
      "question": "Is display order vs decode order separation enforced everywhere?",
      "competitor_refs": [{"target_id": "vq_analyzer", "evidence": ["order toggles"]}],
      "our_refs": ["ui_rendering_contracts.json"],
      "verification": {
        "method": "evidence_bundle_diff",
        "oracle": "evidence_bundle_diff_contracts.json",
        "pass_criteria": "All exported evidence includes explicit order type."
      }
    }
  ]
}"#;

#[test]
fn test_parse_and_validate_parity_matrix() {
    let (matrix, validation) = parse_and_validate_parity_matrix(SEED_JSON).unwrap();

    assert!(
        validation.valid,
        "Validation errors: {:?}",
        validation.errors
    );
    assert_eq!(matrix.meta.version, "v14");
    assert_eq!(matrix.items.len(), 2);
    assert_eq!(matrix.items[0].id, "IA_TRISYNC_PANELS_PRESENT");
    assert_eq!(matrix.items[0].category, ParityCategory::IA);
    assert_eq!(matrix.items[0].severity, Severity::P0);
}

#[test]
fn test_validation_catches_missing_fields() {
    let bad_json = r#"{
      "meta": {"version": "", "generated_at": "2026-01-15", "scope": "test"},
      "items": [
        {
          "id": "",
          "category": "IA",
          "severity": "P0",
          "question": "",
          "competitor_refs": [],
          "our_refs": [],
          "verification": {"method": "manual_review", "oracle": "", "pass_criteria": ""}
        }
      ]
    }"#;

    let (_, validation) = parse_and_validate_parity_matrix(bad_json).unwrap();
    assert!(!validation.valid);
    assert!(!validation.errors.is_empty());
}

#[test]
fn test_parity_scoring() {
    let (matrix, _) = parse_and_validate_parity_matrix(SEED_JSON).unwrap();
    let weights = ScoringWeights::default();

    let mut results = HashMap::new();
    results.insert("IA_TRISYNC_PANELS_PRESENT".to_string(), ParityResult::Pass);
    results.insert(
        "CONTRACT_DISPLAY_DECODE_SEPARATION".to_string(),
        ParityResult::Pass,
    );

    let score = calculate_parity_score(&matrix, &results, &weights);

    assert_eq!(score.percentage, 100.0);
    assert!(score.hard_fails.is_empty());
    assert!(score.parity_ready);
}

#[test]
fn test_parity_scoring_with_failure() {
    let (matrix, _) = parse_and_validate_parity_matrix(SEED_JSON).unwrap();
    let weights = ScoringWeights::default();

    let mut results = HashMap::new();
    results.insert("IA_TRISYNC_PANELS_PRESENT".to_string(), ParityResult::Pass);
    results.insert(
        "CONTRACT_DISPLAY_DECODE_SEPARATION".to_string(),
        ParityResult::Fail,
    );

    let score = calculate_parity_score(&matrix, &results, &weights);

    assert!(score.percentage < 100.0);
    assert!(!score.hard_fails.is_empty());
    assert!(!score.parity_ready);
}

#[test]
fn test_severity_weights() {
    assert_eq!(Severity::P0.weight(), 1.0);
    assert_eq!(Severity::P1.weight(), 0.6);
    assert_eq!(Severity::P2.weight(), 0.3);
    assert_eq!(Severity::P3.weight(), 0.1);
}

#[test]
fn test_hard_fail_gate() {
    let mut gate = HardFailGate::new();
    assert!(!gate.is_failed());

    gate.record_violation(
        HardFailKind::OrderTypeMixingDetected,
        "Mixed display/decode order in timeline",
    );

    assert!(gate.is_failed());
    assert_eq!(gate.violations.len(), 1);
}

#[test]
fn test_parity_gate_evaluation() {
    let (matrix, _) = parse_and_validate_parity_matrix(SEED_JSON).unwrap();
    let weights = ScoringWeights::default();

    let mut results = HashMap::new();
    results.insert("IA_TRISYNC_PANELS_PRESENT".to_string(), ParityResult::Pass);
    results.insert(
        "CONTRACT_DISPLAY_DECODE_SEPARATION".to_string(),
        ParityResult::Pass,
    );

    let score = calculate_parity_score(&matrix, &results, &weights);
    let gate = ParityGate::default();
    let result = gate.evaluate(&score);

    assert!(result.passed);
    assert!(result.parity_ready);
}

#[test]
fn test_perf_gate_evaluation() {
    let gate = PerfGate::default();

    let event = PerfTelemetryEvent {
        event_type: PerfEventType::FrameTime,
        value_ms: 10.0, // Under 16.6ms budget
        timestamp: "2026-01-15T12:00:00Z".to_string(),
        context: HashMap::new(),
    };

    let result = gate.evaluate(&event);
    assert!(result.passed);

    let slow_event = PerfTelemetryEvent {
        event_type: PerfEventType::FrameTime,
        value_ms: 25.0, // Over budget
        timestamp: "2026-01-15T12:00:00Z".to_string(),
        context: HashMap::new(),
    };

    let result = gate.evaluate(&slow_event);
    assert!(!result.passed);
    assert!(result.degrade_triggered.is_some());
}

#[test]
fn test_guard_evaluation() {
    let context_with_selection = GuardContext {
        selected_entity: Some(EntityRef {
            kind: "frame".to_string(),
            id: "frame_42".to_string(),
            frame_index: Some(42),
            byte_offset: None,
        }),
        selected_byte_range: None,
    };

    let result = evaluate_guard("has_selection", &context_with_selection);
    assert!(result.enabled);

    let empty_context = GuardContext {
        selected_entity: None,
        selected_byte_range: None,
    };

    let result = evaluate_guard("has_selection", &empty_context);
    assert!(!result.enabled);
    assert_eq!(result.disabled_reason, Some("No selection.".to_string()));
}

#[test]
fn test_render_snapshot_comparison() {
    let snapshot_a = RenderSnapshot {
        workspace: "player".to_string(),
        panel: "main".to_string(),
        mode: "normal".to_string(),
        codec: "av1".to_string(),
        order_type: OrderType::Display,
        viewport: ViewportState {
            x: 0.0,
            y: 0.0,
            width: 1920.0,
            height: 1080.0,
            zoom: 1.0,
            pan_x: 0.0,
            pan_y: 0.0,
        },
        selection_txn: "txn_001".to_string(),
        layer_stack: vec!["base".to_string()],
        objects: vec![],
        legend: None,
        warnings: vec![],
        backend_fingerprint: "dav1d_1.0".to_string(),
    };

    let snapshot_b = snapshot_a.clone();
    let tolerances = SnapshotTolerances::default();

    let result = compare_render_snapshots(&snapshot_a, &snapshot_b, &tolerances);
    assert!(result.matches);
    assert!(result.differences.is_empty());
}

#[test]
fn test_render_snapshot_order_type_mismatch() {
    let snapshot_a = RenderSnapshot {
        workspace: "player".to_string(),
        panel: "main".to_string(),
        mode: "normal".to_string(),
        codec: "av1".to_string(),
        order_type: OrderType::Display,
        viewport: ViewportState {
            x: 0.0,
            y: 0.0,
            width: 1920.0,
            height: 1080.0,
            zoom: 1.0,
            pan_x: 0.0,
            pan_y: 0.0,
        },
        selection_txn: "txn_001".to_string(),
        layer_stack: vec![],
        objects: vec![],
        legend: None,
        warnings: vec![],
        backend_fingerprint: "dav1d_1.0".to_string(),
    };

    let mut snapshot_b = snapshot_a.clone();
    snapshot_b.order_type = OrderType::Decode;

    let tolerances = SnapshotTolerances::default();
    let result = compare_render_snapshots(&snapshot_a, &snapshot_b, &tolerances);

    assert!(!result.matches);
    assert!(result.differences.iter().any(|d| d.field == "order_type"));
}

#[test]
fn test_evidence_bundle_diff() {
    let manifest_a = EvidenceBundleManifest {
        bundle_version: "1.0".to_string(),
        app_version: "0.1.0".to_string(),
        git_commit: "abc123".to_string(),
        build_profile: "release".to_string(),
        os: "macOS".to_string(),
        gpu: "Apple M1".to_string(),
        cpu: "Apple M1".to_string(),
        backend: "dav1d".to_string(),
        plugin_versions: HashMap::new(),
        stream_fingerprint: "stream_001".to_string(),
        order_type: OrderType::Display,
        selection_state: SelectionSnapshot {
            selected_entity: None,
            selected_byte_range: None,
            order_type: OrderType::Display,
        },
        workspace: "player".to_string(),
        mode: "normal".to_string(),
        warnings: vec![],
        artifacts: vec![],
    };

    let manifest_b = manifest_a.clone();
    let config = EvidenceDiffConfig::default();

    let result = compare_evidence_bundles(&manifest_a, &manifest_b, &config);
    assert!(result.matches);
    assert!(result.abi_compatible);
}

#[test]
fn test_evidence_bundle_diff_order_type_breaking() {
    let manifest_a = EvidenceBundleManifest {
        bundle_version: "1.0".to_string(),
        app_version: "0.1.0".to_string(),
        git_commit: "abc123".to_string(),
        build_profile: "release".to_string(),
        os: "macOS".to_string(),
        gpu: "Apple M1".to_string(),
        cpu: "Apple M1".to_string(),
        backend: "dav1d".to_string(),
        plugin_versions: HashMap::new(),
        stream_fingerprint: "stream_001".to_string(),
        order_type: OrderType::Display,
        selection_state: SelectionSnapshot {
            selected_entity: None,
            selected_byte_range: None,
            order_type: OrderType::Display,
        },
        workspace: "player".to_string(),
        mode: "normal".to_string(),
        warnings: vec![],
        artifacts: vec![],
    };

    let mut manifest_b = manifest_a.clone();
    manifest_b.order_type = OrderType::Decode;

    let config = EvidenceDiffConfig::default();
    let result = compare_evidence_bundles(&manifest_a, &manifest_b, &config);

    assert!(!result.matches);
    assert!(!result.abi_compatible);
    assert!(result
        .differences
        .iter()
        .any(|d| d.severity == DiffSeverity::Breaking));
}

#[test]
fn test_harness_full_workflow() {
    let (matrix, _) = parse_and_validate_parity_matrix(SEED_JSON).unwrap();

    let config = ParityHarnessConfig {
        matrix,
        weights: ScoringWeights::default(),
        hard_fail_gate: HardFailGate::new(),
        parity_gate: ParityGate::default(),
        perf_gate: PerfGate::default(),
    };

    let mut harness = ParityHarness::new(config);

    // Record results
    harness.record_result("IA_TRISYNC_PANELS_PRESENT", ParityResult::Pass);
    harness.record_result("CONTRACT_DISPLAY_DECODE_SEPARATION", ParityResult::Pass);

    // Record telemetry
    harness.record_telemetry(PerfTelemetryEvent {
        event_type: PerfEventType::FrameTime,
        value_ms: 12.0,
        timestamp: chrono::Utc::now().to_rfc3339(),
        context: HashMap::new(),
    });

    // Calculate score
    let score = harness.calculate_score();
    assert_eq!(score.percentage, 100.0);

    // Evaluate gates
    let gate_results = harness.evaluate_gates();
    assert!(gate_results.overall_passed);
    assert!(gate_results.hard_fail.passed);
    assert!(gate_results.parity.passed);
}

#[test]
fn test_full_parity_matrix_parses() {
    let (matrix, validation) = get_full_parity_matrix().unwrap();

    assert!(
        validation.valid,
        "Full parity matrix validation errors: {:?}",
        validation.errors
    );
    assert_eq!(matrix.meta.version, "v14");
    assert_eq!(matrix.items.len(), 31, "Expected 31 parity items");

    // Verify category distribution
    let ia_count = matrix
        .items
        .iter()
        .filter(|i| i.category == ParityCategory::IA)
        .count();
    let interaction_count = matrix
        .items
        .iter()
        .filter(|i| i.category == ParityCategory::Interaction)
        .count();
    let contract_count = matrix
        .items
        .iter()
        .filter(|i| i.category == ParityCategory::Contract)
        .count();
    let evidence_count = matrix
        .items
        .iter()
        .filter(|i| i.category == ParityCategory::Evidence)
        .count();
    let perf_count = matrix
        .items
        .iter()
        .filter(|i| i.category == ParityCategory::Performance)
        .count();
    let failure_count = matrix
        .items
        .iter()
        .filter(|i| i.category == ParityCategory::FailureUX)
        .count();
    let accessibility_count = matrix
        .items
        .iter()
        .filter(|i| i.category == ParityCategory::Accessibility)
        .count();

    assert!(
        ia_count >= 10,
        "Expected at least 10 IA items, got {}",
        ia_count
    );
    assert!(
        interaction_count >= 5,
        "Expected at least 5 Interaction items, got {}",
        interaction_count
    );
    assert!(
        contract_count >= 3,
        "Expected at least 3 Contract items, got {}",
        contract_count
    );
    assert!(
        evidence_count >= 1,
        "Expected at least 1 Evidence item, got {}",
        evidence_count
    );
    assert!(
        perf_count >= 2,
        "Expected at least 2 Performance items, got {}",
        perf_count
    );
    assert!(
        failure_count >= 2,
        "Expected at least 2 FailureUX items, got {}",
        failure_count
    );
    assert!(
        accessibility_count >= 1,
        "Expected at least 1 Accessibility item, got {}",
        accessibility_count
    );
}

#[test]
fn test_full_parity_matrix_severity_distribution() {
    let (matrix, _) = get_full_parity_matrix().unwrap();

    let p0_count = matrix
        .items
        .iter()
        .filter(|i| i.severity == Severity::P0)
        .count();
    let p1_count = matrix
        .items
        .iter()
        .filter(|i| i.severity == Severity::P1)
        .count();
    let p2_count = matrix
        .items
        .iter()
        .filter(|i| i.severity == Severity::P2)
        .count();

    // P0 (critical) items should be majority
    assert!(
        p0_count >= 15,
        "Expected at least 15 P0 items, got {}",
        p0_count
    );
    assert!(
        p1_count >= 8,
        "Expected at least 8 P1 items, got {}",
        p1_count
    );
    assert!(
        p2_count >= 2,
        "Expected at least 2 P2 items, got {}",
        p2_count
    );
}

#[test]
fn test_full_parity_matrix_scoring() {
    let (matrix, _) = get_full_parity_matrix().unwrap();
    let weights = ScoringWeights::default();

    // All pass
    let mut all_pass = HashMap::new();
    for item in &matrix.items {
        all_pass.insert(item.id.clone(), ParityResult::Pass);
    }
    let score = calculate_parity_score(&matrix, &all_pass, &weights);
    assert_eq!(score.percentage, 100.0, "All pass should yield 100%");
    assert!(score.parity_ready, "All pass should be parity ready");

    // All fail
    let mut all_fail = HashMap::new();
    for item in &matrix.items {
        all_fail.insert(item.id.clone(), ParityResult::Fail);
    }
    let fail_score = calculate_parity_score(&matrix, &all_fail, &weights);
    assert_eq!(fail_score.percentage, 0.0, "All fail should yield 0%");
    assert!(
        !fail_score.parity_ready,
        "All fail should not be parity ready"
    );
}

#[test]
fn test_full_parity_harness_integration() {
    let (matrix, _) = get_full_parity_matrix().unwrap();

    let config = ParityHarnessConfig {
        matrix,
        weights: ScoringWeights::default(),
        hard_fail_gate: HardFailGate::new(),
        parity_gate: ParityGate::default(),
        perf_gate: PerfGate::default(),
    };

    let mut harness = ParityHarness::new(config);

    // Record pass for core P0 items
    let p0_items = [
        "IA_MAIN_PANEL_CODING_FLOW",
        "IA_STREAM_VIEW_TIMELINE",
        "IA_SYNTAX_TREE_PER_CODEC",
        "IA_SELECTION_INFO_BLOCK_DETAILS",
        "IA_HEX_VIEW_RAW_BYTES",
        "OVERLAY_QP_HEATMAP",
        "OVERLAY_MV_VECTORS",
        "GLOBAL_DUAL_VIEW_COMPARE",
        "CONTRACT_TRISYNC_PROPAGATION",
        "CONTRACT_DISPLAY_DECODE_ORDER",
        "EVIDENCE_BUNDLE_EXPORT",
    ];

    for item_id in p0_items {
        harness.record_result(item_id, ParityResult::Pass);
    }

    let score = harness.calculate_score();
    // 11 of 31 items = ~35% raw coverage, but weighted by P0 severity
    assert!(
        score.percentage > 30.0,
        "Core P0 items should yield >30%, got {}",
        score.percentage
    );

    let gate_results = harness.evaluate_gates();
    assert!(gate_results.hard_fail.passed, "No hard failures recorded");
}

#[test]
fn test_parity_report_generation() {
    let (matrix, _) = get_full_parity_matrix().unwrap();

    let config = ParityHarnessConfig {
        matrix,
        weights: ScoringWeights::default(),
        hard_fail_gate: HardFailGate::new(),
        parity_gate: ParityGate::default(),
        perf_gate: PerfGate::default(),
    };

    let mut harness = ParityHarness::new(config);

    // Record some results
    harness.record_result("IA_MAIN_PANEL_CODING_FLOW", ParityResult::Pass);
    harness.record_result("IA_STREAM_VIEW_TIMELINE", ParityResult::Pass);
    harness.record_result("OVERLAY_QP_HEATMAP", ParityResult::Fail);

    let report = harness.generate_report();

    assert_eq!(report.matrix_version, "v14");
    assert!(!report.items_by_category.is_empty());
    assert!(!report.category_scores.is_empty());

    // Check that IA category has items
    let ia_score = report.category_scores.get(&ParityCategory::IA).unwrap();
    assert!(ia_score.total > 0);
    assert!(ia_score.passed > 0); // We passed 2 IA items
}

// TODO: Implement export_report_json and export_report_markdown methods on ParityHarness
// #[test]
// fn test_parity_report_json_export() {
//     let (matrix, _) = get_full_parity_matrix().unwrap();
//
//     let config = ParityHarnessConfig {
//         matrix,
//         weights: ScoringWeights::default(),
//         hard_fail_gate: HardFailGate::new(),
//         parity_gate: ParityGate::default(),
//         perf_gate: PerfGate::default(),
//     };
//
//     let harness = ParityHarness::new(config);
//     let json = harness.export_report_json().unwrap();
//
//     assert!(json.contains("\"matrix_version\": \"v14\""));
//     assert!(json.contains("\"overall_score\""));
//     assert!(json.contains("\"category_scores\""));
// }

// TODO: Implement export_report_json and export_report_markdown methods on ParityHarness
// #[test]
// fn test_parity_report_markdown_export() {
//     let (matrix, _) = get_full_parity_matrix().unwrap();
//
//     let config = ParityHarnessConfig {
//         matrix,
//         weights: ScoringWeights::default(),
//         hard_fail_gate: HardFailGate::new(),
//         parity_gate: ParityGate::default(),
//         perf_gate: PerfGate::default(),
//     };
//
//     let mut harness = ParityHarness::new(config);
//     harness.record_result("IA_MAIN_PANEL_CODING_FLOW", ParityResult::Pass);
//
//     let md = harness.export_report_markdown();
//
//     assert!(md.contains("# Parity Harness Report"));
//     assert!(md.contains("## Summary"));
//     assert!(md.contains("## Category Breakdown"));
//     assert!(md.contains("| Overall Score |"));
//     assert!(md.contains("✅") || md.contains("⬜")); // Status icons
// }

#[test]
fn test_parity_report_with_hard_fails() {
    let (matrix, _) = get_full_parity_matrix().unwrap();

    let config = ParityHarnessConfig {
        matrix,
        weights: ScoringWeights::default(),
        hard_fail_gate: HardFailGate::new(),
        parity_gate: ParityGate::default(),
        perf_gate: PerfGate::default(),
    };

    let mut harness = ParityHarness::new(config);

    // Record P0 failure
    harness.record_result("CONTRACT_DISPLAY_DECODE_ORDER", ParityResult::Fail);

    let report = harness.generate_report();

    assert!(
        !report.hard_fails.is_empty(),
        "P0 failure should be in hard_fails"
    );
    assert!(
        !report.parity_ready,
        "Should not be parity ready with P0 failure"
    );
}
