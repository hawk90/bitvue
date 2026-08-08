//! Overlay Stack Manager - T2-2
//!
//! Per WS_PLAYER_SPATIAL:
//! - Supports simultaneous overlay layers (QP, MV, Partition, Diff)
//! - Per-layer opacity control
//! - Layer ordering (z-order management)
//!
//! Acceptance Test WS22: Overlay stack supports 2+ overlays simultaneously

use serde::{Deserialize, Serialize};

/// Overlay layer types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum OverlayLayerType {
    /// QP heatmap overlay
    QpHeatmap,
    /// Motion vector overlay
    MvVectors,
    /// Partition grid overlay
    PartitionGrid,
    /// Diff heatmap (A/B compare)
    DiffHeatmap,
    /// Block grid (scaffold)
    BlockGrid,
}

impl OverlayLayerType {
    /// Get layer name for display
    pub fn name(&self) -> &'static str {
        match self {
            OverlayLayerType::QpHeatmap => "QP Heatmap",
            OverlayLayerType::MvVectors => "MV Vectors",
            OverlayLayerType::PartitionGrid => "Partition Grid",
            OverlayLayerType::DiffHeatmap => "Diff Heatmap",
            OverlayLayerType::BlockGrid => "Block Grid",
        }
    }

    /// Get short name for compact display
    pub fn short_name(&self) -> &'static str {
        match self {
            OverlayLayerType::QpHeatmap => "QP",
            OverlayLayerType::MvVectors => "MV",
            OverlayLayerType::PartitionGrid => "Part",
            OverlayLayerType::DiffHeatmap => "Diff",
            OverlayLayerType::BlockGrid => "Grid",
        }
    }
}

/// Overlay layer configuration
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OverlayLayer {
    /// Layer type
    pub layer_type: OverlayLayerType,

    /// Layer is visible
    pub visible: bool,

    /// Layer opacity (0.0 = fully transparent, 1.0 = fully opaque)
    pub opacity: f32,

    /// Z-order (higher = rendered on top)
    pub z_order: u32,
}

impl OverlayLayer {
    /// Create a new overlay layer with default settings
    pub fn new(layer_type: OverlayLayerType, z_order: u32) -> Self {
        Self {
            layer_type,
            visible: true,
            opacity: 1.0,
            z_order,
        }
    }

    /// Set visibility
    pub fn set_visible(&mut self, visible: bool) {
        self.visible = visible;
    }

    /// Set opacity (clamped to 0.0..=1.0)
    pub fn set_opacity(&mut self, opacity: f32) {
        self.opacity = opacity.clamp(0.0, 1.0);
    }

    /// Adjust opacity by delta
    pub fn adjust_opacity(&mut self, delta: f32) {
        self.set_opacity(self.opacity + delta);
    }

    /// Check if layer is active (visible and has non-zero opacity)
    pub fn is_active(&self) -> bool {
        self.visible && self.opacity > 0.0
    }
}

/// Overlay stack manager
///
/// Manages multiple simultaneous overlay layers with z-ordering and opacity control.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OverlayStack {
    /// Active layers
    layers: Vec<OverlayLayer>,

    /// Next z-order for new layers
    next_z_order: u32,
}

impl OverlayStack {
    /// Create a new empty overlay stack
    pub fn new() -> Self {
        Self {
            layers: Vec::new(),
            next_z_order: 0,
        }
    }

    /// Add a layer to the stack
    pub fn add_layer(&mut self, layer_type: OverlayLayerType) -> &mut OverlayLayer {
        // Check if layer already exists
        if let Some(idx) = self.layers.iter().position(|l| l.layer_type == layer_type) {
            return &mut self.layers[idx];
        }

        // Create new layer
        let layer = OverlayLayer::new(layer_type, self.next_z_order);
        self.next_z_order += 1;
        self.layers.push(layer);

        // Sort by z-order
        self.sort_by_z_order();

        // Return mutable reference to the new layer
        self.layers
            .iter_mut()
            .find(|l| l.layer_type == layer_type)
            .unwrap()
    }

    /// Remove a layer from the stack
    pub fn remove_layer(&mut self, layer_type: OverlayLayerType) -> bool {
        if let Some(idx) = self.layers.iter().position(|l| l.layer_type == layer_type) {
            self.layers.remove(idx);
            true
        } else {
            false
        }
    }

    /// Get a layer by type
    pub fn get_layer(&self, layer_type: OverlayLayerType) -> Option<&OverlayLayer> {
        self.layers.iter().find(|l| l.layer_type == layer_type)
    }

    /// Get a mutable layer by type
    pub fn get_layer_mut(&mut self, layer_type: OverlayLayerType) -> Option<&mut OverlayLayer> {
        self.layers.iter_mut().find(|l| l.layer_type == layer_type)
    }

    /// Set layer visibility
    pub fn set_layer_visible(&mut self, layer_type: OverlayLayerType, visible: bool) {
        if let Some(layer) = self.get_layer_mut(layer_type) {
            layer.set_visible(visible);
        }
    }

    /// Set layer opacity
    pub fn set_layer_opacity(&mut self, layer_type: OverlayLayerType, opacity: f32) {
        if let Some(layer) = self.get_layer_mut(layer_type) {
            layer.set_opacity(opacity);
        }
    }

    /// Adjust layer opacity
    pub fn adjust_layer_opacity(&mut self, layer_type: OverlayLayerType, delta: f32) {
        if let Some(layer) = self.get_layer_mut(layer_type) {
            layer.adjust_opacity(delta);
        }
    }

    /// Toggle layer visibility
    pub fn toggle_layer(&mut self, layer_type: OverlayLayerType) {
        if let Some(layer) = self.get_layer_mut(layer_type) {
            layer.visible = !layer.visible;
        } else {
            // Add layer if it doesn't exist
            self.add_layer(layer_type);
        }
    }

    /// Get all layers sorted by z-order (bottom to top)
    pub fn layers_by_z_order(&self) -> Vec<&OverlayLayer> {
        let mut layers: Vec<_> = self.layers.iter().collect();
        layers.sort_by_key(|l| l.z_order);
        layers
    }

    /// Get active layers (visible and opacity > 0) sorted by z-order
    pub fn active_layers(&self) -> Vec<&OverlayLayer> {
        let mut layers: Vec<_> = self.layers.iter().filter(|l| l.is_active()).collect();
        layers.sort_by_key(|l| l.z_order);
        layers
    }

    /// Get count of active layers
    pub fn active_count(&self) -> usize {
        self.layers.iter().filter(|l| l.is_active()).count()
    }

    /// Check if any layers are active
    pub fn has_active_layers(&self) -> bool {
        self.active_count() > 0
    }

    /// Clear all layers
    pub fn clear(&mut self) {
        self.layers.clear();
    }

    /// Move layer to top (highest z-order)
    pub fn move_to_top(&mut self, layer_type: OverlayLayerType) {
        // Find the layer index first
        let layer_idx = self.layers.iter().position(|l| l.layer_type == layer_type);

        if let Some(idx) = layer_idx {
            self.layers[idx].z_order = self.next_z_order;
            self.next_z_order += 1;
            self.sort_by_z_order();
        }
    }

    /// Move layer to bottom (lowest z-order)
    pub fn move_to_bottom(&mut self, layer_type: OverlayLayerType) {
        if self.layers.is_empty() {
            return;
        }

        // Find minimum z-order
        let min_z = self.layers.iter().map(|l| l.z_order).min().unwrap_or(0);

        // Find the layer index
        let layer_idx = self.layers.iter().position(|l| l.layer_type == layer_type);

        if let Some(idx) = layer_idx {
            if min_z > 0 {
                self.layers[idx].z_order = min_z - 1;
            } else {
                // Need to shift all layers up
                for l in &mut self.layers {
                    l.z_order += 1;
                }
                self.layers[idx].z_order = 0;
            }
            self.sort_by_z_order();
        }
    }

    /// Sort layers by z-order
    fn sort_by_z_order(&mut self) {
        self.layers.sort_by_key(|l| l.z_order);
    }

    /// Get active layer names (for display)
    pub fn active_layer_names(&self) -> Vec<&'static str> {
        self.active_layers()
            .iter()
            .map(|l| l.layer_type.short_name())
            .collect()
    }
}

impl Default for OverlayStack {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_overlay_layer_type() {
        let qp = OverlayLayerType::QpHeatmap;
        assert_eq!(qp.name(), "QP Heatmap");
        assert_eq!(qp.short_name(), "QP");

        let mv = OverlayLayerType::MvVectors;
        assert_eq!(mv.name(), "MV Vectors");
        assert_eq!(mv.short_name(), "MV");
    }

    #[test]
    fn test_overlay_layer_creation() {
        let layer = OverlayLayer::new(OverlayLayerType::QpHeatmap, 0);
        assert_eq!(layer.layer_type, OverlayLayerType::QpHeatmap);
        assert!(layer.visible);
        assert_eq!(layer.opacity, 1.0);
        assert_eq!(layer.z_order, 0);
        assert!(layer.is_active());
    }

    #[test]
    fn test_overlay_layer_opacity() {
        let mut layer = OverlayLayer::new(OverlayLayerType::QpHeatmap, 0);

        layer.set_opacity(0.5);
        assert_eq!(layer.opacity, 0.5);
        assert!(layer.is_active());

        layer.set_opacity(0.0);
        assert_eq!(layer.opacity, 0.0);
        assert!(!layer.is_active());

        layer.set_opacity(1.5); // Should clamp to 1.0
        assert_eq!(layer.opacity, 1.0);

        layer.set_opacity(-0.5); // Should clamp to 0.0
        assert_eq!(layer.opacity, 0.0);
    }

    #[test]
    fn test_overlay_layer_adjust_opacity() {
        let mut layer = OverlayLayer::new(OverlayLayerType::QpHeatmap, 0);
        layer.set_opacity(0.5);

        layer.adjust_opacity(0.2);
        assert!((layer.opacity - 0.7).abs() < 0.001);

        layer.adjust_opacity(-0.3);
        assert!((layer.opacity - 0.4).abs() < 0.001);

        layer.adjust_opacity(1.0); // Should clamp to 1.0
        assert_eq!(layer.opacity, 1.0);
    }

    #[test]
    fn test_overlay_stack_creation() {
        let stack = OverlayStack::new();
        assert_eq!(stack.layers.len(), 0);
        assert!(!stack.has_active_layers());
    }

    #[test]
    fn test_overlay_stack_add_layer() {
        let mut stack = OverlayStack::new();

        stack.add_layer(OverlayLayerType::QpHeatmap);
        assert_eq!(stack.layers.len(), 1);
        assert!(stack.has_active_layers());
        assert_eq!(stack.active_count(), 1);

        stack.add_layer(OverlayLayerType::MvVectors);
        assert_eq!(stack.layers.len(), 2);
        assert_eq!(stack.active_count(), 2);

        // Adding same layer again should not duplicate
        stack.add_layer(OverlayLayerType::QpHeatmap);
        assert_eq!(stack.layers.len(), 2);
    }

    #[test]
    fn test_overlay_stack_remove_layer() {
        let mut stack = OverlayStack::new();
        stack.add_layer(OverlayLayerType::QpHeatmap);
        stack.add_layer(OverlayLayerType::MvVectors);

        assert!(stack.remove_layer(OverlayLayerType::QpHeatmap));
        assert_eq!(stack.layers.len(), 1);

        assert!(!stack.remove_layer(OverlayLayerType::QpHeatmap)); // Already removed
        assert_eq!(stack.layers.len(), 1);
    }

    #[test]
    fn test_overlay_stack_visibility() {
        let mut stack = OverlayStack::new();
        stack.add_layer(OverlayLayerType::QpHeatmap);

        assert_eq!(stack.active_count(), 1);

        stack.set_layer_visible(OverlayLayerType::QpHeatmap, false);
        assert_eq!(stack.active_count(), 0);

        stack.set_layer_visible(OverlayLayerType::QpHeatmap, true);
        assert_eq!(stack.active_count(), 1);
    }

    #[test]
    fn test_overlay_stack_opacity() {
        let mut stack = OverlayStack::new();
        stack.add_layer(OverlayLayerType::QpHeatmap);

        stack.set_layer_opacity(OverlayLayerType::QpHeatmap, 0.5);
        let layer = stack.get_layer(OverlayLayerType::QpHeatmap).unwrap();
        assert_eq!(layer.opacity, 0.5);

        stack.adjust_layer_opacity(OverlayLayerType::QpHeatmap, 0.3);
        let layer = stack.get_layer(OverlayLayerType::QpHeatmap).unwrap();
        assert_eq!(layer.opacity, 0.8);
    }

    #[test]
    fn test_overlay_stack_toggle() {
        let mut stack = OverlayStack::new();

        // Toggle on (adds layer if not exists)
        stack.toggle_layer(OverlayLayerType::QpHeatmap);
        assert_eq!(stack.layers.len(), 1);
        assert!(
            stack
                .get_layer(OverlayLayerType::QpHeatmap)
                .unwrap()
                .visible
        );

        // Toggle off
        stack.toggle_layer(OverlayLayerType::QpHeatmap);
        assert!(
            !stack
                .get_layer(OverlayLayerType::QpHeatmap)
                .unwrap()
                .visible
        );

        // Toggle on
        stack.toggle_layer(OverlayLayerType::QpHeatmap);
        assert!(
            stack
                .get_layer(OverlayLayerType::QpHeatmap)
                .unwrap()
                .visible
        );
    }

    #[test]
    fn test_overlay_stack_z_order() {
        let mut stack = OverlayStack::new();

        stack.add_layer(OverlayLayerType::QpHeatmap);
        stack.add_layer(OverlayLayerType::MvVectors);
        stack.add_layer(OverlayLayerType::PartitionGrid);

        let layers = stack.layers_by_z_order();
        assert_eq!(layers.len(), 3);
        assert_eq!(layers[0].layer_type, OverlayLayerType::QpHeatmap);
        assert_eq!(layers[1].layer_type, OverlayLayerType::MvVectors);
        assert_eq!(layers[2].layer_type, OverlayLayerType::PartitionGrid);

        // Move QP to top
        stack.move_to_top(OverlayLayerType::QpHeatmap);
        let layers = stack.layers_by_z_order();
        assert_eq!(layers[2].layer_type, OverlayLayerType::QpHeatmap);

        // Move Partition to bottom
        stack.move_to_bottom(OverlayLayerType::PartitionGrid);
        let layers = stack.layers_by_z_order();
        assert_eq!(layers[0].layer_type, OverlayLayerType::PartitionGrid);
    }

    #[test]
    fn test_overlay_stack_active_layers() {
        let mut stack = OverlayStack::new();

        stack.add_layer(OverlayLayerType::QpHeatmap);
        stack.add_layer(OverlayLayerType::MvVectors);
        stack.add_layer(OverlayLayerType::PartitionGrid);

        // All active
        assert_eq!(stack.active_count(), 3);

        // Hide one
        stack.set_layer_visible(OverlayLayerType::MvVectors, false);
        assert_eq!(stack.active_count(), 2);

        // Set opacity to 0 for another
        stack.set_layer_opacity(OverlayLayerType::QpHeatmap, 0.0);
        assert_eq!(stack.active_count(), 1);

        let active = stack.active_layers();
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].layer_type, OverlayLayerType::PartitionGrid);
    }

    #[test]
    fn test_overlay_stack_active_names() {
        let mut stack = OverlayStack::new();

        stack.add_layer(OverlayLayerType::QpHeatmap);
        stack.add_layer(OverlayLayerType::MvVectors);

        let names = stack.active_layer_names();
        assert_eq!(names.len(), 2);
        assert!(names.contains(&"QP"));
        assert!(names.contains(&"MV"));
    }

    #[test]
    fn test_overlay_stack_clear() {
        let mut stack = OverlayStack::new();

        stack.add_layer(OverlayLayerType::QpHeatmap);
        stack.add_layer(OverlayLayerType::MvVectors);
        assert_eq!(stack.layers.len(), 2);

        stack.clear();
        assert_eq!(stack.layers.len(), 0);
        assert!(!stack.has_active_layers());
    }

    #[test]
    fn test_ws22_acceptance_simultaneous_overlays() {
        // WS22: Overlay stack supports 2+ overlays simultaneously
        let mut stack = OverlayStack::new();

        // Add multiple overlays
        stack.add_layer(OverlayLayerType::QpHeatmap);
        stack.add_layer(OverlayLayerType::MvVectors);
        stack.add_layer(OverlayLayerType::PartitionGrid);

        // All should be active simultaneously
        assert!(
            stack.active_count() >= 2,
            "WS22: Must support 2+ simultaneous overlays"
        );
        assert_eq!(stack.active_count(), 3);

        // Check they're all visible
        let active = stack.active_layers();
        assert_eq!(active.len(), 3);

        // Verify z-ordering is maintained
        let layers = stack.layers_by_z_order();
        assert!(layers[0].z_order < layers[1].z_order);
        assert!(layers[1].z_order < layers[2].z_order);
    }
}
