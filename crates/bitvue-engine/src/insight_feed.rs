//! Insight Feed Generator - T7-1
//!
//! Generates anomaly summaries with jump links and evidence pointers.
//! Per MCP insight_feed.schema.json:
//! - Detects anomalies from diagnostics data
//! - Generates jump-to-location links
//! - Provides evidence pointers
//!
//! Sources anomalies from:
//! - T4-3: DiagnosticsBands (error bursts, scene changes, reorder)
//! - T0-1: FrameIdentity (PTS quality issues)
//! - Timeline/Metrics data

use crate::{DiagnosticsBands, ErrorBurst, FrameIndexMap, PtsQuality, SceneChange};
use serde::{Deserialize, Serialize};

/// Insight feed containing detected anomalies
///
/// Generates insights from diagnostic data with jump links and evidence.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InsightFeed {
    /// List of insights
    pub insights: Vec<Insight>,
}

impl InsightFeed {
    /// Create empty insight feed
    pub fn new() -> Self {
        Self {
            insights: Vec::new(),
        }
    }

    /// Generate insights from diagnostics and frame data
    ///
    /// Detects anomalies from:
    /// - Error bursts
    /// - Scene changes
    /// - PTS quality issues
    /// - Reorder mismatches
    pub fn generate(frame_map: &FrameIndexMap, diagnostics: &DiagnosticsBands) -> Self {
        let mut insights = Vec::new();

        // Generate PTS quality insights
        if let Some(insight) = Self::generate_pts_quality_insight(frame_map) {
            insights.push(insight);
        }

        // Generate error burst insights
        for (idx, burst) in diagnostics.error_bursts.iter().enumerate() {
            insights.push(Self::generate_error_burst_insight(idx, burst));
        }

        // Generate scene change insights
        for scene_change in &diagnostics.scene_changes {
            if let Some(insight) = Self::generate_scene_change_insight(scene_change) {
                insights.push(insight);
            }
        }

        // Generate reorder insights
        if !diagnostics.reorder_entries.is_empty() {
            insights.push(Self::generate_reorder_insight(diagnostics));
        }

        // Sort by severity (critical first)
        insights.sort_by(|a, b| {
            let a_priority = a.severity.priority();
            let b_priority = b.severity.priority();
            b_priority.cmp(&a_priority) // Descending
        });

        Self { insights }
    }

    /// Generate PTS quality insight
    fn generate_pts_quality_insight(frame_map: &FrameIndexMap) -> Option<Insight> {
        let quality = frame_map.pts_quality();

        match quality {
            PtsQuality::Ok => None, // No insight for OK quality
            PtsQuality::Warn => Some(Insight {
                id: "pts_warn".to_string(),
                insight_type: InsightType::PtsQualityWarn,
                severity: InsightSeverity::Warn,
                stream: StreamScope::A,
                frame_range: (0, frame_map.frame_count().saturating_sub(1)),
                triggers: vec![Trigger {
                    signal: "pts_quality".to_string(),
                    value: 1.0,           // WARN
                    threshold: Some(0.0), // OK threshold
                    method: "quality_assessment".to_string(),
                }],
                jump_targets: vec![JumpTarget {
                    panel: "timeline".to_string(),
                    payload: serde_json::json!({ "show_pts_badge": true }),
                }],
                evidence: vec![EvidencePointer {
                    kind: EvidenceKind::Diagnostic,
                    id: "pts_quality".to_string(),
                    frame_idx: Some(0),
                    range: None,
                    offset_bits: None,
                }],
            }),
            PtsQuality::Bad => Some(Insight {
                id: "pts_bad".to_string(),
                insight_type: InsightType::PtsQualityBad,
                severity: InsightSeverity::Error,
                stream: StreamScope::A,
                frame_range: (0, frame_map.frame_count().saturating_sub(1)),
                triggers: vec![Trigger {
                    signal: "pts_quality".to_string(),
                    value: 2.0,           // BAD
                    threshold: Some(0.0), // OK threshold
                    method: "quality_assessment".to_string(),
                }],
                jump_targets: vec![JumpTarget {
                    panel: "timeline".to_string(),
                    payload: serde_json::json!({ "show_pts_badge": true }),
                }],
                evidence: vec![EvidencePointer {
                    kind: EvidenceKind::Diagnostic,
                    id: "pts_quality".to_string(),
                    frame_idx: Some(0),
                    range: None,
                    offset_bits: None,
                }],
            }),
        }
    }

    /// Generate error burst insight
    fn generate_error_burst_insight(burst_idx: usize, burst: &ErrorBurst) -> Insight {
        let severity = if burst.severity > 0.8 {
            InsightSeverity::Critical
        } else if burst.severity > 0.5 {
            InsightSeverity::Error
        } else {
            InsightSeverity::Warn
        };

        Insight {
            id: format!("error_burst_{}", burst_idx),
            insight_type: InsightType::ErrorBurst,
            severity,
            stream: StreamScope::A,
            frame_range: (burst.start_idx, burst.end_idx),
            triggers: vec![Trigger {
                signal: "error_density".to_string(),
                value: burst.severity as f64,
                threshold: Some(0.3),
                method: "burst_detection".to_string(),
            }],
            jump_targets: vec![
                JumpTarget {
                    panel: "timeline".to_string(),
                    payload: serde_json::json!({
                        "frame_idx": burst.start_idx,
                        "select_range": [burst.start_idx, burst.end_idx]
                    }),
                },
                JumpTarget {
                    panel: "diagnostics".to_string(),
                    payload: serde_json::json!({
                        "burst_idx": burst_idx
                    }),
                },
            ],
            evidence: vec![EvidencePointer {
                kind: EvidenceKind::Range,
                id: format!("burst_{}", burst_idx),
                frame_idx: None,
                range: Some((burst.start_idx, burst.end_idx)),
                offset_bits: None,
            }],
        }
    }

    /// Generate scene change insight
    fn generate_scene_change_insight(scene_change: &SceneChange) -> Option<Insight> {
        // Only generate insights for significant scene changes (high confidence)
        if scene_change.confidence < 0.7 {
            return None;
        }

        Some(Insight {
            id: format!("scene_change_{}", scene_change.display_idx),
            insight_type: InsightType::SceneChange,
            severity: InsightSeverity::Info,
            stream: StreamScope::A,
            frame_range: (scene_change.display_idx, scene_change.display_idx),
            triggers: vec![Trigger {
                signal: "scene_change_confidence".to_string(),
                value: scene_change.confidence as f64,
                threshold: Some(0.5),
                method: "histogram_diff".to_string(),
            }],
            jump_targets: vec![
                JumpTarget {
                    panel: "player".to_string(),
                    payload: serde_json::json!({
                        "frame_idx": scene_change.display_idx
                    }),
                },
                JumpTarget {
                    panel: "timeline".to_string(),
                    payload: serde_json::json!({
                        "frame_idx": scene_change.display_idx,
                        "highlight_scene_change": true
                    }),
                },
            ],
            evidence: vec![EvidencePointer {
                kind: EvidenceKind::Frame,
                id: format!("scene_{}", scene_change.display_idx),
                frame_idx: Some(scene_change.display_idx),
                range: None,
                offset_bits: None,
            }],
        })
    }

    /// Generate reorder insight
    fn generate_reorder_insight(diagnostics: &DiagnosticsBands) -> Insight {
        let reorder_count = diagnostics.reorder_entries.len();
        let first_idx = diagnostics
            .reorder_entries
            .first()
            .map(|e| e.display_idx)
            .unwrap_or(0);
        let last_idx = diagnostics
            .reorder_entries
            .last()
            .map(|e| e.display_idx)
            .unwrap_or(0);

        Insight {
            id: "reorder_detected".to_string(),
            insight_type: InsightType::ReorderDetected,
            severity: InsightSeverity::Warn,
            stream: StreamScope::A,
            frame_range: (first_idx, last_idx),
            triggers: vec![Trigger {
                signal: "reorder_count".to_string(),
                value: reorder_count as f64,
                threshold: Some(0.0),
                method: "pts_dts_comparison".to_string(),
            }],
            jump_targets: vec![JumpTarget {
                panel: "timeline".to_string(),
                payload: serde_json::json!({
                    "show_reorder_band": true,
                    "frame_idx": first_idx
                }),
            }],
            evidence: vec![EvidencePointer {
                kind: EvidenceKind::Diagnostic,
                id: "reorder_band".to_string(),
                frame_idx: Some(first_idx),
                range: None,
                offset_bits: None,
            }],
        }
    }

    /// Filter insights by severity
    pub fn filter_by_severity(&self, min_severity: InsightSeverity) -> Vec<&Insight> {
        let min_priority = min_severity.priority();
        self.insights
            .iter()
            .filter(|i| i.severity.priority() >= min_priority)
            .collect()
    }

    /// Get insights for specific frame range
    pub fn get_insights_in_range(&self, start: usize, end: usize) -> Vec<&Insight> {
        self.insights
            .iter()
            .filter(|i| {
                let (i_start, i_end) = i.frame_range;
                // Check if ranges overlap
                i_start <= end && i_end >= start
            })
            .collect()
    }

    /// Get insight by ID
    pub fn get_insight(&self, id: &str) -> Option<&Insight> {
        self.insights.iter().find(|i| i.id == id)
    }

    /// Count insights by severity
    pub fn count_by_severity(&self) -> SeverityCounts {
        let mut counts = SeverityCounts::default();

        for insight in &self.insights {
            match insight.severity {
                InsightSeverity::Info => counts.info += 1,
                InsightSeverity::Warn => counts.warn += 1,
                InsightSeverity::Error => counts.error += 1,
                InsightSeverity::Critical => counts.critical += 1,
            }
        }

        counts
    }
}

impl Default for InsightFeed {
    fn default() -> Self {
        Self::new()
    }
}

/// Individual insight/anomaly
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Insight {
    /// Unique insight ID
    pub id: String,

    /// Insight type
    #[serde(rename = "type")]
    pub insight_type: InsightType,

    /// Severity level
    pub severity: InsightSeverity,

    /// Stream scope (A, B, or AB)
    pub stream: StreamScope,

    /// Affected frame range [start, end]
    pub frame_range: (usize, usize),

    /// Triggers that caused this insight
    pub triggers: Vec<Trigger>,

    /// Jump targets for navigation
    pub jump_targets: Vec<JumpTarget>,

    /// Evidence pointers
    #[serde(default)]
    pub evidence: Vec<EvidencePointer>,
}

/// Insight type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum InsightType {
    #[serde(rename = "error_burst")]
    ErrorBurst,
    #[serde(rename = "scene_change")]
    SceneChange,
    #[serde(rename = "pts_quality_warn")]
    PtsQualityWarn,
    #[serde(rename = "pts_quality_bad")]
    PtsQualityBad,
    #[serde(rename = "reorder_detected")]
    ReorderDetected,
}

/// Severity level
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum InsightSeverity {
    Info,
    Warn,
    Error,
    Critical,
}

impl InsightSeverity {
    /// Get priority value (higher = more severe)
    pub fn priority(&self) -> u8 {
        match self {
            InsightSeverity::Info => 0,
            InsightSeverity::Warn => 1,
            InsightSeverity::Error => 2,
            InsightSeverity::Critical => 3,
        }
    }

    /// Get display text
    pub fn display_text(&self) -> &'static str {
        match self {
            InsightSeverity::Info => "Info",
            InsightSeverity::Warn => "Warning",
            InsightSeverity::Error => "Error",
            InsightSeverity::Critical => "Critical",
        }
    }
}

/// Stream scope
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StreamScope {
    A,
    B,
    AB, // Both streams
}

/// Trigger that caused an insight
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Trigger {
    /// Signal name
    pub signal: String,

    /// Measured value
    pub value: f64,

    /// Threshold (if applicable)
    pub threshold: Option<f64>,

    /// Detection method
    pub method: String,
}

/// Jump target for navigation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JumpTarget {
    /// Target panel name
    pub panel: String,

    /// Navigation payload
    pub payload: serde_json::Value,
}

/// Evidence pointer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvidencePointer {
    /// Evidence kind
    pub kind: EvidenceKind,

    /// Evidence ID
    pub id: String,

    /// Frame index (if applicable)
    pub frame_idx: Option<usize>,

    /// Frame range (if applicable)
    pub range: Option<(usize, usize)>,

    /// Bit offset (if applicable)
    pub offset_bits: Option<usize>,
}

/// Evidence kind
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceKind {
    Frame,
    Range,
    Offset,
    Insight,
    Diagnostic,
    Rule,
    Bookmark,
    SyntaxNode,
}

/// Severity counts
#[derive(Debug, Clone, Copy, Default)]
pub struct SeverityCounts {
    pub info: usize,
    pub warn: usize,
    pub error: usize,
    pub critical: usize,
}

impl SeverityCounts {
    /// Get total count
    pub fn total(&self) -> usize {
        self.info + self.warn + self.error + self.critical
    }

    /// Check if any errors or critical issues
    pub fn has_issues(&self) -> bool {
        self.error > 0 || self.critical > 0
    }
}

// TODO: Fix insight_feed_test.rs - needs API structure updates
// #[cfg(test)]
// include!("insight_feed_test.rs");
