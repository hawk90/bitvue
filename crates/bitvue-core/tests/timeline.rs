#![allow(dead_code)]
//! Tests for timeline module

use bitvue_core::timeline::{
    FrameMarker, JumpDirection, JumpResult, ScrubMode, TimelineBase, TimelineFrame,
};

fn create_test_timeline() -> TimelineBase {
    let mut timeline = TimelineBase::new("A".to_string());

    // Add frames with various markers
    timeline.add_frame(TimelineFrame::new(0, 1000, "I".to_string()).with_marker(FrameMarker::Key));
    timeline.add_frame(TimelineFrame::new(1, 500, "P".to_string()));
    timeline.add_frame(TimelineFrame::new(2, 500, "P".to_string()));
    timeline.add_frame(TimelineFrame::new(3, 600, "P".to_string()).with_marker(FrameMarker::Error));
    timeline.add_frame(TimelineFrame::new(4, 1200, "I".to_string()).with_marker(FrameMarker::Key));
    timeline
        .add_frame(TimelineFrame::new(5, 500, "P".to_string()).with_marker(FrameMarker::Bookmark));

    timeline
}

#[test]
fn test_timeline_creation() {
    let timeline = TimelineBase::new("A".to_string());
    assert_eq!(timeline.stream_id, "A");
    assert_eq!(timeline.frame_count(), 0);
    assert!(timeline.current_frame.is_none());
}

#[test]
fn test_add_frame() {
    let mut timeline = TimelineBase::new("A".to_string());
    timeline.add_frame(TimelineFrame::new(0, 1000, "I".to_string()));
    timeline.add_frame(TimelineFrame::new(1, 500, "P".to_string()));

    assert_eq!(timeline.frame_count(), 2);
    assert_eq!(timeline.get_frame(0).unwrap().display_idx, 0);
    assert_eq!(timeline.get_frame(1).unwrap().display_idx, 1);
}

#[test]
fn test_frame_marker_color() {
    assert_eq!(FrameMarker::Key.color_hint(), "blue");
    assert_eq!(FrameMarker::Error.color_hint(), "red");
    assert_eq!(FrameMarker::Bookmark.color_hint(), "yellow");
}

#[test]
fn test_frame_marker_is_critical() {
    assert!(FrameMarker::Key.is_critical());
    assert!(FrameMarker::Error.is_critical());
    assert!(!FrameMarker::Bookmark.is_critical());
    assert!(!FrameMarker::None.is_critical());
}

#[test]
fn test_select_frame() {
    let mut timeline = create_test_timeline();

    assert!(timeline.select_frame(2));
    assert_eq!(timeline.current_frame_idx(), Some(2));
    assert!(timeline.get_frame(2).unwrap().is_selected);

    // Select another frame
    assert!(timeline.select_frame(4));
    assert_eq!(timeline.current_frame_idx(), Some(4));
    assert!(timeline.get_frame(4).unwrap().is_selected);
    assert!(!timeline.get_frame(2).unwrap().is_selected); // Previous deselected
}

#[test]
fn test_select_frame_out_of_bounds() {
    let mut timeline = create_test_timeline();
    assert!(!timeline.select_frame(100));
    assert!(timeline.current_frame.is_none());
}

#[test]
fn test_clear_selection() {
    let mut timeline = create_test_timeline();
    timeline.select_frame(2);
    assert!(timeline.get_frame(2).unwrap().is_selected);

    timeline.clear_selection();
    assert!(timeline.current_frame.is_none());
    assert!(!timeline.get_frame(2).unwrap().is_selected);
}

#[test]
fn test_scrub_mode() {
    let mut timeline = TimelineBase::new("A".to_string());
    assert_eq!(timeline.scrub_mode, ScrubMode::Idle);
    assert!(!timeline.is_scrubbing());

    timeline.set_scrub_mode(ScrubMode::Active);
    assert_eq!(timeline.scrub_mode, ScrubMode::Active);
    assert!(timeline.is_scrubbing());
}

#[test]
fn test_jump_next_prev() {
    let mut timeline = create_test_timeline();
    timeline.select_frame(2);

    let result = timeline.jump(JumpDirection::Next);
    assert!(result.success);
    assert_eq!(result.target_idx, 3);
    assert_eq!(timeline.current_frame_idx(), Some(3));

    let result = timeline.jump(JumpDirection::Prev);
    assert!(result.success);
    assert_eq!(result.target_idx, 2);
}

#[test]
fn test_jump_first_last() {
    let mut timeline = create_test_timeline();
    timeline.select_frame(2);

    let result = timeline.jump(JumpDirection::Last);
    assert!(result.success);
    assert_eq!(result.target_idx, 5);

    let result = timeline.jump(JumpDirection::First);
    assert!(result.success);
    assert_eq!(result.target_idx, 0);
}

#[test]
fn test_jump_next_key() {
    let mut timeline = create_test_timeline();
    timeline.select_frame(1);

    let result = timeline.jump(JumpDirection::NextKey);
    assert!(result.success);
    assert_eq!(result.target_idx, 4); // Frame 4 is keyframe
}

#[test]
fn test_jump_prev_key() {
    let mut timeline = create_test_timeline();
    timeline.select_frame(3);

    let result = timeline.jump(JumpDirection::PrevKey);
    assert!(result.success);
    assert_eq!(result.target_idx, 0); // Frame 0 is keyframe
}

#[test]
fn test_jump_next_marker() {
    let mut timeline = create_test_timeline();
    timeline.select_frame(1);

    let result = timeline.jump(JumpDirection::NextMarker);
    assert!(result.success);
    assert_eq!(result.target_idx, 3); // Frame 3 has Error marker
}

#[test]
fn test_jump_prev_marker() {
    let mut timeline = create_test_timeline();
    timeline.select_frame(5);

    let result = timeline.jump(JumpDirection::PrevMarker);
    assert!(result.success);
    assert_eq!(result.target_idx, 4); // Frame 4 has Key marker
}

#[test]
fn test_jump_at_boundary() {
    let mut timeline = create_test_timeline();
    timeline.select_frame(0);

    // Can't go prev from first frame
    let result = timeline.jump(JumpDirection::Prev);
    assert!(!result.success);
    assert_eq!(timeline.current_frame_idx(), Some(0));

    timeline.select_frame(5);
    // Can't go next from last frame
    let result = timeline.jump(JumpDirection::Next);
    assert!(!result.success);
    assert_eq!(timeline.current_frame_idx(), Some(5));
}

#[test]
fn test_jump_no_marker_found() {
    let mut timeline = TimelineBase::new("A".to_string());
    // Add frames without keyframes
    timeline.add_frame(TimelineFrame::new(0, 1000, "P".to_string()));
    timeline.add_frame(TimelineFrame::new(1, 500, "P".to_string()));

    timeline.select_frame(0);
    let result = timeline.jump(JumpDirection::NextKey);
    assert!(!result.success);
    assert_eq!(timeline.current_frame_idx(), Some(0)); // Stays at current
}

#[test]
fn test_viewport() {
    let mut timeline = create_test_timeline();
    timeline.set_viewport(2, 3);

    assert_eq!(timeline.visible_range(), (2, 3));
    assert!(!timeline.is_frame_visible(0));
    assert!(!timeline.is_frame_visible(1));
    assert!(timeline.is_frame_visible(2));
    assert!(timeline.is_frame_visible(3));
    assert!(timeline.is_frame_visible(4));
    assert!(!timeline.is_frame_visible(5));
}

#[test]
fn test_keyframe_indices() {
    let timeline = create_test_timeline();
    let keyframes = timeline.keyframe_indices();

    assert_eq!(keyframes.len(), 2);
    assert_eq!(keyframes[0], 0);
    assert_eq!(keyframes[1], 4);
}

#[test]
fn test_error_indices() {
    let timeline = create_test_timeline();
    let errors = timeline.error_indices();

    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0], 3);
}

#[test]
fn test_bookmark_indices() {
    let timeline = create_test_timeline();
    let bookmarks = timeline.bookmark_indices();

    assert_eq!(bookmarks.len(), 1);
    assert_eq!(bookmarks[0], 5);
}

#[test]
fn test_timeline_frame_builder() {
    let frame = TimelineFrame::new(10, 2000, "I".to_string())
        .with_marker(FrameMarker::Key)
        .with_pts(12345)
        .with_dts(12345);

    assert_eq!(frame.display_idx, 10);
    assert_eq!(frame.size_bytes, 2000);
    assert_eq!(frame.frame_type, "I");
    assert_eq!(frame.marker, FrameMarker::Key);
    assert_eq!(frame.pts, Some(12345));
    assert_eq!(frame.dts, Some(12345));
}

#[test]
fn test_timeline_frame_has_reorder() {
    // No reorder (PTS == DTS)
    let frame = TimelineFrame::new(0, 1000, "P".to_string())
        .with_pts(1000)
        .with_dts(1000);
    assert!(!frame.has_reorder());

    // Has reorder (PTS ≠ DTS)
    let frame = TimelineFrame::new(1, 1000, "B".to_string())
        .with_pts(2000)
        .with_dts(1000);
    assert!(frame.has_reorder());

    // Missing PTS or DTS
    let frame = TimelineFrame::new(2, 1000, "P".to_string()).with_pts(1000);
    assert!(!frame.has_reorder()); // DTS missing

    let frame = TimelineFrame::new(3, 1000, "P".to_string()).with_dts(1000);
    assert!(!frame.has_reorder()); // PTS missing
}

// UX Timeline viz_core integration tests
// Deliverable: viz_core_07_timeline_viz:UX:Timeline:ALL:viz_core

#[test]
fn test_ux_timeline_click_frame_selection() {
    let mut timeline = create_test_timeline();

    // UX Timeline: User clicks on frame 5 in timeline view
    let result = timeline.select_frame(5);
    assert!(result);

    // UX Timeline: Verify frame is marked as selected
    assert_eq!(timeline.current_frame_idx(), Some(5));
    let frame = timeline.get_frame(5).unwrap();
    assert!(frame.is_selected);

    // UX Timeline: User clicks frame 3 (changes selection)
    timeline.select_frame(3);

    // UX Timeline: Verify frame 3 is now selected and frame 5 is not
    assert_eq!(timeline.current_frame_idx(), Some(3));
    let frame3 = timeline.get_frame(3).unwrap();
    assert!(frame3.is_selected);

    let frame5 = timeline.get_frame(5).unwrap();
    assert!(!frame5.is_selected);
}

#[test]
fn test_ux_timeline_scrub_navigation() {
    let timeline = create_test_timeline();

    // UX Timeline: User scrubs through all frames (0-5)
    for i in 0..timeline.frame_count() {
        let frame = timeline.get_frame(i).unwrap();
        assert_eq!(frame.display_idx, i);

        // UX Timeline: Verify frame metadata is available for viz
        assert!(frame.size_bytes > 0);
        assert!(!frame.frame_type.is_empty());
    }
}

#[test]
fn test_ux_timeline_keyframe_jump_navigation() {
    let timeline = create_test_timeline();

    // UX Timeline: User presses "next keyframe" button
    let keyframes = timeline.keyframe_indices();

    // UX Timeline: Should find keyframes at indices 0 and 4
    assert_eq!(keyframes.len(), 2);
    assert_eq!(keyframes, vec![0, 4]);

    // UX Timeline: User can navigate between keyframes
    for &keyframe_idx in keyframes.iter() {
        let frame = timeline.get_frame(keyframe_idx).unwrap();
        assert_eq!(frame.marker, FrameMarker::Key);

        // UX Timeline: Verify keyframe visualization properties
        assert_eq!(frame.marker.color_hint(), "blue");
        assert!(frame.marker.is_critical());
    }
}

#[test]
fn test_ux_timeline_error_marker_visibility() {
    let timeline = create_test_timeline();

    // UX Timeline: User views timeline with error marker at frame 3
    let errors = timeline.error_indices();
    assert_eq!(errors.len(), 1);

    let error_frame = timeline.get_frame(errors[0]).unwrap();

    // UX Timeline: Verify error marker visualization
    assert_eq!(error_frame.marker, FrameMarker::Error);
    assert_eq!(error_frame.marker.color_hint(), "red");
    assert!(error_frame.marker.is_critical());
}

#[test]
fn test_ux_timeline_bookmark_toggle() {
    let mut timeline = create_test_timeline();

    // UX Timeline: User right-clicks frame 2 and adds bookmark
    let frame = timeline.get_frame_mut(2).unwrap();
    frame.marker = FrameMarker::Bookmark;

    // UX Timeline: Verify bookmark is visible
    let bookmarks = timeline.bookmark_indices();
    assert!(bookmarks.contains(&2));
    assert!(bookmarks.contains(&5)); // Frame 5 already has bookmark from test data

    // UX Timeline: Verify bookmark visualization
    let frame = timeline.get_frame(2).unwrap();
    assert_eq!(frame.marker.color_hint(), "yellow");
}

#[test]
fn test_ux_timeline_sequential_selection() {
    let mut timeline = create_test_timeline();

    // UX Timeline: User clicks through frames 1, 2, 3, 4 sequentially
    // (Only one frame can be selected at a time per tri-sync contract)
    for i in 1..=4 {
        timeline.select_frame(i);
        assert_eq!(timeline.current_frame_idx(), Some(i));

        // UX Timeline: Verify current frame is marked as selected
        let frame = timeline.get_frame(i).unwrap();
        assert!(frame.is_selected);
    }

    // UX Timeline: Final selection should be frame 4
    assert_eq!(timeline.current_frame_idx(), Some(4));
}

#[test]
fn test_ux_timeline_frame_size_visualization() {
    let timeline = create_test_timeline();

    // UX Timeline: User hovers over timeline to see frame sizes
    // Larger frames should be visually distinct

    let mut sizes: Vec<u64> = timeline.frames.iter().map(|f| f.size_bytes).collect();

    // UX Timeline: Verify size variation exists for visualization
    sizes.sort();
    let min_size = sizes.first().unwrap();
    let max_size = sizes.last().unwrap();
    assert!(max_size > min_size);

    // UX Timeline: I-frames should be larger than P-frames
    let i_frame = timeline.get_frame(0).unwrap(); // I frame
    let p_frame = timeline.get_frame(1).unwrap(); // P frame
    assert!(i_frame.size_bytes > p_frame.size_bytes);
}

// AV1 Timeline viz_core test - Task 15 (S.T4-2.AV1.Timeline.Timeline.impl.viz_core.001)

#[test]
fn test_av1_timeline_frame_type_visualization() {
    // AV1 Timeline: User views timeline with AV1-specific frame types
    let mut timeline = create_test_timeline();

    // AV1 Timeline: Add frames with AV1 frame type strings
    timeline.add_frame(
        TimelineFrame::new(6, 18000, "KEY_FRAME".to_string())
            .with_marker(FrameMarker::Key)
            .with_pts(6000)
            .with_dts(6000),
    );

    timeline.add_frame(
        TimelineFrame::new(7, 6000, "INTER_FRAME".to_string())
            .with_pts(7000)
            .with_dts(7000),
    );

    timeline.add_frame(
        TimelineFrame::new(8, 200, "INTRA_ONLY_FRAME".to_string())
            .with_pts(8000)
            .with_dts(8000),
    );

    timeline.add_frame(
        TimelineFrame::new(9, 5500, "SWITCH_FRAME".to_string())
            .with_pts(9000)
            .with_dts(9000),
    );

    assert_eq!(timeline.frame_count(), 10); // 6 original + 4 new

    // AV1 Timeline: User hovers over KEY_FRAME (AV1 keyframe)
    let key_frame = timeline.get_frame(6).unwrap();
    assert_eq!(key_frame.frame_type, "KEY_FRAME");
    assert_eq!(key_frame.marker, FrameMarker::Key);
    assert!(key_frame.size_bytes > 10000); // Keyframes are large

    // AV1 Timeline: User hovers over INTER_FRAME (AV1 inter-coded frame)
    let inter_frame = timeline.get_frame(7).unwrap();
    assert_eq!(inter_frame.frame_type, "INTER_FRAME");
    assert_eq!(inter_frame.marker, FrameMarker::None);
    assert!(inter_frame.size_bytes < key_frame.size_bytes);

    // AV1 Timeline: User hovers over INTRA_ONLY_FRAME (non-reference intra)
    let intra_only = timeline.get_frame(8).unwrap();
    assert_eq!(intra_only.frame_type, "INTRA_ONLY_FRAME");
    assert!(intra_only.size_bytes < 1000); // Typically small

    // AV1 Timeline: User hovers over SWITCH_FRAME (switch point)
    let switch_frame = timeline.get_frame(9).unwrap();
    assert_eq!(switch_frame.frame_type, "SWITCH_FRAME");
    assert!(switch_frame.size_bytes > intra_only.size_bytes);

    // AV1 Timeline: Verify size hierarchy (KEY > SWITCH/INTER > INTRA_ONLY)
    assert!(key_frame.size_bytes > switch_frame.size_bytes);
    assert!(switch_frame.size_bytes > intra_only.size_bytes);

    // AV1 Timeline: Verify PTS/DTS are sequential
    for i in 6..10 {
        let frame = timeline.get_frame(i).unwrap();
        assert_eq!(frame.pts, Some((i * 1000) as u64));
        assert_eq!(frame.dts, Some((i * 1000) as u64));
    }
}
