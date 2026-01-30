//! Overlay drawing functions for PlayerWorkspace
//!
//! This module contains all overlay drawing methods, split into focused submodules:
//! - grid: Grid overlay with CTB labels and headers
//! - heatmap: QP heatmap, bit allocation heatmap, MV magnitude heatmap
//! - partition: Partition grid visualization with scaffold/partition modes
//! - motion: Motion vector overlay with L0/L1 layers
//! - labels: Mode labels and PU type overlays

mod grid;
mod heatmap;
mod labels;
mod motion;
mod partition;

pub use labels::find_unit_by_offset;

/// Overlay rendering context - holds shared references needed by all overlay functions
#[allow(dead_code)]
pub struct OverlayRenderContext<'a> {
    pub overlays: &'a crate::workspaces::overlays::OverlayManager,
    pub frame_size: Option<(u32, u32)>,
}
