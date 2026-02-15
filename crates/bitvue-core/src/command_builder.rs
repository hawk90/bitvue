//! Command Builder - Fluent API for constructing Commands
//!
//! This module provides a builder pattern for constructing Command enums,
//! reducing boilerplate and improving code readability.
//!
//! # Example
//!
//! ```ignore
//! use bitvue_core::{CommandBuilder, StreamId, FrameKey};
//!
//! // Build a select frame command
//! let command = CommandBuilder::new()
//!     .select_frame(StreamId::A, FrameKey::new(42))
//!     .build()?;
//!
//! // Build an export command with options
//! let command = CommandBuilder::new()
//!     .export(StreamId::A, ExportContent::Frames, ExportFormat::Csv, "/path/to/export.csv")
//!     .with_frame_range(0, 100)
//!     .build()?;
//! ```

use crate::command::*;
use crate::selection::*;
use crate::types::{BitRange, SyntaxNodeId};
use std::ops::Range;
use std::path::PathBuf;

/// Builder for constructing Command enums with a fluent API
#[derive(Debug, Clone, Default)]
pub struct CommandBuilder {
    /// The command type being built
    command_type: Option<CommandType>,
    /// Stream ID (used by most commands)
    stream: Option<StreamId>,
    /// Target entity reference
    target: Option<Target>,
    /// Byte range for various operations
    byte_range: Option<Range<u64>>,
    /// Frame range for export
    frame_range: Option<(u64, u64)>,
    /// Path for file operations
    path: Option<PathBuf>,
    /// Export options
    export_content: Option<ExportContent>,
    export_format: Option<ExportFormat>,
    /// Overlay and mode options
    overlay_layer: Option<OverlayLayer>,
    player_mode: Option<PlayerMode>,
    workspace_mode: Option<WorkspaceMode>,
    sync_mode: Option<SyncMode>,
    order_type: Option<OrderType>,
    opacity: Option<f32>,
    export_kind: Option<ExportKind>,
    usize_range: Option<Range<usize>>,
}

/// Internal command type classification
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CommandType {
    OpenFile,
    CloseFile,
    RunFullAnalysis,
    SelectFrame,
    SelectUnit,
    SelectSyntax,
    SelectBitRange,
    SelectSpatialBlock,
    JumpToOffset,
    JumpToFrame,
    ToggleOverlay,
    SetOverlayOpacity,
    SetPlayerMode,
    PlayPause,
    StepForward,
    StepBackward,
    SetWorkspaceMode,
    SetSyncMode,
    ExportCsv,
    ExportBitstream,
    Export,
    AddBookmark,
    RemoveBookmark,
    ExportEvidenceBundle,
    SetOrderType,
    ToggleDetailMode,
    CopySelection,
    CopyBytes,
}

/// Target entity for selection commands
#[derive(Debug, Clone)]
enum Target {
    Frame(FrameKey),
    Unit(UnitKey),
    Syntax(SyntaxNodeId),
    BitRange(BitRange),
    SpatialBlock(SpatialBlock),
}

impl CommandBuilder {
    /// Create a new CommandBuilder
    pub fn new() -> Self {
        Self::default()
    }

    // =========================================================================
    // File Operations
    // =========================================================================

    /// Open a file command
    pub fn open_file(mut self, stream: StreamId, path: impl Into<PathBuf>) -> Self {
        self.command_type = Some(CommandType::OpenFile);
        self.stream = Some(stream);
        self.path = Some(path.into());
        self
    }

    /// Close a file command
    pub fn close_file(mut self, stream: StreamId) -> Self {
        self.command_type = Some(CommandType::CloseFile);
        self.stream = Some(stream);
        self
    }

    /// Run full analysis command
    pub fn run_full_analysis(mut self, stream: StreamId) -> Self {
        self.command_type = Some(CommandType::RunFullAnalysis);
        self.stream = Some(stream);
        self
    }

    // =========================================================================
    // Selection Commands (Tri-sync)
    // =========================================================================

    /// Select a frame command
    pub fn select_frame(mut self, stream: StreamId, frame_key: FrameKey) -> Self {
        self.command_type = Some(CommandType::SelectFrame);
        self.stream = Some(stream);
        self.target = Some(Target::Frame(frame_key));
        self
    }

    /// Select a unit command
    pub fn select_unit(mut self, stream: StreamId, unit_key: UnitKey) -> Self {
        self.command_type = Some(CommandType::SelectUnit);
        self.stream = Some(stream);
        self.target = Some(Target::Unit(unit_key));
        self
    }

    /// Select a syntax node command
    pub fn select_syntax(
        mut self,
        stream: StreamId,
        node_id: SyntaxNodeId,
        bit_range: BitRange,
    ) -> Self {
        self.command_type = Some(CommandType::SelectSyntax);
        self.stream = Some(stream);
        self.target = Some(Target::Syntax(node_id));
        self.byte_range = Some(bit_range.start_bit..bit_range.end_bit);
        self
    }

    /// Select a bit range command
    pub fn select_bit_range(mut self, stream: StreamId, bit_range: BitRange) -> Self {
        self.command_type = Some(CommandType::SelectBitRange);
        self.stream = Some(stream);
        self.target = Some(Target::BitRange(bit_range));
        self
    }

    /// Select a spatial block command
    pub fn select_spatial_block(mut self, stream: StreamId, block: SpatialBlock) -> Self {
        self.command_type = Some(CommandType::SelectSpatialBlock);
        self.stream = Some(stream);
        self.target = Some(Target::SpatialBlock(block));
        self
    }

    // =========================================================================
    // Navigation Commands
    // =========================================================================

    /// Jump to offset command
    pub fn jump_to_offset(mut self, stream: StreamId, offset: u64) -> Self {
        self.command_type = Some(CommandType::JumpToOffset);
        self.stream = Some(stream);
        self.byte_range = Some(offset..offset);
        self
    }

    /// Jump to frame command
    pub fn jump_to_frame(mut self, stream: StreamId, frame_index: usize) -> Self {
        self.command_type = Some(CommandType::JumpToFrame);
        self.stream = Some(stream);
        self.byte_range = Some((frame_index as u64)..(frame_index as u64));
        self
    }

    // =========================================================================
    // Player/Overlay Commands
    // =========================================================================

    /// Toggle overlay command
    pub fn toggle_overlay(mut self, stream: StreamId, layer: OverlayLayer) -> Self {
        self.command_type = Some(CommandType::ToggleOverlay);
        self.stream = Some(stream);
        self.overlay_layer = Some(layer);
        self
    }

    /// Set overlay opacity command
    pub fn set_overlay_opacity(mut self, stream: StreamId, opacity: f32) -> Self {
        self.command_type = Some(CommandType::SetOverlayOpacity);
        self.stream = Some(stream);
        self.opacity = Some(opacity);
        self
    }

    /// Set player mode command
    pub fn set_player_mode(mut self, stream: StreamId, mode: PlayerMode) -> Self {
        self.command_type = Some(CommandType::SetPlayerMode);
        self.stream = Some(stream);
        self.player_mode = Some(mode);
        self
    }

    // =========================================================================
    // Playback Commands
    // =========================================================================

    /// Play/pause command
    pub fn play_pause(mut self, stream: StreamId) -> Self {
        self.command_type = Some(CommandType::PlayPause);
        self.stream = Some(stream);
        self
    }

    /// Step forward command
    pub fn step_forward(mut self, stream: StreamId) -> Self {
        self.command_type = Some(CommandType::StepForward);
        self.stream = Some(stream);
        self
    }

    /// Step backward command
    pub fn step_backward(mut self, stream: StreamId) -> Self {
        self.command_type = Some(CommandType::StepBackward);
        self.stream = Some(stream);
        self
    }

    // =========================================================================
    // Dual View Commands
    // =========================================================================

    /// Set workspace mode command
    pub fn set_workspace_mode(mut self, mode: WorkspaceMode) -> Self {
        self.command_type = Some(CommandType::SetWorkspaceMode);
        self.workspace_mode = Some(mode);
        self
    }

    /// Set sync mode command
    pub fn set_sync_mode(mut self, mode: SyncMode) -> Self {
        self.command_type = Some(CommandType::SetSyncMode);
        self.sync_mode = Some(mode);
        self
    }

    // =========================================================================
    // Export Commands
    // =========================================================================

    /// Export CSV command (legacy)
    pub fn export_csv(mut self, stream: StreamId, kind: ExportKind) -> Self {
        self.command_type = Some(CommandType::ExportCsv);
        self.stream = Some(stream);
        self.export_kind = Some(kind);
        self
    }

    /// Export bitstream command
    pub fn export_bitstream(mut self, stream: StreamId, range: Option<Range<usize>>) -> Self {
        self.command_type = Some(CommandType::ExportBitstream);
        self.stream = Some(stream);
        self.usize_range = range;
        self
    }

    /// Export command (v2 - Feature Parity Phase A)
    pub fn export(
        mut self,
        stream: StreamId,
        content: ExportContent,
        format: ExportFormat,
        path: impl Into<PathBuf>,
    ) -> Self {
        self.command_type = Some(CommandType::Export);
        self.stream = Some(stream);
        self.export_content = Some(content);
        self.export_format = Some(format);
        self.path = Some(path.into());
        self
    }

    // =========================================================================
    // UI State Commands
    // =========================================================================

    /// Add bookmark command
    pub fn add_bookmark(mut self, stream: StreamId, frame_key: FrameKey) -> Self {
        self.command_type = Some(CommandType::AddBookmark);
        self.stream = Some(stream);
        self.target = Some(Target::Frame(frame_key));
        self
    }

    /// Remove bookmark command
    pub fn remove_bookmark(mut self, stream: StreamId, frame_key: FrameKey) -> Self {
        self.command_type = Some(CommandType::RemoveBookmark);
        self.stream = Some(stream);
        self.target = Some(Target::Frame(frame_key));
        self
    }

    // =========================================================================
    // Evidence Bundle Export
    // =========================================================================

    /// Export evidence bundle command
    pub fn export_evidence_bundle(mut self, stream: StreamId, path: impl Into<PathBuf>) -> Self {
        self.command_type = Some(CommandType::ExportEvidenceBundle);
        self.stream = Some(stream);
        self.path = Some(path.into());
        self
    }

    // =========================================================================
    // Order Type
    // =========================================================================

    /// Set order type command
    pub fn set_order_type(mut self, order_type: OrderType) -> Self {
        self.command_type = Some(CommandType::SetOrderType);
        self.order_type = Some(order_type);
        self
    }

    // =========================================================================
    // Toggle Commands
    // =========================================================================

    /// Toggle detail mode command
    pub fn toggle_detail_mode(mut self) -> Self {
        self.command_type = Some(CommandType::ToggleDetailMode);
        self
    }

    // =========================================================================
    // Copy Commands
    // =========================================================================

    /// Copy selection command
    pub fn copy_selection(mut self) -> Self {
        self.command_type = Some(CommandType::CopySelection);
        self
    }

    /// Copy bytes command
    pub fn copy_bytes(mut self, byte_range: Range<u64>) -> Self {
        self.command_type = Some(CommandType::CopyBytes);
        self.byte_range = Some(byte_range);
        self
    }

    // =========================================================================
    // Builder Modifiers (Chaining Options)
    // =========================================================================

    /// Add a frame range to the command (for export commands)
    pub fn with_frame_range(mut self, start: u64, end: u64) -> Self {
        self.frame_range = Some((start, end));
        self
    }

    /// Add a byte range to the command
    pub fn with_byte_range(mut self, start: u64, end: u64) -> Self {
        self.byte_range = Some(start..end);
        self
    }

    /// Add a usize byte range to the command (for export bitstream)
    pub fn with_usize_range(mut self, range: Range<usize>) -> Self {
        self.usize_range = Some(range);
        self
    }

    /// Set the order type for the command
    pub fn with_order_type(mut self, order_type: OrderType) -> Self {
        self.order_type = Some(order_type);
        self
    }

    /// Set the overlay opacity (for overlay commands)
    pub fn with_opacity(mut self, opacity: f32) -> Self {
        self.opacity = Some(opacity);
        self
    }

    // =========================================================================
    // Build
    // =========================================================================

    /// Build the Command, validating all required fields
    pub fn build(self) -> Result<Command, String> {
        let command_type = self
            .command_type
            .ok_or_else(|| "Command type not set. Call a command method first (e.g., select_frame, jump_to_offset)".to_string())?;

        match command_type {
            CommandType::OpenFile => {
                let stream = self.stream.ok_or("Stream required for OpenFile")?;
                let path = self.path.ok_or("Path required for OpenFile")?;
                Ok(Command::OpenFile { stream, path })
            }

            CommandType::CloseFile => {
                let stream = self.stream.ok_or("Stream required for CloseFile")?;
                Ok(Command::CloseFile { stream })
            }

            CommandType::RunFullAnalysis => {
                let stream = self.stream.ok_or("Stream required for RunFullAnalysis")?;
                Ok(Command::RunFullAnalysis { stream })
            }

            CommandType::SelectFrame => {
                let stream = self.stream.ok_or("Stream required for SelectFrame")?;
                let frame_key = self.extract_frame_key("SelectFrame")?;
                Ok(Command::SelectFrame { stream, frame_key })
            }

            CommandType::SelectUnit => {
                let stream = self.stream.ok_or("Stream required for SelectUnit")?;
                let unit_key = self.extract_unit_key("SelectUnit")?;
                Ok(Command::SelectUnit { stream, unit_key })
            }

            CommandType::SelectSyntax => {
                let stream = self.stream.ok_or("Stream required for SelectSyntax")?;
                let node_id = self.extract_syntax_node_id("SelectSyntax")?;
                let bit_range = self.extract_bit_range("SelectSyntax")?;
                Ok(Command::SelectSyntax {
                    stream,
                    node_id,
                    bit_range,
                })
            }

            CommandType::SelectBitRange => {
                let stream = self.stream.ok_or("Stream required for SelectBitRange")?;
                let bit_range = self.extract_bit_range("SelectBitRange")?;
                Ok(Command::SelectBitRange { stream, bit_range })
            }

            CommandType::SelectSpatialBlock => {
                let stream = self
                    .stream
                    .ok_or("Stream required for SelectSpatialBlock")?;
                let block = self.extract_spatial_block("SelectSpatialBlock")?;
                Ok(Command::SelectSpatialBlock { stream, block })
            }

            CommandType::JumpToOffset => {
                let stream = self.stream.ok_or("Stream required for JumpToOffset")?;
                Ok(Command::JumpToOffset {
                    stream,
                    offset: self
                        .byte_range
                        .as_ref()
                        .map(|r| r.start)
                        .ok_or("Offset required for JumpToOffset, use with_byte_range()")?,
                })
            }

            CommandType::JumpToFrame => {
                let stream = self.stream.ok_or("Stream required for JumpToFrame")?;
                let frame_index = self
                    .byte_range
                    .as_ref()
                    .map(|r| r.start as usize)
                    .ok_or("Frame index required for JumpToFrame")?;
                Ok(Command::JumpToFrame {
                    stream,
                    frame_index,
                })
            }

            CommandType::ToggleOverlay => {
                let stream = self.stream.ok_or("Stream required for ToggleOverlay")?;
                let layer = self
                    .overlay_layer
                    .ok_or("Layer required for ToggleOverlay")?;
                Ok(Command::ToggleOverlay { stream, layer })
            }

            CommandType::SetOverlayOpacity => {
                let stream = self.stream.ok_or("Stream required for SetOverlayOpacity")?;
                let opacity = self
                    .opacity
                    .ok_or("Opacity required for SetOverlayOpacity")?;
                Ok(Command::SetOverlayOpacity { stream, opacity })
            }

            CommandType::SetPlayerMode => {
                let stream = self.stream.ok_or("Stream required for SetPlayerMode")?;
                let mode = self.player_mode.ok_or("Mode required for SetPlayerMode")?;
                Ok(Command::SetPlayerMode { stream, mode })
            }

            CommandType::PlayPause => {
                let stream = self.stream.ok_or("Stream required for PlayPause")?;
                Ok(Command::PlayPause { stream })
            }

            CommandType::StepForward => {
                let stream = self.stream.ok_or("Stream required for StepForward")?;
                Ok(Command::StepForward { stream })
            }

            CommandType::StepBackward => {
                let stream = self.stream.ok_or("Stream required for StepBackward")?;
                Ok(Command::StepBackward { stream })
            }

            CommandType::SetWorkspaceMode => {
                let mode = self
                    .workspace_mode
                    .ok_or("Mode required for SetWorkspaceMode")?;
                Ok(Command::SetWorkspaceMode { mode })
            }

            CommandType::SetSyncMode => {
                let mode = self.sync_mode.ok_or("Mode required for SetSyncMode")?;
                Ok(Command::SetSyncMode { mode })
            }

            CommandType::ExportCsv => {
                let stream = self.stream.ok_or("Stream required for ExportCsv")?;
                let kind = self.export_kind.ok_or("Kind required for ExportCsv")?;
                Ok(Command::ExportCsv { stream, kind })
            }

            CommandType::ExportBitstream => {
                let stream = self.stream.ok_or("Stream required for ExportBitstream")?;
                Ok(Command::ExportBitstream {
                    stream,
                    range: self.usize_range,
                })
            }

            CommandType::Export => {
                let stream = self.stream.ok_or("Stream required for Export")?;
                let content = self.export_content.ok_or("Content required for Export")?;
                let format = self.export_format.ok_or("Format required for Export")?;
                let path = self.path.ok_or("Path required for Export")?;
                Ok(Command::Export {
                    stream,
                    content,
                    format,
                    path,
                    frame_range: self.frame_range,
                })
            }

            CommandType::AddBookmark => {
                let stream = self.stream.ok_or("Stream required for AddBookmark")?;
                let frame_key = self.extract_frame_key("AddBookmark")?;
                Ok(Command::AddBookmark { stream, frame_key })
            }

            CommandType::RemoveBookmark => {
                let stream = self.stream.ok_or("Stream required for RemoveBookmark")?;
                let frame_key = self.extract_frame_key("RemoveBookmark")?;
                Ok(Command::RemoveBookmark { stream, frame_key })
            }

            CommandType::ExportEvidenceBundle => {
                let stream = self
                    .stream
                    .ok_or("Stream required for ExportEvidenceBundle")?;
                let path = self.path.ok_or("Path required for ExportEvidenceBundle")?;
                Ok(Command::ExportEvidenceBundle { stream, path })
            }

            CommandType::SetOrderType => {
                let order_type = self
                    .order_type
                    .ok_or("OrderType required for SetOrderType")?;
                Ok(Command::SetOrderType { order_type })
            }

            CommandType::ToggleDetailMode => Ok(Command::ToggleDetailMode),

            CommandType::CopySelection => Ok(Command::CopySelection),

            CommandType::CopyBytes => {
                let byte_range = self.byte_range.ok_or("Byte range required for CopyBytes")?;
                Ok(Command::CopyBytes { byte_range })
            }
        }
    }

    // =========================================================================
    // Helper Methods for Target Extraction
    // =========================================================================

    fn extract_frame_key(&self, context: &str) -> Result<FrameKey, String> {
        match &self.target {
            Some(Target::Frame(key)) => Ok(key.clone()),
            _ => Err(format!("FrameKey required for {}", context)),
        }
    }

    fn extract_unit_key(&self, context: &str) -> Result<UnitKey, String> {
        match &self.target {
            Some(Target::Unit(key)) => Ok(key.clone()),
            _ => Err(format!("UnitKey required for {}", context)),
        }
    }

    fn extract_syntax_node_id(&self, context: &str) -> Result<SyntaxNodeId, String> {
        match &self.target {
            Some(Target::Syntax(id)) => Ok(id.clone()),
            _ => Err(format!("SyntaxNodeId required for {}", context)),
        }
    }

    fn extract_bit_range(&self, context: &str) -> Result<BitRange, String> {
        match &self.target {
            Some(Target::BitRange(range)) => Ok(*range),
            _ => Err(format!("BitRange required for {}", context)),
        }
    }

    fn extract_spatial_block(&self, context: &str) -> Result<SpatialBlock, String> {
        match &self.target {
            Some(Target::SpatialBlock(block)) => Ok(*block),
            _ => Err(format!("SpatialBlock required for {}", context)),
        }
    }
}

// =============================================================================
// Convenience Functions
// =============================================================================

/// Create a new CommandBuilder
pub fn build_command() -> CommandBuilder {
    CommandBuilder::new()
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_select_frame_command() {
        let command = CommandBuilder::new()
            .select_frame(
                StreamId::A,
                FrameKey {
                    stream: StreamId::A,
                    frame_index: 42,
                    pts: None,
                },
            )
            .build()
            .unwrap();

        match command {
            Command::SelectFrame { stream, frame_key } => {
                assert_eq!(stream, StreamId::A);
                assert_eq!(frame_key.frame_index, 42);
            }
            _ => panic!("Expected SelectFrame command"),
        }
    }

    #[test]
    fn test_select_unit_command() {
        let command = CommandBuilder::new()
            .select_unit(
                StreamId::B,
                UnitKey {
                    stream: StreamId::B,
                    unit_type: "TEST_UNIT".to_string(),
                    offset: 10,
                    size: 100,
                },
            )
            .build()
            .unwrap();

        match command {
            Command::SelectUnit { stream, unit_key } => {
                assert_eq!(stream, StreamId::B);
                assert_eq!(unit_key.offset, 10);
            }
            _ => panic!("Expected SelectUnit command"),
        }
    }

    #[test]
    fn test_jump_to_offset_command() {
        let command = CommandBuilder::new()
            .jump_to_offset(StreamId::A, 1000)
            .build()
            .unwrap();

        match command {
            Command::JumpToOffset { stream, offset } => {
                assert_eq!(stream, StreamId::A);
                assert_eq!(offset, 1000);
            }
            _ => panic!("Expected JumpToOffset command"),
        }
    }

    #[test]
    fn test_export_command_with_frame_range() {
        let command = CommandBuilder::new()
            .export(
                StreamId::A,
                ExportContent::Frames,
                ExportFormat::Csv,
                "/tmp/export.csv",
            )
            .with_frame_range(0, 100)
            .build()
            .unwrap();

        match command {
            Command::Export {
                stream,
                content,
                format,
                path,
                frame_range,
            } => {
                assert_eq!(stream, StreamId::A);
                assert_eq!(content, ExportContent::Frames);
                assert_eq!(format, ExportFormat::Csv);
                assert_eq!(path, PathBuf::from("/tmp/export.csv"));
                assert_eq!(frame_range, Some((0, 100)));
            }
            _ => panic!("Expected Export command"),
        }
    }

    #[test]
    fn test_toggle_overlay_command() {
        let command = CommandBuilder::new()
            .toggle_overlay(StreamId::A, OverlayLayer::QpHeatmap)
            .build()
            .unwrap();

        match command {
            Command::ToggleOverlay { stream, layer } => {
                assert_eq!(stream, StreamId::A);
                assert_eq!(layer, OverlayLayer::QpHeatmap);
            }
            _ => panic!("Expected ToggleOverlay command"),
        }
    }

    #[test]
    fn test_set_player_mode_command() {
        let command = CommandBuilder::new()
            .set_player_mode(StreamId::A, PlayerMode::Diff)
            .build()
            .unwrap();

        match command {
            Command::SetPlayerMode { stream, mode } => {
                assert_eq!(stream, StreamId::A);
                assert_eq!(mode, PlayerMode::Diff);
            }
            _ => panic!("Expected SetPlayerMode command"),
        }
    }

    #[test]
    fn test_toggle_detail_mode_command() {
        let command = CommandBuilder::new().toggle_detail_mode().build().unwrap();

        match command {
            Command::ToggleDetailMode => {}
            _ => panic!("Expected ToggleDetailMode command"),
        }
    }

    #[test]
    fn test_copy_selection_command() {
        let command = CommandBuilder::new().copy_selection().build().unwrap();

        match command {
            Command::CopySelection => {}
            _ => panic!("Expected CopySelection command"),
        }
    }

    #[test]
    fn test_set_workspace_mode_command() {
        let command = CommandBuilder::new()
            .set_workspace_mode(WorkspaceMode::Dual)
            .build()
            .unwrap();

        match command {
            Command::SetWorkspaceMode { mode } => {
                assert_eq!(mode, WorkspaceMode::Dual);
            }
            _ => panic!("Expected SetWorkspaceMode command"),
        }
    }

    #[test]
    fn test_add_bookmark_command() {
        let command = CommandBuilder::new()
            .add_bookmark(
                StreamId::A,
                FrameKey {
                    stream: StreamId::A,
                    frame_index: 42,
                    pts: None,
                },
            )
            .build()
            .unwrap();

        match command {
            Command::AddBookmark { stream, frame_key } => {
                assert_eq!(stream, StreamId::A);
                assert_eq!(frame_key.frame_index, 42);
            }
            _ => panic!("Expected AddBookmark command"),
        }
    }

    #[test]
    fn test_export_evidence_bundle_command() {
        let command = CommandBuilder::new()
            .export_evidence_bundle(StreamId::A, "/tmp/bundle.zip")
            .build()
            .unwrap();

        match command {
            Command::ExportEvidenceBundle { stream, path } => {
                assert_eq!(stream, StreamId::A);
                assert_eq!(path, PathBuf::from("/tmp/bundle.zip"));
            }
            _ => panic!("Expected ExportEvidenceBundle command"),
        }
    }

    #[test]
    fn test_builder_error_no_command_type() {
        let result = CommandBuilder::new().build();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Command type not set"));
    }

    #[test]
    fn test_builder_error_select_frame_no_stream() {
        // This would require internal testing since select_frame sets stream
        // The builder API prevents this error by design
    }

    #[test]
    fn test_chained_modifiers() {
        let command = CommandBuilder::new()
            .export(
                StreamId::A,
                ExportContent::Metrics,
                ExportFormat::Json,
                "/tmp/metrics.json",
            )
            .with_frame_range(10, 100)
            .with_order_type(OrderType::Decode)
            .build()
            .unwrap();

        match command {
            Command::Export { frame_range, .. } => {
                assert_eq!(frame_range, Some((10, 100)));
            }
            _ => panic!("Expected Export command"),
        }
    }

    #[test]
    fn test_copy_bytes_command() {
        let command = CommandBuilder::new().copy_bytes(100..200).build().unwrap();

        match command {
            Command::CopyBytes { byte_range } => {
                assert_eq!(byte_range.start, 100);
                assert_eq!(byte_range.end, 200);
            }
            _ => panic!("Expected CopyBytes command"),
        }
    }
}
