//! Overlay Manager - Extracted from PlayerWorkspace
//!
//! Coordinates overlay activation and state.
//! Reduces PlayerWorkspace from 27 fields to ~5 fields.

use super::{
    BitAllocationOverlayState, GridOverlayState, ModeLabelOverlayState, MvMagnitudeOverlayState,
    MvOverlayState, PartitionOverlayState, PuTypeOverlayState, QpOverlayState,
};
use crate::workspaces::player_workspace::OverlayType;

/// Overlay manager coordinating all overlay states
pub struct OverlayManager {
    /// Active overlays (can have multiple)
    pub active: Vec<OverlayType>,
    /// Grid overlay state
    pub grid: GridOverlayState,
    /// QP heatmap overlay state
    pub qp: QpOverlayState,
    /// Partition overlay state
    pub partition: PartitionOverlayState,
    /// MV overlay state
    pub mv: MvOverlayState,
    /// Mode labels overlay state (VQAnalyzer parity)
    pub mode_labels: ModeLabelOverlayState,
    /// Bit allocation heatmap (VQAnalyzer parity)
    pub bit_allocation: BitAllocationOverlayState,
    /// MV magnitude heatmap (VQAnalyzer parity)
    pub mv_magnitude: MvMagnitudeOverlayState,
    /// PU type overlay (VQAnalyzer parity)
    pub pu_type: PuTypeOverlayState,
}

impl OverlayManager {
    /// Create new overlay manager with default states
    pub fn new() -> Self {
        Self {
            active: Vec::new(),
            grid: GridOverlayState::new(),
            qp: QpOverlayState::new(),
            partition: PartitionOverlayState::new(),
            mv: MvOverlayState::new(),
            mode_labels: ModeLabelOverlayState::new(),
            bit_allocation: BitAllocationOverlayState::new(),
            mv_magnitude: MvMagnitudeOverlayState::new(),
            pu_type: PuTypeOverlayState::new(),
        }
    }

    /// Check if an overlay is currently active
    pub fn is_active(&self, overlay: OverlayType) -> bool {
        self.active.contains(&overlay)
    }

    /// Toggle an overlay on/off
    pub fn toggle(&mut self, overlay: OverlayType) {
        if let Some(pos) = self.active.iter().position(|o| *o == overlay) {
            self.active.remove(pos);
        } else {
            self.active.push(overlay);
        }
    }

    /// Set overlay active state
    pub fn set_active(&mut self, overlay: OverlayType, active: bool) {
        let is_active = self.active.contains(&overlay);
        if active && !is_active {
            self.active.push(overlay);
        } else if !active && is_active {
            if let Some(pos) = self.active.iter().position(|o| *o == overlay) {
                self.active.remove(pos);
            }
        }
    }

    /// Get count of active overlays
    pub fn active_count(&self) -> usize {
        self.active.len()
    }

    /// Check if any overlays are active
    pub fn has_active(&self) -> bool {
        !self.active.is_empty()
    }

    /// Clear all active overlays
    pub fn clear_active(&mut self) {
        self.active.clear();
    }

    /// Called when frame changes - clears cached data
    pub fn on_frame_change(&mut self) {
        self.qp.invalidate_texture();
        self.partition.clear_cache();
        self.bit_allocation.invalidate_texture();
        self.mv_magnitude.invalidate_texture();
        self.pu_type.invalidate_texture();
    }
}

impl Default for OverlayManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_has_no_active() {
        let mgr = OverlayManager::new();
        assert!(mgr.active.is_empty());
        assert!(!mgr.has_active());
        assert_eq!(mgr.active_count(), 0);
    }

    #[test]
    fn test_toggle_adds_overlay() {
        let mut mgr = OverlayManager::new();
        mgr.toggle(OverlayType::Grid);
        assert!(mgr.is_active(OverlayType::Grid));
        assert_eq!(mgr.active_count(), 1);
    }

    #[test]
    fn test_toggle_removes_overlay() {
        let mut mgr = OverlayManager::new();
        mgr.toggle(OverlayType::Grid);
        mgr.toggle(OverlayType::Grid);
        assert!(!mgr.is_active(OverlayType::Grid));
        assert_eq!(mgr.active_count(), 0);
    }

    #[test]
    fn test_set_active_true() {
        let mut mgr = OverlayManager::new();
        mgr.set_active(OverlayType::QpHeatmap, true);
        assert!(mgr.is_active(OverlayType::QpHeatmap));
    }

    #[test]
    fn test_set_active_false() {
        let mut mgr = OverlayManager::new();
        mgr.set_active(OverlayType::QpHeatmap, true);
        mgr.set_active(OverlayType::QpHeatmap, false);
        assert!(!mgr.is_active(OverlayType::QpHeatmap));
    }

    #[test]
    fn test_set_active_idempotent() {
        let mut mgr = OverlayManager::new();
        mgr.set_active(OverlayType::Grid, true);
        mgr.set_active(OverlayType::Grid, true);
        // Should only have one entry
        assert_eq!(mgr.active.iter().filter(|&&o| o == OverlayType::Grid).count(), 1);
    }

    #[test]
    fn test_multiple_overlays() {
        let mut mgr = OverlayManager::new();
        mgr.toggle(OverlayType::Grid);
        mgr.toggle(OverlayType::QpHeatmap);
        mgr.toggle(OverlayType::MotionVectors);

        assert!(mgr.is_active(OverlayType::Grid));
        assert!(mgr.is_active(OverlayType::QpHeatmap));
        assert!(mgr.is_active(OverlayType::MotionVectors));
        assert!(!mgr.is_active(OverlayType::Partition));
        assert_eq!(mgr.active_count(), 3);
    }

    #[test]
    fn test_clear_active() {
        let mut mgr = OverlayManager::new();
        mgr.toggle(OverlayType::Grid);
        mgr.toggle(OverlayType::QpHeatmap);
        mgr.clear_active();
        assert!(!mgr.has_active());
    }

    #[test]
    fn test_default() {
        let mgr: OverlayManager = Default::default();
        assert!(!mgr.has_active());
    }
}
