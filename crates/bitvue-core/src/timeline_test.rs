// Timeline module tests
//
// Per generate-tests skill: Arrange-Act-Assert pattern with fixtures
// and edge case coverage.

use super::*;

/// Create test timeline with sample frames
fn create_test_timeline(frame_count: usize) -> TimelineBase {
    let mut timeline = TimelineBase::new("test_stream".to_string());

    for i in 0..frame_count {
        let marker = match i {
            0 => FrameMarker::Key,        // First frame is keyframe
            10 => FrameMarker::Key,       // Every 10th frame is keyframe
            20 => FrameMarker::Key,
            5 => FrameMarker::Error,      // Frame 5 has error
            15 => FrameMarker::Bookmark,  // Frame 15 has bookmark
            25 => FrameMarker::SceneChange,
            _ => FrameMarker::None,
        };

        let frame_type = match i % 3 {
            0 => "I",
            1 => "P",
            _ => "B",
        };

        let frame = TimelineFrame::new(i, 1000 + i as u64, frame_type.to_string())
            .with_marker(marker)
            .with_pts(i as u64)
            .with_dts(i as u64);

        timeline.add_frame(frame);
    }

    timeline
}

#[cfg(test)]
mod tests {
    use super::*;

    // ============================================================================
    // FrameMarker Tests
    // ============================================================================

    #[test]
    fn test_frame_marker_is_critical() {
        // Arrange
        let key = FrameMarker::Key;
        let error = FrameMarker::Error;
        let bookmark = FrameMarker::Bookmark;
        let none = FrameMarker::None;

        // Act & Assert
        assert!(key.is_critical());
        assert!(error.is_critical());
        assert!(!bookmark.is_critical());
        assert!(!none.is_critical());
    }

    #[test]
    fn test_frame_marker_color_hint() {
        // Arrange & Act & Assert
        assert_eq!(FrameMarker::Key.color_hint(), "blue");
        assert_eq!(FrameMarker::Error.color_hint(), "red");
        assert_eq!(FrameMarker::Bookmark.color_hint(), "yellow");
        assert_eq!(FrameMarker::SceneChange.color_hint(), "green");
        assert_eq!(FrameMarker::None.color_hint(), "transparent");
    }

    // ============================================================================
    // TimelineFrame Tests
    // ============================================================================

    #[test]
    fn test_timeline_frame_new() {
        // Arrange & Act
        let frame = TimelineFrame::new(10, 5000, "I".to_string());

        // Assert
        assert_eq!(frame.display_idx, 10);
        assert_eq!(frame.size_bytes, 5000);
        assert_eq!(frame.frame_type, "I");
        assert_eq!(frame.marker, FrameMarker::None);
        assert_eq!(frame.pts, None);
        assert_eq!(frame.dts, None);
        assert!(!frame.is_selected);
    }

    #[test]
    fn test_timeline_frame_builder_pattern() {
        // Arrange & Act
        let frame = TimelineFrame::new(5, 2000, "P".to_string())
            .with_marker(FrameMarker::Error)
            .with_pts(100)
            .with_dts(99);

        // Assert
        assert_eq!(frame.display_idx, 5);
        assert_eq!(frame.marker, FrameMarker::Error);
        assert_eq!(frame.pts, Some(100));
        assert_eq!(frame.dts, Some(99));
    }

    #[test]
    fn test_timeline_frame_has_reorder() {
        // Arrange
        let frame_no_reorder = TimelineFrame::new(0, 1000, "I".to_string())
            .with_pts(100)
            .with_dts(100);

        let frame_reorder = TimelineFrame::new(1, 1000, "P".to_string())
            .with_pts(101)
            .with_dts(99);

        let frame_no_timestamp = TimelineFrame::new(2, 1000, "B".to_string());

        // Act & Assert
        assert!(!frame_no_reorder.has_reorder());
        assert!(frame_reorder.has_reorder());
        assert!(!frame_no_timestamp.has_reorder());
    }

    // ============================================================================
    // TimelineBase Tests - Basic Operations
    // ============================================================================

    #[test]
    fn test_timeline_new() {
        // Arrange & Act
        let timeline = TimelineBase::new("stream_1".to_string());

        // Assert
        assert_eq!(timeline.stream_id, "stream_1");
        assert_eq!(timeline.frame_count(), 0);
        assert_eq!(timeline.current_frame, None);
        assert_eq!(timeline.scrub_mode, ScrubMode::Idle);
    }

    #[test]
    fn test_timeline_add_and_count() {
        // Arrange
        let mut timeline = TimelineBase::new("test".to_string());

        // Act
        timeline.add_frame(TimelineFrame::new(0, 1000, "I".to_string()));
        timeline.add_frame(TimelineFrame::new(1, 1500, "P".to_string()));

        // Assert
        assert_eq!(timeline.frame_count(), 2);
    }

    #[test]
    fn test_timeline_get_frame() {
        // Arrange
        let timeline = create_test_timeline(30);

        // Act
        let frame = timeline.get_frame(10);

        // Assert
        assert!(frame.is_some());
        assert_eq!(frame.unwrap().display_idx, 10);
    }

    #[test]
    fn test_timeline_get_frame_out_of_bounds() {
        // Arrange
        let timeline = create_test_timeline(10);

        // Act
        let frame = timeline.get_frame(100);

        // Assert
        assert!(frame.is_none());
    }

    // ============================================================================
    // TimelineBase Tests - Frame Selection
    // ============================================================================

    #[test]
    fn test_timeline_select_frame() {
        // Arrange
        let mut timeline = create_test_timeline(30);

        // Act
        let success = timeline.select_frame(5);

        // Assert
        assert!(success);
        assert_eq!(timeline.current_frame_idx(), Some(5));
        assert!(timeline.get_frame(5).unwrap().is_selected);
    }

    #[test]
    fn test_timeline_select_frame_out_of_bounds() {
        // Arrange
        let mut timeline = create_test_timeline(30);

        // Act
        let success = timeline.select_frame(100);

        // Assert
        assert!(!success);
        assert_eq!(timeline.current_frame_idx(), None);
    }

    #[test]
    fn test_timeline_select_frame_updates_previous() {
        // Arrange
        let mut timeline = create_test_timeline(30);
        timeline.select_frame(5);

        // Act
        timeline.select_frame(10);

        // Assert
        assert_eq!(timeline.current_frame_idx(), Some(10));
        assert!(timeline.get_frame(10).unwrap().is_selected);
        assert!(!timeline.get_frame(5).unwrap().is_selected);
    }

    #[test]
    fn test_timeline_clear_selection() {
        // Arrange
        let mut timeline = create_test_timeline(30);
        timeline.select_frame(5);

        // Act
        timeline.clear_selection();

        // Assert
        assert_eq!(timeline.current_frame_idx(), None);
        assert!(!timeline.get_frame(5).unwrap().is_selected);
    }

    // ============================================================================
    // TimelineBase Tests - Jump Navigation
    // ============================================================================

    #[test]
    fn test_timeline_jump_next() {
        // Arrange
        let mut timeline = create_test_timeline(30);
        timeline.select_frame(10);

        // Act
        let result = timeline.jump(JumpDirection::Next);

        // Assert
        assert!(result.success);
        assert_eq!(result.target_idx, 11);
        assert_eq!(timeline.current_frame_idx(), Some(11));
    }

    #[test]
    fn test_timeline_jump_next_at_end() {
        // Arrange
        let mut timeline = create_test_timeline(30);
        timeline.select_frame(29);

        // Act
        let result = timeline.jump(JumpDirection::Next);

        // Assert
        assert!(!result.success);
        assert_eq!(result.target_idx, 29);
    }

    #[test]
    fn test_timeline_jump_prev() {
        // Arrange
        let mut timeline = create_test_timeline(30);
        timeline.select_frame(10);

        // Act
        let result = timeline.jump(JumpDirection::Prev);

        // Assert
        assert!(result.success);
        assert_eq!(result.target_idx, 9);
    }

    #[test]
    fn test_timeline_jump_prev_at_start() {
        // Arrange
        let mut timeline = create_test_timeline(30);
        timeline.select_frame(0);

        // Act
        let result = timeline.jump(JumpDirection::Prev);

        // Assert
        assert!(!result.success);
        assert_eq!(result.target_idx, 0);
    }

    #[test]
    fn test_timeline_jump_first() {
        // Arrange
        let mut timeline = create_test_timeline(30);
        timeline.select_frame(15);

        // Act
        let result = timeline.jump(JumpDirection::First);

        // Assert
        assert!(result.success);
        assert_eq!(result.target_idx, 0);
    }

    #[test]
    fn test_timeline_jump_last() {
        // Arrange
        let mut timeline = create_test_timeline(30);
        timeline.select_frame(0);

        // Act
        let result = timeline.jump(JumpDirection::Last);

        // Assert
        assert!(result.success);
        assert_eq!(result.target_idx, 29);
    }

    #[test]
    fn test_timeline_jump_next_keyframe() {
        // Arrange
        let mut timeline = create_test_timeline(30);
        timeline.select_frame(5);

        // Act
        let result = timeline.jump(JumpDirection::NextKey);

        // Assert
        assert!(result.success);
        assert_eq!(result.target_idx, 10);
    }

    #[test]
    fn test_timeline_jump_prev_keyframe() {
        // Arrange
        let mut timeline = create_test_timeline(30);
        timeline.select_frame(15);

        // Act
        let result = timeline.jump(JumpDirection::PrevKey);

        // Assert
        assert!(result.success);
        assert_eq!(result.target_idx, 10);
    }

    #[test]
    fn test_timeline_jump_next_marker() {
        // Arrange
        let mut timeline = create_test_timeline(30);
        timeline.select_frame(5);

        // Act - Next marker is error at 5 (skipped), then keyframe at 10
        let result = timeline.jump(JumpDirection::NextMarker);

        // Assert
        assert!(result.success);
        assert_eq!(result.target_idx, 10);
    }

    #[test]
    fn test_timeline_jump_prev_marker() {
        // Arrange
        let mut timeline = create_test_timeline(30);
        timeline.select_frame(20);

        // Act - Prev marker from 20 is bookmark at 15
        let result = timeline.jump(JumpDirection::PrevMarker);

        // Assert
        assert!(result.success);
        assert_eq!(result.target_idx, 15);
    }

    // ============================================================================
    // TimelineBase Tests - Viewport
    // ============================================================================

    #[test]
    fn test_timeline_set_viewport() {
        // Arrange
        let mut timeline = create_test_timeline(100);

        // Act
        timeline.set_viewport(50, 20);

        // Assert
        assert_eq!(timeline.visible_range(), (50, 20));
    }

    #[test]
    fn test_timeline_pan_x_positive() {
        // Arrange
        let mut timeline = create_test_timeline(100);
        timeline.set_viewport(10, 20);

        // Act
        timeline.pan_x(5);

        // Assert
        assert_eq!(timeline.visible_range(), (15, 20));
    }

    #[test]
    fn test_timeline_pan_x_negative() {
        // Arrange
        let mut timeline = create_test_timeline(100);
        timeline.set_viewport(20, 20);

        // Act
        timeline.pan_x(-5);

        // Assert
        assert_eq!(timeline.visible_range(), (15, 20));
    }

    #[test]
    fn test_timeline_pan_x_saturating() {
        // Arrange
        let mut timeline = create_test_timeline(100);
        timeline.set_viewport(5, 20);

        // Act
        timeline.pan_x(-10); // Try to pan before 0

        // Assert
        assert_eq!(timeline.visible_range(), (0, 20));
    }

    #[test]
    fn test_timeline_zoom_in() {
        // Arrange
        let mut timeline = create_test_timeline(100);
        timeline.set_viewport(0, 100);

        // Act - Zoom in by 2x (fewer frames visible)
        timeline.zoom(0.5, None);

        // Assert - Zoom centers on middle of viewport (frame 50)
        // new_count = 100 * 0.5 = 50
        // new_first = 50 - (50 * 0.5) = 25
        let (first, count) = timeline.visible_range();
        assert_eq!(count, 50);
        assert_eq!(first, 25);
    }

    #[test]
    fn test_timeline_zoom_out() {
        // Arrange
        let mut timeline = create_test_timeline(100);
        timeline.set_viewport(0, 50);

        // Act - Zoom out by 2x (more frames visible)
        timeline.zoom(2.0, None);

        // Assert
        assert_eq!(timeline.visible_range().1, 100);
    }

    #[test]
    fn test_timeline_zoom_with_focal_point() {
        // Arrange
        let mut timeline = create_test_timeline(100);
        timeline.set_viewport(0, 100);

        // Act - Zoom with focal point at frame 75
        timeline.zoom(0.5, Some(75));

        // Assert - Should keep frame 75 in same relative position
        let (first, count) = timeline.visible_range();
        assert_eq!(count, 50);
        // Frame 75 should be at ~75% of viewport
        assert!(first <= 75 && first + count > 75);
    }

    #[test]
    fn test_timeline_is_frame_visible() {
        // Arrange
        let mut timeline = create_test_timeline(100);
        timeline.set_viewport(20, 10);

        // Assert
        assert!(!timeline.is_frame_visible(19)); // Before viewport
        assert!(timeline.is_frame_visible(20));  // First visible
        assert!(timeline.is_frame_visible(25));  // Middle
        assert!(timeline.is_frame_visible(29));  // Last visible
        assert!(!timeline.is_frame_visible(30)); // After viewport
    }

    // ============================================================================
    // TimelineBase Tests - Index Collections
    // ============================================================================

    #[test]
    fn test_timeline_keyframe_indices() {
        // Arrange
        let timeline = create_test_timeline(30);

        // Act
        let keyframes = timeline.keyframe_indices();

        // Assert - Frames 0, 10, 20 are keyframes
        assert_eq!(keyframes, vec![0, 10, 20]);
    }

    #[test]
    fn test_timeline_error_indices() {
        // Arrange
        let timeline = create_test_timeline(30);

        // Act
        let errors = timeline.error_indices();

        // Assert - Frame 5 is error
        assert_eq!(errors, vec![5]);
    }

    #[test]
    fn test_timeline_bookmark_indices() {
        // Arrange
        let timeline = create_test_timeline(30);

        // Act
        let bookmarks = timeline.bookmark_indices();

        // Assert - Frame 15 is bookmark
        assert_eq!(bookmarks, vec![15]);
    }

    // ============================================================================
    // TimelineBase Tests - Scrub Mode
    // ============================================================================

    #[test]
    fn test_timeline_set_scrub_mode() {
        // Arrange
        let mut timeline = create_test_timeline(30);

        // Act
        timeline.set_scrub_mode(ScrubMode::Active);

        // Assert
        assert!(timeline.is_scrubbing());
    }

    #[test]
    fn test_timeline_is_scrubbing() {
        // Arrange
        let mut timeline = create_test_timeline(30);

        // Act & Assert
        assert!(!timeline.is_scrubbing());
        timeline.set_scrub_mode(ScrubMode::Active);
        assert!(timeline.is_scrubbing());
        timeline.set_scrub_mode(ScrubMode::Idle);
        assert!(!timeline.is_scrubbing());
    }

    // ============================================================================
    // Edge Cases
    // ============================================================================

    #[test]
    fn test_empty_timeline() {
        // Arrange
        let mut timeline = TimelineBase::new("empty".to_string());

        // Assert
        assert_eq!(timeline.frame_count(), 0);
        assert!(!timeline.select_frame(0));
        assert!(timeline.get_frame(0).is_none());
        assert_eq!(timeline.keyframe_indices(), Vec::<usize>::new());
        assert_eq!(timeline.error_indices(), Vec::<usize>::new());
    }

    #[test]
    fn test_single_frame_timeline() {
        // Arrange
        let mut timeline = TimelineBase::new("single".to_string());
        timeline.add_frame(TimelineFrame::new(0, 1000, "I".to_string()));

        // Act & Assert
        assert!(timeline.select_frame(0));
        assert!(!timeline.select_frame(1));
        let result = timeline.jump(JumpDirection::Next);
        assert!(!result.success);
        assert_eq!(result.target_idx, 0);
    }

    #[test]
    fn test_jump_without_selection() {
        // Arrange
        let mut timeline = create_test_timeline(30);
        // No frame selected, current_frame is None

        // Act - jump defaults to 0
        let result = timeline.jump(JumpDirection::Next);

        // Assert
        assert!(result.success);
        assert_eq!(result.target_idx, 1); // From 0 to 1
    }
}
