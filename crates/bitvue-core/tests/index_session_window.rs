#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unused_mut)]
#![allow(unused_comparisons)]
#![allow(unused_doc_comments)]
//! Tests for index session out-of-core windowing

use bitvue_core::{
    indexing::{FrameMetadata, SeekPoint},
    IndexSessionWindow, IndexWindowPolicy,
};

fn create_test_frame_metadata(display_idx: usize) -> FrameMetadata {
    FrameMetadata {
        display_idx,
        decode_idx: display_idx,
        byte_offset: display_idx as u64 * 1024,
        size: 512,
        is_keyframe: display_idx % 10 == 0,
        pts: Some(display_idx as u64 * 1000),
        dts: Some(display_idx as u64 * 1000),
        frame_type: Some(if display_idx % 10 == 0 {
            "I".to_string()
        } else {
            "P".to_string()
        }),
    }
}

fn create_test_sparse_keyframes(total_frames: usize) -> Vec<SeekPoint> {
    (0..total_frames)
        .step_by(10)
        .map(|i| SeekPoint {
            display_idx: i,
            byte_offset: i as u64 * 1024,
            is_keyframe: true,
            pts: Some(i as u64 * 1000),
        })
        .collect()
}

#[test]
fn test_window_creation() {
    let sparse_kf = create_test_sparse_keyframes(100);
    let window = IndexSessionWindow::new(
        "test_session".to_string(),
        100,
        IndexWindowPolicy::Fixed(20),
        sparse_kf,
    );

    assert_eq!(window.total_frames, 100);
    assert_eq!(window.window_size, 20);
    assert_eq!(window.window_start, 0);
    assert_eq!(window.current_position, 0);
}

#[test]
fn test_materialize_frame() {
    let sparse_kf = create_test_sparse_keyframes(100);
    let mut window = IndexSessionWindow::new(
        "test_session".to_string(),
        100,
        IndexWindowPolicy::Fixed(20),
        sparse_kf,
    );

    let frame = create_test_frame_metadata(5);
    let was_new = window.materialize_frame(frame);

    assert!(was_new);
    assert_eq!(window.materialized_count(), 1);
    assert!(window.is_materialized(5));
}

#[test]
fn test_materialize_duplicate() {
    let sparse_kf = create_test_sparse_keyframes(100);
    let mut window = IndexSessionWindow::new(
        "test_session".to_string(),
        100,
        IndexWindowPolicy::Fixed(20),
        sparse_kf,
    );

    let frame = create_test_frame_metadata(5);
    window.materialize_frame(frame.clone());

    let was_new = window.materialize_frame(frame);
    assert!(!was_new);
    assert_eq!(window.materialized_count(), 1);
}

#[test]
fn test_lru_eviction() {
    let sparse_kf = create_test_sparse_keyframes(100);
    let mut window = IndexSessionWindow::new(
        "test_session".to_string(),
        100,
        IndexWindowPolicy::Fixed(3), // Small window for testing
        sparse_kf,
    );

    // Materialize 4 frames (exceeds capacity of 3)
    for i in 0..4 {
        window.materialize_frame(create_test_frame_metadata(i));
    }

    // Should have evicted frame 0 (LRU)
    assert_eq!(window.materialized_count(), 3);
    assert!(!window.is_materialized(0));
    assert!(window.is_materialized(1));
    assert!(window.is_materialized(2));
    assert!(window.is_materialized(3));
}

#[test]
fn test_set_position_within_window() {
    let sparse_kf = create_test_sparse_keyframes(100);
    let mut window = IndexSessionWindow::new(
        "test_session".to_string(),
        100,
        IndexWindowPolicy::Fixed(20),
        sparse_kf,
    );

    let initial_revision = window.window_revision();

    window.set_position(10);

    assert_eq!(window.current_position, 10);
    assert_eq!(window.window_start, 0); // Window didn't move
    assert_eq!(window.window_revision(), initial_revision); // Revision unchanged
}

#[test]
fn test_set_position_outside_window() {
    let sparse_kf = create_test_sparse_keyframes(100);
    let mut window = IndexSessionWindow::new(
        "test_session".to_string(),
        100,
        IndexWindowPolicy::Fixed(20),
        sparse_kf,
    );

    let initial_revision = window.window_revision();

    window.set_position(50);

    assert_eq!(window.current_position, 50);
    assert!(window.window_start > 0); // Window moved
    assert_eq!(window.window_revision(), initial_revision + 1); // Revision incremented
}

#[test]
fn test_evict_outside_window() {
    let sparse_kf = create_test_sparse_keyframes(100);
    let mut window = IndexSessionWindow::new(
        "test_session".to_string(),
        100,
        IndexWindowPolicy::Fixed(20),
        sparse_kf,
    );

    // Materialize frames 0-9
    for i in 0..10 {
        window.materialize_frame(create_test_frame_metadata(i));
    }

    assert_eq!(window.materialized_count(), 10);

    // Move window to center on frame 50
    window.set_position(50);

    // Frames 0-9 should be evicted
    for i in 0..10 {
        assert!(!window.is_materialized(i));
    }
}

#[test]
fn test_find_nearest_keyframe() {
    let sparse_kf = create_test_sparse_keyframes(100);
    let window = IndexSessionWindow::new(
        "test_session".to_string(),
        100,
        IndexWindowPolicy::Fixed(20),
        sparse_kf,
    );

    // Find keyframe for frame 15 (should be frame 10)
    let kf = window.find_nearest_keyframe(15).unwrap();
    assert_eq!(kf.display_idx, 10);

    // Find keyframe for frame 25 (should be frame 20)
    let kf = window.find_nearest_keyframe(25).unwrap();
    assert_eq!(kf.display_idx, 20);

    // Find keyframe for frame 5 (should be frame 0)
    let kf = window.find_nearest_keyframe(5).unwrap();
    assert_eq!(kf.display_idx, 0);
}

#[test]
fn test_window_range() {
    let sparse_kf = create_test_sparse_keyframes(100);
    let window = IndexSessionWindow::new(
        "test_session".to_string(),
        100,
        IndexWindowPolicy::Fixed(20),
        sparse_kf,
    );

    let (start, end) = window.window_range();
    assert_eq!(start, 0);
    assert_eq!(end, 20);
}

#[test]
fn test_cache_hit_rate() {
    let sparse_kf = create_test_sparse_keyframes(100);
    let mut window = IndexSessionWindow::new(
        "test_session".to_string(),
        100,
        IndexWindowPolicy::Fixed(20),
        sparse_kf,
    );

    // Materialize frame 5 (counts as cache miss)
    window.materialize_frame(create_test_frame_metadata(5));

    // First get: cache hit
    let _ = window.get_frame(5);
    // Second get: another cache hit
    let _ = window.get_frame(5);
    // Get non-existent: cache miss
    let _ = window.get_frame(10);

    let hit_rate = window.hit_rate();
    // materialize_frame: 1 miss
    // get_frame(5): 1 hit
    // get_frame(5): 1 hit
    // get_frame(10): 1 miss
    // Total: 2 hits out of 4 accesses = 0.5
    assert!((hit_rate - 0.5).abs() < 0.01);
}

#[test]
fn test_should_use_out_of_core() {
    assert!(!IndexWindowPolicy::should_use_out_of_core(5_000));
    assert!(IndexWindowPolicy::should_use_out_of_core(15_000));
    assert!(IndexWindowPolicy::should_use_out_of_core(100_000));
}

#[test]
fn test_clear() {
    let sparse_kf = create_test_sparse_keyframes(100);
    let mut window = IndexSessionWindow::new(
        "test_session".to_string(),
        100,
        IndexWindowPolicy::Fixed(20),
        sparse_kf,
    );

    // Materialize some frames
    for i in 0..5 {
        window.materialize_frame(create_test_frame_metadata(i));
    }

    assert_eq!(window.materialized_count(), 5);

    window.clear();

    assert_eq!(window.materialized_count(), 0);
}
