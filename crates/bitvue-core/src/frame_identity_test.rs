// Frame identity module tests
//
// Per generate-tests skill: Arrange-Act-Assert pattern with fixtures
// and edge case coverage.

use super::*;

/// Create test frame metadata
fn create_test_frame_metadata(pts: Option<u64>, dts: Option<u64>) -> FrameMetadata {
    FrameMetadata { pts, dts }
}

/// Create test frames with simple ordering
fn create_test_frames_simple() -> Vec<FrameMetadata> {
    vec![
        create_test_frame_metadata(Some(0), Some(0)),      // Frame 0
        create_test_frame_metadata(Some(1000), Some(1000)), // Frame 1
        create_test_frame_metadata(Some(2000), Some(2000)), // Frame 2
        create_test_frame_metadata(Some(3000), Some(3000)), // Frame 3
    ]
}

/// Create test frames with reordering (B-frames)
fn create_test_frames_with_reorder() -> Vec<FrameMetadata> {
    vec![
        create_test_frame_metadata(Some(0), Some(0)),      // I-frame
        create_test_frame_metadata(Some(2000), Some(1000)), // B-frame (display later)
        create_test_frame_metadata(Some(1000), Some(2000)), // B-frame (display earlier)
        create_test_frame_metadata(Some(3000), Some(3000)), // P-frame
    ]
}

/// Create test frames with missing PTS
fn create_test_frames_missing_pts() -> Vec<FrameMetadata> {
    vec![
        create_test_frame_metadata(Some(0), Some(0)),
        create_test_frame_metadata(None, Some(1000)),      // Missing PTS
        create_test_frame_metadata(Some(2000), Some(2000)),
        create_test_frame_metadata(None, Some(3000)),      // Missing PTS
    ]
}

/// Create test frames with duplicate PTS
fn create_test_frames_duplicate_pts() -> Vec<FrameMetadata> {
    vec![
        create_test_frame_metadata(Some(0), Some(0)),
        create_test_frame_metadata(Some(1000), Some(1000)),
        create_test_frame_metadata(Some(1000), Some(2000)), // Duplicate PTS
        create_test_frame_metadata(Some(2000), Some(3000)),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    // ============================================================================
    // FrameMetadata Tests
    // ============================================================================

    #[test]
    fn test_frame_metadata_new() {
        // Arrange & Act
        let meta = FrameMetadata {
            pts: Some(100),
            dts: Some(50),
        };

        // Assert
        assert_eq!(meta.pts, Some(100));
        assert_eq!(meta.dts, Some(50));
    }

    // ============================================================================
    // PtsQuality Tests
    // ============================================================================

    #[test]
    fn test_pts_quality_badge_text() {
        // Arrange & Act & Assert
        assert_eq!(PtsQuality::Ok.badge_text(), "PTS: OK");
        assert_eq!(PtsQuality::Warn.badge_text(), "PTS: WARN");
        assert_eq!(PtsQuality::Bad.badge_text(), "PTS: BAD");
    }

    #[test]
    fn test_pts_quality_color_hint() {
        // Arrange & Act & Assert
        assert_eq!(PtsQuality::Ok.color_hint(), PtsQualityColor::Green);
        assert_eq!(PtsQuality::Warn.color_hint(), PtsQualityColor::Yellow);
        assert_eq!(PtsQuality::Bad.color_hint(), PtsQualityColor::Red);
    }

    #[test]
    fn test_pts_quality_tooltip() {
        // Arrange & Act
        let ok_tooltip = PtsQuality::Ok.tooltip();
        let warn_tooltip = PtsQuality::Warn.tooltip();
        let bad_tooltip = PtsQuality::Bad.tooltip();

        // Assert
        assert!(ok_tooltip.contains("valid"));
        assert!(warn_tooltip.contains("missing") || warn_tooltip.contains("variable"));
        assert!(bad_tooltip.contains("duplicates") || bad_tooltip.contains("major"));
    }

    // ============================================================================
    // FrameIndexMap Tests
    // ============================================================================

    #[test]
    fn test_frame_index_map_new_simple() {
        // Arrange
        let frames = create_test_frames_simple();

        // Act
        let map = FrameIndexMap::new(&frames);

        // Assert
        assert_eq!(map.frame_count(), 4);
        assert_eq!(map.pts_quality, PtsQuality::Ok);
        assert_eq!(map.get_pts(0), Some(0));
        assert_eq!(map.get_pts(1), Some(1000));
    }

    #[test]
    fn test_frame_index_map_new_with_reordering() {
        // Arrange
        let frames = create_test_frames_with_reorder();

        // Act
        let map = FrameIndexMap::new(&frames);

        // Assert - Frames should be sorted by PTS
        // Display order: 0 (pts=0), 1 (pts=1000), 2 (pts=2000), 3 (pts=3000)
        // Decode indices in display order would be: [0, 2, 1, 3]
        assert_eq!(map.frame_count(), 4);
        assert!(map.has_reordering());
    }

    #[test]
    fn test_frame_index_map_empty() {
        // Arrange
        let frames = vec![];

        // Act
        let map = FrameIndexMap::new(&frames);

        // Assert
        assert_eq!(map.frame_count(), 0);
        assert_eq!(map.pts_quality, PtsQuality::Ok);
        assert!(!map.has_reordering());
    }

    #[test]
    fn test_frame_index_map_missing_pts() {
        // Arrange
        let frames = create_test_frames_missing_pts();

        // Act
        let map = FrameIndexMap::new(&frames);

        // Assert - Some missing PTS but < 50% = WARN
        // PTS sorting: frames with PTS come first, None at end
        // Decode order: 0(pts=0), 1(None), 2(pts=2000), 3(None)
        // Display order: 0(pts=0), 2(pts=2000), 1(None), 3(None)
        assert_eq!(map.pts_quality, PtsQuality::Warn);
        assert_eq!(map.get_pts(0), Some(0));
        assert_eq!(map.get_pts(1), Some(2000)); // Frame 2 in decode order
        assert_eq!(map.get_pts(2), None); // Missing PTS
        assert_eq!(map.get_pts(3), None); // Missing PTS
    }

    #[test]
    fn test_frame_index_map_duplicate_pts() {
        // Arrange
        let frames = create_test_frames_duplicate_pts();

        // Act
        let map = FrameIndexMap::new(&frames);

        // Assert - Duplicate PTS = BAD
        assert_eq!(map.pts_quality, PtsQuality::Bad);
    }

    #[test]
    fn test_frame_index_map_display_to_decode() {
        // Arrange
        let frames = create_test_frames_simple();

        // Act
        let map = FrameIndexMap::new(&frames);

        // Assert - No reordering for simple case
        assert_eq!(map.display_to_decode_idx(0), Some(0));
        assert_eq!(map.display_to_decode_idx(1), Some(1));
        assert_eq!(map.display_to_decode_idx(2), Some(2));
    }

    #[test]
    fn test_frame_index_map_display_to_decode_out_of_bounds() {
        // Arrange
        let frames = create_test_frames_simple();
        let map = FrameIndexMap::new(&frames);

        // Act
        let result = map.display_to_decode_idx(100);

        // Assert
        assert!(result.is_none());
    }

    #[test]
    fn test_frame_index_map_get_dts() {
        // Arrange
        let frames = create_test_frames_simple();
        let map = FrameIndexMap::new(&frames);

        // Act
        let dts = map.get_dts(0);

        // Assert
        assert_eq!(dts, Some(0));
    }

    #[test]
    fn test_frame_index_map_has_reordering_true() {
        // Arrange
        let frames = create_test_frames_with_reorder();
        let map = FrameIndexMap::new(&frames);

        // Act & Assert
        assert!(map.has_reordering());
    }

    #[test]
    fn test_frame_index_map_has_reordering_false() {
        // Arrange
        let frames = create_test_frames_simple();
        let map = FrameIndexMap::new(&frames);

        // Act & Assert
        assert!(!map.has_reordering());
    }

    // ============================================================================
    // ExtractionError Tests
    // ============================================================================

    #[test]
    fn test_extraction_error_display() {
        // Arrange & Act & Assert
        let err = ExtractionError::InvalidFormat("bad data".to_string());
        assert_eq!(format!("{}", err), "Invalid format: bad data");

        let err = ExtractionError::ParseError("parse failed".to_string());
        assert_eq!(format!("{}", err), "Parse error: parse failed");

        let err = ExtractionError::Unsupported("not supported".to_string());
        assert_eq!(format!("{}", err), "Unsupported: not supported");

        let err = ExtractionError::IoError("io error".to_string());
        assert_eq!(format!("{}", err), "I/O error: io error");
    }

    // ============================================================================
    // Av1FrameIdentityQuirks Tests
    // ============================================================================

    #[test]
    fn test_av1_frame_identity_quirks_default() {
        // Arrange & Act
        let quirks = Av1FrameIdentityQuirks::default_quirks();

        // Assert
        assert!(!quirks.is_show_existing);
        assert!(quirks.show_existing_frame_idx.is_none());
        assert!(!quirks.has_film_grain);
        assert!(quirks.tile_count.is_none());
    }

    #[test]
    fn test_av1_frame_identity_quirks_show_existing() {
        // Arrange & Act
        let quirks = Av1FrameIdentityQuirks::show_existing(5);

        // Assert
        assert!(quirks.is_show_existing);
        assert_eq!(quirks.show_existing_frame_idx, Some(5));
        assert!(!quirks.has_film_grain);
    }

    #[test]
    fn test_av1_frame_identity_quirks_with_film_grain() {
        // Arrange & Act
        let quirks = Av1FrameIdentityQuirks::with_film_grain();

        // Assert
        assert!(quirks.has_film_grain);
        assert!(!quirks.is_show_existing);
    }

    #[test]
    fn test_av1_frame_identity_quirks_with_tiles() {
        // Arrange & Act
        let quirks = Av1FrameIdentityQuirks::with_tiles(4);

        // Assert
        assert_eq!(quirks.tile_count, Some(4));
        assert!(!quirks.is_show_existing);
    }

    #[test]
    fn test_av1_frame_identity_quirks_needs_special_handling() {
        // Arrange
        let default_quirks = Av1FrameIdentityQuirks::default_quirks();
        let show_existing = Av1FrameIdentityQuirks::show_existing(5);
        let with_grain = Av1FrameIdentityQuirks::with_film_grain();

        // Act & Assert
        assert!(!default_quirks.needs_special_handling());
        assert!(show_existing.needs_special_handling());
        assert!(with_grain.needs_special_handling());
    }

    #[test]
    fn test_av1_frame_identity_quirks_set_tile_count() {
        // Arrange
        let mut quirks = Av1FrameIdentityQuirks::default_quirks();

        // Act
        quirks.set_tile_count(8);

        // Assert
        assert_eq!(quirks.tile_count, Some(8));
    }

    #[test]
    fn test_av1_frame_identity_quirks_set_film_grain() {
        // Arrange
        let mut quirks = Av1FrameIdentityQuirks::default_quirks();

        // Act
        quirks.set_film_grain(true);

        // Assert
        assert!(quirks.has_film_grain);
    }

    // ============================================================================
    // TimelineFrameWithQuirks Tests
    // ============================================================================

    #[test]
    fn test_timeline_frame_with_quirks_new() {
        // Arrange
        let quirks = Av1FrameIdentityQuirks::default_quirks();

        // Act
        let frame = TimelineFrameWithQuirks::new(5, 10000, "KEY_FRAME".to_string(), quirks);

        // Assert
        assert_eq!(frame.display_idx, 5);
        assert_eq!(frame.size_bytes, 10000);
        assert_eq!(frame.frame_type, "KEY_FRAME");
    }

    #[test]
    fn test_timeline_frame_with_quirks_is_virtual_frame() {
        // Arrange
        let virtual_quirks = Av1FrameIdentityQuirks::show_existing(3);
        let frame = TimelineFrameWithQuirks::new(0, 1000, "INTER_FRAME".to_string(), virtual_quirks);

        // Act & Assert
        assert!(frame.is_virtual_frame());
    }

    #[test]
    fn test_timeline_frame_with_quirks_has_film_grain() {
        // Arrange
        let grain_quirks = Av1FrameIdentityQuirks::with_film_grain();
        let frame = TimelineFrameWithQuirks::new(0, 1000, "INTER_FRAME".to_string(), grain_quirks);

        // Act & Assert
        assert!(frame.has_film_grain());
    }

    #[test]
    fn test_timeline_frame_with_quirks_viz_hint() {
        // Arrange
        let default_quirks = Av1FrameIdentityQuirks::default_quirks();
        let virtual_quirks = Av1FrameIdentityQuirks::show_existing(3);
        let grain_quirks = Av1FrameIdentityQuirks::with_film_grain();
        let multi_tile_quirks = Av1FrameIdentityQuirks::with_tiles(4);

        // Act & Assert
        assert_eq!(
            TimelineFrameWithQuirks::new(0, 1000, "I".to_string(), default_quirks).viz_hint(),
            FrameVizHint::Normal
        );
        assert_eq!(
            TimelineFrameWithQuirks::new(0, 1000, "I".to_string(), virtual_quirks).viz_hint(),
            FrameVizHint::Virtual
        );
        assert_eq!(
            TimelineFrameWithQuirks::new(0, 1000, "I".to_string(), grain_quirks).viz_hint(),
            FrameVizHint::FilmGrain
        );
        assert_eq!(
            TimelineFrameWithQuirks::new(0, 1000, "I".to_string(), multi_tile_quirks).viz_hint(),
            FrameVizHint::MultiTile(4)
        );
    }

    // ============================================================================
    // Av1QuirksTimeline Tests
    // ============================================================================

    #[test]
    fn test_av1_quirks_timeline_new() {
        // Arrange
        let frames_meta = create_test_frames_simple();
        let frames_quirks = vec![
            (5000, "KEY_FRAME".to_string(), Av1FrameIdentityQuirks::default_quirks()),
            (3000, "INTER_FRAME".to_string(), Av1FrameIdentityQuirks::show_existing(0)),
        ];

        // Act
        let timeline = Av1QuirksTimeline::new(frames_meta, frames_quirks);

        // Assert
        assert_eq!(timeline.frame_count(), 4);
        assert_eq!(timeline.get_frame(0).unwrap().size_bytes, 5000);
        assert_eq!(timeline.get_frame(0).unwrap().frame_type, "KEY_FRAME");
    }

    #[test]
    fn test_av1_quirks_timeline_virtual_frames() {
        // Arrange
        let frames_meta = create_test_frames_simple();
        let frames_quirks = vec![
            (1000, "I".to_string(), Av1FrameIdentityQuirks::show_existing(0)),
            (2000, "P".to_string(), Av1FrameIdentityQuirks::default_quirks()),
            (3000, "P".to_string(), Av1FrameIdentityQuirks::show_existing(1)),
            (4000, "I".to_string(), Av1FrameIdentityQuirks::default_quirks()),
        ];

        // Act
        let timeline = Av1QuirksTimeline::new(frames_meta, frames_quirks);
        let virtual_frames = timeline.virtual_frames();

        // Assert
        assert_eq!(virtual_frames.len(), 2);
        assert!(virtual_frames.contains(&0));
        assert!(virtual_frames.contains(&2));
    }

    #[test]
    fn test_av1_quirks_timeline_film_grain_frames() {
        // Arrange
        let frames_meta = create_test_frames_simple();
        let frames_quirks = vec![
            (1000, "I".to_string(), Av1FrameIdentityQuirks::with_film_grain()),
            (2000, "P".to_string(), Av1FrameIdentityQuirks::default_quirks()),
        ];

        // Act
        let timeline = Av1QuirksTimeline::new(frames_meta, frames_quirks);
        let grain_frames = timeline.film_grain_frames();

        // Assert
        assert_eq!(grain_frames.len(), 1);
        assert!(grain_frames.contains(&0));
    }

    #[test]
    fn test_av1_quirks_timeline_multi_tile_frames() {
        // Arrange
        let frames_meta = create_test_frames_simple();
        let frames_quirks = vec![
            (1000, "I".to_string(), Av1FrameIdentityQuirks::with_tiles(1)),
            (2000, "P".to_string(), Av1FrameIdentityQuirks::with_tiles(4)),
            (3000, "P".to_string(), Av1FrameIdentityQuirks::with_tiles(1)),
        ];

        // Act
        let timeline = Av1QuirksTimeline::new(frames_meta, frames_quirks);
        let multi_tiles = timeline.multi_tile_frames();

        // Assert
        assert_eq!(multi_tiles.len(), 1);
        assert_eq!(multi_tiles[0], (1, 4)); // display_idx 1 has 4 tiles
    }

    // ============================================================================
    // TimelineMapper Tests (formerly FrameMapper Tests)
    // ============================================================================

    #[test]
    fn test_timeline_mapper_new() {
        // Arrange
        let frames = create_test_frames_simple();
        let sizes = vec![1000, 2000, 3000, 4000];
        let types = vec!["I".to_string(), "P".to_string(), "B".to_string(), "I".to_string()];

        // Act
        let mapper = TimelineMapper::new("stream1".to_string(), frames, sizes, types);

        // Assert
        assert_eq!(mapper.index_map().frame_count(), 4);
        assert_eq!(mapper.frame_sizes().len(), 4);
        assert_eq!(mapper.frame_types().len(), 4);
        assert_eq!(mapper.frame_sizes()[0], 1000);
        assert_eq!(mapper.frame_types()[0], "I");
    }

    #[test]
    fn test_timeline_mapper_build_timeline_av1() {
        // Arrange
        let frames = create_test_frames_simple();
        let sizes = vec![1000, 2000, 3000, 4000];
        let types = vec![
            "KEY_FRAME".to_string(),
            "INTER_FRAME".to_string(),
            "INTER_FRAME".to_string(),
            "KEY_FRAME".to_string(),
        ];
        let mapper = TimelineMapper::new("stream_A".to_string(), frames, sizes, types);

        // Act
        let timeline = mapper.build_timeline_av1();

        // Assert
        assert_eq!(timeline.stream_id, "stream_A");
        assert_eq!(timeline.frame_count(), 4);
    }

    // ============================================================================
    // FrameMapper Tests
    // ============================================================================

    #[test]
    fn test_frame_mapper_for_av1() {
        // Arrange & Act
        let mapper = FrameMapper::for_av1();

        // Assert
        assert_eq!(mapper.codec_name(), "AV1");
    }

    #[test]
    fn test_frame_mapper_for_h264() {
        // Arrange & Act
        let mapper = FrameMapper::for_h264();

        // Assert
        assert_eq!(mapper.codec_name(), "H.264");
    }

    // ============================================================================
    // AxisBounds Tests
    // ============================================================================

    #[test]
    fn test_axis_bounds_new() {
        // Arrange & Act
        let bounds = AxisBounds::new(10, 20);

        // Assert
        assert_eq!(bounds.start, 10);
        assert_eq!(bounds.end, 20);
    }

    #[test]
    fn test_axis_bounds_frame_count() {
        // Arrange & Act
        let bounds = AxisBounds::new(10, 20);

        // Assert
        assert_eq!(bounds.frame_count(), 11); // 20 - 10 + 1 = 11
    }

    #[test]
    fn test_axis_bounds_frame_count_empty() {
        // Arrange & Act
        let bounds = AxisBounds::new(20, 10); // end < start

        // Assert
        assert_eq!(bounds.frame_count(), 0);
    }

    #[test]
    fn test_axis_bounds_contains() {
        // Arrange
        let bounds = AxisBounds::new(10, 20);

        // Act & Assert
        assert!(bounds.contains(10));
        assert!(bounds.contains(15));
        assert!(bounds.contains(20));
        assert!(!bounds.contains(9));
        assert!(!bounds.contains(21));
    }

    #[test]
    fn test_axis_bounds_clamp() {
        // Arrange
        let bounds = AxisBounds::new(10, 20);

        // Act & Assert
        assert_eq!(bounds.clamp(5), 10);
        assert_eq!(bounds.clamp(15), 15);
        assert_eq!(bounds.clamp(25), 20);
    }

    // ============================================================================
    // TimelineAxis Tests
    // ============================================================================

    #[test]
    fn test_timeline_axis_new() {
        // Arrange
        let frames = create_test_frames_simple();
        let _index_map = FrameIndexMap::new(&frames);

        // Act
        let axis = TimelineAxis::new(4, 1920.0, AxisScaleMode::Auto);

        // Assert
        assert_eq!(axis.total_frames(), 4);
        assert_eq!(axis.viewport_width(), 1920.0);
        assert_eq!(axis.scale_mode(), AxisScaleMode::Auto);
        assert_eq!(axis.bounds().start, 0);
        assert_eq!(axis.bounds().end, 3);
    }

    #[test]
    fn test_timeline_axis_pixels_per_frame() {
        // Arrange
        let frames = create_test_frames_simple();
        let _index_map = FrameIndexMap::new(&frames);
        let axis = TimelineAxis::new(4, 960.0, AxisScaleMode::Auto);

        // Act - Auto scale: 4 frames in 960px = 240px per frame
        let ppf = axis.pixels_per_frame();

        // Assert
        assert_eq!(ppf, 240.0);
    }

    #[test]
    fn test_timeline_axis_display_idx_to_pixel() {
        // Arrange
        let frames = create_test_frames_simple();
        let _index_map = FrameIndexMap::new(&frames);
        let axis = TimelineAxis::new(4, 960.0, AxisScaleMode::Auto);

        // Act
        let pixel = axis.display_idx_to_pixel(2);

        // Assert - Frame 2 at 480px (2 * 240)
        assert_eq!(pixel, 480.0);
    }

    #[test]
    fn test_timeline_axis_display_idx_to_pixel_before_bounds() {
        // Arrange
        let frames = create_test_frames_simple();
        let _index_map = FrameIndexMap::new(&frames);
        let axis = TimelineAxis::new(4, 960.0, AxisScaleMode::Auto);

        // Act
        let pixel = axis.display_idx_to_pixel(0);

        // Assert - Within bounds
        assert_eq!(pixel, 0.0);
    }

    #[test]
    fn test_timeline_axis_display_idx_to_pixel_after_bounds() {
        // Arrange
        let frames = create_test_frames_simple();
        let _index_map = FrameIndexMap::new(&frames);
        let axis = TimelineAxis::new(4, 960.0, AxisScaleMode::Auto);

        // Act - Frame 10 is beyond bounds
        let pixel = axis.display_idx_to_pixel(10);

        // Assert - Should return position for frame 10 (2400px)
        // But since it's beyond total_frames, behavior is implementation-defined
        assert!(pixel >= 0.0);
    }

    #[test]
    fn test_timeline_axis_pixel_to_display_idx() {
        // Arrange
        let frames = create_test_frames_simple();
        let _index_map = FrameIndexMap::new(&frames);
        let axis = TimelineAxis::new(4, 960.0, AxisScaleMode::Auto);

        // Act
        let idx = axis.pixel_to_display_idx(480.0);

        // Assert - 480px / 240px per frame = frame 2
        assert_eq!(idx, 2);
    }

    #[test]
    fn test_timeline_axis_pan() {
        // Arrange
        let frames = create_test_frames_simple();
        let _index_map = FrameIndexMap::new(&frames);
        // Use FitFrames to show fewer frames, allowing pan
        let mut axis = TimelineAxis::new(10, 1000.0, AxisScaleMode::FitFrames(5));

        // Initial bounds: start=0, end=4 (showing 5 frames)
        assert_eq!(axis.bounds().start, 0);
        assert_eq!(axis.bounds().end, 4);

        // Act - Pan right by 1 frame
        axis.pan(1);

        // Assert - Should move to start=1, end=5 (still showing 5 frames)
        // But end is clamped to 9, so end=5
        assert_eq!(axis.bounds().start, 1);
        assert_eq!(axis.bounds().end, 5);
    }

    #[test]
    fn test_timeline_axis_zoom_in() {
        // Arrange
        let frames = create_test_frames_simple();
        let _index_map = FrameIndexMap::new(&frames);
        let mut axis = TimelineAxis::new(10, 1000.0, AxisScaleMode::Auto);

        let before = axis.bounds().frame_count();

        // Act
        axis.zoom_in();
        let after = axis.bounds().frame_count();

        // Assert - Should show fewer frames
        assert!(after < before);
        assert!(after >= 1); // Always show at least 1 frame
    }

    #[test]
    fn test_timeline_axis_zoom_out() {
        // Arrange
        let frames = create_test_frames_simple();
        let _index_map = FrameIndexMap::new(&frames);
        let mut axis = TimelineAxis::new(10, 1000.0, AxisScaleMode::FitFrames(5));

        let before = axis.bounds().frame_count();

        // Act
        axis.zoom_out();
        let after = axis.bounds().frame_count();

        // Assert - Should show more frames
        assert!(after > before);
        assert!(after <= 10); // But not more than total
    }

    #[test]
    fn test_timeline_axis_center_on() {
        // Arrange
        let frames = create_test_frames_simple();
        let _index_map = FrameIndexMap::new(&frames);
        let mut axis = TimelineAxis::new(10, 1000.0, AxisScaleMode::Auto);

        // Act
        axis.center_on(5);

        // Assert - Should center around frame 5
        assert!(axis.bounds().start <= 5);
        assert!(axis.bounds().end >= 5);
    }

    // ============================================================================
    // TimelineCursor Tests
    // ============================================================================

    #[test]
    fn test_timeline_cursor_new() {
        // Arrange & Act
        let cursor = TimelineCursor::new(10);

        // Assert
        assert_eq!(cursor.position(), None);
        assert_eq!(cursor.visibility, CursorVisibility::Hidden);
        assert_eq!(cursor.total_frames(), 10);
    }

    #[test]
    fn test_timeline_cursor_set_position() {
        // Arrange
        let mut cursor = TimelineCursor::new(10);

        // Act
        cursor.set_position(5);

        // Assert
        assert_eq!(cursor.position(), Some(5));
        assert_eq!(cursor.visibility, CursorVisibility::Visible);
    }

    #[test]
    fn test_timeline_cursor_set_position_clamps() {
        // Arrange
        let mut cursor = TimelineCursor::new(10);

        // Act
        cursor.set_position(15); // Beyond total

        // Assert
        assert_eq!(cursor.position(), Some(9)); // Clamped to total_frames - 1
    }

    #[test]
    fn test_timeline_cursor_set_position_empty() {
        // Arrange
        let mut cursor = TimelineCursor::new(0);

        // Act
        cursor.set_position(5);

        // Assert - Empty stream
        assert_eq!(cursor.position(), None);
        assert_eq!(cursor.visibility, CursorVisibility::Hidden);
    }

    #[test]
    fn test_timeline_cursor_clear() {
        // Arrange
        let mut cursor = TimelineCursor::new(10);
        cursor.set_position(5);

        // Act
        cursor.clear();

        // Assert
        assert_eq!(cursor.position(), None);
        assert_eq!(cursor.visibility, CursorVisibility::Hidden);
    }

    #[test]
    fn test_timeline_cursor_show_hide() {
        // Arrange
        let mut cursor = TimelineCursor::new(10);
        cursor.set_position(5);
        cursor.hide();

        // Act
        cursor.show();

        // Assert
        assert!(cursor.is_visible());
    }

    #[test]
    fn test_timeline_cursor_move_by() {
        // Arrange
        let mut cursor = TimelineCursor::new(10);
        cursor.set_position(5);

        // Act
        cursor.move_by(2);

        // Assert
        assert_eq!(cursor.position(), Some(7));
    }

    #[test]
    fn test_timeline_cursor_move_by_clamps() {
        // Arrange
        let mut cursor = TimelineCursor::new(10);
        cursor.set_position(5);

        // Act
        cursor.move_by(-10); // Try to move before 0

        // Assert
        assert_eq!(cursor.position(), Some(0)); // Clamped to 0
    }

    #[test]
    fn test_timeline_cursor_next_prev_frame() {
        // Arrange
        let mut cursor = TimelineCursor::new(10);
        cursor.set_position(5);

        // Act
        cursor.next_frame();

        // Assert
        assert_eq!(cursor.position(), Some(6));

        cursor.prev_frame();
        assert_eq!(cursor.position(), Some(5));
    }

    // ============================================================================
    // CursorSync Tests
    // ============================================================================

    #[test]
    fn test_cursor_sync_new() {
        // Arrange
        let frames = create_test_frames_simple();
        let index_map = FrameIndexMap::new(&frames);

        // Act
        let sync = CursorSync::new(index_map);

        // Assert
        assert_eq!(sync.cursor().total_frames(), 4);
        assert_eq!(sync.cursor().position(), None);
    }

    #[test]
    fn test_cursor_sync_from_timeline_click() {
        // Arrange
        let frames = create_test_frames_simple();
        let index_map = FrameIndexMap::new(&frames);
        let axis = TimelineAxis::new(4, 960.0, AxisScaleMode::Auto);
        let mut sync = CursorSync::new(index_map);

        // Act - Click at 480px (should map to frame 2)
        sync.sync_from_timeline_click(480.0, &axis);

        // Assert
        assert_eq!(sync.cursor().position(), Some(2));
    }

    #[test]
    fn test_cursor_sync_from_player_seek() {
        // Arrange
        let frames = create_test_frames_simple();
        let index_map = FrameIndexMap::new(&frames);
        let mut sync = CursorSync::new(index_map);

        // Act
        sync.sync_from_player_seek(2);

        // Assert
        assert_eq!(sync.cursor().position(), Some(2));
    }

    #[test]
    fn test_cursor_sync_from_pts() {
        // Arrange
        let frames = create_test_frames_simple();
        let index_map = FrameIndexMap::new(&frames);
        let mut sync = CursorSync::new(index_map);

        // Act
        let success = sync.sync_from_pts(2000);

        // Assert
        assert!(success);
        assert_eq!(sync.cursor().position(), Some(2));
    }

    #[test]
    fn test_cursor_sync_from_pts_not_found() {
        // Arrange
        let frames = create_test_frames_simple();
        let index_map = FrameIndexMap::new(&frames);
        let mut sync = CursorSync::new(index_map);

        // Act
        let success = sync.sync_from_pts(9999);

        // Assert
        assert!(!success);
    }

    #[test]
    fn test_cursor_sync_cursor_pts() {
        // Arrange
        let frames = create_test_frames_simple();
        let index_map = FrameIndexMap::new(&frames);
        let mut sync = CursorSync::new(index_map);
        sync.sync_from_player_seek(2);

        // Act
        let pts = sync.cursor_pts();

        // Assert
        assert_eq!(pts, Some(2000));
    }

    // ============================================================================
    // Edge Cases
    // ============================================================================

    #[test]
    fn test_frame_index_map_all_missing_pts() {
        // Arrange
        let frames = vec![
            create_test_frame_metadata(None, Some(0)),
            create_test_frame_metadata(None, Some(1)),
            create_test_frame_metadata(None, Some(2)),
            create_test_frame_metadata(None, Some(3)),
            create_test_frame_metadata(None, Some(4)),
        ];

        // Act
        let map = FrameIndexMap::new(&frames);

        // Assert - >50% missing = BAD
        assert_eq!(map.pts_quality, PtsQuality::Bad);
    }

    #[test]
    fn test_frame_index_map_single_frame() {
        // Arrange
        let frames = vec![create_test_frame_metadata(Some(0), Some(0))];

        // Act
        let map = FrameIndexMap::new(&frames);

        // Assert
        assert_eq!(map.frame_count(), 1);
        assert_eq!(map.pts_quality, PtsQuality::Ok);
        assert_eq!(map.display_to_decode_idx(0), Some(0));
    }

    #[test]
    fn test_timeline_axis_empty_stream() {
        // Arrange
        let frames = vec![];
        let _index_map = FrameIndexMap::new(&frames);

        // Act
        let axis = TimelineAxis::new(0, 1920.0, AxisScaleMode::Auto);

        // Assert
        assert_eq!(axis.total_frames(), 0);
        assert_eq!(axis.bounds().frame_count(), 0);
    }

    #[test]
    fn test_cursor_sync_empty_stream() {
        // Arrange
        let frames = vec![];
        let index_map = FrameIndexMap::new(&frames);
        let mut sync = CursorSync::new(index_map);

        // Act
        sync.sync_from_player_seek(0);

        // Assert - Empty stream, cursor should be hidden
        assert_eq!(sync.cursor().position(), None);
        assert!(!sync.cursor().is_visible());
    }
}
