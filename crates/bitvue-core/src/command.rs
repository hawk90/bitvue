//! Command - UI → Core commands
//!
//! Monster Pack v3: ARCHITECTURE.md §3.2

use crate::selection::*;
use crate::types::{BitRange, SyntaxNodeId};
use std::ops::Range;
use std::path::PathBuf;

/// Commands emitted by UI panels
#[derive(Debug, Clone)]
pub enum Command {
    // File operations
    OpenFile {
        stream: StreamId,
        path: PathBuf,
    },
    CloseFile {
        stream: StreamId,
    },
    RunFullAnalysis {
        stream: StreamId,
    },

    // Selection commands (Tri-sync)
    SelectFrame {
        stream: StreamId,
        frame_key: FrameKey,
    },
    SelectUnit {
        stream: StreamId,
        unit_key: UnitKey,
    },
    SelectSyntax {
        stream: StreamId,
        node_id: SyntaxNodeId,
        bit_range: BitRange,
    },
    SelectBitRange {
        stream: StreamId,
        bit_range: BitRange,
    },
    SelectSpatialBlock {
        stream: StreamId,
        block: SpatialBlock,
    },

    // Navigation
    JumpToOffset {
        stream: StreamId,
        offset: u64,
    },
    JumpToFrame {
        stream: StreamId,
        frame_index: usize,
    },

    // Player/Overlay
    ToggleOverlay {
        stream: StreamId,
        layer: OverlayLayer,
    },
    SetOverlayOpacity {
        stream: StreamId,
        opacity: f32,
    },
    SetPlayerMode {
        stream: StreamId,
        mode: PlayerMode,
    },

    // Playback
    PlayPause {
        stream: StreamId,
    },
    StepForward {
        stream: StreamId,
    },
    StepBackward {
        stream: StreamId,
    },

    // Dual View
    SetWorkspaceMode {
        mode: WorkspaceMode,
    },
    SetSyncMode {
        mode: SyncMode,
    },

    // Export (legacy)
    ExportCsv {
        stream: StreamId,
        kind: ExportKind,
    },
    ExportBitstream {
        stream: StreamId,
        range: Option<Range<usize>>,
    },

    // Export v2: Feature Parity Phase A
    Export {
        stream: StreamId,
        content: ExportContent,
        format: ExportFormat,
        path: PathBuf,
        /// Frame range (start, end) - None for all frames
        frame_range: Option<(u64, u64)>,
    },

    // UI state
    AddBookmark {
        stream: StreamId,
        frame_key: FrameKey,
    },
    RemoveBookmark {
        stream: StreamId,
        frame_key: FrameKey,
    },

    // Evidence Bundle Export (per export_entrypoints.json)
    // Reachable from: MainMenu, BottomBar, ContextMenu, CompareWorkspace
    ExportEvidenceBundle {
        stream: StreamId,
        path: PathBuf,
    },

    // Order Type (per guard_rules.json)
    SetOrderType {
        order_type: OrderType,
    },

    // Toggle Detail Mode (per context_menus.json)
    ToggleDetailMode,

    // Copy operations (per context_menus.json)
    CopySelection,
    CopyBytes {
        byte_range: Range<u64>,
    },
}

/// Order type for explicit tracking (display vs decode)
/// Per compare_alignment_contracts.json: "Never infer from decode order" / "Never infer from display order"
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum OrderType {
    Display,
    Decode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OverlayLayer {
    Grid,
    Transform,
    MvL0,
    MvL1,
    QpHeatmap,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerMode {
    Decoded,
    Residual,
    Diff,
    Predicted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkspaceMode {
    Single,
    Dual,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum SyncMode {
    Off,
    Playhead,
    Full,
}

/// Export content type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportContent {
    /// Timeline frames with frame_type, size, pts, dts
    Frames,
    /// Metrics (PSNR, SSIM, VMAF)
    Metrics,
    /// Diagnostic records
    Diagnostics,
    /// Summary statistics
    Summary,
    /// Raw bitstream extract
    Bitstream,
}

/// Export format
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportFormat {
    Csv,
    Json,
    JsonPretty,
}

/// Legacy export kind (kept for compatibility)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportKind {
    Csv,
    Json,
    Bitstream,
}

/// Comprehensive test suite with Arrange-Act-Assert pattern
#[cfg(test)]
mod tests {
    include!("command_test.rs");
}
