//! `calculate_parity_score`: weights per category and severity, what each result is worth, the
//! per-category and per-severity roll-ups, hard fails and the ready flag.

use crate::parity_harness::{
    calculate_parity_score, CategoryScore, ParityCategory, ParityItem, ParityMatrix,
    ParityMatrixMeta, ParityResult, ScoringWeights, Severity, Verification, VerificationMethod,
};
use std::collections::HashMap;

const ALL_CATEGORIES: [ParityCategory; 7] = [
    ParityCategory::IA,
    ParityCategory::Interaction,
    ParityCategory::Contract,
    ParityCategory::Evidence,
    ParityCategory::Performance,
    ParityCategory::FailureUX,
    ParityCategory::Accessibility,
];

fn item(id: &str, category: ParityCategory, severity: Severity) -> ParityItem {
    ParityItem {
        id: id.to_string(),
        category,
        severity,
        question: String::new(),
        competitor_refs: Vec::new(),
        our_refs: Vec::new(),
        verification: Verification {
            method: VerificationMethod::SemanticProbe,
            oracle: String::new(),
            pass_criteria: String::new(),
        },
    }
}

fn matrix(items: Vec<ParityItem>) -> ParityMatrix {
    ParityMatrix {
        meta: ParityMatrixMeta {
            version: "1".to_string(),
            generated_at: String::new(),
            scope: String::new(),
            competitors: Vec::new(),
        },
        items,
    }
}

fn results(pairs: &[(&str, ParityResult)]) -> HashMap<String, ParityResult> {
    pairs.iter().map(|(id, r)| (id.to_string(), *r)).collect()
}

/// Weights that make every category distinguishable.
fn distinct_weights() -> ScoringWeights {
    ScoringWeights {
        information_architecture: 1.0,
        interaction_intent: 2.0,
        contract_correctness: 4.0,
        evidence_reproducibility: 8.0,
    }
}

fn unit_weights() -> ScoringWeights {
    ScoringWeights {
        information_architecture: 1.0,
        interaction_intent: 1.0,
        contract_correctness: 1.0,
        evidence_reproducibility: 1.0,
    }
}

fn approx(a: f64, b: f64) {
    assert!((a - b).abs() < 1e-9, "{a} != {b}");
}

#[test]
fn empty_matrix_scores_zero_and_is_ready() {
    let s = calculate_parity_score(&matrix(vec![]), &HashMap::new(), &ScoringWeights::default());
    assert_eq!(
        (s.total_score, s.max_possible, s.percentage),
        (0.0, 0.0, 0.0)
    );
    assert!(s.by_category.is_empty() && s.by_severity.is_empty());
    assert!(s.items.is_empty() && s.hard_fails.is_empty());
    assert!(s.parity_ready);
}

#[test]
fn each_category_uses_its_weight() {
    let expected = [
        (ParityCategory::IA, 1.0),
        (ParityCategory::Interaction, 2.0),
        (ParityCategory::Contract, 4.0),
        (ParityCategory::Evidence, 8.0),
        (ParityCategory::Performance, 8.0), // shares the evidence weight
        (ParityCategory::FailureUX, 2.0),   // shares the interaction weight
        (ParityCategory::Accessibility, 1.0), // shares the information-architecture weight
    ];
    for (category, weight) in expected {
        let m = matrix(vec![item("a", category, Severity::P0)]);
        let s = calculate_parity_score(&m, &HashMap::new(), &distinct_weights());
        approx(s.max_possible, weight);
        approx(s.by_category[&category].max_possible, weight);
    }
}

#[test]
fn each_severity_scales_the_category_weight() {
    for (severity, factor) in [
        (Severity::P0, 1.0),
        (Severity::P1, 0.6),
        (Severity::P2, 0.3),
        (Severity::P3, 0.1),
    ] {
        let m = matrix(vec![item("a", ParityCategory::Contract, severity)]);
        let s = calculate_parity_score(&m, &HashMap::new(), &distinct_weights());
        approx(s.max_possible, 4.0 * factor);
        approx(s.by_severity[&severity].max_possible, 4.0 * factor);
    }
}

#[test]
fn only_a_pass_earns_points() {
    for (result, earns) in [
        (ParityResult::Pass, true),
        (ParityResult::Fail, false),
        (ParityResult::Blocked, false),
        (ParityResult::Skipped, false),
        (ParityResult::NotTested, false),
    ] {
        let m = matrix(vec![item("a", ParityCategory::IA, Severity::P1)]);
        let s = calculate_parity_score(&m, &results(&[("a", result)]), &unit_weights());
        approx(s.total_score, if earns { 0.6 } else { 0.0 });
        approx(s.max_possible, 0.6); // every result keeps its share of the maximum
        approx(s.items[0].weighted_score, if earns { 0.6 } else { 0.0 });
        approx(s.items[0].max_possible_score, 0.6);
        assert_eq!(s.items[0].result, result);
    }
}

#[test]
fn an_item_without_a_result_counts_as_not_tested() {
    let m = matrix(vec![item("a", ParityCategory::IA, Severity::P0)]);
    let s = calculate_parity_score(&m, &HashMap::new(), &unit_weights());
    assert_eq!(s.items[0].result, ParityResult::NotTested);
    let c = &s.by_category[&ParityCategory::IA];
    assert_eq!((c.passed, c.failed, c.blocked), (0, 0, 0));
    approx(c.max_possible, 1.0);
}

#[test]
fn results_for_unknown_ids_are_ignored() {
    let m = matrix(vec![item("a", ParityCategory::IA, Severity::P0)]);
    let s = calculate_parity_score(
        &m,
        &results(&[("a", ParityResult::Pass), ("ghost", ParityResult::Fail)]),
        &unit_weights(),
    );
    assert_eq!(s.items.len(), 1);
    assert!(s.hard_fails.is_empty());
}

#[test]
fn category_counters_track_pass_fail_and_blocked_only() {
    let m = matrix(vec![
        item("p1", ParityCategory::Contract, Severity::P2),
        item("p2", ParityCategory::Contract, Severity::P2),
        item("f", ParityCategory::Contract, Severity::P2),
        item("b", ParityCategory::Contract, Severity::P2),
        item("s", ParityCategory::Contract, Severity::P2),
        item("n", ParityCategory::Contract, Severity::P2),
    ]);
    let r = results(&[
        ("p1", ParityResult::Pass),
        ("p2", ParityResult::Pass),
        ("f", ParityResult::Fail),
        ("b", ParityResult::Blocked),
        ("s", ParityResult::Skipped),
    ]);
    let s = calculate_parity_score(&m, &r, &unit_weights());
    let c: &CategoryScore = &s.by_category[&ParityCategory::Contract];
    assert_eq!((c.passed, c.failed, c.blocked), (2, 1, 1));
}

#[test]
fn severity_counters_track_pass_and_fail_only() {
    let m = matrix(vec![
        item("p", ParityCategory::IA, Severity::P1),
        item("f", ParityCategory::IA, Severity::P1),
        item("b", ParityCategory::IA, Severity::P1),
        item("s", ParityCategory::IA, Severity::P1),
    ]);
    let r = results(&[
        ("p", ParityResult::Pass),
        ("f", ParityResult::Fail),
        ("b", ParityResult::Blocked),
        ("s", ParityResult::Skipped),
    ]);
    let s = calculate_parity_score(&m, &r, &unit_weights());
    let v = &s.by_severity[&Severity::P1];
    assert_eq!((v.passed, v.failed), (1, 1));
}

#[test]
fn roll_ups_cover_only_the_categories_and_severities_present() {
    let m = matrix(vec![
        item("a", ParityCategory::IA, Severity::P0),
        item("b", ParityCategory::Evidence, Severity::P3),
    ]);
    let s = calculate_parity_score(&m, &HashMap::new(), &unit_weights());
    assert_eq!(s.by_category.len(), 2);
    assert!(s.by_category.contains_key(&ParityCategory::IA));
    assert!(s.by_category.contains_key(&ParityCategory::Evidence));
    assert_eq!(s.by_severity.len(), 2);
    assert!(s.by_severity.contains_key(&Severity::P0));
    assert!(s.by_severity.contains_key(&Severity::P3));
}

#[test]
fn percentages_are_score_over_maximum() {
    // IA/P0 passes (1.0 of 1.0); IA/P1 fails (0 of 0.6); Contract/P0 passes (4.0 of 4.0).
    let m = matrix(vec![
        item("a", ParityCategory::IA, Severity::P0),
        item("b", ParityCategory::IA, Severity::P1),
        item("c", ParityCategory::Contract, Severity::P0),
    ]);
    let r = results(&[
        ("a", ParityResult::Pass),
        ("b", ParityResult::Fail),
        ("c", ParityResult::Pass),
    ]);
    let s = calculate_parity_score(&m, &r, &distinct_weights());
    approx(s.total_score, 5.0);
    approx(s.max_possible, 5.6);
    approx(s.percentage, 5.0 / 5.6 * 100.0);
    approx(
        s.by_category[&ParityCategory::IA].percentage,
        1.0 / 1.6 * 100.0,
    );
    approx(s.by_category[&ParityCategory::Contract].percentage, 100.0);
    approx(s.by_severity[&Severity::P0].percentage, 100.0);
    approx(s.by_severity[&Severity::P1].percentage, 0.0);
}

#[test]
fn zero_weights_give_zero_percent_instead_of_nan() {
    let w = ScoringWeights {
        information_architecture: 0.0,
        interaction_intent: 0.0,
        contract_correctness: 0.0,
        evidence_reproducibility: 0.0,
    };
    let m = matrix(vec![item("a", ParityCategory::IA, Severity::P0)]);
    let s = calculate_parity_score(&m, &results(&[("a", ParityResult::Pass)]), &w);
    assert_eq!(s.percentage, 0.0);
    assert_eq!(s.by_category[&ParityCategory::IA].percentage, 0.0);
    assert_eq!(s.by_severity[&Severity::P0].percentage, 0.0);
    assert_eq!(s.by_category[&ParityCategory::IA].passed, 1);
}

#[test]
fn items_keep_matrix_order_and_start_without_notes() {
    let m = matrix(vec![
        item("z", ParityCategory::Performance, Severity::P3),
        item("a", ParityCategory::IA, Severity::P0),
        item("m", ParityCategory::FailureUX, Severity::P2),
    ]);
    let s = calculate_parity_score(&m, &HashMap::new(), &unit_weights());
    let ids: Vec<_> = s.items.iter().map(|i| i.id.as_str()).collect();
    assert_eq!(ids, ["z", "a", "m"]);
    assert_eq!(s.items[0].category, ParityCategory::Performance);
    assert_eq!(s.items[0].severity, Severity::P3);
    assert!(s.items.iter().all(|i| i.notes.is_empty()));
}

#[test]
fn a_failed_p0_is_a_hard_fail_and_blocks_readiness() {
    let m = matrix(vec![
        item("first", ParityCategory::IA, Severity::P0),
        item("ok", ParityCategory::IA, Severity::P0),
        item("second", ParityCategory::Contract, Severity::P0),
    ]);
    let r = results(&[
        ("first", ParityResult::Fail),
        ("ok", ParityResult::Pass),
        ("second", ParityResult::Fail),
    ]);
    let s = calculate_parity_score(&m, &r, &unit_weights());
    assert_eq!(s.hard_fails, ["first", "second"]);
    assert!(!s.parity_ready);
}

#[test]
fn a_failed_lower_severity_item_is_not_a_hard_fail() {
    for severity in [Severity::P1, Severity::P2, Severity::P3] {
        let m = matrix(vec![item("a", ParityCategory::IA, severity)]);
        let s = calculate_parity_score(&m, &results(&[("a", ParityResult::Fail)]), &unit_weights());
        assert!(s.hard_fails.is_empty());
        assert!(s.parity_ready);
    }
}

#[test]
fn unfinished_p0_items_block_readiness_without_being_hard_fails() {
    // #287: blocked, skipped and not-tested P0 items used to leave `parity_ready` true. They are
    // not failures (`hard_fails` stays empty), but a P0 item that has not passed is not parity.
    for result in [
        ParityResult::Blocked,
        ParityResult::Skipped,
        ParityResult::NotTested,
    ] {
        let m = matrix(vec![item("a", ParityCategory::IA, Severity::P0)]);
        let s = calculate_parity_score(&m, &results(&[("a", result)]), &unit_weights());
        assert!(s.hard_fails.is_empty(), "{result:?}");
        assert_eq!(s.p0_unfinished, ["a"], "{result:?}");
        assert!(!s.parity_ready, "{result:?}");
    }
}

#[test]
fn a_p0_item_without_any_recorded_result_is_unfinished() {
    let m = matrix(vec![item("a", ParityCategory::IA, Severity::P0)]);
    let s = calculate_parity_score(&m, &HashMap::new(), &unit_weights());
    assert_eq!(s.p0_unfinished, ["a"]);
    assert!(!s.parity_ready);
}

#[test]
fn readiness_needs_every_p0_to_pass_and_ignores_lower_severities() {
    let m = matrix(vec![
        item("p0a", ParityCategory::IA, Severity::P0),
        item("p0b", ParityCategory::Contract, Severity::P0),
        item("p1", ParityCategory::IA, Severity::P1),
        item("p2", ParityCategory::IA, Severity::P2),
    ]);
    // all P0 pass; lower severities failed or untested
    let ready = results(&[
        ("p0a", ParityResult::Pass),
        ("p0b", ParityResult::Pass),
        ("p1", ParityResult::Fail),
    ]);
    let s = calculate_parity_score(&m, &ready, &unit_weights());
    assert!(s.hard_fails.is_empty() && s.p0_unfinished.is_empty());
    assert!(s.parity_ready);
    // one P0 short of a verdict is enough to withdraw it
    let one_short = results(&[("p0a", ParityResult::Pass), ("p1", ParityResult::Pass)]);
    let s = calculate_parity_score(&m, &one_short, &unit_weights());
    assert_eq!(s.p0_unfinished, ["p0b"]);
    assert!(!s.parity_ready);
}

#[test]
fn the_parity_gate_fails_on_an_unfinished_p0_and_names_it() {
    use crate::parity_harness::ParityGate;
    let m = matrix(vec![
        item("a", ParityCategory::IA, Severity::P0),
        item("b", ParityCategory::IA, Severity::P0),
        item("c", ParityCategory::IA, Severity::P1),
    ]);
    // 2 of 3 items pass (66%), so lower the percentage bar to isolate the P0 rule
    let r = results(&[("a", ParityResult::Pass), ("c", ParityResult::Pass)]);
    let score = calculate_parity_score(&m, &r, &unit_weights());
    let mut gate = ParityGate {
        required_pass_percentage: 10.0,
        p0_must_all_pass: true,
    };
    let result = gate.evaluate(&score);
    assert!(!result.passed);
    assert_eq!(result.p0_unfinished, ["b"]);
    assert!(result.p0_failures.is_empty());
    assert!(!result.parity_ready);
    // with the P0 rule off only the percentage counts
    gate.p0_must_all_pass = false;
    assert!(gate.evaluate(&score).passed);
}

#[test]
fn every_category_maps_somewhere() {
    let items: Vec<_> = ALL_CATEGORIES
        .iter()
        .enumerate()
        .map(|(i, c)| item(&format!("i{i}"), *c, Severity::P0))
        .collect();
    let s = calculate_parity_score(&matrix(items), &HashMap::new(), &distinct_weights());
    assert_eq!(s.by_category.len(), 7);
    approx(s.max_possible, 1.0 + 2.0 + 4.0 + 8.0 + 8.0 + 2.0 + 1.0);
}

#[test]
fn duplicate_ids_share_one_result() {
    let m = matrix(vec![
        item("dup", ParityCategory::IA, Severity::P0),
        item("dup", ParityCategory::Contract, Severity::P0),
    ]);
    let s = calculate_parity_score(
        &m,
        &results(&[("dup", ParityResult::Pass)]),
        &unit_weights(),
    );
    assert_eq!(s.items.len(), 2);
    approx(s.total_score, 2.0);
}
