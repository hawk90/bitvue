// Command module tests
//
// Per generate-tests skill: Arrange-Act-Assert pattern with fixtures
// and edge case coverage.

use super::*;
use crate::{
    BitRange, FrameKey, PlayerMode, SpatialBlock, StreamId, SyncMode, UnitKey,
};
use std::ops::Range;
use std::path::PathBuf;

// Use command::OverlayLayer explicitly to avoid conflict with types::OverlayLayer
use crate::command::OverlayLayer;

// ============================================================================
// Fixtures
// ============================================================================

/// Create a test path
fn create_test_path() -> PathBuf {
    PathBuf::from("/tmp/test.ivf")
}

/// Create a test stream ID A
fn test_stream_a() -> StreamId {
    StreamId::A
}

/// Create a test stream ID B
fn test_stream_b() -> StreamId {
    StreamId::B
}

/// Create a test frame key
fn create_test_frame_key() -> FrameKey {
    FrameKey {
        stream: StreamId::A,
        frame_index: 10,
        pts: Some(100),
    }
}

/// Create a test unit key
fn create_test_unit_key() -> UnitKey {
    UnitKey {
        stream: StreamId::A,
        unit_type: "OBU_FRAME".to_string(),
        offset: 1000,
        size: 500,
    }
}

/// Create a test bit range
fn create_test_bit_range() -> BitRange {
    BitRange::new(1000, 1100)
}

/// Create a test spatial block
fn create_test_spatial_block() -> SpatialBlock {
    SpatialBlock {
        x: 10,
        y: 20,
        w: 32,
        h: 32,
    }
}

// ============================================================================
// Command::OpenFile Tests
// ============================================================================

#[cfg(test)]
mod command_open_file_tests {
    use super::*;

    #[test]
    fn test_command_open_file_construct() {
        // Arrange
        let stream = test_stream_a();
        let path = create_test_path();

        // Act
        let cmd = Command::OpenFile {
            stream,
            path: path.clone(),
        };

        // Assert
        assert!(matches!(cmd, Command::OpenFile { .. }));
        match cmd {
            Command::OpenFile { stream: s, path: p } => {
                assert_eq!(s, StreamId::A);
                assert_eq!(p, path);
            }
            _ => panic!("Expected OpenFile command"),
        }
    }

    #[test]
    fn test_command_open_file_clone() {
        // Arrange
        let cmd = Command::OpenFile {
            stream: test_stream_a(),
            path: create_test_path(),
        };

        // Act
        let cloned = cmd.clone();

        // Assert
        assert!(matches!(cloned, Command::OpenFile { .. }));
    }
}

// ============================================================================
// Command::CloseFile Tests
// ============================================================================

#[cfg(test)]
mod command_close_file_tests {
    use super::*;

    #[test]
    fn test_command_close_file_construct() {
        // Arrange
        let stream = test_stream_b();

        // Act
        let cmd = Command::CloseFile { stream };

        // Assert
        assert!(matches!(cmd, Command::CloseFile { .. }));
        match cmd {
            Command::CloseFile { stream: s } => {
                assert_eq!(s, StreamId::B);
            }
            _ => panic!("Expected CloseFile command"),
        }
    }
}

// ============================================================================
// Command::RunFullAnalysis Tests
// ============================================================================

#[cfg(test)]
mod command_run_full_analysis_tests {
    use super::*;

    #[test]
    fn test_command_run_full_analysis_construct() {
        // Arrange & Act
        let cmd = Command::RunFullAnalysis {
            stream: test_stream_a(),
        };

        // Assert
        assert!(matches!(cmd, Command::RunFullAnalysis { .. }));
    }
}

// ============================================================================
// Selection Commands Tests
// ============================================================================

#[cfg(test)]
mod command_selection_tests {
    use super::*;

    #[test]
    fn test_command_select_frame() {
        // Arrange
        let frame_key = create_test_frame_key();

        // Act
        let cmd = Command::SelectFrame {
            stream: test_stream_a(),
            frame_key,
        };

        // Assert
        assert!(matches!(cmd, Command::SelectFrame { .. }));
    }

    #[test]
    fn test_command_select_unit() {
        // Arrange
        let unit_key = create_test_unit_key();

        // Act
        let cmd = Command::SelectUnit {
            stream: test_stream_a(),
            unit_key,
        };

        // Assert
        assert!(matches!(cmd, Command::SelectUnit { .. }));
    }

    #[test]
    fn test_command_select_syntax() {
        // Arrange
        let node_id = "test_node_1".to_string();
        let bit_range = create_test_bit_range();

        // Act
        let cmd = Command::SelectSyntax {
            stream: test_stream_a(),
            node_id,
            bit_range,
        };

        // Assert
        assert!(matches!(cmd, Command::SelectSyntax { .. }));
    }

    #[test]
    fn test_command_select_bit_range() {
        // Arrange
        let bit_range = create_test_bit_range();

        // Act
        let cmd = Command::SelectBitRange {
            stream: test_stream_a(),
            bit_range,
        };

        // Assert
        assert!(matches!(cmd, Command::SelectBitRange { .. }));
    }

    #[test]
    fn test_command_select_spatial_block() {
        // Arrange
        let block = create_test_spatial_block();

        // Act
        let cmd = Command::SelectSpatialBlock {
            stream: test_stream_a(),
            block,
        };

        // Assert
        assert!(matches!(cmd, Command::SelectSpatialBlock { .. }));
    }
}

// ============================================================================
// Navigation Commands Tests
// ============================================================================

#[cfg(test)]
mod command_navigation_tests {
    use super::*;

    #[test]
    fn test_command_jump_to_offset() {
        // Arrange
        let offset = 10000u64;

        // Act
        let cmd = Command::JumpToOffset {
            stream: test_stream_a(),
            offset,
        };

        // Assert
        assert!(matches!(cmd, Command::JumpToOffset { .. }));
        match cmd {
            Command::JumpToOffset { stream: s, offset: o } => {
                assert_eq!(s, StreamId::A);
                assert_eq!(o, 10000);
            }
            _ => panic!("Expected JumpToOffset command"),
        }
    }

    #[test]
    fn test_command_jump_to_frame() {
        // Arrange
        let frame_index = 50usize;

        // Act
        let cmd = Command::JumpToFrame {
            stream: test_stream_a(),
            frame_index,
        };

        // Assert
        assert!(matches!(cmd, Command::JumpToFrame { .. }));
        match cmd {
            Command::JumpToFrame { stream: s, frame_index: fi } => {
                assert_eq!(s, StreamId::A);
                assert_eq!(fi, 50);
            }
            _ => panic!("Expected JumpToFrame command"),
        }
    }
}

// ============================================================================
// Player/Overlay Commands Tests
// ============================================================================

#[cfg(test)]
mod command_player_overlay_tests {
    use super::*;

    #[test]
    fn test_command_toggle_overlay() {
        // Arrange
        let layer = OverlayLayer::QpHeatmap;

        // Act
        let cmd = Command::ToggleOverlay {
            stream: test_stream_a(),
            layer,
        };

        // Assert
        assert!(matches!(cmd, Command::ToggleOverlay { .. }));
    }

    #[test]
    fn test_command_set_overlay_opacity() {
        // Arrange
        let opacity = 0.7f32;

        // Act
        let cmd = Command::SetOverlayOpacity {
            stream: test_stream_a(),
            opacity,
        };

        // Assert
        assert!(matches!(cmd, Command::SetOverlayOpacity { .. }));
        match cmd {
            Command::SetOverlayOpacity { stream: s, opacity: o } => {
                assert_eq!(s, StreamId::A);
                assert_eq!(o, 0.7);
            }
            _ => panic!("Expected SetOverlayOpacity command"),
        }
    }

    #[test]
    fn test_command_set_player_mode() {
        // Arrange
        let mode = PlayerMode::Decoded;

        // Act
        let cmd = Command::SetPlayerMode {
            stream: test_stream_a(),
            mode,
        };

        // Assert
        assert!(matches!(cmd, Command::SetPlayerMode { .. }));
    }

    #[test]
    fn test_command_set_player_mode_variants() {
        // Arrange & Act - Test all player modes
        let modes = [
            PlayerMode::Decoded,
            PlayerMode::Residual,
            PlayerMode::Diff,
            PlayerMode::Predicted,
        ];

        for mode in modes {
            let cmd = Command::SetPlayerMode {
                stream: test_stream_a(),
                mode,
            };
            assert!(matches!(cmd, Command::SetPlayerMode { .. }));
        }
    }
}

// ============================================================================
// Playback Commands Tests
// ============================================================================

#[cfg(test)]
mod command_playback_tests {
    use super::*;

    #[test]
    fn test_command_play_pause() {
        // Arrange & Act
        let cmd = Command::PlayPause {
            stream: test_stream_a(),
        };

        // Assert
        assert!(matches!(cmd, Command::PlayPause { .. }));
    }

    #[test]
    fn test_command_step_forward() {
        // Arrange & Act
        let cmd = Command::StepForward {
            stream: test_stream_a(),
        };

        // Assert
        assert!(matches!(cmd, Command::StepForward { .. }));
    }

    #[test]
    fn test_command_step_backward() {
        // Arrange & Act
        let cmd = Command::StepBackward {
            stream: test_stream_a(),
        };

        // Assert
        assert!(matches!(cmd, Command::StepBackward { .. }));
    }
}

// ============================================================================
// Dual View Commands Tests
// ============================================================================

#[cfg(test)]
mod command_dual_view_tests {
    use super::*;

    #[test]
    fn test_command_set_workspace_mode() {
        // Arrange
        let mode = WorkspaceMode::Dual;

        // Act
        let cmd = Command::SetWorkspaceMode { mode };

        // Assert
        assert!(matches!(cmd, Command::SetWorkspaceMode { .. }));
    }

    #[test]
    fn test_command_set_sync_mode() {
        // Arrange
        let mode = SyncMode::Full;

        // Act
        let cmd = Command::SetSyncMode { mode };

        // Assert
        assert!(matches!(cmd, Command::SetSyncMode { .. }));
    }
}

// ============================================================================
// Export Commands Tests
// ============================================================================

#[cfg(test)]
mod command_export_tests {
    use super::*;

    #[test]
    fn test_command_export_csv() {
        // Arrange
        let path = create_test_path();

        // Act
        let cmd = Command::ExportCsv {
            stream: test_stream_a(),
            kind: ExportKind::Csv,
        };

        // Assert
        assert!(matches!(cmd, Command::ExportCsv { .. }));
    }

    #[test]
    fn test_command_export_bitstream() {
        // Arrange
        let range = Some(100..200usize);

        // Act
        let cmd = Command::ExportBitstream {
            stream: test_stream_a(),
            range,
        };

        // Assert
        assert!(matches!(cmd, Command::ExportBitstream { .. }));
    }

    #[test]
    fn test_command_export_v2() {
        // Arrange
        let path = create_test_path();

        // Act
        let cmd = Command::Export {
            stream: test_stream_a(),
            content: ExportContent::Frames,
            format: ExportFormat::Csv,
            path,
            frame_range: Some((0, 100)),
        };

        // Assert
        assert!(matches!(cmd, Command::Export { .. }));
    }
}

// ============================================================================
// Bookmark Commands Tests
// ============================================================================

#[cfg(test)]
mod command_bookmark_tests {
    use super::*;

    #[test]
    fn test_command_add_bookmark() {
        // Arrange
        let frame_key = create_test_frame_key();

        // Act
        let cmd = Command::AddBookmark {
            stream: test_stream_a(),
            frame_key,
        };

        // Assert
        assert!(matches!(cmd, Command::AddBookmark { .. }));
    }

    #[test]
    fn test_command_remove_bookmark() {
        // Arrange
        let frame_key = create_test_frame_key();

        // Act
        let cmd = Command::RemoveBookmark {
            stream: test_stream_a(),
            frame_key,
        };

        // Assert
        assert!(matches!(cmd, Command::RemoveBookmark { .. }));
    }
}

// ============================================================================
// Evidence Export Commands Tests
// ============================================================================

#[cfg(test)]
mod command_evidence_tests {
    use super::*;

    #[test]
    fn test_command_export_evidence_bundle() {
        // Arrange
        let path = create_test_path();

        // Act
        let cmd = Command::ExportEvidenceBundle {
            stream: test_stream_a(),
            path,
        };

        // Assert
        assert!(matches!(cmd, Command::ExportEvidenceBundle { .. }));
    }
}

// ============================================================================
// Order Type Tests
// ============================================================================

#[cfg(test)]
mod command_order_type_tests {
    use super::*;

    #[test]
    fn test_order_type_display() {
        // Arrange & Act
        let order_type = OrderType::Display;

        // Assert
        assert_eq!(order_type, OrderType::Display);
    }

    #[test]
    fn test_order_type_decode() {
        // Arrange & Act
        let order_type = OrderType::Decode;

        // Assert
        assert_eq!(order_type, OrderType::Decode);
    }

    #[test]
    fn test_order_type_equality() {
        // Arrange
        let display1 = OrderType::Display;
        let display2 = OrderType::Display;
        let decode = OrderType::Decode;

        // Assert
        assert_eq!(display1, display2);
        assert_ne!(display1, decode);
    }
}

// ============================================================================
// Copy Operations Tests
// ============================================================================

#[cfg(test)]
mod command_copy_tests {
    use super::*;

    #[test]
    fn test_command_copy_selection() {
        // Arrange & Act
        let cmd = Command::CopySelection;

        // Assert
        assert!(matches!(cmd, Command::CopySelection));
    }

    #[test]
    fn test_command_copy_bytes() {
        // Arrange
        let byte_range = 1000..2000u64;

        // Act
        let cmd = Command::CopyBytes { byte_range };

        // Assert
        assert!(matches!(cmd, Command::CopyBytes { .. }));
    }
}

// ============================================================================
// Enum Values Tests
// ============================================================================

#[cfg(test)]
mod enum_values_tests {
    use super::*;

    #[test]
    fn test_overlay_layer_values() {
        // Assert all OverlayLayer values exist
        let layers = [
            OverlayLayer::Grid,
            OverlayLayer::Transform,
            OverlayLayer::MvL0,
            OverlayLayer::MvL1,
            OverlayLayer::QpHeatmap,
        ];

        assert_eq!(layers.len(), 5);
    }

    #[test]
    fn test_sync_mode_values() {
        // Assert all SyncMode values exist
        let modes = [SyncMode::Off, SyncMode::Playhead, SyncMode::Full];

        assert_eq!(modes.len(), 3);
    }

    #[test]
    fn test_export_content_values() {
        // Assert all ExportContent values exist
        let contents = [
            ExportContent::Frames,
            ExportContent::Metrics,
            ExportContent::Diagnostics,
            ExportContent::Summary,
            ExportContent::Bitstream,
        ];

        assert_eq!(contents.len(), 5);
    }

    #[test]
    fn test_export_format_values() {
        // Assert all ExportFormat values exist
        let formats = [ExportFormat::Csv, ExportFormat::Json, ExportFormat::JsonPretty];

        assert_eq!(formats.len(), 3);
    }
}

// ============================================================================
// Edge Cases
// ============================================================================

#[cfg(test)]
mod edge_case_tests {
    use super::*;

    #[test]
    fn test_export_bitstream_no_range() {
        // Arrange
        let range: Option<Range<usize>> = None;

        // Act
        let cmd = Command::ExportBitstream {
            stream: test_stream_a(),
            range,
        };

        // Assert
        assert!(matches!(cmd, Command::ExportBitstream { .. }));
    }

    #[test]
    fn test_export_v2_no_frame_range() {
        // Arrange
        let path = create_test_path();

        // Act
        let cmd = Command::Export {
            stream: test_stream_a(),
            content: ExportContent::Frames,
            format: ExportFormat::Csv,
            path,
            frame_range: None,
        };

        // Assert
        assert!(matches!(cmd, Command::Export { .. }));
    }

    #[test]
    fn test_toggle_detail_mode() {
        // Arrange & Act
        let cmd = Command::ToggleDetailMode;

        // Assert
        assert!(matches!(cmd, Command::ToggleDetailMode));
    }
}
