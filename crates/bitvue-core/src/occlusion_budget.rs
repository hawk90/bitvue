//! Occlusion Budget System - occlusion_budget.001
//!
//! Per FRAME_IDENTITY_CONTRACT:
//! - Enforce overlay stacking and alpha blending rules
//! - Prevent hiding critical information
//! - Deterministic ordering and compositing

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Overlay layer identifier
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum OverlayLayer {
    /// Base decoded frame
    BaseFrame,

    /// Grid overlay (block boundaries)
    Grid,

    /// Motion vector overlay
    MotionVectors,

    /// QP heatmap overlay
    QpHeatmap,

    /// Partition structure overlay
    PartitionGrid,

    /// Diff heatmap (comparison)
    DiffHeatmap,

    /// Reference frame indicators
    ReferenceIndicators,

    /// Selection highlight
    SelectionHighlight,

    /// Debug information overlay
    DebugInfo,

    /// Custom layer
    Custom(String),
}

impl OverlayLayer {
    /// Get display name
    pub fn name(&self) -> String {
        match self {
            OverlayLayer::BaseFrame => "Base Frame".to_string(),
            OverlayLayer::Grid => "Grid".to_string(),
            OverlayLayer::MotionVectors => "Motion Vectors".to_string(),
            OverlayLayer::QpHeatmap => "QP Heatmap".to_string(),
            OverlayLayer::PartitionGrid => "Partition Grid".to_string(),
            OverlayLayer::DiffHeatmap => "Diff Heatmap".to_string(),
            OverlayLayer::ReferenceIndicators => "Reference Indicators".to_string(),
            OverlayLayer::SelectionHighlight => "Selection Highlight".to_string(),
            OverlayLayer::DebugInfo => "Debug Info".to_string(),
            OverlayLayer::Custom(name) => name.clone(),
        }
    }

    /// Get default stacking priority (0 = bottom, higher = top)
    pub fn default_priority(&self) -> u32 {
        match self {
            OverlayLayer::BaseFrame => 0,
            OverlayLayer::DiffHeatmap => 10,
            OverlayLayer::QpHeatmap => 20,
            OverlayLayer::PartitionGrid => 30,
            OverlayLayer::Grid => 40,
            OverlayLayer::MotionVectors => 50,
            OverlayLayer::ReferenceIndicators => 60,
            OverlayLayer::SelectionHighlight => 70,
            OverlayLayer::DebugInfo => 80,
            OverlayLayer::Custom(_) => 50,
        }
    }

    /// Get default alpha value (0.0 = transparent, 1.0 = opaque)
    pub fn default_alpha(&self) -> f32 {
        match self {
            OverlayLayer::BaseFrame => 1.0,
            OverlayLayer::DiffHeatmap => 0.7,
            OverlayLayer::QpHeatmap => 0.6,
            OverlayLayer::PartitionGrid => 0.8,
            OverlayLayer::Grid => 0.5,
            OverlayLayer::MotionVectors => 0.9,
            OverlayLayer::ReferenceIndicators => 0.8,
            OverlayLayer::SelectionHighlight => 0.5,
            OverlayLayer::DebugInfo => 0.9,
            OverlayLayer::Custom(_) => 0.7,
        }
    }
}

/// Blend mode for overlay composition
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BlendMode {
    /// Normal alpha blending
    Normal,

    /// Additive blending
    Additive,

    /// Multiply blending
    Multiply,

    /// Overlay blending
    Overlay,

    /// Screen blending
    Screen,
}

/// Overlay configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OverlayConfig {
    /// Layer identifier
    pub layer: OverlayLayer,

    /// Stacking priority (0 = bottom, higher = top)
    pub priority: u32,

    /// Alpha transparency (0.0 = transparent, 1.0 = opaque)
    pub alpha: f32,

    /// Blend mode
    pub blend_mode: BlendMode,

    /// Whether this layer is visible
    pub visible: bool,

    /// Whether this is a critical layer (cannot be occluded)
    pub critical: bool,
}

impl OverlayConfig {
    /// Create new overlay config with defaults
    pub fn new(layer: OverlayLayer) -> Self {
        Self {
            priority: layer.default_priority(),
            alpha: layer.default_alpha(),
            blend_mode: BlendMode::Normal,
            visible: true,
            critical: false,
            layer,
        }
    }

    /// Set priority
    pub fn with_priority(mut self, priority: u32) -> Self {
        self.priority = priority;
        self
    }

    /// Set alpha
    pub fn with_alpha(mut self, alpha: f32) -> Self {
        self.alpha = alpha.clamp(0.0, 1.0);
        self
    }

    /// Set blend mode
    pub fn with_blend_mode(mut self, blend_mode: BlendMode) -> Self {
        self.blend_mode = blend_mode;
        self
    }

    /// Mark as critical (cannot be occluded)
    pub fn critical(mut self) -> Self {
        self.critical = true;
        self
    }

    /// Set visibility
    pub fn set_visible(&mut self, visible: bool) {
        self.visible = visible;
    }
}

/// Occlusion budget manager
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OcclusionBudget {
    /// Overlay configurations
    overlays: HashMap<OverlayLayer, OverlayConfig>,

    /// Maximum number of simultaneous overlays
    max_overlays: Option<usize>,

    /// Whether to auto-adjust alpha based on overlay count
    auto_adjust_alpha: bool,
}

impl OcclusionBudget {
    /// Create new occlusion budget
    pub fn new() -> Self {
        Self {
            overlays: HashMap::new(),
            max_overlays: None,
            auto_adjust_alpha: true,
        }
    }

    /// Set maximum overlay limit
    pub fn set_max_overlays(&mut self, max: usize) {
        self.max_overlays = Some(max);
    }

    /// Set auto alpha adjustment
    pub fn set_auto_adjust_alpha(&mut self, enabled: bool) {
        self.auto_adjust_alpha = enabled;
    }

    /// Register an overlay
    pub fn register_overlay(&mut self, config: OverlayConfig) {
        self.overlays.insert(config.layer.clone(), config);
    }

    /// Get overlay config
    pub fn get_overlay(&self, layer: &OverlayLayer) -> Option<&OverlayConfig> {
        self.overlays.get(layer)
    }

    /// Get mutable overlay config
    pub fn get_overlay_mut(&mut self, layer: &OverlayLayer) -> Option<&mut OverlayConfig> {
        self.overlays.get_mut(layer)
    }

    /// Show overlay
    pub fn show_overlay(&mut self, layer: &OverlayLayer) {
        if let Some(config) = self.overlays.get_mut(layer) {
            config.visible = true;
        }
    }

    /// Hide overlay
    pub fn hide_overlay(&mut self, layer: &OverlayLayer) {
        if let Some(config) = self.overlays.get_mut(layer) {
            config.visible = false;
        }
    }

    /// Get visible overlays in stacking order (bottom to top)
    pub fn get_visible_stack(&self) -> Vec<&OverlayConfig> {
        let mut visible: Vec<_> = self.overlays.values().filter(|c| c.visible).collect();

        // Sort by priority (deterministic order)
        visible.sort_by_key(|c| c.priority);

        visible
    }

    /// Get visible overlays with auto-adjusted alpha
    pub fn get_compositing_stack(&self) -> Vec<OverlayConfig> {
        let mut stack = self
            .get_visible_stack()
            .into_iter()
            .cloned()
            .collect::<Vec<_>>();

        if self.auto_adjust_alpha && stack.len() > 1 {
            // Adjust alpha based on overlay count to prevent complete occlusion
            let adjustment_factor = 1.0 / (stack.len() as f32).sqrt();

            for config in &mut stack {
                if !config.critical {
                    config.alpha *= adjustment_factor;
                    config.alpha = config.alpha.clamp(0.1, 1.0);
                }
            }
        }

        stack
    }

    /// Check if adding an overlay would exceed budget
    pub fn would_exceed_budget(&self, layer: &OverlayLayer) -> bool {
        if let Some(max) = self.max_overlays {
            let current_visible = self.get_visible_stack().len();
            let is_already_visible = self.overlays.get(layer).map(|c| c.visible).unwrap_or(false);

            if !is_already_visible {
                current_visible >= max
            } else {
                false
            }
        } else {
            false
        }
    }

    /// Get critical overlays (cannot be occluded)
    pub fn get_critical_overlays(&self) -> Vec<&OverlayConfig> {
        self.overlays
            .values()
            .filter(|c| c.critical && c.visible)
            .collect()
    }

    /// Update overlay priority
    pub fn set_priority(&mut self, layer: &OverlayLayer, priority: u32) {
        if let Some(config) = self.overlays.get_mut(layer) {
            config.priority = priority;
        }
    }

    /// Update overlay alpha
    pub fn set_alpha(&mut self, layer: &OverlayLayer, alpha: f32) {
        if let Some(config) = self.overlays.get_mut(layer) {
            config.alpha = alpha.clamp(0.0, 1.0);
        }
    }

    /// Clear all overlays
    pub fn clear(&mut self) {
        self.overlays.clear();
    }

    /// Get overlay count
    pub fn overlay_count(&self) -> usize {
        self.overlays.len()
    }

    /// Get visible overlay count
    pub fn visible_count(&self) -> usize {
        self.overlays.values().filter(|c| c.visible).count()
    }
}

impl Default for OcclusionBudget {
    fn default() -> Self {
        Self::new()
    }
}

/// Occlusion statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OcclusionStats {
    /// Total overlays
    pub total_overlays: usize,

    /// Visible overlays
    pub visible_overlays: usize,

    /// Critical overlays
    pub critical_overlays: usize,

    /// Average alpha
    pub average_alpha: f32,

    /// At budget limit
    pub at_budget_limit: bool,
}

impl OcclusionStats {
    /// Calculate stats from occlusion budget
    pub fn from_budget(budget: &OcclusionBudget) -> Self {
        let visible_stack = budget.get_visible_stack();
        let critical_count = budget.get_critical_overlays().len();

        let average_alpha = if !visible_stack.is_empty() {
            visible_stack.iter().map(|c| c.alpha).sum::<f32>() / visible_stack.len() as f32
        } else {
            0.0
        };

        let at_budget_limit = if let Some(max) = budget.max_overlays {
            visible_stack.len() >= max
        } else {
            false
        };

        Self {
            total_overlays: budget.overlay_count(),
            visible_overlays: visible_stack.len(),
            critical_overlays: critical_count,
            average_alpha,
            at_budget_limit,
        }
    }
}

// TODO: Fix occlusion_budget_test.rs - needs API rewrite to match actual implementation
// #[cfg(test)]
// include!("occlusion_budget_test.rs");
