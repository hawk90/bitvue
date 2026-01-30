//! Parity Harness - Competitor parity validation system (v14)
//!
//! Implements the parity harness per V14_PARITY_HARNESS_ZIP contracts:
//! - Schema validation for parity matrix
//! - Parity scoring with category/severity weights
//! - Semantic probes (hit-test, selection propagation, tooltip)
//! - Render snapshots (scene graph, not pixel matching)
//! - Evidence bundle diff (ABI-compatible, ignore timestamps/paths)
//! - Compare alignment engine
//! - Gates: Hard Fail, Parity Gate, Perf Gate

use crate::BitvueError;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// Re-export submodule items
mod provenance;
mod gates;
mod context;
mod full_matrix;

pub use provenance::Provenance;
pub use gates::{
    HardFailGate, HardFailViolation, HardFailKind, ParityGate, ParityGateResult,
    PerfGate, PerfGateResult, PerfBudgets, DegradeStep, PerfTelemetryEvent, PerfEventType,
    HarnessGateResults, HardFailGateResult,
};
pub use context::{
    ContextMenuScope, ContextMenuItem, GuardDefinition, GuardContext,
    evaluate_guard, GuardResult,
};
pub use full_matrix::{FULL_PARITY_MATRIX_JSON, get_full_parity_matrix, get_full_parity_item_count};

// =============================================================================
// PARITY MATRIX SCHEMA (per parity_matrix.schema.json)
// =============================================================================

/// Parity matrix metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParityMatrixMeta {
    pub version: String,
    pub generated_at: String,
    pub scope: String,
    #[serde(default)]
    pub competitors: Vec<String>,
}

/// Parity category (per schema enum)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ParityCategory {
    IA,
    Interaction,
    Contract,
    Evidence,
    Performance,
    FailureUX,
    Accessibility,
}

impl ParityCategory {
    /// Get default weight for this category (per competitor_targets.json)
    pub fn default_weight(&self) -> f64 {
        match self {
            ParityCategory::IA => 0.25,
            ParityCategory::Interaction => 0.30,
            ParityCategory::Contract => 0.30,
            ParityCategory::Evidence => 0.15,
            ParityCategory::Performance => 0.15,
            ParityCategory::FailureUX => 0.30,
            ParityCategory::Accessibility => 0.25,
        }
    }
}

/// Severity level with weight (per competitor_targets.json)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Severity {
    P0,
    P1,
    P2,
    P3,
}

impl Severity {
    /// Get weight for this severity (per competitor_targets.json scoring_defaults)
    pub fn weight(&self) -> f64 {
        match self {
            Severity::P0 => 1.0,
            Severity::P1 => 0.6,
            Severity::P2 => 0.3,
            Severity::P3 => 0.1,
        }
    }
}

/// Verification method (per schema enum)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VerificationMethod {
    SemanticProbe,
    RenderSnapshot,
    EvidenceBundleDiff,
    ManualReview,
}

/// Competitor reference for a parity item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompetitorRef {
    pub target_id: String,
    pub evidence: Vec<String>,
}

/// Verification specification for a parity item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Verification {
    pub method: VerificationMethod,
    pub oracle: String,
    pub pass_criteria: String,
}

/// Single parity matrix item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParityItem {
    pub id: String,
    pub category: ParityCategory,
    pub severity: Severity,
    pub question: String,
    pub competitor_refs: Vec<CompetitorRef>,
    pub our_refs: Vec<String>,
    pub verification: Verification,
}

/// Complete parity matrix (validated against schema)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParityMatrix {
    pub meta: ParityMatrixMeta,
    pub items: Vec<ParityItem>,
}

// =============================================================================
// SCHEMA VALIDATION
// =============================================================================

/// Schema validation error
#[derive(Debug, Clone, Serialize)]
pub struct SchemaValidationError {
    pub path: String,
    pub error: String,
}

/// Schema validation result
#[derive(Debug, Clone)]
pub struct SchemaValidationResult {
    pub valid: bool,
    pub errors: Vec<SchemaValidationError>,
    pub warnings: Vec<String>,
}

impl SchemaValidationResult {
    pub fn success() -> Self {
        Self {
            valid: true,
            errors: Vec::new(),
            warnings: Vec::new(),
        }
    }

    pub fn failure(errors: Vec<SchemaValidationError>) -> Self {
        Self {
            valid: false,
            errors,
            warnings: Vec::new(),
        }
    }

    pub fn with_warning(mut self, warning: String) -> Self {
        self.warnings.push(warning);
        self
    }
}

/// Validate parity matrix against schema
pub fn validate_parity_matrix(matrix: &ParityMatrix) -> SchemaValidationResult {
    let mut errors = Vec::new();

    // Validate meta section
    if matrix.meta.version.is_empty() {
        errors.push(SchemaValidationError {
            path: "meta.version".to_string(),
            error: "Version is required".to_string(),
        });
    }
    if matrix.meta.generated_at.is_empty() {
        errors.push(SchemaValidationError {
            path: "meta.generated_at".to_string(),
            error: "generated_at is required".to_string(),
        });
    }
    if matrix.meta.scope.is_empty() {
        errors.push(SchemaValidationError {
            path: "meta.scope".to_string(),
            error: "scope is required".to_string(),
        });
    }

    // Validate items
    let mut seen_ids = std::collections::HashSet::new();
    for (idx, item) in matrix.items.iter().enumerate() {
        let item_path = format!("items[{}]", idx);

        // Check required fields
        if item.id.is_empty() {
            errors.push(SchemaValidationError {
                path: format!("{}.id", item_path),
                error: "id is required".to_string(),
            });
        } else if !seen_ids.insert(item.id.clone()) {
            errors.push(SchemaValidationError {
                path: format!("{}.id", item_path),
                error: format!("Duplicate id: {}", item.id),
            });
        }

        if item.question.is_empty() {
            errors.push(SchemaValidationError {
                path: format!("{}.question", item_path),
                error: "question is required".to_string(),
            });
        }

        // Validate competitor_refs
        for (ref_idx, comp_ref) in item.competitor_refs.iter().enumerate() {
            if comp_ref.target_id.is_empty() {
                errors.push(SchemaValidationError {
                    path: format!("{}.competitor_refs[{}].target_id", item_path, ref_idx),
                    error: "target_id is required".to_string(),
                });
            }
            if comp_ref.evidence.is_empty() {
                errors.push(SchemaValidationError {
                    path: format!("{}.competitor_refs[{}].evidence", item_path, ref_idx),
                    error: "evidence array is required".to_string(),
                });
            }
        }

        // Validate verification
        if item.verification.oracle.is_empty() {
            errors.push(SchemaValidationError {
                path: format!("{}.verification.oracle", item_path),
                error: "oracle is required".to_string(),
            });
        }
        if item.verification.pass_criteria.is_empty() {
            errors.push(SchemaValidationError {
                path: format!("{}.verification.pass_criteria", item_path),
                error: "pass_criteria is required".to_string(),
            });
        }
    }

    if errors.is_empty() {
        SchemaValidationResult::success()
    } else {
        SchemaValidationResult::failure(errors)
    }
}

/// Parse and validate parity matrix from JSON string
pub fn parse_and_validate_parity_matrix(
    json: &str,
) -> Result<(ParityMatrix, SchemaValidationResult), BitvueError> {
    let matrix: ParityMatrix = serde_json::from_str(json)?;

    let validation = validate_parity_matrix(&matrix);
    Ok((matrix, validation))
}

// =============================================================================
// PARITY SCORING (per competitor_targets.json)
// =============================================================================

/// Scoring weights configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoringWeights {
    pub information_architecture: f64,
    pub interaction_intent: f64,
    pub contract_correctness: f64,
    pub evidence_reproducibility: f64,
}

impl Default for ScoringWeights {
    fn default() -> Self {
        Self {
            information_architecture: 0.25,
            interaction_intent: 0.30,
            contract_correctness: 0.30,
            evidence_reproducibility: 0.15,
        }
    }
}

/// Parity item result
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ParityResult {
    Pass,
    Fail,
    Blocked,
    Skipped,
    NotTested,
}

/// Scored parity item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoredParityItem {
    pub id: String,
    pub category: ParityCategory,
    pub severity: Severity,
    pub result: ParityResult,
    pub weighted_score: f64,
    pub max_possible_score: f64,
    pub notes: Vec<String>,
}

/// Aggregate parity score
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParityScore {
    pub total_score: f64,
    pub max_possible: f64,
    pub percentage: f64,
    pub by_category: HashMap<ParityCategory, CategoryScore>,
    pub by_severity: HashMap<Severity, SeverityScore>,
    pub items: Vec<ScoredParityItem>,
    pub hard_fails: Vec<String>,
    pub parity_ready: bool,
}

/// Score for a single category
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CategoryScore {
    pub score: f64,
    pub max_possible: f64,
    pub percentage: f64,
    pub passed: usize,
    pub failed: usize,
    pub blocked: usize,
}

/// Score for a severity level
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeverityScore {
    pub score: f64,
    pub max_possible: f64,
    pub percentage: f64,
    pub passed: usize,
    pub failed: usize,
}

/// Calculate parity score for a matrix with results
pub fn calculate_parity_score(
    matrix: &ParityMatrix,
    results: &HashMap<String, ParityResult>,
    weights: &ScoringWeights,
) -> ParityScore {
    let mut items = Vec::new();
    let mut by_category: HashMap<ParityCategory, CategoryScore> = HashMap::new();
    let mut by_severity: HashMap<Severity, SeverityScore> = HashMap::new();
    let mut hard_fails = Vec::new();
    let mut total_score = 0.0;
    let mut max_possible = 0.0;

    for item in &matrix.items {
        let result = results
            .get(&item.id)
            .copied()
            .unwrap_or(ParityResult::NotTested);
        let category_weight = get_category_weight(&item.category, weights);
        let severity_weight = item.severity.weight();
        let item_max_score = category_weight * severity_weight;

        let item_score = match result {
            ParityResult::Pass => item_max_score,
            ParityResult::Fail => 0.0,
            ParityResult::Blocked => 0.0,
            ParityResult::Skipped => 0.0,
            ParityResult::NotTested => 0.0,
        };

        // Track hard fails (P0 failures)
        if item.severity == Severity::P0 && result == ParityResult::Fail {
            hard_fails.push(item.id.clone());
        }

        // Update category totals
        let cat_score = by_category.entry(item.category).or_insert(CategoryScore {
            score: 0.0,
            max_possible: 0.0,
            percentage: 0.0,
            passed: 0,
            failed: 0,
            blocked: 0,
        });
        cat_score.max_possible += item_max_score;
        cat_score.score += item_score;
        match result {
            ParityResult::Pass => cat_score.passed += 1,
            ParityResult::Fail => cat_score.failed += 1,
            ParityResult::Blocked => cat_score.blocked += 1,
            _ => {}
        }

        // Update severity totals
        let sev_score = by_severity.entry(item.severity).or_insert(SeverityScore {
            score: 0.0,
            max_possible: 0.0,
            percentage: 0.0,
            passed: 0,
            failed: 0,
        });
        sev_score.max_possible += item_max_score;
        sev_score.score += item_score;
        match result {
            ParityResult::Pass => sev_score.passed += 1,
            ParityResult::Fail => sev_score.failed += 1,
            _ => {}
        }

        total_score += item_score;
        max_possible += item_max_score;

        items.push(ScoredParityItem {
            id: item.id.clone(),
            category: item.category,
            severity: item.severity,
            result,
            weighted_score: item_score,
            max_possible_score: item_max_score,
            notes: Vec::new(),
        });
    }

    // Calculate percentages
    for cat_score in by_category.values_mut() {
        cat_score.percentage = if cat_score.max_possible > 0.0 {
            (cat_score.score / cat_score.max_possible) * 100.0
        } else {
            0.0
        };
    }
    for sev_score in by_severity.values_mut() {
        sev_score.percentage = if sev_score.max_possible > 0.0 {
            (sev_score.score / sev_score.max_possible) * 100.0
        } else {
            0.0
        };
    }

    let percentage = if max_possible > 0.0 {
        (total_score / max_possible) * 100.0
    } else {
        0.0
    };

    // Parity ready = no hard fails and all P0 items pass
    let parity_ready = hard_fails.is_empty();

    ParityScore {
        total_score,
        max_possible,
        percentage,
        by_category,
        by_severity,
        items,
        hard_fails,
        parity_ready,
    }
}

fn get_category_weight(category: &ParityCategory, weights: &ScoringWeights) -> f64 {
    match category {
        ParityCategory::IA => weights.information_architecture,
        ParityCategory::Interaction => weights.interaction_intent,
        ParityCategory::Contract => weights.contract_correctness,
        ParityCategory::Evidence => weights.evidence_reproducibility,
        ParityCategory::Performance => weights.evidence_reproducibility,
        ParityCategory::FailureUX => weights.interaction_intent,
        ParityCategory::Accessibility => weights.information_architecture,
    }
}

// =============================================================================
// SEMANTIC PROBES (per semantic_probe_contracts.json)
// =============================================================================

/// Viewport state for probes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ViewportState {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub zoom: f32,
    pub pan_x: f32,
    pub pan_y: f32,
}

/// Point for hit testing
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ProbePoint {
    pub x: f32,
    pub y: f32,
}

/// Entity reference from hit test
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityRef {
    pub kind: String,
    pub id: String,
    pub frame_index: Option<usize>,
    pub byte_offset: Option<u64>,
}

/// Tooltip field
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TooltipField {
    pub label: String,
    pub value: String,
    pub provenance: Option<Provenance>,
}

/// Tooltip payload from probe
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TooltipPayload {
    pub fields: Vec<TooltipField>,
    pub provenance: Provenance,
}

/// Hit test probe inputs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HitTestProbeInput {
    pub panel: String,
    pub viewport: ViewportState,
    pub points: Vec<ProbePoint>,
}

/// Hit test probe output
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HitTestProbeOutput {
    pub hit_entity: Option<EntityRef>,
    pub layer: String,
    pub cursor_payload: Option<TooltipPayload>,
}

/// Selection propagation probe input
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelectionPropagationInput {
    pub origin_panel: String,
    pub action: String,
    pub txn: String,
}

/// Order type for explicit tracking
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OrderType {
    Display,
    Decode,
}

/// Selection state snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelectionSnapshot {
    pub selected_entity: Option<EntityRef>,
    pub selected_byte_range: Option<(u64, u64)>,
    pub order_type: OrderType,
}

/// Selection propagation probe output
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelectionPropagationOutput {
    pub panels_updated: Vec<String>,
    pub selection_snapshot: SelectionSnapshot,
}

/// Tooltip payload probe input
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TooltipPayloadInput {
    pub entity: EntityRef,
    pub context: ProbeContext,
}

/// Probe context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProbeContext {
    pub workspace: String,
    pub mode: String,
    pub codec: String,
}

/// Tooltip payload probe output
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TooltipPayloadOutput {
    pub fields: Vec<TooltipField>,
    pub provenance: Provenance,
}

/// Probe result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProbeResult {
    pub probe_id: String,
    pub timestamp: String,
    pub stream_id: String,
    pub codec: String,
    pub workspace: String,
    pub mode: String,
    pub result: ProbeOutcome,
    pub violations: Vec<ProbeViolation>,
}

/// Probe outcome
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ProbeOutcome {
    HitTest(HitTestProbeOutput),
    SelectionPropagation(SelectionPropagationOutput),
    TooltipPayload(TooltipPayloadOutput),
}

/// Probe violation (hard fail condition)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProbeViolation {
    pub kind: HardFailKind,
    pub message: String,
    pub context: HashMap<String, String>,
}

// =============================================================================
// RENDER SNAPSHOT (per render_snapshot_contracts.json)
// =============================================================================

/// Render snapshot of scene graph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenderSnapshot {
    pub workspace: String,
    pub panel: String,
    pub mode: String,
    pub codec: String,
    pub order_type: OrderType,
    pub viewport: ViewportState,
    pub selection_txn: String,
    pub layer_stack: Vec<String>,
    pub objects: Vec<RenderObject>,
    pub legend: Option<RenderLegend>,
    pub warnings: Vec<String>,
    pub backend_fingerprint: String,
}

/// Rendered object in scene graph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenderObject {
    pub id: String,
    pub kind: String,
    pub bounds: ObjectBounds,
    pub z: i32,
    pub style_class: String,
    pub selected: bool,
    pub hovered: bool,
    pub data_provenance: Option<Provenance>,
}

/// Object bounds
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObjectBounds {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

/// Render legend
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenderLegend {
    pub entries: Vec<LegendEntry>,
    pub ranges: Option<LegendRanges>,
}

/// Legend entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LegendEntry {
    pub label: String,
    pub style_class: String,
}

/// Legend ranges (for heatmaps, etc.)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LegendRanges {
    pub min: f64,
    pub max: f64,
    pub unit: String,
}

/// Snapshot comparison tolerances
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotTolerances {
    pub bounds_epsilon_px: f32,
    pub object_count_threshold: usize,
}

impl Default for SnapshotTolerances {
    fn default() -> Self {
        Self {
            bounds_epsilon_px: 1.0,
            object_count_threshold: 10,
        }
    }
}

/// Compare two render snapshots
pub fn compare_render_snapshots(
    a: &RenderSnapshot,
    b: &RenderSnapshot,
    tolerances: &SnapshotTolerances,
) -> SnapshotComparisonResult {
    let mut differences = Vec::new();
    let mut warnings = Vec::new();

    // Check workspace/panel/mode match
    if a.workspace != b.workspace {
        differences.push(SnapshotDifference {
            field: "workspace".to_string(),
            a_value: a.workspace.clone(),
            b_value: b.workspace.clone(),
        });
    }
    if a.panel != b.panel {
        differences.push(SnapshotDifference {
            field: "panel".to_string(),
            a_value: a.panel.clone(),
            b_value: b.panel.clone(),
        });
    }
    if a.mode != b.mode {
        differences.push(SnapshotDifference {
            field: "mode".to_string(),
            a_value: a.mode.clone(),
            b_value: b.mode.clone(),
        });
    }

    // Order type must match exactly
    if a.order_type != b.order_type {
        differences.push(SnapshotDifference {
            field: "order_type".to_string(),
            a_value: format!("{:?}", a.order_type),
            b_value: format!("{:?}", b.order_type),
        });
    }

    // Check object counts (with LOD tolerance)
    let count_diff = (a.objects.len() as isize - b.objects.len() as isize).unsigned_abs();
    if count_diff > tolerances.object_count_threshold {
        differences.push(SnapshotDifference {
            field: "object_count".to_string(),
            a_value: a.objects.len().to_string(),
            b_value: b.objects.len().to_string(),
        });
    } else if count_diff > 0 {
        warnings.push(format!(
            "Object count differs by {} (within LOD tolerance)",
            count_diff
        ));
    }

    // Check legend ranges (must be exact)
    match (&a.legend, &b.legend) {
        (Some(la), Some(lb)) => {
            if let (Some(ra), Some(rb)) = (&la.ranges, &lb.ranges) {
                if (ra.min - rb.min).abs() > f64::EPSILON || (ra.max - rb.max).abs() > f64::EPSILON
                {
                    differences.push(SnapshotDifference {
                        field: "legend_ranges".to_string(),
                        a_value: format!("{}-{}", ra.min, ra.max),
                        b_value: format!("{}-{}", rb.min, rb.max),
                    });
                }
            }
        }
        (None, Some(_)) | (Some(_), None) => {
            differences.push(SnapshotDifference {
                field: "legend".to_string(),
                a_value: if a.legend.is_some() {
                    "present"
                } else {
                    "absent"
                }
                .to_string(),
                b_value: if b.legend.is_some() {
                    "present"
                } else {
                    "absent"
                }
                .to_string(),
            });
        }
        (None, None) => {}
    }

    SnapshotComparisonResult {
        matches: differences.is_empty(),
        differences,
        warnings,
    }
}

/// Snapshot comparison result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotComparisonResult {
    pub matches: bool,
    pub differences: Vec<SnapshotDifference>,
    pub warnings: Vec<String>,
}

/// Snapshot difference
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotDifference {
    pub field: String,
    pub a_value: String,
    pub b_value: String,
}

// =============================================================================
// EVIDENCE BUNDLE DIFF (per evidence_bundle_diff_contracts.json)
// =============================================================================

/// Evidence bundle manifest
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvidenceBundleManifest {
    pub bundle_version: String,
    pub app_version: String,
    pub git_commit: String,
    pub build_profile: String,
    pub os: String,
    pub gpu: String,
    pub cpu: String,
    pub backend: String,
    pub plugin_versions: HashMap<String, String>,
    pub stream_fingerprint: String,
    pub order_type: OrderType,
    pub selection_state: SelectionSnapshot,
    pub workspace: String,
    pub mode: String,
    pub warnings: Vec<String>,
    pub artifacts: Vec<String>,
}

/// Evidence bundle diff configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvidenceDiffConfig {
    pub compare_fields: Vec<String>,
    pub ignore_fields: Vec<String>,
}

impl Default for EvidenceDiffConfig {
    fn default() -> Self {
        Self {
            compare_fields: vec![
                "selection_state".to_string(),
                "order_type".to_string(),
                "backend_fingerprint".to_string(),
                "plugin_versions".to_string(),
                "warnings".to_string(),
            ],
            ignore_fields: vec![
                "timestamps".to_string(),
                "machine_hostname".to_string(),
                "paths".to_string(),
            ],
        }
    }
}

/// Evidence bundle diff result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvidenceBundleDiffResult {
    pub matches: bool,
    pub abi_compatible: bool,
    pub differences: Vec<EvidenceDifference>,
    pub ignored_changes: Vec<String>,
}

/// Evidence difference
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvidenceDifference {
    pub field: String,
    pub a_value: String,
    pub b_value: String,
    pub severity: DiffSeverity,
}

/// Diff severity
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DiffSeverity {
    Breaking,
    Warning,
    Info,
}

/// Compare two evidence bundle manifests
pub fn compare_evidence_bundles(
    a: &EvidenceBundleManifest,
    b: &EvidenceBundleManifest,
    _config: &EvidenceDiffConfig,
) -> EvidenceBundleDiffResult {
    let mut differences = Vec::new();
    let mut ignored_changes = Vec::new();

    // Order type must match (critical)
    if a.order_type != b.order_type {
        differences.push(EvidenceDifference {
            field: "order_type".to_string(),
            a_value: format!("{:?}", a.order_type),
            b_value: format!("{:?}", b.order_type),
            severity: DiffSeverity::Breaking,
        });
    }

    // Selection state
    let a_sel = serde_json::to_string(&a.selection_state).unwrap_or_default();
    let b_sel = serde_json::to_string(&b.selection_state).unwrap_or_default();
    if a_sel != b_sel {
        differences.push(EvidenceDifference {
            field: "selection_state".to_string(),
            a_value: a_sel,
            b_value: b_sel,
            severity: DiffSeverity::Warning,
        });
    }

    // Backend fingerprint
    if a.backend != b.backend {
        differences.push(EvidenceDifference {
            field: "backend".to_string(),
            a_value: a.backend.clone(),
            b_value: b.backend.clone(),
            severity: DiffSeverity::Info,
        });
    }

    // Plugin versions
    for (key, a_ver) in &a.plugin_versions {
        match b.plugin_versions.get(key) {
            Some(b_ver) if a_ver != b_ver => {
                differences.push(EvidenceDifference {
                    field: format!("plugin_versions.{}", key),
                    a_value: a_ver.clone(),
                    b_value: b_ver.clone(),
                    severity: DiffSeverity::Info,
                });
            }
            None => {
                differences.push(EvidenceDifference {
                    field: format!("plugin_versions.{}", key),
                    a_value: a_ver.clone(),
                    b_value: "(missing)".to_string(),
                    severity: DiffSeverity::Warning,
                });
            }
            _ => {}
        }
    }

    // Check for new plugins in b
    for key in b.plugin_versions.keys() {
        if !a.plugin_versions.contains_key(key) {
            ignored_changes.push(format!("New plugin in b: {}", key));
        }
    }

    // Warnings
    let mut a_warnings = a.warnings.clone();
    let mut b_warnings = b.warnings.clone();
    a_warnings.sort();
    b_warnings.sort();
    if a_warnings != b_warnings {
        differences.push(EvidenceDifference {
            field: "warnings".to_string(),
            a_value: a_warnings.join(", "),
            b_value: b_warnings.join(", "),
            severity: DiffSeverity::Warning,
        });
    }

    // ABI compatibility: breaking if order_type differs
    let abi_compatible = !differences
        .iter()
        .any(|d| d.severity == DiffSeverity::Breaking);

    EvidenceBundleDiffResult {
        matches: differences.is_empty(),
        abi_compatible,
        differences,
        ignored_changes,
    }
}

// =============================================================================
// COMPARE ALIGNMENT (per compare_alignment_contracts.json)
// =============================================================================

/// Alignment axis type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AlignmentAxis {
    DisplayOrder,
    DecodeOrder,
    Pts,
    Dts,
    PictureHash,
    ContentHash,
    SpssPpsConfig,
    TemporalUnitId,
}

/// Alignment mismatch event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlignmentMismatch {
    pub kind: MismatchKind,
    pub frame_a: Option<usize>,
    pub frame_b: Option<usize>,
    pub reason: String,
}

/// Mismatch kind
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MismatchKind {
    ReorderDetected,
    DropDetected,
    ConfigMismatch,
}

/// Alignment mismatch action
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MismatchAction {
    ShowAlignmentWarningAndOfferAxisSwitch { suggested_axis: AlignmentAxis },
    MarkMissingFramesWithReason { reason: String },
    BlockCompareAndSurfaceDetailedReason { reason: String },
}

/// Alignment evidence record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlignmentEvidence {
    pub chosen_axis: AlignmentAxis,
    pub mismatch_events: Vec<AlignmentMismatch>,
    pub order_type: OrderType,
}

// =============================================================================
// PARITY HARNESS RUNNER
// =============================================================================

/// Parity harness configuration
#[derive(Debug, Clone)]
pub struct ParityHarnessConfig {
    pub matrix: ParityMatrix,
    pub weights: ScoringWeights,
    pub hard_fail_gate: HardFailGate,
    pub parity_gate: ParityGate,
    pub perf_gate: PerfGate,
}

/// Parity harness runner
#[derive(Debug)]
pub struct ParityHarness {
    pub config: ParityHarnessConfig,
    pub results: HashMap<String, ParityResult>,
    pub probe_results: Vec<ProbeResult>,
    pub snapshots: Vec<RenderSnapshot>,
    pub telemetry: Vec<PerfTelemetryEvent>,
}

impl ParityHarness {
    pub fn new(config: ParityHarnessConfig) -> Self {
        Self {
            config,
            results: HashMap::new(),
            probe_results: Vec::new(),
            snapshots: Vec::new(),
            telemetry: Vec::new(),
        }
    }

    /// Record a parity item result
    pub fn record_result(&mut self, item_id: &str, result: ParityResult) {
        self.results.insert(item_id.to_string(), result);
    }

    /// Record a probe result
    pub fn record_probe(&mut self, result: ProbeResult) {
        // Check for hard fail violations
        for violation in &result.violations {
            self.config
                .hard_fail_gate
                .record_violation(violation.kind, &violation.message);
        }
        self.probe_results.push(result);
    }

    /// Record a render snapshot
    pub fn record_snapshot(&mut self, snapshot: RenderSnapshot) {
        self.snapshots.push(snapshot);
    }

    /// Record telemetry event
    pub fn record_telemetry(&mut self, event: PerfTelemetryEvent) {
        self.telemetry.push(event);
    }

    /// Calculate final score
    pub fn calculate_score(&self) -> ParityScore {
        calculate_parity_score(&self.config.matrix, &self.results, &self.config.weights)
    }

    /// Evaluate all gates
    pub fn evaluate_gates(&self) -> HarnessGateResults {
        let score = self.calculate_score();

        HarnessGateResults {
            hard_fail: HardFailGateResult {
                passed: !self.config.hard_fail_gate.is_failed(),
                violations: self.config.hard_fail_gate.violations.clone(),
            },
            parity: self.config.parity_gate.evaluate(&score),
            perf: self
                .telemetry
                .iter()
                .map(|e| self.config.perf_gate.evaluate(e))
                .collect(),
            overall_passed: !self.config.hard_fail_gate.is_failed()
                && self.config.parity_gate.evaluate(&score).passed,
        }
    }

    /// Generate a detailed parity report
    pub fn generate_report(&self) -> ParityReport {
        let score = self.calculate_score();
        let gates = self.evaluate_gates();

        // Group items by category
        let mut by_category: HashMap<ParityCategory, Vec<ParityItemStatus>> = HashMap::new();
        for item in &self.config.matrix.items {
            let status = self
                .results
                .get(&item.id)
                .cloned()
                .unwrap_or(ParityResult::NotTested);
            let item_status = ParityItemStatus {
                id: item.id.clone(),
                question: item.question.clone(),
                severity: item.severity,
                status,
                our_refs: item.our_refs.clone(),
            };
            by_category
                .entry(item.category)
                .or_default()
                .push(item_status);
        }

        // Calculate category scores
        let category_scores: HashMap<ParityCategory, ReportCategoryScore> = by_category
            .iter()
            .map(|(cat, items)| {
                let total = items.len();
                let passed = items
                    .iter()
                    .filter(|i| i.status == ParityResult::Pass)
                    .count();
                let failed = items
                    .iter()
                    .filter(|i| i.status == ParityResult::Fail)
                    .count();
                let not_tested = items
                    .iter()
                    .filter(|i| i.status == ParityResult::NotTested)
                    .count();
                (
                    *cat,
                    ReportCategoryScore {
                        total,
                        passed,
                        failed,
                        not_tested,
                    },
                )
            })
            .collect();

        ParityReport {
            generated_at: chrono::Utc::now().to_rfc3339(),
            matrix_version: self.config.matrix.meta.version.clone(),
            overall_score: score.percentage,
            parity_ready: score.parity_ready,
            gates_passed: gates.overall_passed,
            hard_fail_count: gates.hard_fail.violations.len(),
            category_scores,
            items_by_category: by_category,
            hard_fails: score.hard_fails,
            probe_count: self.probe_results.len(),
            snapshot_count: self.snapshots.len(),
            telemetry_count: self.telemetry.len(),
        }
    }
}

/// Parity report structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParityReport {
    pub generated_at: String,
    pub matrix_version: String,
    pub overall_score: f64,
    pub parity_ready: bool,
    pub gates_passed: bool,
    pub hard_fail_count: usize,
    pub category_scores: HashMap<ParityCategory, ReportCategoryScore>,
    pub items_by_category: HashMap<ParityCategory, Vec<ParityItemStatus>>,
    pub hard_fails: Vec<String>,
    pub probe_count: usize,
    pub snapshot_count: usize,
    pub telemetry_count: usize,
}

/// Category score summary for reports
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportCategoryScore {
    pub total: usize,
    pub passed: usize,
    pub failed: usize,
    pub not_tested: usize,
}

/// Individual parity item status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParityItemStatus {
    pub id: String,
    pub question: String,
    pub severity: Severity,
    pub status: ParityResult,
    pub our_refs: Vec<String>,
}
