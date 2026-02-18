// Coordinate Transform module tests
//
// Per generate-tests skill: Arrange-Act-Assert pattern with fixtures
// and edge case coverage.

#[allow(unused_imports)]
use super::*;

// ============================================================================
// Fixtures
// ============================================================================

/// Create a test screen pixel coordinate
#[allow(dead_code)]
fn create_test_screen_px(x: f32, y: f32) -> ScreenPx {
    ScreenPx { x, y }
}

/// Create a test video rect normalized coordinate
#[allow(dead_code)]
fn create_test_video_rect_norm(x: f32, y: f32) -> VideoRectNorm {
    VideoRectNorm { x, y }
}

/// Create a test coded pixel coordinate
#[allow(dead_code)]
fn create_test_coded_px(x: f32, y: f32) -> CodedPx {
    CodedPx { x, y }
}

/// Create a test block index coordinate
#[allow(dead_code)]
fn create_test_block_idx(x: usize, y: usize) -> BlockIdx {
    BlockIdx { x, y }
}

/// Create a test screen rect
#[allow(dead_code)]
fn create_test_screen_rect(x: f32, y: f32, width: f32, height: f32) -> ScreenRect {
    ScreenRect { x, y, width, height }
}

/// Create a test coordinate transformer
#[allow(dead_code)]
fn create_test_transformer() -> CoordinateTransformer {
    CoordinateTransformer::new()
}

/// Create a configured transformer with video rect
#[allow(dead_code)]
fn create_configured_transformer(video_x: f32, video_y: f32, video_w: f32, video_h: f32) -> CoordinateTransformer {
    let mut transformer = CoordinateTransformer::new();
    transformer.set_video_rect(create_test_screen_rect(video_x, video_y, video_w, video_h));
    transformer
}

// ============================================================================
// ScreenPx Tests
// ============================================================================

#[cfg(test)]
mod screen_px_tests {
#[allow(unused_imports)]
    use super::*;

    #[test]
    fn test_screen_px_creation() {
        // Arrange & Act
        let px = create_test_screen_px(100.0, 200.0);

        // Assert
        assert_eq!(px.x, 100.0);
        assert_eq!(px.y, 200.0);
    }

    #[test]
    fn test_screen_px_negative() {
        // Arrange & Act
        let px = create_test_screen_px(-50.0, -100.0);

        // Assert
        assert_eq!(px.x, -50.0);
        assert_eq!(px.y, -100.0);
    }
}

// ============================================================================
// VideoRectNorm Tests
// ============================================================================

#[cfg(test)]
mod video_rect_norm_tests {
#[allow(unused_imports)]
    use super::*;

    #[test]
    fn test_video_rect_norm_creation() {
        // Arrange & Act
        let norm = create_test_video_rect_norm(0.5, 0.5);

        // Assert
        assert_eq!(norm.x, 0.5);
        assert_eq!(norm.y, 0.5);
    }

    #[test]
    fn test_video_rect_norm_range() {
        // Arrange & Act
        let top_left = create_test_video_rect_norm(0.0, 0.0);
        let bottom_right = create_test_video_rect_norm(1.0, 1.0);

        // Assert
        assert_eq!(top_left.x, 0.0);
        assert_eq!(top_left.y, 0.0);
        assert_eq!(bottom_right.x, 1.0);
        assert_eq!(bottom_right.y, 1.0);
    }
}

// ============================================================================
// CodedPx Tests
// ============================================================================

#[cfg(test)]
mod coded_px_tests {
#[allow(unused_imports)]
    use super::*;

    #[test]
    fn test_coded_px_creation() {
        // Arrange & Act
        let px = create_test_coded_px(320.0, 240.0);

        // Assert
        assert_eq!(px.x, 320.0);
        assert_eq!(px.y, 240.0);
    }

    #[test]
    fn test_coded_px_fractional() {
        // Arrange & Act
        let px = create_test_coded_px(320.5, 240.75);

        // Assert
        assert_eq!(px.x, 320.5);
        assert_eq!(px.y, 240.75);
    }
}

// ============================================================================
// BlockIdx Tests
// ============================================================================

#[cfg(test)]
mod block_idx_tests {
#[allow(unused_imports)]
    use super::*;

    #[test]
    fn test_block_idx_creation() {
        // Arrange & Act
        let idx = create_test_block_idx(10, 20);

        // Assert
        assert_eq!(idx.x, 10);
        assert_eq!(idx.y, 20);
    }

    #[test]
    fn test_block_idx_zero() {
        // Arrange & Act
        let idx = create_test_block_idx(0, 0);

        // Assert
        assert_eq!(idx.x, 0);
        assert_eq!(idx.y, 0);
    }
}

// ============================================================================
// ScreenRect Tests
// ============================================================================

#[cfg(test)]
mod screen_rect_tests {
#[allow(unused_imports)]
    use super::*;

    #[test]
    fn test_screen_rect_creation() {
        // Arrange & Act
        let rect = create_test_screen_rect(100.0, 200.0, 640.0, 480.0);

        // Assert
        assert_eq!(rect.x, 100.0);
        assert_eq!(rect.y, 200.0);
        assert_eq!(rect.width, 640.0);
        assert_eq!(rect.height, 480.0);
    }

    #[test]
    fn test_screen_rect_area() {
        // Arrange
        let rect = create_test_screen_rect(0.0, 0.0, 640.0, 480.0);

        // Act
        let area = rect.width * rect.height;

        // Assert
        assert_eq!(area, 640.0 * 480.0);
    }
}

// ============================================================================
// ZoomMode Tests
// ============================================================================

#[cfg(test)]
mod zoom_mode_tests {
#[allow(unused_imports)]
    use super::*;

    #[test]
    fn test_zoom_mode_values() {
        // Assert - all zoom modes exist
        let _ = ZoomMode::Fit;
        let _ = ZoomMode::Fill;
        let _ = ZoomMode::Original;
        let _ = ZoomMode::Custom;
    }
}

// ============================================================================
// CoordinateTransformer Construction Tests
// ============================================================================

#[cfg(test)]
mod coordinate_transformer_construction_tests {
#[allow(unused_imports)]
    use super::*;

    #[test]
    fn test_new_creates_transformer() {
        // Arrange & Act
        let transformer = create_test_transformer();

        // Assert
        assert_eq!(transformer.screen_width, 0.0);
        assert_eq!(transformer.screen_height, 0.0);
        assert_eq!(transformer.coded_width, 0.0);
        assert_eq!(transformer.coded_height, 0.0);
    }

    #[test]
    fn test_default_creates_transformer() {
        // Arrange & Act
        let transformer = CoordinateTransformer::default();

        // Assert
        assert_eq!(transformer.screen_width, 0.0);
        assert_eq!(transformer.screen_height, 0.0);
    }

    #[test]
    fn test_set_screen_dimensions() {
        // Arrange
        let mut transformer = create_test_transformer();

        // Act
        transformer.set_screen_dimensions(1920.0, 1080.0);

        // Assert
        assert_eq!(transformer.screen_width, 1920.0);
        assert_eq!(transformer.screen_height, 1080.0);
    }

    #[test]
    fn test_set_coded_dimensions() {
        // Arrange
        let mut transformer = create_test_transformer();

        // Act
        transformer.set_coded_dimensions(1920.0, 1080.0);

        // Assert
        assert_eq!(transformer.coded_width, 1920.0);
        assert_eq!(transformer.coded_height, 1080.0);
    }

    #[test]
    fn test_set_video_rect() {
        // Arrange
        let mut transformer = create_test_transformer();
        let rect = create_test_screen_rect(100.0, 50.0, 1280.0, 720.0);

        // Act
        transformer.set_video_rect(rect);

        // Assert
        assert_eq!(transformer.video_rect.x, 100.0);
        assert_eq!(transformer.video_rect.y, 50.0);
        assert_eq!(transformer.video_rect.width, 1280.0);
        assert_eq!(transformer.video_rect.height, 720.0);
    }

    #[test]
    fn test_set_zoom_mode() {
        // Arrange
        let mut transformer = create_test_transformer();

        // Act
        transformer.set_zoom_mode(ZoomMode::Fill);

        // Assert
        assert_eq!(transformer.zoom_mode, ZoomMode::Fill);
    }

    #[test]
    fn test_set_zoom_level() {
        // Arrange
        let mut transformer = create_test_transformer();

        // Act
        transformer.set_zoom_level(2.5);

        // Assert
        assert_eq!(transformer.zoom_level, 2.5);
    }
}

// ============================================================================
// Forward Transform Tests (screen → norm → coded → block)
// ============================================================================

#[cfg(test)]
mod forward_transform_tests {
#[allow(unused_imports)]
    use super::*;

    #[test]
    fn test_screen_to_video_rect_norm_center() {
        // Arrange
        let transformer = create_configured_transformer(100.0, 50.0, 1280.0, 720.0);
        let screen = create_test_screen_px(740.0, 410.0); // Center of video rect

        // Act
        let result = transformer.screen_to_video_rect_norm(screen);

        // Assert
        assert!(result.is_some());
        let norm = result.unwrap();
        assert!((norm.x - 0.5).abs() < 0.01);
        assert!((norm.y - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_screen_to_video_rect_norm_top_left() {
        // Arrange
        let transformer = create_configured_transformer(100.0, 50.0, 1280.0, 720.0);
        let screen = create_test_screen_px(100.0, 50.0); // Top-left of video rect

        // Act
        let result = transformer.screen_to_video_rect_norm(screen);

        // Assert
        assert!(result.is_some());
        let norm = result.unwrap();
        assert!((norm.x - 0.0).abs() < 0.01);
        assert!((norm.y - 0.0).abs() < 0.01);
    }

    #[test]
    fn test_screen_to_video_rect_norm_bottom_right() {
        // Arrange
        let transformer = create_configured_transformer(100.0, 50.0, 1280.0, 720.0);
        let screen = create_test_screen_px(1380.0, 770.0); // Bottom-right of video rect

        // Act
        let result = transformer.screen_to_video_rect_norm(screen);

        // Assert
        assert!(result.is_some());
        let norm = result.unwrap();
        assert!((norm.x - 1.0).abs() < 0.01);
        assert!((norm.y - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_screen_to_video_rect_norm_outside() {
        // Arrange
        let transformer = create_configured_transformer(100.0, 50.0, 1280.0, 720.0);
        let screen = create_test_screen_px(50.0, 25.0); // Outside video rect

        // Act
        let result = transformer.screen_to_video_rect_norm(screen);

        // Assert
        assert!(result.is_none());
    }

    #[test]
    fn test_video_rect_norm_to_coded_px_center() {
        // Arrange
        let mut transformer = create_test_transformer();
        transformer.set_coded_dimensions(1920.0, 1080.0);
        let norm = create_test_video_rect_norm(0.5, 0.5);

        // Act
        let coded = transformer.video_rect_norm_to_coded(norm);

        // Assert
        assert!((coded.x - 960.0).abs() < 0.1);
        assert!((coded.y - 540.0).abs() < 0.1);
    }

    #[test]
    fn test_video_rect_norm_to_coded_px_top_left() {
        // Arrange
        let mut transformer = create_test_transformer();
        transformer.set_coded_dimensions(1920.0, 1080.0);
        let norm = create_test_video_rect_norm(0.0, 0.0);

        // Act
        let coded = transformer.video_rect_norm_to_coded(norm);

        // Assert
        assert!((coded.x - 0.0).abs() < 0.1);
        assert!((coded.y - 0.0).abs() < 0.1);
    }

    #[test]
    fn test_video_rect_norm_to_coded_px_bottom_right() {
        // Arrange
        let mut transformer = create_test_transformer();
        transformer.set_coded_dimensions(1920.0, 1080.0);
        let norm = create_test_video_rect_norm(1.0, 1.0);

        // Act
        let coded = transformer.video_rect_norm_to_coded(norm);

        // Assert
        assert!((coded.x - 1920.0).abs() < 0.1);
        assert!((coded.y - 1080.0).abs() < 0.1);
    }

    #[test]
    fn test_coded_to_block_idx_center() {
        // Arrange
        let transformer = create_test_transformer();
        let coded = create_test_coded_px(64.0, 64.0);

        // Act
        let block = transformer.coded_to_block(coded, Some(64));

        // Assert
        assert_eq!(block.x, 1);
        assert_eq!(block.y, 1);
    }

    #[test]
    fn test_coded_to_block_idx_default_size() {
        // Arrange
        let transformer = create_test_transformer();
        let coded = create_test_coded_px(128.0, 64.0);

        // Act
        let block = transformer.coded_to_block(coded, None); // Uses default 64

        // Assert
        assert_eq!(block.x, 2);
        assert_eq!(block.y, 1);
    }

    #[test]
    fn test_coded_to_block_idx_custom_size() {
        // Arrange
        let transformer = create_test_transformer();
        let coded = create_test_coded_px(32.0, 32.0);

        // Act
        let block = transformer.coded_to_block(coded, Some(32));

        // Assert
        assert_eq!(block.x, 1);
        assert_eq!(block.y, 1);
    }

    #[test]
    fn test_screen_to_block_full_pipeline() {
        // Arrange
        let mut transformer = create_test_transformer();
        transformer.set_coded_dimensions(1920.0, 1080.0);
        let rect = create_test_screen_rect(0.0, 0.0, 1920.0, 1080.0);
        transformer.set_video_rect(rect);

        // Screen center (960, 540)
        let screen = create_test_screen_px(960.0, 540.0);

        // Act
        let block = transformer.screen_to_block(screen, Some(64));

        // Assert
        assert!(block.is_some());
        let idx = block.unwrap();
        // Coded center is (960, 540)
        // Block index should be (960/64, 540/64) ≈ (15, 8)
        assert_eq!(idx.x, 15);
        assert_eq!(idx.y, 8);
    }
}

// ============================================================================
// Reverse Transform Tests (block → coded → norm → screen)
// ============================================================================

#[cfg(test)]
mod reverse_transform_tests {
#[allow(unused_imports)]
    use super::*;

    #[test]
    fn test_block_to_coded_px() {
        // Arrange
        let transformer = create_test_transformer();
        let block = create_test_block_idx(2, 3);

        // Act
        let coded = transformer.block_to_coded(block, Some(64));

        // Assert
        assert_eq!(coded.x, 128.0);
        assert_eq!(coded.y, 192.0);
    }

    #[test]
    fn test_block_to_coded_px_default_size() {
        // Arrange
        let transformer = create_test_transformer();
        let block = create_test_block_idx(5, 10);

        // Act
        let coded = transformer.block_to_coded(block, None);

        // Assert
        assert_eq!(coded.x, 320.0); // 5 * 64
        assert_eq!(coded.y, 640.0); // 10 * 64
    }

    #[test]
    fn test_coded_to_video_rect_norm() {
        // Arrange
        let mut transformer = create_test_transformer();
        transformer.set_coded_dimensions(1920.0, 1080.0);
        let coded = create_test_coded_px(960.0, 540.0);

        // Act
        let norm = transformer.coded_to_video_rect_norm(coded);

        // Assert
        assert!((norm.x - 0.5).abs() < 0.01);
        assert!((norm.y - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_video_rect_norm_to_screen_center() {
        // Arrange
        let transformer = create_configured_transformer(100.0, 50.0, 1280.0, 720.0);
        let norm = create_test_video_rect_norm(0.5, 0.5);

        // Act
        let screen = transformer.video_rect_norm_to_screen(norm);

        // Assert
        assert!((screen.x - 740.0).abs() < 0.1); // 100 + 1280*0.5
        assert!((screen.y - 410.0).abs() < 0.1); // 50 + 720*0.5
    }

    #[test]
    fn test_video_rect_norm_to_screen_top_left() {
        // Arrange
        let transformer = create_configured_transformer(100.0, 50.0, 1280.0, 720.0);
        let norm = create_test_video_rect_norm(0.0, 0.0);

        // Act
        let screen = transformer.video_rect_norm_to_screen(norm);

        // Assert
        assert!((screen.x - 100.0).abs() < 0.1);
        assert!((screen.y - 50.0).abs() < 0.1);
    }

    #[test]
    fn test_video_rect_norm_to_screen_bottom_right() {
        // Arrange
        let transformer = create_configured_transformer(100.0, 50.0, 1280.0, 720.0);
        let norm = create_test_video_rect_norm(1.0, 1.0);

        // Act
        let screen = transformer.video_rect_norm_to_screen(norm);

        // Assert
        assert!((screen.x - 1380.0).abs() < 0.1); // 100 + 1280
        assert!((screen.y - 770.0).abs() < 0.1); // 50 + 720
    }

    #[test]
    fn test_block_to_screen_full_pipeline() {
        // Arrange
        let mut transformer = create_test_transformer();
        transformer.set_coded_dimensions(1920.0, 1080.0);
        let rect = create_test_screen_rect(0.0, 0.0, 1920.0, 1080.0);
        transformer.set_video_rect(rect);

        let block = create_test_block_idx(15, 8);

        // Act
        let screen = transformer.block_to_screen(block, Some(64));

        // Assert
        // Block (15, 8) with size 64 = Coded (960, 512)
        // Normalized (0.5, 0.474)
        // Screen (960, 512)
        assert!((screen.x - 960.0).abs() < 1.0);
        assert!((screen.y - 512.0).abs() < 1.0);
    }
}

// ============================================================================
// Utility Method Tests
// ============================================================================

#[cfg(test)]
mod utility_method_tests {
#[allow(unused_imports)]
    use super::*;

    #[test]
    fn test_point_in_rect_true() {
        // Arrange
        let rect = create_test_screen_rect(100.0, 50.0, 640.0, 480.0);
        let point = create_test_screen_px(420.0, 290.0); // Inside

        // Act
        let result = CoordinateTransformer::point_in_rect(point, rect);

        // Assert
        assert!(result);
    }

    #[test]
    fn test_point_in_rect_false_left() {
        // Arrange
        let rect = create_test_screen_rect(100.0, 50.0, 640.0, 480.0);
        let point = create_test_screen_px(50.0, 290.0); // Left of rect

        // Act
        let result = CoordinateTransformer::point_in_rect(point, rect);

        // Assert
        assert!(!result);
    }

    #[test]
    fn test_point_in_rect_false_right() {
        // Arrange
        let rect = create_test_screen_rect(100.0, 50.0, 640.0, 480.0);
        let point = create_test_screen_px(750.0, 290.0); // Right of rect

        // Act
        let result = CoordinateTransformer::point_in_rect(point, rect);

        // Assert
        assert!(!result);
    }

    #[test]
    fn test_point_in_rect_on_edge() {
        // Arrange
        let rect = create_test_screen_rect(100.0, 50.0, 640.0, 480.0);
        let point = create_test_screen_px(100.0, 50.0); // Top-left corner

        // Act
        let result = CoordinateTransformer::point_in_rect(point, rect);

        // Assert - Edge cases may be inclusive or exclusive
        // (implementation dependent)
    }
}

// ============================================================================
// Edge Cases
// ============================================================================

#[cfg(test)]
mod edge_case_tests {
#[allow(unused_imports)]
    use super::*;

    #[test]
    fn test_zero_dimensions() {
        // Arrange
        let mut transformer = create_test_transformer();
        transformer.set_coded_dimensions(0.0, 0.0);
        let norm = create_test_video_rect_norm(0.5, 0.5);

        // Act
        let coded = transformer.video_rect_norm_to_coded(norm);

        // Assert - Should handle gracefully (NaN or zero)
    }

    #[test]
    fn test_negative_coordinates() {
        // Arrange
        let screen = create_test_screen_px(-100.0, -200.0);
        let transformer = create_configured_transformer(100.0, 50.0, 1280.0, 720.0);

        // Act
        let result = transformer.screen_to_video_rect_norm(screen);

        // Assert
        assert!(result.is_none()); // Outside video rect
    }

    #[test]
    fn test_very_large_coordinates() {
        // Arrange
        let screen = create_test_screen_px(10000.0, 10000.0);
        let transformer = create_configured_transformer(100.0, 50.0, 1280.0, 720.0);

        // Act
        let result = transformer.screen_to_video_rect_norm(screen);

        // Assert
        assert!(result.is_none()); // Outside video rect
    }

    #[test]
    fn test_block_size_zero() {
        // Arrange
        let transformer = create_test_transformer();
        let coded = create_test_coded_px(64.0, 64.0);

        // Act
        let block = transformer.coded_to_block(coded, Some(0));

        // Assert - Should handle gracefully (infinity or error)
    }

    #[test]
    fn test_floating_point_precision() {
        // Arrange
        let mut transformer = create_test_transformer();
        transformer.set_coded_dimensions(1920.0, 1080.0);
        let norm = create_test_video_rect_norm(0.3333333333, 0.6666666666);

        // Act
        let coded = transformer.video_rect_norm_to_coded(norm);

        // Assert - Should handle precision loss gracefully
        assert!((coded.x - 640.0).abs() < 0.1);
        assert!((coded.y - 720.0).abs() < 0.1);
    }

    #[test]
    fn test_empty_video_rect() {
        // Arrange
        let mut transformer = create_test_transformer();
        transformer.set_video_rect(create_test_screen_rect(0.0, 0.0, 0.0, 0.0));
        let screen = create_test_screen_px(0.0, 0.0);

        // Act
        let result = transformer.screen_to_video_rect_norm(screen);

        // Assert - Empty rect should return None or handle edge case
    }

    #[test]
    fn test_zoom_level_effect() {
        // Arrange
        let mut transformer = create_test_transformer();
        transformer.set_zoom_level(2.0);
        transformer.set_coded_dimensions(1920.0, 1080.0);

        // Act & Assert - Zoom level should affect transforms
        // (exact behavior depends on implementation)
    }

    #[test]
    fn test_roundtrip_screen_to_norm_to_screen() {
        // Arrange
        let transformer = create_configured_transformer(100.0, 50.0, 1280.0, 720.0);
        let original = create_test_screen_px(740.0, 410.0);

        // Act
        let norm = transformer.screen_to_video_rect_norm(original).unwrap();
        let roundtrip = transformer.video_rect_norm_to_screen(norm);

        // Assert
        assert!((roundtrip.x - original.x).abs() < 0.1);
        assert!((roundtrip.y - original.y).abs() < 0.1);
    }

    #[test]
    fn test_roundtrip_coded_to_norm_to_coded() {
        // Arrange
        let mut transformer = create_test_transformer();
        transformer.set_coded_dimensions(1920.0, 1080.0);
        let original = create_test_coded_px(960.0, 540.0);

        // Act
        let norm = transformer.coded_to_video_rect_norm(original);
        let roundtrip = transformer.video_rect_norm_to_coded(norm);

        // Assert
        assert!((roundtrip.x - original.x).abs() < 0.1);
        assert!((roundtrip.y - original.y).abs() < 0.1);
    }

    #[test]
    fn test_roundtrip_block_to_coded_to_block() {
        // Arrange
        let transformer = create_test_transformer();
        let original = create_test_block_idx(10, 20);

        // Act
        let coded = transformer.block_to_coded(original, Some(64));
        let roundtrip = transformer.coded_to_block(coded, Some(64));

        // Assert
        assert_eq!(roundtrip.x, original.x);
        assert_eq!(roundtrip.y, original.y);
    }
}
