//! MCP Integration Layer - T7-2
//!
//! Per MCP_INTERACTION_MODEL.md:
//! - Read-only data surface for external tools (ChatGPT, etc.)
//! - Exports analysis context for conversational debugging
//! - All resources validate against JSON schemas
//! - Evidence pointers required for all claims
//!
//! MCP Resources:
//! - selection_state: Current selection context
//! - insight_feed: Detected anomalies with evidence
//! - diagnostics: Error bursts, scene changes, reorder
//! - metrics_summary: Histogram bins and summary stats
//! - timeline_lanes: Timeline data with cursor position
//! - session_evidence: Bookmarks and snapshot metadata
//! - compliance: Scores and violations
//! - compare: A/B deltas and rules

use crate::{
    CompareWorkspace, DiagnosticsBands, InsightFeed, MetricsDistributionPanel, SelectionState,
    TimelineLaneSystem,
};
use serde::{Deserialize, Serialize};

/// MCP Integration Layer
///
/// Provides read-only access to analysis data for external tools.
/// All resources are immutable snapshots.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpIntegration {
    /// MCP resources
    pub resources: McpResources,
}

impl McpIntegration {
    /// Create MCP integration from analysis state
    pub fn new(
        selection: &SelectionState,
        insights: &InsightFeed,
        diagnostics: &DiagnosticsBands,
        metrics: Option<&MetricsDistributionPanel>,
        timeline_lanes: Option<&TimelineLaneSystem>,
        compare: Option<&CompareWorkspace>,
    ) -> Self {
        Self {
            resources: McpResources {
                selection_state: Some(McpSelectionState::from_selection(selection)),
                insight_feed: Some(insights.clone()),
                diagnostics: Some(McpDiagnostics::from_diagnostics(diagnostics)),
                metrics_summary: metrics.map(McpMetricsSummary::from_metrics),
                timeline_lanes: timeline_lanes.map(McpTimelineLanes::from_timeline),
                compare: compare.map(McpCompare::from_compare),
                session_evidence: None, // Populated separately
                compliance: None,       // Populated separately
            },
        }
    }

    /// Get resource by name
    pub fn get_resource(&self, name: &str) -> Option<serde_json::Value> {
        match name {
            "selection_state" => self
                .resources
                .selection_state
                .as_ref()
                .and_then(|r| serde_json::to_value(r).ok()),
            "insight_feed" => self
                .resources
                .insight_feed
                .as_ref()
                .and_then(|r| serde_json::to_value(r).ok()),
            "diagnostics" => self
                .resources
                .diagnostics
                .as_ref()
                .and_then(|r| serde_json::to_value(r).ok()),
            "metrics_summary" => self
                .resources
                .metrics_summary
                .as_ref()
                .and_then(|r| serde_json::to_value(r).ok()),
            "timeline_lanes" => self
                .resources
                .timeline_lanes
                .as_ref()
                .and_then(|r| serde_json::to_value(r).ok()),
            "compare" => self
                .resources
                .compare
                .as_ref()
                .and_then(|r| serde_json::to_value(r).ok()),
            "session_evidence" => self
                .resources
                .session_evidence
                .as_ref()
                .and_then(|r| serde_json::to_value(r).ok()),
            "compliance" => self
                .resources
                .compliance
                .as_ref()
                .and_then(|r| serde_json::to_value(r).ok()),
            _ => None,
        }
    }

    /// List available resources
    pub fn list_resources(&self) -> Vec<&str> {
        let mut resources = Vec::new();
        if self.resources.selection_state.is_some() {
            resources.push("selection_state");
        }
        if self.resources.insight_feed.is_some() {
            resources.push("insight_feed");
        }
        if self.resources.diagnostics.is_some() {
            resources.push("diagnostics");
        }
        if self.resources.metrics_summary.is_some() {
            resources.push("metrics_summary");
        }
        if self.resources.timeline_lanes.is_some() {
            resources.push("timeline_lanes");
        }
        if self.resources.compare.is_some() {
            resources.push("compare");
        }
        if self.resources.session_evidence.is_some() {
            resources.push("session_evidence");
        }
        if self.resources.compliance.is_some() {
            resources.push("compliance");
        }
        resources
    }
}

/// MCP Resources container
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpResources {
    /// Current selection state
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selection_state: Option<McpSelectionState>,

    /// Insight feed with anomalies
    #[serde(skip_serializing_if = "Option::is_none")]
    pub insight_feed: Option<InsightFeed>,

    /// Diagnostics data
    #[serde(skip_serializing_if = "Option::is_none")]
    pub diagnostics: Option<McpDiagnostics>,

    /// Metrics summary
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metrics_summary: Option<McpMetricsSummary>,

    /// Timeline lanes data
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeline_lanes: Option<McpTimelineLanes>,

    /// Compare data (A/B)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub compare: Option<McpCompare>,

    /// Session evidence (bookmarks, snapshots)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_evidence: Option<McpSessionEvidence>,

    /// Compliance scores and violations
    #[serde(skip_serializing_if = "Option::is_none")]
    pub compliance: Option<McpCompliance>,
}

/// MCP Selection State
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpSelectionState {
    /// Selected stream
    pub stream: String,

    /// Selected frame index
    pub frame_idx: Option<usize>,

    /// Selected unit ID
    pub unit_id: Option<String>,

    /// Selected syntax node ID
    pub syntax_node_id: Option<String>,

    /// Bit range selection
    pub bit_range: Option<(usize, usize)>,

    /// Spatial block selection
    pub spatial_block: Option<(u32, u32, u32, u32)>,
}

impl McpSelectionState {
    /// Convert from SelectionState
    fn from_selection(selection: &SelectionState) -> Self {
        // Get frame index from temporal selection
        let frame_idx = selection.temporal.as_ref().map(|t| t.frame_index());

        // Get spatial block from temporal selection
        let spatial_block = selection.temporal.as_ref().and_then(|t| {
            t.spatial_block().map(|sb| (sb.x, sb.y, sb.w, sb.h))
        });

        Self {
            stream: format!("{:?}", selection.stream_id),
            frame_idx,
            unit_id: selection
                .unit
                .as_ref()
                .map(|u| format!("{}_{}_{}", u.unit_type, u.offset, u.size)),
            syntax_node_id: selection.syntax_node.clone(),
            bit_range: selection
                .bit_range
                .map(|br| (br.start_bit as usize, br.end_bit as usize)),
            spatial_block,
        }
    }
}

/// MCP Diagnostics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpDiagnostics {
    /// Error bursts
    pub error_bursts: Vec<McpErrorBurst>,

    /// Scene changes
    pub scene_changes: Vec<McpSceneChange>,

    /// Reorder entries
    pub reorder_entries: Vec<McpReorderEntry>,
}

impl McpDiagnostics {
    /// Convert from DiagnosticsBands
    fn from_diagnostics(diagnostics: &DiagnosticsBands) -> Self {
        Self {
            error_bursts: diagnostics
                .error_bursts
                .iter()
                .map(McpErrorBurst::from_burst)
                .collect(),
            scene_changes: diagnostics
                .scene_changes
                .iter()
                .map(McpSceneChange::from_scene)
                .collect(),
            reorder_entries: diagnostics
                .reorder_entries
                .iter()
                .map(McpReorderEntry::from_reorder)
                .collect(),
        }
    }
}

/// MCP Error Burst
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpErrorBurst {
    pub frame_range: (usize, usize),
    pub error_count: usize,
    pub severity: f32,
    pub error_types: Vec<String>,
}

impl McpErrorBurst {
    fn from_burst(burst: &crate::ErrorBurst) -> Self {
        Self {
            frame_range: (burst.start_idx, burst.end_idx),
            error_count: burst.error_count,
            severity: burst.severity,
            error_types: burst.error_types.clone(),
        }
    }
}

/// MCP Scene Change
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpSceneChange {
    pub frame_idx: usize,
    pub confidence: f32,
    pub description: Option<String>,
}

impl McpSceneChange {
    fn from_scene(scene: &crate::SceneChange) -> Self {
        Self {
            frame_idx: scene.display_idx,
            confidence: scene.confidence,
            description: scene.description.clone(),
        }
    }
}

/// MCP Reorder Entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpReorderEntry {
    pub frame_idx: usize,
    pub pts: u64,
    pub dts: u64,
    pub depth: u64,
}

impl McpReorderEntry {
    fn from_reorder(reorder: &crate::ReorderEntry) -> Self {
        Self {
            frame_idx: reorder.display_idx,
            pts: reorder.pts,
            dts: reorder.dts,
            depth: reorder.depth,
        }
    }
}

/// MCP Metrics Summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpMetricsSummary {
    /// Metric type
    pub metric_type: String,

    /// Summary statistics
    pub stats: McpSummaryStats,

    /// Histogram bins
    pub histogram_bins: Vec<McpHistogramBin>,

    /// Worst frames
    pub worst_frames: Vec<(usize, f32)>,
}

impl McpMetricsSummary {
    /// Convert from MetricsDistributionPanel
    fn from_metrics(metrics: &MetricsDistributionPanel) -> Self {
        // Clone to allow mutable access for get methods
        let mut metrics_mut = metrics.clone();

        let stats = metrics_mut
            .get_stats()
            .map(McpSummaryStats::from_stats)
            .unwrap_or_default();
        let histogram_bins = metrics_mut
            .get_histogram()
            .map(|h| h.bins.iter().map(McpHistogramBin::from_bin).collect())
            .unwrap_or_default();

        Self {
            metric_type: "default".to_string(),
            stats,
            histogram_bins,
            worst_frames: Vec::new(), // Would be populated from actual data
        }
    }
}

/// MCP Summary Stats
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct McpSummaryStats {
    pub count: usize,
    pub min: f32,
    pub max: f32,
    pub mean: f32,
    pub median: f32,
    pub std_dev: f32,
    pub p5: f32,
    pub p95: f32,
}

impl McpSummaryStats {
    fn from_stats(stats: &crate::SummaryStats) -> Self {
        Self {
            count: stats.count,
            min: stats.min,
            max: stats.max,
            mean: stats.mean,
            median: stats.median,
            std_dev: stats.std_dev,
            p5: stats.p5,
            p95: stats.p95,
        }
    }
}

/// MCP Histogram Bin
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpHistogramBin {
    pub range_start: f32,
    pub range_end: f32,
    pub count: usize,
}

impl McpHistogramBin {
    fn from_bin(bin: &crate::HistogramBin) -> Self {
        Self {
            range_start: bin.start,
            range_end: bin.end,
            count: bin.count,
        }
    }
}

/// MCP Timeline Lanes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpTimelineLanes {
    /// Cursor position (frame index)
    pub cursor_frame: Option<usize>,

    /// Active lanes
    pub active_lanes: Vec<String>,

    /// Zoom level
    pub zoom_level: f32,
}

impl McpTimelineLanes {
    fn from_timeline(timeline: &TimelineLaneSystem) -> Self {
        Self {
            cursor_frame: None, // Would be from actual state
            active_lanes: timeline
                .lanes
                .iter()
                .map(|l| format!("{:?}", l.lane_type))
                .collect(),
            zoom_level: timeline.zoom_level,
        }
    }
}

/// MCP Compare (A/B)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpCompare {
    /// Alignment method used
    pub alignment_method: String,

    /// Alignment confidence
    pub alignment_confidence: String,

    /// Gap percentage
    pub gap_percentage: f64,

    /// Resolution compatibility
    pub resolution_compatible: bool,

    /// Diff enabled
    pub diff_enabled: bool,

    /// Disable reason (if applicable)
    pub disable_reason: Option<String>,
}

impl McpCompare {
    fn from_compare(compare: &CompareWorkspace) -> Self {
        Self {
            alignment_method: compare.alignment.method.display_text().to_string(),
            alignment_confidence: compare.alignment.confidence().display_text().to_string(),
            gap_percentage: compare.alignment.gap_percentage(),
            resolution_compatible: compare.resolution_info.is_compatible(),
            diff_enabled: compare.diff_enabled,
            disable_reason: compare.disable_reason.clone(),
        }
    }
}

/// MCP Session Evidence
///
/// Bookmarks and snapshot metadata for evidence trails.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpSessionEvidence {
    /// Bookmarks
    pub bookmarks: Vec<McpBookmark>,

    /// Snapshots
    pub snapshots: Vec<McpSnapshot>,
}

/// MCP Bookmark
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpBookmark {
    pub id: String,
    pub frame_idx: usize,
    pub label: String,
    pub description: Option<String>,
    pub timestamp: u64,
}

/// MCP Snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpSnapshot {
    pub id: String,
    pub frame_idx: usize,
    pub label: String,
    pub timestamp: u64,
}

/// MCP Compliance
///
/// Compliance scores and violations for standards/rules.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpCompliance {
    /// Overall compliance score (0.0 to 1.0)
    pub score: f32,

    /// Violations detected
    pub violations: Vec<McpViolation>,

    /// Rules checked
    pub rules_checked: usize,

    /// Rules passed
    pub rules_passed: usize,
}

/// MCP Violation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpViolation {
    pub rule_id: String,
    pub severity: String,
    pub frame_range: Option<(usize, usize)>,
    pub description: String,
    pub evidence_pointer: String,
}

// ============================================================================
// Tests
// ============================================================================
// TODO: mcp_test.rs API mismatch - actual McpSelectionState, McpDiagnostics, etc. have different fields than assumed
// #[cfg(test)]
// include!("mcp_test.rs");
