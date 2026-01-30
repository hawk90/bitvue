//! Gates - Hard Fail, Parity, and Performance gates

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// Forward declare types from parent module
//
// Note: ParityScore is defined in mod.rs since gates.rs is a submodule.
// This is a cyclic dependency pattern that's resolved by:
// 1. mod.rs defines ParityScore (aggregates gate results)
// 2. gates.rs defines ParityGate (evaluates ParityScore)
// 3. We use a forward declaration pattern here

// Re-export ParityScore from parent for use in this module
pub use super::ParityScore;

// =============================================================================
// HARD FAIL GATE
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

/// Hard fail gate result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HardFailGateResult {
    pub passed: bool,
    pub violations: Vec<HardFailViolation>,
}

// =============================================================================
// PARITY GATE
// =============================================================================

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

// =============================================================================
// PERFORMANCE GATE
// =============================================================================

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

/// Combined gate results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HarnessGateResults {
    pub hard_fail: HardFailGateResult,
    pub parity: ParityGateResult,
    pub perf: Vec<PerfGateResult>,
    pub overall_passed: bool,
}

// =============================================================================
// HARD FAIL KIND
// =============================================================================

/// Hard fail types (per semantic_probe_contracts.json)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HardFailKind {
    OrderTypeMixingDetected,
    StaleAsyncApplied,
    HitTestRenderTransformMismatch,
    CacheInvalidationViolation,
}
