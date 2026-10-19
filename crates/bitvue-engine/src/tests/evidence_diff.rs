//! `compare_evidence_bundles`: one field at a time, then in combination.

use crate::export::EvidenceBundleManifest;
use crate::parity_harness::{
    compare_evidence_bundles, load_evidence_bundle_manifest, DiffSeverity, EntityRef,
    EvidenceBundleDiffResult, EvidenceDiffConfig, OrderType,
};

fn manifest() -> EvidenceBundleManifest {
    EvidenceBundleManifest::default()
}

fn only(fields: &[&str]) -> EvidenceDiffConfig {
    EvidenceDiffConfig {
        compare_fields: fields.iter().map(|s| s.to_string()).collect(),
        ignore_fields: vec![],
    }
}

fn diff(
    a: &EvidenceBundleManifest,
    b: &EvidenceBundleManifest,
    config: &EvidenceDiffConfig,
) -> EvidenceBundleDiffResult {
    compare_evidence_bundles(a, b, config)
}

fn fields(r: &EvidenceBundleDiffResult) -> Vec<&str> {
    r.differences.iter().map(|d| d.field.as_str()).collect()
}

fn plugins(pairs: &[(&str, &str)]) -> std::collections::HashMap<String, String> {
    pairs
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect()
}

#[test]
fn identical_manifests_match_under_the_default_config() {
    let r = diff(&manifest(), &manifest(), &EvidenceDiffConfig::default());
    assert!(r.matches && r.abi_compatible);
    assert!(r.differences.is_empty() && r.ignored_changes.is_empty());
}

#[test]
fn order_type_difference_is_breaking_and_not_abi_compatible() {
    let mut b = manifest();
    b.order_type = OrderType::Decode;
    let r = diff(&manifest(), &b, &only(&["order_type"]));
    assert_eq!(fields(&r), ["order_type"]);
    let d = &r.differences[0];
    assert_eq!(
        (d.a_value.as_str(), d.b_value.as_str()),
        ("Display", "Decode")
    );
    assert_eq!(d.severity, DiffSeverity::Breaking);
    assert!(!r.matches && !r.abi_compatible);
}

#[test]
fn selection_state_difference_is_a_warning_carrying_both_json_values() {
    let mut b = manifest();
    b.selection_state.selected_byte_range = Some((1, 2));
    b.selection_state.selected_entity = Some(EntityRef {
        kind: "frame".into(),
        id: "f1".into(),
        frame_index: None,
        byte_offset: None,
    });
    let r = diff(&manifest(), &b, &only(&["selection_state"]));
    assert_eq!(fields(&r), ["selection_state"]);
    let d = &r.differences[0];
    assert_eq!(d.severity, DiffSeverity::Warning);
    assert!(
        d.a_value.contains("\"selected_byte_range\":null"),
        "{}",
        d.a_value
    );
    assert!(
        d.b_value.contains("\"selected_byte_range\":[1,2]"),
        "{}",
        d.b_value
    );
    assert!(!r.matches);
    assert!(r.abi_compatible, "a warning does not break ABI");
}

#[test]
fn backend_difference_is_info_and_keeps_abi() {
    let mut b = manifest();
    b.backend = "libaom".into();
    let r = diff(&manifest(), &b, &only(&["backend_fingerprint"]));
    assert_eq!(fields(&r), ["backend_fingerprint"]);
    let d = &r.differences[0];
    assert_eq!(
        (d.a_value.as_str(), d.b_value.as_str()),
        ("dav1d", "libaom")
    );
    assert_eq!(d.severity, DiffSeverity::Info);
    assert!(r.abi_compatible && !r.matches);
}

#[test]
fn changed_plugin_version_is_info() {
    let mut a = manifest();
    let mut b = manifest();
    a.plugin_versions = plugins(&[("x", "1")]);
    b.plugin_versions = plugins(&[("x", "2")]);
    let r = diff(&a, &b, &only(&["plugin_versions"]));
    assert_eq!(fields(&r), ["plugin_versions.x"]);
    let d = &r.differences[0];
    assert_eq!((d.a_value.as_str(), d.b_value.as_str()), ("1", "2"));
    assert_eq!(d.severity, DiffSeverity::Info);
}

#[test]
fn unchanged_plugin_version_is_not_reported() {
    let mut a = manifest();
    a.plugin_versions = plugins(&[("x", "1")]);
    let r = diff(&a, &a.clone(), &only(&["plugin_versions"]));
    assert!(r.matches && r.ignored_changes.is_empty());
}

#[test]
fn plugin_missing_from_b_is_a_warning() {
    let mut a = manifest();
    a.plugin_versions = plugins(&[("x", "1")]);
    let r = diff(&a, &manifest(), &only(&["plugin_versions"]));
    assert_eq!(fields(&r), ["plugin_versions.x"]);
    let d = &r.differences[0];
    assert_eq!((d.a_value.as_str(), d.b_value.as_str()), ("1", "(missing)"));
    assert_eq!(d.severity, DiffSeverity::Warning);
}

#[test]
fn plugin_only_in_b_is_noted_but_not_a_difference() {
    let mut b = manifest();
    b.plugin_versions = plugins(&[("y", "3")]);
    let r = diff(&manifest(), &b, &only(&["plugin_versions"]));
    assert!(r.matches, "an added plugin does not fail the diff");
    assert_eq!(r.ignored_changes, ["New plugin in b: y"]);
}

#[test]
fn plugin_notes_require_plugin_versions_to_be_compared() {
    let mut b = manifest();
    b.plugin_versions = plugins(&[("y", "3")]);
    let r = diff(&manifest(), &b, &only(&["order_type"]));
    assert!(r.ignored_changes.is_empty() && r.matches);
}

#[test]
fn warnings_compare_as_sorted_sets_and_join_with_commas() {
    let mut a = manifest();
    let mut b = manifest();
    a.warnings = vec!["z".into(), "a".into()];
    b.warnings = vec!["a".into(), "z".into()];
    assert!(
        diff(&a, &b, &only(&["warnings"])).matches,
        "order is ignored"
    );

    b.warnings = vec!["q".into(), "a".into()];
    let r = diff(&a, &b, &only(&["warnings"]));
    assert_eq!(fields(&r), ["warnings"]);
    let d = &r.differences[0];
    assert_eq!((d.a_value.as_str(), d.b_value.as_str()), ("a, z", "a, q"));
    assert_eq!(d.severity, DiffSeverity::Warning);
}

#[test]
fn warnings_count_duplicates() {
    let mut a = manifest();
    let mut b = manifest();
    a.warnings = vec!["w".into(), "w".into()];
    b.warnings = vec!["w".into()];
    assert!(!diff(&a, &b, &only(&["warnings"])).matches);
}

#[test]
fn unlisted_fields_are_never_compared() {
    let mut b = manifest();
    b.order_type = OrderType::Decode;
    b.backend = "x".into();
    b.warnings = vec!["w".into()];
    b.plugin_versions = plugins(&[("p", "1")]);
    b.selection_state.selected_byte_range = Some((0, 1));
    let r = diff(&manifest(), &b, &only(&[]));
    assert!(r.matches && r.abi_compatible && r.ignored_changes.is_empty());
}

#[test]
fn fields_the_diff_does_not_know_are_accepted_and_ignored() {
    let mut b = manifest();
    b.workspace = "other".into();
    b.mode = "other".into();
    b.stream_fingerprint = "other".into();
    let r = diff(
        &manifest(),
        &b,
        &only(&["workspace", "mode", "stream_fingerprint"]),
    );
    assert!(r.matches);
}

#[test]
fn schema_major_mismatch_is_reported_even_with_no_fields_compared() {
    let mut b = manifest();
    b.bundle_version = "2.1".into();
    let r = diff(&manifest(), &b, &only(&[]));
    assert_eq!(fields(&r), ["bundle_version"]);
    let d = &r.differences[0];
    assert_eq!(d.a_value, "1.0 (schema major 1)");
    assert_eq!(d.b_value, "2.1 (schema major 2)");
    assert_eq!(d.severity, DiffSeverity::Breaking);
    assert!(!r.abi_compatible);
}

#[test]
fn minor_or_unparseable_versions_are_not_a_schema_difference() {
    let mut b = manifest();
    b.bundle_version = "1.7".into();
    assert!(diff(&manifest(), &b, &only(&[])).matches);
    b.bundle_version = "garbage".into();
    assert!(diff(&manifest(), &b, &only(&[])).matches);
}

#[test]
fn differences_are_listed_in_a_fixed_order() {
    let mut a = manifest();
    let mut b = manifest();
    b.bundle_version = "9.0".into();
    b.order_type = OrderType::Decode;
    b.selection_state.selected_byte_range = Some((1, 1));
    b.backend = "x".into();
    a.plugin_versions = plugins(&[("p", "1")]);
    b.warnings = vec!["w".into()];
    let r = diff(&a, &b, &EvidenceDiffConfig::default());
    assert_eq!(
        fields(&r),
        [
            "bundle_version",
            "order_type",
            "selection_state",
            "backend_fingerprint",
            "plugin_versions.p",
            "warnings"
        ]
    );
    assert!(!r.abi_compatible && !r.matches);
}

#[test]
fn abi_ignores_non_breaking_differences() {
    let mut b = manifest();
    b.backend = "x".into();
    b.warnings = vec!["w".into()];
    let r = diff(&manifest(), &b, &EvidenceDiffConfig::default());
    assert_eq!(r.differences.len(), 2);
    assert!(r.abi_compatible);
}

#[test]
fn diff_is_not_symmetric_for_plugins() {
    let mut a = manifest();
    a.plugin_versions = plugins(&[("p", "1")]);
    let cfg = only(&["plugin_versions"]);
    assert!(!diff(&a, &manifest(), &cfg).matches);
    assert!(diff(&manifest(), &a, &cfg).matches);
}

#[test]
fn loading_a_missing_manifest_names_the_path() {
    let dir = tempfile::tempdir().unwrap();
    let err = load_evidence_bundle_manifest(dir.path()).unwrap_err();
    assert!(err.starts_with("failed to read "), "{err}");
    assert!(err.contains("bundle_manifest.json"), "{err}");
}

#[test]
fn loading_invalid_json_reports_a_parse_error() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("bundle_manifest.json"), "{not json").unwrap();
    let err = load_evidence_bundle_manifest(dir.path()).unwrap_err();
    assert!(err.starts_with("failed to parse "), "{err}");
}

#[test]
fn loading_an_empty_object_fills_every_field_from_defaults() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("bundle_manifest.json"), "{}").unwrap();
    let m = load_evidence_bundle_manifest(dir.path()).unwrap();
    assert_eq!(m.bundle_version, "1.0");
    assert_eq!(m.backend, "dav1d");
}
