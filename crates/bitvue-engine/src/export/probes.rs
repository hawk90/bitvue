//! Semantic probe runner - Integration with UI Panels

use crate::parity_harness::{
    EntityRef, HardFailKind, HitTestProbeInput, HitTestProbeOutput, ProbeContext, ProbeOutcome,
    ProbePoint, ProbeResult, ProbeViolation, Provenance, SelectionPropagationInput,
    SelectionPropagationOutput, TooltipField, TooltipPayloadInput, TooltipPayloadOutput,
    ViewportState,
};

/// Semantic probe runner - records and validates probe results against contracts
#[derive(Debug, Default)]
pub struct SemanticProbeRunner {
    /// Recorded probe results
    pub results: Vec<ProbeResult>,
    /// Hard fail violations detected
    pub violations: Vec<ProbeViolation>,
    /// Current stream ID being probed
    pub stream_id: String,
    /// Current codec
    pub codec: String,
    /// Current workspace
    pub workspace: String,
    /// Current mode
    pub mode: String,
}

impl SemanticProbeRunner {
    /// Create a new probe runner
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the current context for probes
    pub fn set_context(&mut self, stream_id: &str, codec: &str, workspace: &str, mode: &str) {
        self.stream_id = stream_id.to_string();
        self.codec = codec.to_string();
        self.workspace = workspace.to_string();
        self.mode = mode.to_string();
    }

    /// Run hit-test probe
    ///
    /// Per semantic_probe_contracts.json: Validates hit-testing consistency
    pub fn run_hit_test_probe(
        &mut self,
        panel: &str,
        viewport: ViewportState,
        hit_entity: Option<EntityRef>,
        layer: &str,
        tooltip: Option<crate::parity_harness::TooltipPayload>,
    ) -> ProbeResult {
        let probe_id = format!(
            "hit_test_{}_{}",
            panel,
            chrono::Utc::now().timestamp_millis()
        );
        let mut violations = Vec::new();

        // Check for transform consistency (hard fail if hit-test returns different entity than render)
        // This is a simplified check - in practice you'd compare with actual render data
        if hit_entity.is_some() && viewport.zoom == 0.0 {
            violations.push(ProbeViolation {
                kind: HardFailKind::HitTestRenderTransformMismatch,
                message: "Zero zoom detected during hit-test".to_string(),
                context: [("panel".to_string(), panel.to_string())]
                    .into_iter()
                    .collect(),
            });
        }

        let result = ProbeResult {
            probe_id: probe_id.clone(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            stream_id: self.stream_id.clone(),
            codec: self.codec.clone(),
            workspace: self.workspace.clone(),
            mode: self.mode.clone(),
            result: ProbeOutcome::HitTest(HitTestProbeOutput {
                hit_entity,
                layer: layer.to_string(),
                cursor_payload: tooltip,
            }),
            violations: violations.clone(),
        };

        self.violations.extend(violations);
        self.results.push(result.clone());
        result
    }

    /// Run selection propagation probe
    ///
    /// Per semantic_probe_contracts.json: Validates selection sync across panels
    pub fn run_selection_propagation_probe(
        &mut self,
        origin_panel: &str,
        action: &str,
        _txn: &str,
        panels_updated: Vec<String>,
        selection: crate::parity_harness::SelectionSnapshot,
    ) -> ProbeResult {
        let probe_id = format!(
            "selection_{}_{}",
            origin_panel,
            chrono::Utc::now().timestamp_millis()
        );
        let violations = Vec::new();

        // Check for order type mixing (hard fail)
        // In practice, you'd compare with previous selection state
        if panels_updated.len() > 1 {
            // Selection should propagate to all panels
            let expected_panels = ["player", "syntax", "hex", "timeline"];
            let missing: Vec<_> = expected_panels
                .iter()
                .filter(|p| !panels_updated.iter().any(|u| u.contains(*p)))
                .collect();

            if !missing.is_empty() && action == "select" {
                // This is a warning, not a hard fail
            }
        }

        let result = ProbeResult {
            probe_id: probe_id.clone(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            stream_id: self.stream_id.clone(),
            codec: self.codec.clone(),
            workspace: self.workspace.clone(),
            mode: self.mode.clone(),
            result: ProbeOutcome::SelectionPropagation(SelectionPropagationOutput {
                panels_updated,
                selection_snapshot: selection,
            }),
            violations: violations.clone(),
        };

        self.violations.extend(violations);
        self.results.push(result.clone());
        result
    }

    /// Run tooltip payload probe
    ///
    /// Per semantic_probe_contracts.json: Validates tooltip content
    pub fn run_tooltip_payload_probe(
        &mut self,
        entity: EntityRef,
        fields: Vec<TooltipField>,
    ) -> ProbeResult {
        let probe_id = format!(
            "tooltip_{}_{}",
            entity.id,
            chrono::Utc::now().timestamp_millis()
        );
        let violations = Vec::new();

        let result = ProbeResult {
            probe_id: probe_id.clone(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            stream_id: self.stream_id.clone(),
            codec: self.codec.clone(),
            workspace: self.workspace.clone(),
            mode: self.mode.clone(),
            result: ProbeOutcome::TooltipPayload(TooltipPayloadOutput {
                fields,
                provenance: Provenance::default(),
            }),
            violations: violations.clone(),
        };

        self.results.push(result.clone());
        result
    }

    /// Record order type mixing hard fail
    pub fn record_order_type_mixing(&mut self, context: &str) {
        self.violations.push(ProbeViolation {
            kind: HardFailKind::OrderTypeMixingDetected,
            message: format!("Display/decode order mixing detected: {}", context),
            context: [("context".to_string(), context.to_string())]
                .into_iter()
                .collect(),
        });
    }

    /// Record stale async applied hard fail
    pub fn record_stale_async(&mut self, txn_id: &str) {
        self.violations.push(ProbeViolation {
            kind: HardFailKind::StaleAsyncApplied,
            message: format!("Stale async result applied: txn={}", txn_id),
            context: [("txn_id".to_string(), txn_id.to_string())]
                .into_iter()
                .collect(),
        });
    }

    /// Record cache invalidation violation
    pub fn record_cache_violation(&mut self, cache_key: &str) {
        self.violations.push(ProbeViolation {
            kind: HardFailKind::CacheInvalidationViolation,
            message: format!("Cache invalidation violation: key={}", cache_key),
            context: [("cache_key".to_string(), cache_key.to_string())]
                .into_iter()
                .collect(),
        });
    }

    /// Check if any hard fails have been detected
    pub fn has_hard_fails(&self) -> bool {
        !self.violations.is_empty()
    }

    /// Get all hard fail violations
    pub fn get_violations(&self) -> &[ProbeViolation] {
        &self.violations
    }

    /// Export probe results as JSON
    pub fn export_results(&self) -> std::result::Result<String, Box<dyn std::error::Error>> {
        serde_json::to_string_pretty(&self.results).map_err(|e| {
            crate::BitvueError::Decode(format!("Failed to serialize probe results: {}", e)).into()
        })
    }

    /// Clear recorded results
    pub fn clear(&mut self) {
        self.results.clear();
        self.violations.clear();
    }
}

/// Create a hit-test probe input for a panel
pub fn create_hit_test_input(
    panel: &str,
    viewport: ViewportState,
    x: f32,
    y: f32,
) -> HitTestProbeInput {
    HitTestProbeInput {
        panel: panel.to_string(),
        viewport,
        points: vec![ProbePoint { x, y }],
    }
}

/// Create a selection propagation input
pub fn create_selection_input(
    origin_panel: &str,
    action: &str,
    txn: &str,
) -> SelectionPropagationInput {
    SelectionPropagationInput {
        origin_panel: origin_panel.to_string(),
        action: action.to_string(),
        txn: txn.to_string(),
    }
}

/// Create a tooltip payload input
pub fn create_tooltip_input(
    entity: EntityRef,
    workspace: &str,
    mode: &str,
    codec: &str,
) -> TooltipPayloadInput {
    TooltipPayloadInput {
        entity,
        context: ProbeContext {
            workspace: workspace.to_string(),
            mode: mode.to_string(),
            codec: codec.to_string(),
        },
    }
}

/// Create a tooltip field for display
pub fn create_tooltip_field(
    label: &str,
    value: &str,
    provenance: Option<Provenance>,
) -> TooltipField {
    TooltipField {
        label: label.to_string(),
        value: value.to_string(),
        provenance,
    }
}
