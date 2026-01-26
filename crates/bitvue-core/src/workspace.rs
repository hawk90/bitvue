//! WorkspaceState - Single/Dual stream workspace management
//!
//! Monster Pack v9: ARCHITECTURE.md §6

use crate::{SelectionState, SyncMode};
use serde::{Deserialize, Serialize};

/// Workspace mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum WorkspaceMode {
    /// Single stream view (Stream A only)
    #[default]
    Single,
    /// Dual stream view (Stream A + B side-by-side)
    Dual,
}

/// Workspace state (manages Single/Dual mode)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceState {
    /// Current workspace mode
    pub mode: WorkspaceMode,

    /// Sync mode (only applies when mode = Dual)
    pub sync_mode: SyncMode,

    /// Active stream (which stream has focus)
    pub active_stream: crate::StreamId,
}

impl Default for WorkspaceState {
    fn default() -> Self {
        Self {
            mode: WorkspaceMode::Single,
            sync_mode: SyncMode::Off,
            active_stream: crate::StreamId::A,
        }
    }
}

impl WorkspaceState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Switch workspace mode
    pub fn set_mode(&mut self, mode: WorkspaceMode) {
        self.mode = mode;

        // Reset sync mode when switching to Single
        if matches!(mode, WorkspaceMode::Single) {
            self.sync_mode = SyncMode::Off;
        }
    }

    /// Set sync mode (only applies in Dual mode)
    pub fn set_sync_mode(&mut self, sync_mode: SyncMode) {
        if matches!(self.mode, WorkspaceMode::Dual) {
            self.sync_mode = sync_mode;
        }
    }

    /// Set active stream
    pub fn set_active_stream(&mut self, stream: crate::StreamId) {
        self.active_stream = stream;
    }

    /// Check if dual mode is active
    pub fn is_dual(&self) -> bool {
        matches!(self.mode, WorkspaceMode::Dual)
    }

    /// Check if sync is enabled
    pub fn is_synced(&self) -> bool {
        self.is_dual() && !matches!(self.sync_mode, SyncMode::Off)
    }
}

/// SyncController - handles selection synchronization in Dual mode
pub struct SyncController;

impl SyncController {
    /// Sync selection from source stream to target stream
    ///
    /// Based on sync_mode:
    /// - Off: No sync
    /// - Playhead: Sync temporal (frame position) only
    /// - Full: Sync temporal + unit + syntax_node + bit_range
    pub fn sync_selection(
        sync_mode: SyncMode,
        source: &SelectionState,
        target: &mut SelectionState,
    ) {
        match sync_mode {
            SyncMode::Off => {
                // No synchronization
            }
            SyncMode::Playhead => {
                // Sync frame position only (temporal selection)
                if let Some(ref temporal) = source.temporal {
                    match temporal {
                        crate::TemporalSelection::Point { frame_index } => {
                            target.select_point(*frame_index);
                        }
                        crate::TemporalSelection::Block { frame_index, block } => {
                            target.select_block(*frame_index, *block);
                        }
                        crate::TemporalSelection::Range { start, end } => {
                            target.select_range(*start, *end);
                        }
                        crate::TemporalSelection::Marker { frame_index } => {
                            target.select_marker(*frame_index);
                        }
                    }
                }
            }
            SyncMode::Full => {
                // Full sync: temporal + unit + syntax + bitRange
                // Sync temporal selection
                if let Some(ref temporal) = source.temporal {
                    match temporal {
                        crate::TemporalSelection::Point { frame_index } => {
                            target.select_point(*frame_index);
                        }
                        crate::TemporalSelection::Block { frame_index, block } => {
                            target.select_block(*frame_index, *block);
                        }
                        crate::TemporalSelection::Range { start, end } => {
                            target.select_range(*start, *end);
                        }
                        crate::TemporalSelection::Marker { frame_index } => {
                            target.select_marker(*frame_index);
                        }
                    }
                }

                // Sync unit selection
                if let Some(ref unit) = source.unit {
                    let mut target_unit = unit.clone();
                    target_unit.stream = target.stream_id;
                    target.unit = Some(target_unit);
                }

                // SyntaxNodeId is now a String (stream-agnostic), can be copied directly
                target.syntax_node = source.syntax_node.clone();

                // BitRange is stream-agnostic, can be copied directly
                target.bit_range = source.bit_range;
            }
        }
    }
}

/// Comprehensive test suite with Arrange-Act-Assert pattern
#[cfg(test)]
mod tests {
    include!("workspace_test.rs");
}
