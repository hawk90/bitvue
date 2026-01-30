//! Player Workspace - WS_PLAYER_SPATIAL (Monster Pack v9)
//!
//! Decoded frame viewer with multiple overlay layers:
//! - Grid overlay
//! - Motion vectors
//! - QP heatmap
//! - Partition visualization
//! - Reference frame visualization
//!
//! Module structure:
//! - mod.rs: Main PlayerWorkspace struct, OverlayType enum, show() method
//! - controls.rs: Overlay-specific control panels
//! - navigation.rs: Keyboard shortcuts, header, navigation controls, toolbar
//! - display.rs: Frame display area with overlay rendering
//! - overlays/: Overlay drawing functions (grid, heatmap, labels, motion, partition)

mod controls;
mod display;
mod navigation;
mod overlays;

use super::overlays::OverlayManager;
use super::player::{NavigationManager, PartitionLoader, TextureManager, ZoomManager};

pub use overlays::find_unit_by_offset;

/// Overlay types for player
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OverlayType {
    None,
    Grid,
    MotionVectors,
    QpHeatmap, // Per QP_HEATMAP_IMPLEMENTATION_SPEC.md
    Partition,
    ReferenceFrames,
    ModeLabels,    // VQAnalyzer parity: AMVP/Merge/Skip/Intra labels
    BitAllocation, // VQAnalyzer parity: bits per CTB heatmap
    MvMagnitude,   // VQAnalyzer parity: MV magnitude heatmap
    PuType,        // VQAnalyzer parity: PU type categorical overlay
}

impl OverlayType {
    pub fn label(&self) -> &'static str {
        match self {
            OverlayType::None => "None",
            OverlayType::Grid => "Grid",
            OverlayType::MotionVectors => "Motion Vectors",
            OverlayType::QpHeatmap => "QP Heatmap",
            OverlayType::Partition => "Partition",
            OverlayType::ReferenceFrames => "Ref Frames",
            OverlayType::ModeLabels => "Mode Labels",
            OverlayType::BitAllocation => "Bit Alloc",
            OverlayType::MvMagnitude => "MV Magnitude",
            OverlayType::PuType => "PU Type",
        }
    }
}

/// Player workspace state
///
/// After god object refactoring (Batch 2): 27 fields → 5 fields
/// After module decomposition: Texture, Navigation, Zoom, Partition extracted.
pub struct PlayerWorkspace {
    /// Texture manager (frame texture and dimensions)
    texture: TextureManager,
    /// Navigation manager (frame navigation and finding)
    navigation: NavigationManager,
    /// Zoom manager (zoom level and pan offset)
    zoom: ZoomManager,
    /// Overlay manager (contains all overlay state)
    overlays: OverlayManager,
}

impl PlayerWorkspace {
    pub fn new() -> Self {
        Self {
            texture: TextureManager::new(),
            navigation: NavigationManager::new(),
            zoom: ZoomManager::new(),
            overlays: OverlayManager::new(),
        }
    }

    /// Update the displayed frame
    pub fn set_frame(&mut self, ctx: &egui::Context, image: egui::ColorImage) {
        self.texture.set_frame(ctx, image);

        // Notify overlay manager of frame change
        self.overlays.on_frame_change();

        // Try to load partition data when frame changes
        self.load_partition_data();
        self.load_partition_grid();
    }

    /// Check if an overlay is currently active
    pub fn is_overlay_active(&self, overlay: OverlayType) -> bool {
        self.overlays.is_active(overlay)
    }

    /// Toggle an overlay on/off
    pub fn toggle_overlay(&mut self, overlay: OverlayType) {
        self.overlays.toggle(overlay);
    }

    /// Set overlay active state
    pub fn set_overlay(&mut self, overlay: OverlayType, active: bool) {
        self.overlays.set_active(overlay, active);
    }

    /// Load partition data from JSON mock file
    /// Per PARTITION_GRID_IMPLEMENTATION_SPEC.md §1
    fn load_partition_data(&mut self) {
        if self.overlays.partition.data.is_some() {
            return; // Already loaded
        }

        if let Some((w, h)) = self.texture.frame_size() {
            self.overlays.partition.data = Some(PartitionLoader::load_partition_data(w, h));
        }
    }

    /// Load partition grid (hierarchical blocks)
    fn load_partition_grid(&mut self) {
        if self.overlays.partition.grid.is_some() {
            return; // Already loaded
        }

        if let Some((w, h)) = self.texture.frame_size() {
            self.overlays.partition.grid = Some(PartitionLoader::load_partition_grid(w, h));
        }
    }

    /// Find frame unit by frame index (for navigation)
    fn find_frame_by_index(
        units: Option<&[bitvue_core::UnitNode]>,
        target_index: usize,
    ) -> Option<&bitvue_core::UnitNode> {
        units.and_then(|u| Self::find_frame_recursive(u, target_index))
    }

    fn find_frame_recursive(
        units: &[bitvue_core::UnitNode],
        target_index: usize,
    ) -> Option<&bitvue_core::UnitNode> {
        for unit in units {
            if unit.frame_index == Some(target_index) {
                return Some(unit);
            }
            if !unit.children.is_empty() {
                if let Some(found) = Self::find_frame_recursive(&unit.children, target_index) {
                    return Some(found);
                }
            }
        }
        None
    }

    /// Show the player workspace
    /// Returns Command for navigation (StepForward, StepBackward, JumpToFrame)
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        container: Option<&bitvue_core::ContainerModel>,
        selection: Option<&bitvue_core::SelectionState>,
        units: Option<&[bitvue_core::UnitNode]>,
        total_frames: usize,
    ) -> Option<bitvue_core::Command> {
        let mut result_command: Option<bitvue_core::Command> = None;

        // Get current frame index
        let current_frame = selection
            .and_then(|sel| sel.temporal.as_ref())
            .map(|t| t.frame_index())
            .unwrap_or(0);

        // Keyboard shortcuts (VQAnalyzer parity)
        if let Some(cmd) = self.handle_keyboard_shortcuts(ui, units, current_frame, total_frames) {
            result_command = Some(cmd);
        }

        // Header with HUD info
        self.show_header(ui, container, selection, units, total_frames);

        ui.separator();

        // Navigation controls (VQAnalyzer parity)
        if let Some(cmd) = self.show_navigation_controls(ui, units, current_frame, total_frames) {
            result_command = Some(cmd);
        }

        // Toolbar: Overlay toggles + zoom controls
        self.show_toolbar(ui);

        // Overlay-specific control panels
        self.show_all_controls(ui);

        ui.separator();

        // Frame display area with overlays
        if let Some(cmd) = self.show_frame_display(ui, selection, units) {
            result_command = Some(cmd);
        }

        result_command
    }
}

impl Default for PlayerWorkspace {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_overlay_type_label() {
        assert_eq!(OverlayType::None.label(), "None");
        assert_eq!(OverlayType::Grid.label(), "Grid");
        assert_eq!(OverlayType::MotionVectors.label(), "Motion Vectors");
        assert_eq!(OverlayType::QpHeatmap.label(), "QP Heatmap");
        assert_eq!(OverlayType::Partition.label(), "Partition");
        assert_eq!(OverlayType::ReferenceFrames.label(), "Ref Frames");
        assert_eq!(OverlayType::ModeLabels.label(), "Mode Labels");
        assert_eq!(OverlayType::BitAllocation.label(), "Bit Alloc");
        assert_eq!(OverlayType::MvMagnitude.label(), "MV Magnitude");
        assert_eq!(OverlayType::PuType.label(), "PU Type");
    }

    #[test]
    fn test_overlay_type_equality() {
        assert_eq!(OverlayType::Grid, OverlayType::Grid);
        assert_ne!(OverlayType::Grid, OverlayType::QpHeatmap);
    }

    #[test]
    fn test_player_workspace_new_defaults() {
        let ws = PlayerWorkspace::new();

        // Verify zoom starts at 1.0
        assert!((ws.zoom.zoom() - 1.0).abs() < f32::EPSILON);

        // Verify no active overlays initially
        assert!(ws.overlays.active.is_empty());

        // Verify grid size is 64 (default per spec)
        assert_eq!(ws.overlays.grid.size, 64);

        // Verify qp_opacity is 0.45 (default per QP_HEATMAP_IMPLEMENTATION_SPEC.md)
        assert!((ws.overlays.qp.opacity - 0.45).abs() < f32::EPSILON);
    }

    #[test]
    fn test_toggle_overlay_adds_when_not_present() {
        let mut ws = PlayerWorkspace::new();

        assert!(!ws.is_overlay_active(OverlayType::Grid));
        ws.toggle_overlay(OverlayType::Grid);
        assert!(ws.is_overlay_active(OverlayType::Grid));
    }

    #[test]
    fn test_toggle_overlay_removes_when_present() {
        let mut ws = PlayerWorkspace::new();

        ws.toggle_overlay(OverlayType::Grid);
        assert!(ws.is_overlay_active(OverlayType::Grid));

        ws.toggle_overlay(OverlayType::Grid);
        assert!(!ws.is_overlay_active(OverlayType::Grid));
    }
}
