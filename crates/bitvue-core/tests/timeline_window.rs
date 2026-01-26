//! Tests for timeline_window module

use bitvue_core::timeline::{FrameMarker, TimelineFrame};
use bitvue_core::timeline_window::{
    SparseIndexEntry, TimelineWindow, WindowLoadStatus, WindowLoader, WindowPolicy,
};

#[test]
fn test_window_policy_fixed() {
    let policy = WindowPolicy::Fixed(1000);
    assert_eq!(policy.calculate_window_size(1.0, 10000), 1000);
    assert_eq!(policy.calculate_window_size(10.0, 10000), 1000);
    assert_eq!(policy.calculate_window_size(0.1, 10000), 1000);

    // Clamp to total frames
    assert_eq!(policy.calculate_window_size(1.0, 500), 500);
}

#[test]
fn test_window_policy_adaptive() {
    let policy = WindowPolicy::Adaptive {
        min: 100,
        max: 5000,
    };

    // High zoom (10 px/frame): fewer frames
    let size_high = policy.calculate_window_size(10.0, 100000);
    assert!(size_high >= 100 && size_high <= 5000);

    // Low zoom (0.1 px/frame): more frames
    let size_low = policy.calculate_window_size(0.1, 100000);
    assert!(size_low >= 100 && size_low <= 5000);
    assert!(size_low > size_high); // More frames at lower zoom
}

#[test]
fn test_window_policy_full() {
    let policy = WindowPolicy::Full;
    assert_eq!(policy.calculate_window_size(1.0, 10000), 10000);
    assert_eq!(policy.calculate_window_size(0.1, 500), 500);
}

#[test]
fn test_timeline_window_creation() {
    let window = TimelineWindow::new("A".to_string(), 100000, WindowPolicy::Fixed(1000));

    assert_eq!(window.stream_id, "A");
    assert_eq!(window.total_frames, 100000);
    assert_eq!(window.window_start, 0);
    assert_eq!(window.materialized_count(), 0);
}

#[test]
fn test_set_zoom() {
    let mut window = TimelineWindow::new(
        "A".to_string(),
        100000,
        WindowPolicy::Adaptive {
            min: 100,
            max: 5000,
        },
    );

    window.set_zoom(1.0);
    let size1 = window.window_size;

    window.set_zoom(10.0);
    let size2 = window.window_size;

    assert!(size2 < size1); // Higher zoom = smaller window
}

#[test]
fn test_scroll_to() {
    let mut window = TimelineWindow::new("A".to_string(), 10000, WindowPolicy::Fixed(100));

    window.scroll_to(5000);
    assert_eq!(window.window_start, 5000 - 50); // Centered

    // Clamp to bounds
    window.scroll_to(9999);
    assert!(window.window_start <= 9999);
}

#[test]
fn test_materialize_frame() {
    let mut window = TimelineWindow::new("A".to_string(), 1000, WindowPolicy::Fixed(100));
    window.window_start = 500;
    window.window_size = 100;

    let frame = TimelineFrame::new(550, 1000, "P".to_string());
    window.materialize_frame(frame);

    assert!(window.is_materialized(550));
    assert_eq!(window.materialized_count(), 1);

    // Frame outside window should not materialize
    let frame_outside = TimelineFrame::new(100, 1000, "P".to_string());
    window.materialize_frame(frame_outside);
    assert!(!window.is_materialized(100));
    assert_eq!(window.materialized_count(), 1);
}

#[test]
fn test_dematerialize_outside_window() {
    let mut window = TimelineWindow::new("A".to_string(), 1000, WindowPolicy::Fixed(100));
    window.window_start = 500;
    window.window_size = 100;

    // Materialize some frames
    for i in 0..10 {
        let frame = TimelineFrame::new(500 + i * 10, 1000, "P".to_string());
        window.materialize_frame(frame);
    }

    // Also add frames outside window
    window
        .materialized
        .insert(100, TimelineFrame::new(100, 1000, "P".to_string()));
    window
        .materialized
        .insert(800, TimelineFrame::new(800, 1000, "P".to_string()));

    let before_count = window.materialized_count();
    assert!(before_count > 10);

    window.dematerialize_outside_window();

    let after_count = window.materialized_count();
    assert!(after_count <= 10);
    assert!(!window.is_materialized(100));
    assert!(!window.is_materialized(800));
}

#[test]
fn test_sparse_index() {
    let mut window = TimelineWindow::new("A".to_string(), 1000, WindowPolicy::Fixed(100));

    window.add_sparse_entry(SparseIndexEntry {
        display_idx: 0,
        frame_type: "I".to_string(),
        marker: FrameMarker::Key,
        byte_offset: 0,
        size_bytes: 5000,
    });

    window.add_sparse_entry(SparseIndexEntry {
        display_idx: 100,
        frame_type: "I".to_string(),
        marker: FrameMarker::Key,
        byte_offset: 50000,
        size_bytes: 4800,
    });

    assert_eq!(window.sparse_index.len(), 2);
    assert_eq!(window.sparse_index[0].display_idx, 0);
    assert_eq!(window.sparse_index[1].display_idx, 100);
}

#[test]
fn test_find_nearest_keyframe() {
    let mut window = TimelineWindow::new("A".to_string(), 1000, WindowPolicy::Fixed(100));

    // Add keyframes
    for i in &[0, 100, 200, 300, 400] {
        window.add_sparse_entry(SparseIndexEntry {
            display_idx: *i,
            frame_type: "I".to_string(),
            marker: FrameMarker::Key,
            byte_offset: 0,
            size_bytes: 5000,
        });
    }

    // Find next keyframe from 150
    assert_eq!(window.find_nearest_keyframe(150, true), Some(200));

    // Find prev keyframe from 150
    assert_eq!(window.find_nearest_keyframe(150, false), Some(100));

    // No keyframe after 400
    assert_eq!(window.find_nearest_keyframe(400, true), None);
}

#[test]
fn test_sparse_entries_in_range() {
    let mut window = TimelineWindow::new("A".to_string(), 1000, WindowPolicy::Fixed(100));

    for i in 0..10 {
        window.add_sparse_entry(SparseIndexEntry {
            display_idx: i * 100,
            frame_type: "I".to_string(),
            marker: FrameMarker::Key,
            byte_offset: 0,
            size_bytes: 5000,
        });
    }

    let entries = window.sparse_entries_in_range(200, 600);
    assert_eq!(entries.len(), 4); // 200, 300, 400, 500
    assert_eq!(entries[0].display_idx, 200);
    assert_eq!(entries[3].display_idx, 500);
}

#[test]
fn test_cache_invalidation() {
    let mut window = TimelineWindow::new("A".to_string(), 1000, WindowPolicy::Fixed(100));

    let key1 = window.create_cache_key(1000);
    let (total1, valid1, _) = window.cache_stats();
    assert_eq!(total1, 1);
    assert_eq!(valid1, 1);

    // Zoom change invalidates
    window.set_zoom(2.0);
    let (_, valid2, _) = window.cache_stats();
    assert_eq!(valid2, 0);

    // Create new key after invalidation
    let key2 = window.create_cache_key(1000);
    assert_ne!(key1, key2);
}

#[test]
fn test_coverage_ratio() {
    let mut window = TimelineWindow::new("A".to_string(), 1000, WindowPolicy::Fixed(100));
    window.window_start = 500;
    window.window_size = 100;

    assert_eq!(window.coverage_ratio(), 0.0);

    // Materialize half the window
    for i in 0..50 {
        window.materialize_frame(TimelineFrame::new(500 + i, 1000, "P".to_string()));
    }

    assert!((window.coverage_ratio() - 0.5).abs() < 0.01);

    // Materialize full window
    for i in 50..100 {
        window.materialize_frame(TimelineFrame::new(500 + i, 1000, "P".to_string()));
    }

    assert!((window.coverage_ratio() - 1.0).abs() < 0.01);
}

#[test]
fn test_window_loader() {
    let mut loader = WindowLoader::new();

    assert_eq!(loader.status, WindowLoadStatus::Idle);
    assert!(!loader.is_loading());

    loader.start_load(100);
    assert!(loader.is_loading());
    assert_eq!(loader.progress, 0.0);

    loader.update_progress(50);
    assert!((loader.progress - 0.5).abs() < 0.01);

    loader.complete();
    assert_eq!(loader.status, WindowLoadStatus::Completed);
    assert_eq!(loader.progress, 1.0);
}

#[test]
fn test_window_loader_cancel() {
    let mut loader = WindowLoader::new();

    loader.start_load(100);
    let gen1 = loader.current_generation();

    loader.cancel();
    let gen2 = loader.current_generation();

    assert_eq!(gen2, gen1 + 1);
    assert!(!loader.is_loading());
}

#[test]
fn test_set_filter() {
    let mut window = TimelineWindow::new("A".to_string(), 1000, WindowPolicy::Fixed(100));

    window.create_cache_key(1000);
    let (_, valid1, _) = window.cache_stats();
    assert_eq!(valid1, 1);

    window.set_filter(12345);
    let (_, valid2, _) = window.cache_stats();
    assert_eq!(valid2, 0); // Invalidated
    assert_eq!(window.filter_hash, 12345);
}

#[test]
fn test_increment_revision() {
    let mut window = TimelineWindow::new("A".to_string(), 1000, WindowPolicy::Fixed(100));

    assert_eq!(window.data_revision, 0);

    window.create_cache_key(1000);
    let (_, valid1, _) = window.cache_stats();
    assert_eq!(valid1, 1);

    window.increment_revision();
    assert_eq!(window.data_revision, 1);

    let (_, valid2, _) = window.cache_stats();
    assert_eq!(valid2, 0); // Invalidated
}

#[test]
fn test_estimated_memory_usage() {
    let mut window = TimelineWindow::new("A".to_string(), 1000, WindowPolicy::Fixed(100));

    let usage_empty = window.estimated_memory_usage();

    // Add some frames
    for i in 0..50 {
        window.materialize_frame(TimelineFrame::new(i, 1000, "P".to_string()));
    }

    let usage_50 = window.estimated_memory_usage();
    assert!(usage_50 > usage_empty);

    // Add sparse entries
    for i in 0..10 {
        window.add_sparse_entry(SparseIndexEntry {
            display_idx: i * 100,
            frame_type: "I".to_string(),
            marker: FrameMarker::Key,
            byte_offset: 0,
            size_bytes: 5000,
        });
    }

    let usage_with_sparse = window.estimated_memory_usage();
    assert!(usage_with_sparse > usage_50);
}
