//! Disable Reason Matrix - disable_reason_matrix.001
//!
//! Per FRAME_IDENTITY_CONTRACT:
//! - Track which features/panels are disabled and why
//! - User-facing explanations for disabled states
//! - Support for missing dependencies, invalid stream, unsupported codec features

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Reason why a feature is disabled
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DisableReason {
    /// Missing dependency (e.g., decoder not available)
    MissingDependency(String),

    /// Invalid stream state (e.g., no frames loaded)
    InvalidStream(String),

    /// Unsupported codec feature (e.g., film grain not supported)
    UnsupportedCodecFeature(String),

    /// Configuration issue (e.g., missing config file)
    ConfigurationIssue(String),

    /// Insufficient data (e.g., not enough frames for comparison)
    InsufficientData(String),

    /// Resource limitation (e.g., not enough memory)
    ResourceLimitation(String),
}

impl DisableReason {
    /// Get user-facing description of the disable reason
    pub fn description(&self) -> String {
        match self {
            DisableReason::MissingDependency(dep) => {
                format!("Missing dependency: {}", dep)
            }
            DisableReason::InvalidStream(reason) => {
                format!("Invalid stream: {}", reason)
            }
            DisableReason::UnsupportedCodecFeature(feature) => {
                format!("Unsupported codec feature: {}", feature)
            }
            DisableReason::ConfigurationIssue(issue) => {
                format!("Configuration issue: {}", issue)
            }
            DisableReason::InsufficientData(reason) => {
                format!("Insufficient data: {}", reason)
            }
            DisableReason::ResourceLimitation(reason) => {
                format!("Resource limitation: {}", reason)
            }
        }
    }

    /// Get short label for the reason type
    pub fn label(&self) -> &'static str {
        match self {
            DisableReason::MissingDependency(_) => "Missing Dependency",
            DisableReason::InvalidStream(_) => "Invalid Stream",
            DisableReason::UnsupportedCodecFeature(_) => "Unsupported Feature",
            DisableReason::ConfigurationIssue(_) => "Configuration Issue",
            DisableReason::InsufficientData(_) => "Insufficient Data",
            DisableReason::ResourceLimitation(_) => "Resource Limited",
        }
    }
}

/// Feature/panel identifier
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FeatureId {
    // Timeline features
    TimelinePanel,
    TimelineMarkers,
    TimelineJumpControls,

    // Player features
    PlayerPanel,
    PlayerOverlayGrid,
    PlayerOverlayMv,
    PlayerOverlayQp,
    PlayerOverlayPartition,

    // Comparison features
    ComparePanel,
    CompareSideBySide,
    CompareDiffHeatmap,

    // Metrics features
    MetricsPanel,
    MetricsPsnr,
    MetricsSsim,
    MetricsVmaf,

    // Tree features
    TreePanel,
    TreeObuView,
    TreeSyntaxView,

    // Hex/Bit features
    HexPanel,
    BitPanel,

    // Custom feature
    Custom(String),
}

impl FeatureId {
    /// Get display name for feature
    pub fn name(&self) -> String {
        match self {
            FeatureId::TimelinePanel => "Timeline Panel".to_string(),
            FeatureId::TimelineMarkers => "Timeline Markers".to_string(),
            FeatureId::TimelineJumpControls => "Timeline Jump Controls".to_string(),
            FeatureId::PlayerPanel => "Player Panel".to_string(),
            FeatureId::PlayerOverlayGrid => "Grid Overlay".to_string(),
            FeatureId::PlayerOverlayMv => "Motion Vector Overlay".to_string(),
            FeatureId::PlayerOverlayQp => "QP Heatmap Overlay".to_string(),
            FeatureId::PlayerOverlayPartition => "Partition Overlay".to_string(),
            FeatureId::ComparePanel => "Compare Panel".to_string(),
            FeatureId::CompareSideBySide => "Side-by-Side Compare".to_string(),
            FeatureId::CompareDiffHeatmap => "Diff Heatmap".to_string(),
            FeatureId::MetricsPanel => "Metrics Panel".to_string(),
            FeatureId::MetricsPsnr => "PSNR Metrics".to_string(),
            FeatureId::MetricsSsim => "SSIM Metrics".to_string(),
            FeatureId::MetricsVmaf => "VMAF Metrics".to_string(),
            FeatureId::TreePanel => "Tree Panel".to_string(),
            FeatureId::TreeObuView => "OBU Tree View".to_string(),
            FeatureId::TreeSyntaxView => "Syntax Tree View".to_string(),
            FeatureId::HexPanel => "Hex Panel".to_string(),
            FeatureId::BitPanel => "Bit Panel".to_string(),
            FeatureId::Custom(name) => name.clone(),
        }
    }
}

/// Feature capability matrix
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisableReasonMatrix {
    /// Map of feature ID to optional disable reason
    /// If None, feature is enabled
    /// If Some(reason), feature is disabled with that reason
    features: HashMap<FeatureId, Option<DisableReason>>,
}

impl DisableReasonMatrix {
    /// Create a new empty matrix (all features enabled by default)
    pub fn new() -> Self {
        Self {
            features: HashMap::new(),
        }
    }

    /// Check if a feature is enabled
    pub fn is_enabled(&self, feature: &FeatureId) -> bool {
        self.features
            .get(feature)
            .is_none_or(|reason| reason.is_none())
    }

    /// Check if a feature is disabled
    pub fn is_disabled(&self, feature: &FeatureId) -> bool {
        !self.is_enabled(feature)
    }

    /// Get the reason why a feature is disabled (if any)
    pub fn get_disable_reason(&self, feature: &FeatureId) -> Option<&DisableReason> {
        self.features.get(feature).and_then(|r| r.as_ref())
    }

    /// Disable a feature with a reason
    pub fn disable(&mut self, feature: FeatureId, reason: DisableReason) {
        self.features.insert(feature, Some(reason));
    }

    /// Enable a feature
    pub fn enable(&mut self, feature: FeatureId) {
        self.features.insert(feature, None);
    }

    /// Remove a feature from the matrix (reverts to default enabled state)
    pub fn remove(&mut self, feature: &FeatureId) {
        self.features.remove(feature);
    }

    /// Get all disabled features
    pub fn disabled_features(&self) -> Vec<(&FeatureId, &DisableReason)> {
        self.features
            .iter()
            .filter_map(|(id, reason)| reason.as_ref().map(|r| (id, r)))
            .collect()
    }

    /// Get all enabled features
    pub fn enabled_features(&self) -> Vec<&FeatureId> {
        self.features
            .iter()
            .filter_map(
                |(id, reason)| {
                    if reason.is_none() {
                        Some(id)
                    } else {
                        None
                    }
                },
            )
            .collect()
    }

    /// Get count of disabled features
    pub fn disabled_count(&self) -> usize {
        self.features.values().filter(|r| r.is_some()).count()
    }

    /// Get count of enabled features
    pub fn enabled_count(&self) -> usize {
        self.features.values().filter(|r| r.is_none()).count()
    }

    /// Clear all disable reasons (enable all features)
    pub fn clear(&mut self) {
        self.features.clear();
    }
}

impl Default for DisableReasonMatrix {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Tests
// ============================================================================
#[cfg(test)]
include!("disable_reason_test.rs");
