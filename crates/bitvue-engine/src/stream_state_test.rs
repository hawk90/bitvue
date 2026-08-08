// StreamState module tests
//
// Per generate-tests skill: Arrange-Act-Assert pattern with fixtures
// and edge case coverage.
//
// Tests for refactored god objects:
// - UnitNode → UnitHeader, FrameInfo, FrameAnalysis
// - CachedFrame → FrameMetadata, FrameRgbData, FrameYuvData

use super::*;
// Use stream_state types explicitly to avoid ambiguity
use crate::{
    stream_state::{FrameInfo, FrameMetadata}, CachedFrame, FrameAnalysis, FrameRgbData,
    FrameYuvData, StreamId, UnitHeader,
};

// ============================================================================
// Fixtures
// ============================================================================

/// Create a test unit key
fn create_test_unit_key() -> crate::UnitKey {
    crate::UnitKey {
        stream: StreamId::A,
        unit_type: "OBU_FRAME".to_string(),
        offset: 1000,
        size: 500,
    }
}

/// Create a test unit header
fn create_test_unit_header() -> UnitHeader {
    UnitHeader {
        key: create_test_unit_key(),
        unit_type: std::sync::Arc::from("OBU_FRAME"),
        offset: 1000,
        size: 500,
        display_name: std::sync::Arc::from("Frame OBU"),
    }
}

/// Create a test frame info
fn create_test_frame_info() -> FrameInfo {
    FrameInfo {
        frame_index: Some(10),
        frame_type: Some(std::sync::Arc::from("KEY")),
        pts: Some(100),
        dts: Some(90),
        temporal_id: Some(0),
    }
}

/// Create a test frame analysis
fn create_test_frame_analysis() -> FrameAnalysis {
    FrameAnalysis {
        qp_avg: Some(30),
        mv_grid: None,
        ref_frames: Some(vec![0, 5]),
        ref_slots: Some(vec![0, 3]),
    }
}

/// Create a test cached frame with RGB data
fn create_test_cached_frame_rgb(index: usize, width: u32, height: u32) -> CachedFrame {
    let rgb_data = vec![0u8; (width * height * 3) as usize];
    CachedFrame {
        index,
        width,
        height,
        rgb_data,
        decoded: true,
        error: None,
        y_plane: None,
        u_plane: None,
        v_plane: None,
        chroma_width: None,
        chroma_height: None,
    }
}

/// Create a test cached frame with YUV data
fn create_test_cached_frame_yuv(index: usize, width: u32, height: u32) -> CachedFrame {
    let y_size = (width * height) as usize;
    let chroma_width = width / 2;
    let chroma_height = height / 2;
    let uv_size = (chroma_width * chroma_height) as usize;

    CachedFrame {
        index,
        width,
        height,
        rgb_data: vec![],
        decoded: true,
        error: None,
        y_plane: Some(std::sync::Arc::new(vec![0u8; y_size])),
        u_plane: Some(std::sync::Arc::new(vec![0u8; uv_size])),
        v_plane: Some(std::sync::Arc::new(vec![0u8; uv_size])),
        chroma_width: Some(chroma_width),
        chroma_height: Some(chroma_height),
    }
}

// ============================================================================
// UnitNode Tests (refactored from god object)
// ============================================================================

#[cfg(test)]
mod unit_node_tests {
    use super::*;

    #[test]
    fn test_unit_node_new() {
        // Arrange & Act
        let unit = UnitNode::new(
            StreamId::A,
            "OBU_FRAME".to_string(),
            1000,
            500,
        );

        // Assert
        assert_eq!(unit.key.stream, StreamId::A);
        assert_eq!(unit.key.unit_type, "OBU_FRAME");
        assert_eq!(unit.offset, 1000);
        assert_eq!(unit.size, 500);
    }

    #[test]
    fn test_unit_node_header() {
        // Arrange
        let unit = UnitNode::new(
            StreamId::A,
            "OBU_FRAME".to_string(),
            1000,
            500,
        );

        // Act
        let header = unit.header();

        // Assert
        assert_eq!(header.key.stream, StreamId::A);
        assert_eq!(&*header.unit_type, "OBU_FRAME");
        assert_eq!(header.offset, 1000);
        assert_eq!(header.size, 500);
        assert_eq!(&*header.display_name, "OBU_FRAME @ 0x000003E8");
    }

    #[test]
    fn test_unit_node_frame_info() {
        // Arrange
        let mut unit = UnitNode::new(
            StreamId::A,
            "OBU_FRAME".to_string(),
            1000,
            500,
        );
        unit.frame_index = Some(10);
        unit.frame_type = Some(std::sync::Arc::from("KEY"));
        unit.pts = Some(100);
        unit.dts = Some(90);
        unit.temporal_id = Some(0);

        // Act
        let frame_info = unit.frame_info();

        // Assert
        assert_eq!(frame_info.frame_index, Some(10));
        assert_eq!(frame_info.frame_type, Some(std::sync::Arc::from("KEY")));
    assert_eq!(&*frame_info.frame_type.unwrap(), "KEY");
        assert_eq!(frame_info.pts, Some(100));
        assert_eq!(frame_info.dts, Some(90));
        assert_eq!(frame_info.temporal_id, Some(0));
    }

    #[test]
    fn test_unit_node_frame_info_missing() {
        // Arrange
        let unit = UnitNode::new(
            StreamId::A,
            "OBU_SEQUENCE_HEADER".to_string(),
            0,
            100,
        );

        // Act
        let frame_info = unit.frame_info();

        // Assert
        assert!(frame_info.frame_index.is_none());
        assert!(frame_info.frame_type.is_none());
        assert!(frame_info.pts.is_none());
        assert!(frame_info.dts.is_none());
        assert!(frame_info.temporal_id.is_none());
    }

    #[test]
    fn test_unit_node_analysis() {
        // Arrange
        let mut unit = UnitNode::new(
            StreamId::A,
            "OBU_FRAME".to_string(),
            1000,
            500,
        );
        unit.qp_avg = Some(30);
        unit.ref_frames = Some(vec![0, 5]);
        unit.ref_slots = Some(vec![0, 3]);

        // Act
        let analysis = unit.analysis();

        // Assert
        assert_eq!(analysis.qp_avg, Some(30));
        assert_eq!(analysis.ref_frames, Some(vec![0, 5]));
        assert_eq!(analysis.ref_slots, Some(vec![0, 3]));
    }

    #[test]
    fn test_unit_node_analysis_empty() {
        // Arrange
        let unit = UnitNode::new(
            StreamId::A,
            "OBU_SEQUENCE_HEADER".to_string(),
            0,
            100,
        );

        // Act
        let analysis = unit.analysis();

        // Assert
        assert!(analysis.qp_avg.is_none());
        assert!(analysis.mv_grid.is_none());
        assert!(analysis.ref_frames.is_none());
        assert!(analysis.ref_slots.is_none());
    }

    #[test]
    fn test_unit_node_from_components() {
        // Arrange
        let header = create_test_unit_header();
        let frame_info = create_test_frame_info();
        let analysis = create_test_frame_analysis();

        // Act
        let unit = UnitNode::from_components(header, frame_info, analysis, vec![]);

        // Assert
        assert_eq!(unit.key.stream, StreamId::A);
        assert_eq!(unit.frame_index, Some(10));
        assert_eq!(unit.frame_type, Some(std::sync::Arc::from("KEY")));
        assert_eq!(&*unit.frame_type.unwrap(), "KEY");
        assert_eq!(unit.qp_avg, Some(30));
    }

    #[test]
    fn test_unit_node_with_frame_info() {
        // Arrange
        let unit = UnitNode::new(
            StreamId::A,
            "OBU_FRAME".to_string(),
            1000,
            500,
        );
        let frame_info = create_test_frame_info();

        // Act
        let updated = unit.with_frame_info(frame_info);

        // Assert
        assert_eq!(updated.frame_index, Some(10));
        assert_eq!(updated.frame_type, Some(std::sync::Arc::from("KEY")));
        assert_eq!(&*updated.frame_type.unwrap(), "KEY");
        assert_eq!(updated.pts, Some(100));
    }

    #[test]
    fn test_unit_node_with_analysis() {
        // Arrange
        let unit = UnitNode::new(
            StreamId::A,
            "OBU_FRAME".to_string(),
            1000,
            500,
        );
        let analysis = create_test_frame_analysis();

        // Act
        let updated = unit.with_analysis(analysis);

        // Assert
        assert_eq!(updated.qp_avg, Some(30));
        assert_eq!(updated.ref_frames, Some(vec![0, 5]));
    }

    #[test]
    fn test_unit_node_has_frame_info() {
        // Arrange
        let unit = UnitNode::new(
            StreamId::A,
            "OBU_FRAME".to_string(),
            1000,
            500,
        );

        // Act & Assert
        assert!(!unit.has_frame_info());

        let mut unit_with_frame = unit;
        unit_with_frame.frame_index = Some(10);
        assert!(unit_with_frame.has_frame_info());
    }

    #[test]
    fn test_unit_node_has_analysis() {
        // Arrange
        let unit = UnitNode::new(
            StreamId::A,
            "OBU_FRAME".to_string(),
            1000,
            500,
        );

        // Act & Assert
        assert!(!unit.has_analysis());

        let mut unit_with_qp = unit;
        unit_with_qp.qp_avg = Some(25);
        assert!(unit_with_qp.has_analysis());
    }
}

// ============================================================================
// UnitHeader Tests
// ============================================================================

#[cfg(test)]
mod unit_header_tests {
    use super::*;

    #[test]
    fn test_unit_header_complete() {
        // Arrange & Act
        let header = UnitHeader {
            key: create_test_unit_key(),
            unit_type: std::sync::Arc::from("OBU_FRAME"),
            offset: 1000,
            size: 500,
            display_name: std::sync::Arc::from("Frame OBU"),
        };

        // Assert
        assert_eq!(&*header.unit_type, "OBU_FRAME");
        assert_eq!(header.offset, 1000);
        assert_eq!(header.size, 500);
        assert_eq!(&*header.display_name, "Frame OBU");
    }

    #[test]
    fn test_unit_header_from_unit_node() {
        // Arrange
        let unit = UnitNode::new(
            StreamId::A,
            "OBU_FRAME".to_string(),
            2000,
            1000,
        );

        // Act
        let header = unit.header();

        // Assert
        assert_eq!(header.offset, 2000);
        assert_eq!(header.size, 1000);
        assert_eq!(&*header.display_name, "OBU_FRAME @ 0x000007D0");
    }
}

// ============================================================================
// FrameInfo Tests
// ============================================================================

#[cfg(test)]
mod frame_info_tests {
    use super::*;

    #[test]
    fn test_frame_info_complete() {
        // Arrange & Act
        let frame_info = FrameInfo {
            frame_index: Some(10),
            frame_type: Some(std::sync::Arc::from("KEY")),
            pts: Some(100),
            dts: Some(90),
            temporal_id: Some(0),
        };

        // Assert
        assert_eq!(frame_info.frame_index, Some(10));
        assert_eq!(frame_info.frame_type, Some(std::sync::Arc::from("KEY")));
    assert_eq!(&*frame_info.frame_type.unwrap(), "KEY");
        assert_eq!(frame_info.pts, Some(100));
        assert_eq!(frame_info.dts, Some(90));
        assert_eq!(frame_info.temporal_id, Some(0));
    }

    #[test]
    fn test_frame_info_empty() {
        // Arrange & Act
        let frame_info = FrameInfo {
            frame_index: None,
            frame_type: None,
            pts: None,
            dts: None,
            temporal_id: None,
        };

        // Assert
        assert!(frame_info.frame_index.is_none());
        assert!(frame_info.frame_type.is_none());
        assert!(frame_info.pts.is_none());
        assert!(frame_info.dts.is_none());
        assert!(frame_info.temporal_id.is_none());
    }

    #[test]
    fn test_frame_info_from_unit_node() {
        // Arrange
        let mut unit = UnitNode::new(
            StreamId::A,
            "OBU_FRAME".to_string(),
            1000,
            500,
        );
        unit.frame_index = Some(5);
        unit.frame_type = Some(std::sync::Arc::from("INTER"));
        unit.pts = Some(50);
        unit.dts = Some(40);
        unit.temporal_id = Some(1);

        // Act
        let frame_info = unit.frame_info();

        // Assert
        assert_eq!(frame_info.frame_index, Some(5));
        assert_eq!(frame_info.frame_type, Some(std::sync::Arc::from("INTER")));
    }
}

// ============================================================================
// FrameAnalysis Tests
// ============================================================================

#[cfg(test)]
mod frame_analysis_tests {
    use super::*;

    #[test]
    fn test_frame_analysis_complete() {
        // Arrange & Act
        let analysis = FrameAnalysis {
            qp_avg: Some(30),
            mv_grid: Some(crate::MVGrid {
                coded_width: 128,
                coded_height: 128,
                grid_w: 8,
                grid_h: 8,
                block_w: 16,
                block_h: 16,
                mv_l0: vec![],
                mv_l1: vec![],
                mode: None,
            }),
            ref_frames: Some(vec![0, 1, 2]),
            ref_slots: Some(vec![0, 1, 2]),
        };

        // Assert
        assert_eq!(analysis.qp_avg, Some(30));
        assert!(analysis.mv_grid.is_some());
        assert_eq!(analysis.ref_frames, Some(vec![0, 1, 2]));
        assert_eq!(analysis.ref_slots, Some(vec![0, 1, 2]));
    }

    #[test]
    fn test_frame_analysis_minimal() {
        // Arrange & Act
        let analysis = FrameAnalysis {
            qp_avg: None,
            mv_grid: None,
            ref_frames: None,
            ref_slots: None,
        };

        // Assert
        assert!(analysis.qp_avg.is_none());
        assert!(analysis.mv_grid.is_none());
        assert!(analysis.ref_frames.is_none());
        assert!(analysis.ref_slots.is_none());
    }
}

// ============================================================================
// CachedFrame Tests (refactored from god object)
// ============================================================================

#[cfg(test)]
mod cached_frame_tests {
    use super::*;

    #[test]
    fn test_cached_frame_new_rgb() {
        // Arrange
        let frame = create_test_cached_frame_rgb(0, 1920, 1080);

        // Assert
        assert_eq!(frame.index, 0);
        assert_eq!(frame.width, 1920);
        assert_eq!(frame.height, 1080);
        assert_eq!(frame.rgb_data.len(), 1920 * 1080 * 3);
        assert!(frame.decoded);
        assert!(frame.error.is_none());
    }

    #[test]
    fn test_cached_frame_new_yuv() {
        // Arrange
        let frame = create_test_cached_frame_yuv(5, 1280, 720);

        // Assert
        assert_eq!(frame.index, 5);
        assert_eq!(frame.width, 1280);
        assert_eq!(frame.height, 720);
        assert!(frame.y_plane.is_some());
        assert!(frame.u_plane.is_some());
        assert!(frame.v_plane.is_some());
        assert_eq!(frame.chroma_width, Some(640));
        assert_eq!(frame.chroma_height, Some(360));
    }

    #[test]
    fn test_cached_frame_metadata() {
        // Arrange
        let frame = create_test_cached_frame_rgb(10, 640, 480);

        // Act
        let metadata = frame.metadata();

        // Assert
        assert_eq!(metadata.index, 10);
        assert_eq!(metadata.width, 640);
        assert_eq!(metadata.height, 480);
        assert!(metadata.decoded);
        assert!(metadata.error.is_none());
    }

    #[test]
    fn test_cached_frame_metadata_with_error() {
        // Arrange
        let mut frame = create_test_cached_frame_rgb(0, 100, 100);
        frame.decoded = false;
        frame.error = Some("Decode error".to_string());

        // Act
        let metadata = frame.metadata();

        // Assert
        assert!(!metadata.decoded);
        assert_eq!(metadata.error, Some("Decode error".to_string()));
    }

    #[test]
    fn test_cached_frame_rgb() {
        // Arrange
        let mut frame = create_test_cached_frame_rgb(0, 100, 100);
        // Fill with test pattern
        for i in 0..frame.rgb_data.len() {
            frame.rgb_data[i] = (i % 256) as u8;
        }

        // Act
        let rgb_data = frame.rgb();

        // Assert
        assert_eq!(rgb_data.data.len(), 100 * 100 * 3);
        assert_eq!(rgb_data.data[0], 0);
        assert_eq!(rgb_data.data[1], 1);
        assert_eq!(rgb_data.data[2], 2);
    }

    #[test]
    fn test_cached_frame_yuv() {
        // Arrange
        let frame = create_test_cached_frame_yuv(0, 100, 100);

        // Act
        let yuv_data = frame.yuv();

        // Assert
        assert!(yuv_data.y_plane.is_some());
        assert!(yuv_data.u_plane.is_some());
        assert!(yuv_data.v_plane.is_some());
        assert_eq!(yuv_data.chroma_width, Some(50));
        assert_eq!(yuv_data.chroma_height, Some(50));
    }

    #[test]
    fn test_cached_frame_rgb_size_calculation() {
        // Arrange
        let frame = create_test_cached_frame_rgb(0, 1920, 1080);

        // Act
        let expected_size = 1920 * 1080 * 3;

        // Assert
        assert_eq!(frame.rgb_data.len(), expected_size);
    }

    #[test]
    fn test_cached_frame_yuv_size_calculation() {
        // Arrange
        let frame = create_test_cached_frame_yuv(0, 1920, 1080);

        // Act
        let y_size = 1920 * 1080;
        let chroma_size = (1920 / 2) * (1080 / 2);

        // Assert
        assert_eq!(frame.y_plane.as_ref().unwrap().len(), y_size);
        assert_eq!(frame.u_plane.as_ref().unwrap().len(), chroma_size);
        assert_eq!(frame.v_plane.as_ref().unwrap().len(), chroma_size);
    }
}

// ============================================================================
// FrameMetadata Tests
// ============================================================================

#[cfg(test)]
mod frame_metadata_tests {
    use super::*;

    #[test]
    fn test_frame_metadata_complete() {
        // Arrange & Act
        let metadata = FrameMetadata {
            index: 10,
            width: 1920,
            height: 1080,
            decoded: true,
            error: None,
        };

        // Assert
        assert_eq!(metadata.index, 10);
        assert_eq!(metadata.width, 1920);
        assert_eq!(metadata.height, 1080);
        assert!(metadata.decoded);
    }

    #[test]
    fn test_frame_metadata_with_error() {
        // Arrange & Act
        let metadata = FrameMetadata {
            index: 0,
            width: 640,
            height: 480,
            decoded: false,
            error: Some("Memory allocation failed".to_string()),
        };

        // Assert
        assert!(!metadata.decoded);
        assert_eq!(metadata.error, Some("Memory allocation failed".to_string()));
    }
}

// ============================================================================
// FrameRgbData Tests
// ============================================================================

#[cfg(test)]
mod frame_rgb_data_tests {
    use super::*;

    #[test]
    fn test_frame_rgb_data_complete() {
        // Arrange & Act
        let rgb_data = FrameRgbData {
            data: vec![255u8; 100 * 100 * 3],
        };

        // Assert
        assert_eq!(rgb_data.data.len(), 100 * 100 * 3);
        assert_eq!(rgb_data.data[0], 255);
    }

    #[test]
    fn test_frame_rgb_data_empty() {
        // Arrange & Act
        let rgb_data = FrameRgbData { data: vec![] };

        // Assert
        assert!(rgb_data.data.is_empty());
    }
}

// ============================================================================
// FrameYuvData Tests
// ============================================================================

#[cfg(test)]
mod frame_yuv_data_tests {
    use super::*;

    #[test]
    fn test_frame_yuv_data_complete() {
        // Arrange & Act
        let yuv_data = FrameYuvData {
            y_plane: Some(std::sync::Arc::new(vec![0u8; 100 * 100])),
            u_plane: Some(std::sync::Arc::new(vec![0u8; 50 * 50])),
            v_plane: Some(std::sync::Arc::new(vec![0u8; 50 * 50])),
            chroma_width: Some(50),
            chroma_height: Some(50),
        };

        // Assert
        assert!(yuv_data.y_plane.is_some());
        assert!(yuv_data.u_plane.is_some());
        assert!(yuv_data.v_plane.is_some());
        assert_eq!(yuv_data.chroma_width, Some(50));
        assert_eq!(yuv_data.chroma_height, Some(50));
    }

    #[test]
    fn test_frame_yuv_data_y_only() {
        // Arrange & Act
        let yuv_data = FrameYuvData {
            y_plane: Some(std::sync::Arc::new(vec![0u8; 100 * 100])),
            u_plane: None,
            v_plane: None,
            chroma_width: None,
            chroma_height: None,
        };

        // Assert
        assert!(yuv_data.y_plane.is_some());
        assert!(yuv_data.u_plane.is_none());
        assert!(yuv_data.v_plane.is_none());
    }
}

// ============================================================================
// Edge Cases
// ============================================================================

#[cfg(test)]
mod edge_case_tests {
    use super::*;

    #[test]
    fn test_unit_node_zero_size() {
        // Arrange & Act
        let unit = UnitNode::new(
            StreamId::A,
            "EMPTY".to_string(),
            0,
            0,
        );

        // Assert
        assert_eq!(unit.size, 0);
    }

    #[test]
    fn test_cached_frame_zero_dimensions() {
        // Arrange & Act
        let frame = CachedFrame {
            index: 0,
            width: 0,
            height: 0,
            rgb_data: vec![],
            decoded: true,
            error: None,
            y_plane: None,
            u_plane: None,
            v_plane: None,
            chroma_width: None,
            chroma_height: None,
        };

        // Assert
        assert_eq!(frame.width, 0);
        assert_eq!(frame.height, 0);
        assert!(frame.rgb_data.is_empty());
    }

    #[test]
    fn test_cached_frame_large_dimensions() {
        // Arrange & Act
        let frame = create_test_cached_frame_rgb(0, 7680, 4320); // 8K

        // Assert
        assert_eq!(frame.width, 7680);
        assert_eq!(frame.height, 4320);
        assert_eq!(frame.rgb_data.len(), 7680 * 4320 * 3);
    }

    #[test]
    fn test_unit_node_large_frame_index() {
        // Arrange
        let mut unit = UnitNode::new(
            StreamId::A,
            "OBU_FRAME".to_string(),
            0,
            100,
        );
        unit.frame_index = Some(usize::MAX);

        // Act
        let frame_info = unit.frame_info();

        // Assert
        assert_eq!(frame_info.frame_index, Some(usize::MAX));
    }

    #[test]
    fn test_multiple_frame_composition() {
        // Arrange
        let frame = create_test_cached_frame_rgb(0, 100, 100);
        let metadata = frame.metadata();
        let rgb = frame.rgb();

        // Act & Assert
        assert_eq!(metadata.index, 0);
        assert_eq!(metadata.width, 100);
        assert_eq!(metadata.height, 100);
        assert_eq!(rgb.data.len(), 100 * 100 * 3);
    }
}
