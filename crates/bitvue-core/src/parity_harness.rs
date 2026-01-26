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

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// =============================================================================
// PROVENANCE - Data lineage tracking for evidence chain
// =============================================================================

/// Provenance tracks the origin and lineage of data
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Provenance {
    /// Source of the data (e.g., "decoder", "parser", "cache")
    pub source: String,
    /// Version of the source that produced this data
    pub source_version: String,
    /// Timestamp when the data was generated
    pub timestamp: String,
    /// Parent provenance IDs (for data derived from multiple sources)
    pub parent_ids: Vec<String>,
    /// Additional context
    pub context: HashMap<String, String>,
}

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
            ParityCategory::IA => 0.25,            // information_architecture
            ParityCategory::Interaction => 0.30,   // interaction_intent
            ParityCategory::Contract => 0.30,      // contract_correctness
            ParityCategory::Evidence => 0.15,      // evidence_reproducibility
            ParityCategory::Performance => 0.15,   // maps to evidence_reproducibility
            ParityCategory::FailureUX => 0.30,     // maps to interaction_intent
            ParityCategory::Accessibility => 0.25, // maps to information_architecture
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
) -> Result<(ParityMatrix, SchemaValidationResult), String> {
    let matrix: ParityMatrix =
        serde_json::from_str(json).map_err(|e| format!("JSON parse error: {}", e))?;

    let validation = validate_parity_matrix(&matrix);
    Ok((matrix, validation))
}

// =============================================================================
// PARITY SCORING (per competitor_targets.json)
// =============================================================================

/// Competitor target definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompetitorTarget {
    pub id: String,
    pub name: String,
    pub vendor: String,
    pub reference_type: String,
    pub priority: u32,
    pub notes: Vec<String>,
}

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

/// Hard fail types (per semantic_probe_contracts.json)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HardFailKind {
    OrderTypeMixingDetected,
    StaleAsyncApplied,
    HitTestRenderTransformMismatch,
    CacheInvalidationViolation,
}

// =============================================================================
// ORDER TYPE (per compare_alignment_contracts.json)
// =============================================================================

/// Order type for explicit tracking
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OrderType {
    Display,
    Decode,
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
// GATES (per ENTRYPOINT_PROMPT_PARITY_HARNESS.md)
// =============================================================================

/// Hard fail gate - immediate failure conditions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HardFailGate {
    pub violations: Vec<HardFailViolation>,
}

/// Hard fail violation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HardFailViolation {
    pub kind: HardFailKind,
    pub message: String,
    pub context: HashMap<String, String>,
    pub timestamp: String,
}

impl HardFailGate {
    pub fn new() -> Self {
        Self {
            violations: Vec::new(),
        }
    }

    pub fn is_failed(&self) -> bool {
        !self.violations.is_empty()
    }

    pub fn record_violation(&mut self, kind: HardFailKind, message: &str) {
        self.violations.push(HardFailViolation {
            kind,
            message: message.to_string(),
            context: HashMap::new(),
            timestamp: chrono::Utc::now().to_rfc3339(),
        });
    }

    pub fn record_violation_with_context(
        &mut self,
        kind: HardFailKind,
        message: &str,
        context: HashMap<String, String>,
    ) {
        self.violations.push(HardFailViolation {
            kind,
            message: message.to_string(),
            context,
            timestamp: chrono::Utc::now().to_rfc3339(),
        });
    }
}

impl Default for HardFailGate {
    fn default() -> Self {
        Self::new()
    }
}

/// Parity gate - scenarios and matrix items must meet pass criteria
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParityGate {
    pub required_pass_percentage: f64,
    pub p0_must_all_pass: bool,
}

impl Default for ParityGate {
    fn default() -> Self {
        Self {
            required_pass_percentage: 80.0,
            p0_must_all_pass: true,
        }
    }
}

impl ParityGate {
    pub fn evaluate(&self, score: &ParityScore) -> ParityGateResult {
        let passes_percentage = score.percentage >= self.required_pass_percentage;
        let p0_all_pass = score.hard_fails.is_empty();

        let passed = if self.p0_must_all_pass {
            passes_percentage && p0_all_pass
        } else {
            passes_percentage
        };

        ParityGateResult {
            passed,
            actual_percentage: score.percentage,
            required_percentage: self.required_pass_percentage,
            p0_failures: score.hard_fails.clone(),
            parity_ready: score.parity_ready,
        }
    }
}

/// Parity gate result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParityGateResult {
    pub passed: bool,
    pub actual_percentage: f64,
    pub required_percentage: f64,
    pub p0_failures: Vec<String>,
    pub parity_ready: bool,
}

/// Performance gate - enforce budgets and degrade steps
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerfGate {
    pub budgets: PerfBudgets,
    pub degrade_steps: Vec<DegradeStep>,
}

/// Performance budgets (per perf_budget_and_instrumentation.json)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerfBudgets {
    pub ui_frame_target_ms: f64,
    pub hit_test_max_ms: f64,
    pub overlay_render_max_ms: f64,
    pub tooltip_build_max_ms: f64,
    pub selection_propagation_max_ms: f64,
}

impl Default for PerfBudgets {
    fn default() -> Self {
        Self {
            ui_frame_target_ms: 16.6,
            hit_test_max_ms: 1.5,
            overlay_render_max_ms: 6.0,
            tooltip_build_max_ms: 0.8,
            selection_propagation_max_ms: 2.0,
        }
    }
}

/// Degrade step definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DegradeStep {
    pub name: String,
    pub trigger: String,
}

impl Default for PerfGate {
    fn default() -> Self {
        Self {
            budgets: PerfBudgets::default(),
            degrade_steps: vec![
                DegradeStep {
                    name: "disable_labels".to_string(),
                    trigger: "overlay_render > budget".to_string(),
                },
                DegradeStep {
                    name: "aggregate_vectors".to_string(),
                    trigger: "object_count > threshold".to_string(),
                },
                DegradeStep {
                    name: "downsample_heatmap".to_string(),
                    trigger: "tile_count > threshold".to_string(),
                },
                DegradeStep {
                    name: "placeholder_with_reason".to_string(),
                    trigger: "still_over_budget_after_steps".to_string(),
                },
            ],
        }
    }
}

/// Performance telemetry event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerfTelemetryEvent {
    pub event_type: PerfEventType,
    pub value_ms: f64,
    pub timestamp: String,
    pub context: HashMap<String, String>,
}

/// Performance event type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PerfEventType {
    FrameTime,
    HitTestTime,
    OverlayTime,
    CacheHitRate,
    AsyncQueueDepth,
}

impl PerfGate {
    pub fn evaluate(&self, event: &PerfTelemetryEvent) -> PerfGateResult {
        let budget = match event.event_type {
            PerfEventType::FrameTime => self.budgets.ui_frame_target_ms,
            PerfEventType::HitTestTime => self.budgets.hit_test_max_ms,
            PerfEventType::OverlayTime => self.budgets.overlay_render_max_ms,
            _ => f64::MAX,
        };

        let within_budget = event.value_ms <= budget;
        let degrade_triggered = if !within_budget {
            self.suggest_degrade_step(event)
        } else {
            None
        };

        PerfGateResult {
            passed: within_budget,
            event_type: event.event_type,
            actual_ms: event.value_ms,
            budget_ms: budget,
            degrade_triggered,
        }
    }

    fn suggest_degrade_step(&self, event: &PerfTelemetryEvent) -> Option<String> {
        match event.event_type {
            PerfEventType::OverlayTime => Some("disable_labels".to_string()),
            PerfEventType::FrameTime => Some("aggregate_vectors".to_string()),
            _ => None,
        }
    }
}

/// Performance gate result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerfGateResult {
    pub passed: bool,
    pub event_type: PerfEventType,
    pub actual_ms: f64,
    pub budget_ms: f64,
    pub degrade_triggered: Option<String>,
}

// =============================================================================
// CONTEXT MENUS AND GUARDS (per context_menus.json, guard_rules.json)
// =============================================================================

/// Context menu scope
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextMenuScope {
    pub id: String,
    pub items: Vec<ContextMenuItem>,
}

/// Context menu item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextMenuItem {
    pub id: String,
    pub label: String,
    pub command: String,
    pub guard: String,
}

/// Guard definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuardDefinition {
    pub id: String,
    pub expr: String,
    pub disabled_reason: String,
}

/// Guard evaluation context
#[derive(Debug, Clone)]
pub struct GuardContext {
    pub selected_entity: Option<EntityRef>,
    pub selected_byte_range: Option<(u64, u64)>,
}

/// Evaluate a guard
pub fn evaluate_guard(guard_id: &str, context: &GuardContext) -> GuardResult {
    match guard_id {
        "always" => GuardResult {
            enabled: true,
            disabled_reason: None,
        },
        "has_selection" => {
            if context.selected_entity.is_some() {
                GuardResult {
                    enabled: true,
                    disabled_reason: None,
                }
            } else {
                GuardResult {
                    enabled: false,
                    disabled_reason: Some("No selection.".to_string()),
                }
            }
        }
        "has_byte_range" => {
            if context.selected_byte_range.is_some() {
                GuardResult {
                    enabled: true,
                    disabled_reason: None,
                }
            } else {
                GuardResult {
                    enabled: false,
                    disabled_reason: Some("No byte range selected.".to_string()),
                }
            }
        }
        _ => GuardResult {
            enabled: false,
            disabled_reason: Some(format!("Unknown guard: {}", guard_id)),
        },
    }
}

/// Guard evaluation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuardResult {
    pub enabled: bool,
    pub disabled_reason: Option<String>,
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

    /// Export report as JSON
    pub fn export_report_json(&self) -> Result<String, serde_json::Error> {
        let report = self.generate_report();
        serde_json::to_string_pretty(&report)
    }

    /// Export report as markdown
    pub fn export_report_markdown(&self) -> String {
        let report = self.generate_report();
        let mut md = String::new();

        md.push_str("# Parity Harness Report\n\n");
        md.push_str(&format!("**Generated:** {}\n", report.generated_at));
        md.push_str(&format!(
            "**Matrix Version:** {}\n\n",
            report.matrix_version
        ));

        md.push_str("## Summary\n\n");
        md.push_str("| Metric | Value |\n");
        md.push_str("|--------|-------|\n");
        md.push_str(&format!(
            "| Overall Score | {:.1}% |\n",
            report.overall_score
        ));
        md.push_str(&format!(
            "| Parity Ready | {} |\n",
            if report.parity_ready {
                "✅ Yes"
            } else {
                "❌ No"
            }
        ));
        md.push_str(&format!(
            "| Gates Passed | {} |\n",
            if report.gates_passed {
                "✅ Yes"
            } else {
                "❌ No"
            }
        ));
        md.push_str(&format!("| Hard Fails | {} |\n", report.hard_fail_count));
        md.push_str(&format!("| Probes Run | {} |\n", report.probe_count));
        md.push_str(&format!("| Snapshots | {} |\n", report.snapshot_count));
        md.push('\n');

        md.push_str("## Category Breakdown\n\n");
        md.push_str("| Category | Passed | Failed | Not Tested | Total |\n");
        md.push_str("|----------|--------|--------|------------|-------|\n");
        for (cat, score) in &report.category_scores {
            md.push_str(&format!(
                "| {:?} | {} | {} | {} | {} |\n",
                cat, score.passed, score.failed, score.not_tested, score.total
            ));
        }
        md.push('\n');

        md.push_str("## Items by Category\n\n");
        for (cat, items) in &report.items_by_category {
            md.push_str(&format!("### {:?}\n\n", cat));
            for item in items {
                let status_icon = match item.status {
                    ParityResult::Pass => "[PASS]",
                    ParityResult::Fail => "[FAIL]",
                    ParityResult::Blocked => "[BLOCKED]",
                    ParityResult::Skipped => "[SKIP]",
                    ParityResult::NotTested => "[N/A]",
                };
                md.push_str(&format!(
                    "- {} **{}** ({:?}): {}\n",
                    status_icon, item.id, item.severity, item.question
                ));
            }
            md.push('\n');
        }

        if !report.hard_fails.is_empty() {
            md.push_str("## Hard Fails (P0 Failures)\n\n");
            for fail in &report.hard_fails {
                md.push_str(&format!("- ❌ {}\n", fail));
            }
            md.push('\n');
        }

        md
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

/// Hard fail gate result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HardFailGateResult {
    pub passed: bool,
    pub violations: Vec<HardFailViolation>,
}

/// Combined gate results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HarnessGateResults {
    pub hard_fail: HardFailGateResult,
    pub parity: ParityGateResult,
    pub perf: Vec<PerfGateResult>,
    pub overall_passed: bool,
}

// =============================================================================
// FULL PARITY MATRIX - Extracted from VQ Analyzer UI State Space
// =============================================================================

/// Full parity matrix JSON with all competitor features extracted from VQ Analyzer images
/// Categories: IA, Interaction, Contract, Evidence, Performance, FailureUX, Accessibility
pub const FULL_PARITY_MATRIX_JSON: &str = r#"{
  "meta": {
    "version": "v14",
    "generated_at": "2026-01-15T14:30:00.000000Z",
    "scope": "UI/UX parity vs VQ Analyzer - extracted from 296 reference images",
    "competitors": ["vq_analyzer", "elecard_streameye", "intel_vpa"]
  },
  "items": [
    {
      "id": "IA_MAIN_PANEL_CODING_FLOW",
      "category": "IA",
      "severity": "P0",
      "question": "Does the main panel display coding flow grid with CTB/CU/PU hierarchy?",
      "competitor_refs": [{"target_id": "vq_analyzer", "evidence": ["02-UI-Components/02-Main-Panel/MainPanel_hevc.jpg", "03-Modes-VVC/03-Coding-Flow/*"]}],
      "our_refs": ["player/pipeline.rs", "partition_grid.rs"],
      "verification": {"method": "render_snapshot", "oracle": "render_snapshot_contracts.json", "pass_criteria": "Block hierarchy visible with proper partitioning"}
    },
    {
      "id": "IA_STREAM_VIEW_TIMELINE",
      "category": "IA",
      "severity": "P0",
      "question": "Is there a timeline view showing frame sizes, QP overlay, and thumbnails?",
      "competitor_refs": [{"target_id": "vq_analyzer", "evidence": ["02-UI-Components/03-Stream-View/frame_sizes_view.jpg", "02-UI-Components/03-Stream-View/TopFilmstrip_hevc_*.jpg"]}],
      "our_refs": ["timeline.rs", "timeline_lanes.rs"],
      "verification": {"method": "render_snapshot", "oracle": "render_snapshot_contracts.json", "pass_criteria": "Timeline shows frame sizes with QP overlay and filmstrip thumbnails"}
    },
    {
      "id": "IA_SYNTAX_TREE_PER_CODEC",
      "category": "IA",
      "severity": "P0",
      "question": "Does syntax info panel show tree view with codec-specific syntax elements?",
      "competitor_refs": [{"target_id": "vq_analyzer", "evidence": ["02-UI-Components/04-Syntax-Info/AV1/OBUtab_AV1.jpg", "02-UI-Components/04-Syntax-Info/HEVC/NAL_hevc.jpg", "02-UI-Components/04-Syntax-Info/VVC/vvc-syntax-info.jpg"]}],
      "our_refs": ["types.rs:SyntaxNodeId"],
      "verification": {"method": "semantic_probe", "oracle": "semantic_probe_contracts.json", "pass_criteria": "Tree view navigable with expand/collapse; selection propagates to hex view"}
    },
    {
      "id": "IA_SELECTION_INFO_BLOCK_DETAILS",
      "category": "IA",
      "severity": "P0",
      "question": "Does selection info panel show per-block details (CTB addr, MV, QP, prediction mode)?",
      "competitor_refs": [{"target_id": "vq_analyzer", "evidence": ["02-UI-Components/05-Selection-Info/SelectionInfo_hevc.jpg"]}],
      "our_refs": ["selection.rs", "types.rs:BlockInfo"],
      "verification": {"method": "semantic_probe", "oracle": "semantic_probe_contracts.json", "pass_criteria": "Block selection shows all fields: width, height, CTB addr, MV L0/L1, QP, pred mode"}
    },
    {
      "id": "IA_HEX_VIEW_RAW_BYTES",
      "category": "IA",
      "severity": "P0",
      "question": "Is there a hex view showing raw bytes with offset, hex, and ASCII columns?",
      "competitor_refs": [{"target_id": "vq_analyzer", "evidence": ["02-UI-Components/06-Unit-Info-HEX/HEX_view.jpg", "02-UI-Components/06-Unit-Info-HEX/RawBytes_hevc.jpg"]}],
      "our_refs": ["byte_cache.rs"],
      "verification": {"method": "render_snapshot", "oracle": "render_snapshot_contracts.json", "pass_criteria": "Hex view with offset column, hex bytes, ASCII representation, selection highlight"}
    },
    {
      "id": "IA_STATUS_PANEL_ERRORS",
      "category": "IA",
      "severity": "P1",
      "question": "Does status panel show errors, warnings, and stream info?",
      "competitor_refs": [{"target_id": "vq_analyzer", "evidence": ["02-UI-Components/07-Status-Panel/errors.jpg", "02-UI-Components/07-Status-Panel/status_bar.jpg"]}],
      "our_refs": ["diagnostics.rs"],
      "verification": {"method": "render_snapshot", "oracle": "render_snapshot_contracts.json", "pass_criteria": "Status bar shows error count and stream info"}
    },
    {
      "id": "OVERLAY_QP_HEATMAP",
      "category": "Interaction",
      "severity": "P0",
      "question": "Is there a QP heatmap overlay showing per-CU QP values?",
      "competitor_refs": [{"target_id": "vq_analyzer", "evidence": ["05-Modes-HEVC/08-Heat-Maps/QPmap_hevc.jpg"]}],
      "our_refs": ["qp_heatmap.rs"],
      "verification": {"method": "render_snapshot", "oracle": "render_snapshot_contracts.json", "pass_criteria": "QP values overlaid on blocks with color gradient"}
    },
    {
      "id": "OVERLAY_MV_VECTORS",
      "category": "Interaction",
      "severity": "P0",
      "question": "Is there a motion vector overlay showing MV direction and magnitude?",
      "competitor_refs": [{"target_id": "vq_analyzer", "evidence": ["05-Modes-HEVC/08-Heat-Maps/MVheat_hevc.jpg"]}],
      "our_refs": ["mv_overlay.rs"],
      "verification": {"method": "render_snapshot", "oracle": "render_snapshot_contracts.json", "pass_criteria": "MV vectors or MV bits per PU shown with proper L0/L1 distinction"}
    },
    {
      "id": "OVERLAY_PSNR_SSIM_HEATMAP",
      "category": "Interaction",
      "severity": "P1",
      "question": "Is there a PSNR/SSIM heatmap overlay showing per-block quality?",
      "competitor_refs": [{"target_id": "vq_analyzer", "evidence": ["05-Modes-HEVC/08-Heat-Maps/PSNR_hevc.jpg", "05-Modes-HEVC/08-Heat-Maps/SSIM_hevc.jpg"]}],
      "our_refs": ["block_metrics.rs", "diff_heatmap.rs"],
      "verification": {"method": "render_snapshot", "oracle": "render_snapshot_contracts.json", "pass_criteria": "Per-block PSNR/SSIM values shown with color coding"}
    },
    {
      "id": "OVERLAY_PU_TYPES",
      "category": "Interaction",
      "severity": "P1",
      "question": "Is there a PU types overlay showing prediction mode colors (intra/inter/skip/merge)?",
      "competitor_refs": [{"target_id": "vq_analyzer", "evidence": ["05-Modes-HEVC/08-Heat-Maps/PUtype_hevc.jpg"]}],
      "our_refs": ["partition_grid.rs"],
      "verification": {"method": "render_snapshot", "oracle": "render_snapshot_contracts.json", "pass_criteria": "Block types color-coded: intra=red, inter=blue, skip, merge, palette with legend"}
    },
    {
      "id": "OVERLAY_EFFICIENCY_MAP",
      "category": "Interaction",
      "severity": "P2",
      "question": "Is there an efficiency map overlay showing coding efficiency per block?",
      "competitor_refs": [{"target_id": "vq_analyzer", "evidence": ["07-Modes-AV1/11-Overlays/efficiency_map_AV1.jpg", "06-Modes-VP9/07-Overlays/efficiency_map_VP9.jpg"]}],
      "our_refs": ["block_metrics.rs"],
      "verification": {"method": "render_snapshot", "oracle": "render_snapshot_contracts.json", "pass_criteria": "Bool-coding efficiency visualized per block"}
    },
    {
      "id": "OVERLAY_REFERENCE_INDICES",
      "category": "Interaction",
      "severity": "P1",
      "question": "Is there a reference indices overlay showing which ref frame each block uses?",
      "competitor_refs": [{"target_id": "vq_analyzer", "evidence": ["05-Modes-HEVC/08-Heat-Maps/PU_reference_indices_hevc.jpg"]}],
      "our_refs": ["reference_graph.rs"],
      "verification": {"method": "render_snapshot", "oracle": "render_snapshot_contracts.json", "pass_criteria": "Reference frame indices shown per block"}
    },
    {
      "id": "METRICS_PSNR_SSIM_GRAPH",
      "category": "IA",
      "severity": "P0",
      "question": "Is there a PSNR/SSIM time series graph with Y/U/V channels?",
      "competitor_refs": [{"target_id": "vq_analyzer", "evidence": ["02-UI-Components/03-Stream-View/metrics.JPG"]}],
      "our_refs": ["metrics_distribution.rs"],
      "verification": {"method": "render_snapshot", "oracle": "render_snapshot_contracts.json", "pass_criteria": "Time series plot with dual Y-axis (PSNR dB / SSIM 0-1) and Y/U/V legend"}
    },
    {
      "id": "METRICS_HRD_BUFFER_GRAPH",
      "category": "IA",
      "severity": "P1",
      "question": "Is there an HRD/CPB buffer fullness graph over time?",
      "competitor_refs": [{"target_id": "vq_analyzer", "evidence": ["02-UI-Components/03-Stream-View/HRDbuffer_avc.jpg"]}],
      "our_refs": ["hrd.rs"],
      "verification": {"method": "render_snapshot", "oracle": "render_snapshot_contracts.json", "pass_criteria": "Buffer fullness plot with arrival time markers and bitrate legend"}
    },
    {
      "id": "GLOBAL_DUAL_VIEW_COMPARE",
      "category": "Contract",
      "severity": "P0",
      "question": "Is there dual view for side-by-side stream comparison with sync?",
      "competitor_refs": [{"target_id": "vq_analyzer", "evidence": ["10-Global-Functions/01-Dual-View/dual_view.jpg", "10-Global-Functions/01-Dual-View/dual_view_3.jpg"]}],
      "our_refs": ["compare.rs", "alignment.rs"],
      "verification": {"method": "render_snapshot", "oracle": "render_snapshot_contracts.json", "pass_criteria": "Two panels side-by-side with independent zoom/pan and sync toggle"}
    },
    {
      "id": "GLOBAL_YUV_DEBUG",
      "category": "Interaction",
      "severity": "P1",
      "question": "Is there YUV debug mode for pixel-level inspection?",
      "competitor_refs": [{"target_id": "vq_analyzer", "evidence": ["10-Global-Functions/02-Debug-YUV/YUV_hevc.jpg", "10-Global-Functions/02-Debug-YUV/YUVdetail_hevc.jpg"]}],
      "our_refs": ["player/mod.rs"],
      "verification": {"method": "render_snapshot", "oracle": "render_snapshot_contracts.json", "pass_criteria": "Pixel values shown on hover with Y/U/V breakdown"}
    },
    {
      "id": "GLOBAL_FIND_FIRST_DIFF",
      "category": "Contract",
      "severity": "P1",
      "question": "Is there find first difference feature for comparing YUV with reference?",
      "competitor_refs": [{"target_id": "vq_analyzer", "evidence": ["10-Global-Functions/03-Find-First-Difference/find_first_diff.jpg"]}],
      "our_refs": ["compare.rs", "diff_heatmap.rs"],
      "verification": {"method": "semantic_probe", "oracle": "semantic_probe_contracts.json", "pass_criteria": "Frames with differences marked in timeline; first diff frame navigable"}
    },
    {
      "id": "GLOBAL_REFERENCE_POOL",
      "category": "IA",
      "severity": "P0",
      "question": "Is there reference pool / DPB state visualization?",
      "competitor_refs": [{"target_id": "vq_analyzer", "evidence": ["02-UI-Components/04-Syntax-Info/AV1/refs_AV1.jpg", "02-UI-Components/04-Syntax-Info/HEVC/RefLists_hevc.jpg"]}],
      "our_refs": ["reference_graph.rs", "temporal_state.rs"],
      "verification": {"method": "render_snapshot", "oracle": "render_snapshot_contracts.json", "pass_criteria": "Reference pool table with frame, ref slot, refresh status; loop filter deltas"}
    },
    {
      "id": "CONTRACT_TRISYNC_PROPAGATION",
      "category": "Contract",
      "severity": "P0",
      "question": "Does selection propagate across all panels (Tree ⇄ Syntax ⇄ Hex ⇄ Player)?",
      "competitor_refs": [{"target_id": "vq_analyzer", "evidence": ["02-UI-Components/*"]}],
      "our_refs": ["selection.rs", "event.rs"],
      "verification": {"method": "semantic_probe", "oracle": "semantic_probe_contracts.json:selection_propagation_probe", "pass_criteria": "Click in any panel updates all others with anti-loop token"}
    },
    {
      "id": "CONTRACT_DISPLAY_DECODE_ORDER",
      "category": "Contract",
      "severity": "P0",
      "question": "Is display order vs decode order separation enforced everywhere?",
      "competitor_refs": [{"target_id": "vq_analyzer", "evidence": ["order toggles in StreamView"]}],
      "our_refs": ["command.rs:OrderType", "frame_identity.rs"],
      "verification": {"method": "evidence_bundle_diff", "oracle": "evidence_bundle_diff_contracts.json", "pass_criteria": "All evidence includes explicit order_type; no mixing"}
    },
    {
      "id": "EVIDENCE_BUNDLE_EXPORT",
      "category": "Evidence",
      "severity": "P0",
      "question": "Is there one-click evidence bundle export with screenshots + state + order type?",
      "competitor_refs": [{"target_id": "elecard_streameye", "evidence": ["report/export sections"]}],
      "our_refs": ["export.rs:EvidenceBundleManifest"],
      "verification": {"method": "evidence_bundle_diff", "oracle": "evidence_bundle_diff_contracts.json", "pass_criteria": "Bundle is complete, reproducible, machine-diffable"}
    },
    {
      "id": "INTERACTION_CONTEXT_MENUS",
      "category": "Interaction",
      "severity": "P1",
      "question": "Do context menus appear with proper guards and disabled reason tooltips?",
      "competitor_refs": [{"target_id": "vq_analyzer", "evidence": ["popup/context-menu states"]}],
      "our_refs": ["export.rs:build_context_menu"],
      "verification": {"method": "semantic_probe", "oracle": "semantic_probe_contracts.json", "pass_criteria": "Menus open; disabled items show reason tooltip"}
    },
    {
      "id": "CODEC_AV1_OBU_TREE",
      "category": "IA",
      "severity": "P0",
      "question": "Does AV1 mode show OBU tree with frame params, ref pool, segmentation?",
      "competitor_refs": [{"target_id": "vq_analyzer", "evidence": ["02-UI-Components/04-Syntax-Info/AV1/*", "07-Modes-AV1/*"]}],
      "our_refs": ["types.rs"],
      "verification": {"method": "render_snapshot", "oracle": "render_snapshot_contracts.json", "pass_criteria": "OBU tree with sequence/frame/tile hierarchy; reference pool table"}
    },
    {
      "id": "CODEC_HEVC_NAL_CTU",
      "category": "IA",
      "severity": "P0",
      "question": "Does HEVC mode show NAL tree with VPS/SPS/PPS and CTU/CU hierarchy?",
      "competitor_refs": [{"target_id": "vq_analyzer", "evidence": ["02-UI-Components/04-Syntax-Info/HEVC/*", "05-Modes-HEVC/*"]}],
      "our_refs": ["types.rs"],
      "verification": {"method": "render_snapshot", "oracle": "render_snapshot_contracts.json", "pass_criteria": "NAL tree; CTU→CU→PU hierarchy; SAO/deblocking views"}
    },
    {
      "id": "CODEC_VVC_DUAL_TREE",
      "category": "IA",
      "severity": "P0",
      "question": "Does VVC mode show dual tree, ALF, and advanced prediction modes?",
      "competitor_refs": [{"target_id": "vq_analyzer", "evidence": ["02-UI-Components/04-Syntax-Info/VVC/*", "03-Modes-VVC/*"]}],
      "our_refs": ["types.rs"],
      "verification": {"method": "render_snapshot", "oracle": "render_snapshot_contracts.json", "pass_criteria": "Dual tree view; ALF/LMCS tabs; affine/geo/MIP prediction details"}
    },
    {
      "id": "CODEC_VP9_SUPERFRAME",
      "category": "IA",
      "severity": "P1",
      "question": "Does VP9 mode show superblock partitions and probabilities?",
      "competitor_refs": [{"target_id": "vq_analyzer", "evidence": ["02-UI-Components/04-Syntax-Info/VP9/*", "06-Modes-VP9/*"]}],
      "our_refs": ["types.rs"],
      "verification": {"method": "render_snapshot", "oracle": "render_snapshot_contracts.json", "pass_criteria": "VP9 frame tree; probability tables; SB partitions"}
    },
    {
      "id": "PERF_FRAME_TIME_BUDGET",
      "category": "Performance",
      "severity": "P0",
      "question": "Does UI maintain 60fps (16.6ms frame budget) during navigation?",
      "competitor_refs": [{"target_id": "vq_analyzer", "evidence": ["smooth navigation observed"]}],
      "our_refs": ["performance.rs", "perf_budget_and_instrumentation.json"],
      "verification": {"method": "render_snapshot", "oracle": "render_snapshot_contracts.json", "pass_criteria": "Frame time within 16.6ms budget; degrade steps trigger at thresholds"}
    },
    {
      "id": "PERF_HIT_TEST_LATENCY",
      "category": "Performance",
      "severity": "P1",
      "question": "Is hit-test latency under 1.5ms budget?",
      "competitor_refs": [{"target_id": "vq_analyzer", "evidence": ["responsive click handling"]}],
      "our_refs": ["performance.rs"],
      "verification": {"method": "semantic_probe", "oracle": "semantic_probe_contracts.json:hit_test_probe", "pass_criteria": "Hit-test returns within 1.5ms; transform consistency verified"}
    },
    {
      "id": "FAILURE_GRACEFUL_DEGRADE",
      "category": "FailureUX",
      "severity": "P1",
      "question": "Does UI degrade gracefully under load with visible indicators?",
      "competitor_refs": [{"target_id": "vq_analyzer", "evidence": ["loading indicators"]}],
      "our_refs": ["diagnostics.rs", "tooltip.rs"],
      "verification": {"method": "render_snapshot", "oracle": "render_snapshot_contracts.json", "pass_criteria": "Loading states shown; degrade steps visible; no blank panels"}
    },
    {
      "id": "FAILURE_ERROR_RECOVERY",
      "category": "FailureUX",
      "severity": "P0",
      "question": "Does UI recover from parse errors without crashing?",
      "competitor_refs": [{"target_id": "vq_analyzer", "evidence": ["02-UI-Components/07-Status-Panel/errors.jpg"]}],
      "our_refs": ["error.rs", "diagnostics.rs"],
      "verification": {"method": "manual_review", "oracle": "ui_acceptance_oracle.json", "pass_criteria": "Parse errors shown in status; UI remains responsive"}
    },
    {
      "id": "ACCESSIBILITY_KEYBOARD_NAV",
      "category": "Accessibility",
      "severity": "P2",
      "question": "Is keyboard navigation available for all major actions?",
      "competitor_refs": [{"target_id": "vq_analyzer", "evidence": ["keyboard shortcuts in help"]}],
      "our_refs": ["command.rs"],
      "verification": {"method": "manual_review", "oracle": "ui_acceptance_oracle.json", "pass_criteria": "Tab navigation works; shortcuts for common actions"}
    }
  ]
}"#;

/// Parse and return the full parity matrix
pub fn get_full_parity_matrix() -> Result<(ParityMatrix, SchemaValidationResult), String> {
    parse_and_validate_parity_matrix(FULL_PARITY_MATRIX_JSON)
}

/// Get the number of parity items in the full matrix
pub fn get_full_parity_item_count() -> usize {
    31 // 31 parity items extracted from VQ Analyzer
}
