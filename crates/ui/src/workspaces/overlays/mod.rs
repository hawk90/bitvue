//! Overlay State - Extracted from PlayerWorkspace
//!
//! Groups overlay-related state into separate structs for better organization.
//! Per god object refactoring (Batch 2).
//! VQAnalyzer parity: Extended with additional heatmap types.

mod bit_allocation;
mod grid;
mod manager;
mod mode_labels;
mod mv;
mod mv_magnitude;
mod partition;
mod pu_type;
mod qp;

pub use bit_allocation::{BitAllocationOverlayState, BitAllocationScale};
pub use grid::GridOverlayState;
pub use manager::OverlayManager;
pub use mode_labels::{BlockModeLabel, ModeLabelOverlayState};
pub use mv::MvOverlayState;
pub use mv_magnitude::{MvMagnitudeOverlayState, MvMagnitudeScale};
pub use partition::PartitionOverlayState;
pub use pu_type::{PuType, PuTypeOverlayState};
pub use qp::QpOverlayState;
