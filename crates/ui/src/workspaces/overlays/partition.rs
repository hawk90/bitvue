//! Partition Overlay State - Extracted from PlayerWorkspace
//!
//! Contains partition grid overlay configuration.
//! Per PARTITION_GRID_IMPLEMENTATION_SPEC.md

/// Partition overlay state
pub struct PartitionOverlayState {
    /// Display mode (Scaffold or Partition)
    pub mode: bitvue_core::GridMode,
    /// Cell tint opacity (0.0..0.25)
    pub opacity: f32,
    /// Cached partition data (loaded from JSON or generated)
    pub data: Option<bitvue_core::PartitionData>,
    /// Cached partition grid (hierarchical blocks)
    pub grid: Option<bitvue_core::PartitionGrid>,
    /// Selected partition cell (grid_x, grid_y)
    #[allow(dead_code)]
    pub selected_cell: Option<(u32, u32)>,
    /// Selected partition block index
    pub selected_block: Option<usize>,
}

impl PartitionOverlayState {
    /// Create new partition overlay state with defaults
    /// Per PARTITION_GRID_IMPLEMENTATION_SPEC.md
    pub fn new() -> Self {
        Self {
            mode: bitvue_core::GridMode::Scaffold,
            opacity: 0.15, // Default per spec
            data: None,
            grid: None,
            selected_cell: None,
            selected_block: None,
        }
    }

    /// Set display mode
    pub fn set_mode(&mut self, mode: bitvue_core::GridMode) {
        self.mode = mode;
    }

    /// Set opacity (clamped to 0.0..0.25)
    pub fn set_opacity(&mut self, opacity: f32) {
        self.opacity = opacity.clamp(0.0, 0.25);
    }

    /// Clear cached data (call when frame changes)
    pub fn clear_cache(&mut self) {
        self.data = None;
        self.grid = None;
        self.selected_cell = None;
        self.selected_block = None;
    }

    /// Select block at index
    pub fn select_block(&mut self, index: Option<usize>) {
        self.selected_block = index;
    }
}

impl Default for PartitionOverlayState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_values() {
        let state = PartitionOverlayState::new();
        assert!(matches!(state.mode, bitvue_core::GridMode::Scaffold));
        assert!((state.opacity - 0.15).abs() < f32::EPSILON);
        assert!(state.data.is_none());
        assert!(state.grid.is_none());
        assert!(state.selected_block.is_none());
    }

    #[test]
    fn test_set_opacity_clamps() {
        let mut state = PartitionOverlayState::new();
        state.set_opacity(0.5);
        assert!((state.opacity - 0.25).abs() < f32::EPSILON);

        state.set_opacity(-0.1);
        assert!((state.opacity - 0.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_clear_cache() {
        let mut state = PartitionOverlayState::new();
        state.selected_block = Some(5);
        state.clear_cache();
        assert!(state.selected_block.is_none());
    }
}
