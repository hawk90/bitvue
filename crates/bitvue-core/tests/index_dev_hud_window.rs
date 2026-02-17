#![allow(dead_code)]
//! Tests for index_dev_hud_window module

use bitvue_core::{
    indexing::FrameMetadata, AccessEvent, IndexSessionWindow, IndexWindowPolicy, SeekPoint,
    TimelineWindowHUD,
};

#[test]
fn test_timeline_window_hud_creation() {
    let hud = TimelineWindowHUD::new("test_session".to_string());

    assert_eq!(hud.session_id(), "test_session");
    assert_eq!(hud.update_count(), 0);
    assert_eq!(hud.window_viz().total_frames, 0);
}

#[test]
fn test_update_from_window() {
    let mut hud = TimelineWindowHUD::new("test_session".to_string());

    let keyframes = vec![
        SeekPoint {
            display_idx: 0,
            byte_offset: 0,
            is_keyframe: true,
            pts: None,
        },
        SeekPoint {
            display_idx: 50,
            byte_offset: 1000,
            is_keyframe: true,
            pts: None,
        },
    ];

    let mut window = IndexSessionWindow::new(
        "test_session".to_string(),
        100,
        IndexWindowPolicy::Fixed(20),
        keyframes,
    );

    window.set_position(10);
    hud.update_from_window(&window);

    assert_eq!(hud.update_count(), 1);
    assert_eq!(hud.window_viz().total_frames, 100);
    assert_eq!(hud.window_viz().window_start, 0); // Window stays at 0 since pos 10 is within [0, 20)
    assert_eq!(hud.window_viz().current_position, 10);
}

#[test]
fn test_record_access() {
    let mut hud = TimelineWindowHUD::new("test_session".to_string());

    // Record cache hit
    hud.record_access(0, true, 100);
    assert_eq!(hud.materialization_tracker().cache_hits, 1);
    assert_eq!(hud.materialization_tracker().total_requests, 1);
    assert_eq!(hud.performance().avg_access_time_us, 100.0);

    // Record cache miss
    hud.record_access(1, false, 500);
    assert_eq!(hud.materialization_tracker().cache_misses, 1);
    assert_eq!(hud.materialization_tracker().total_requests, 2);
    assert_eq!(hud.performance().avg_access_time_us, 300.0); // (100 + 500) / 2

    // Record blocking operation (> 16ms)
    hud.record_access(2, true, 20_000);
    assert_eq!(hud.performance().blocking_operations, 1);
}

#[test]
fn test_record_window_move() {
    let mut hud = TimelineWindowHUD::new("test_session".to_string());

    hud.record_window_move(1000);

    assert_eq!(hud.materialization_tracker().window_moves, 1);
    assert_eq!(hud.performance().window_adjust_latency_us.len(), 1);
    assert_eq!(hud.performance().window_adjust_latency_us[0], 1000);
}

#[test]
fn test_record_eviction() {
    let mut hud = TimelineWindowHUD::new("test_session".to_string());

    hud.record_eviction(5);

    assert_eq!(hud.materialization_tracker().frames_evicted, 1);
    assert_eq!(hud.materialization_tracker().recent_pattern.len(), 1);
    assert_eq!(
        hud.materialization_tracker().recent_pattern[0],
        AccessEvent::Eviction
    );
}

#[test]
fn test_recent_pattern_bounded() {
    let mut hud = TimelineWindowHUD::new("test_session".to_string());

    // Record 150 events (should keep only last 100)
    for i in 0..150 {
        hud.record_access(i, i % 2 == 0, 100);
    }

    assert_eq!(hud.materialization_tracker().recent_pattern.len(), 100);
}

#[test]
fn test_format_window_viz() {
    let mut hud = TimelineWindowHUD::new("test_session".to_string());

    let keyframes = vec![SeekPoint {
        display_idx: 0,
        byte_offset: 0,
        is_keyframe: true,
        pts: None,
    }];

    let window = IndexSessionWindow::new(
        "test_session".to_string(),
        100,
        IndexWindowPolicy::Fixed(20),
        keyframes,
    );

    hud.update_from_window(&window);

    let viz = hud.format_window_viz(60);
    assert!(viz.contains('['));
    assert!(viz.contains(']'));
    assert!(viz.contains("window"));
}

#[test]
fn test_format_access_pattern() {
    let mut hud = TimelineWindowHUD::new("test_session".to_string());

    hud.record_access(0, true, 100);
    hud.record_access(1, false, 200);
    hud.record_window_move(500);

    let pattern = hud.format_access_pattern();
    assert!(pattern.contains("HMW"));
    assert!(pattern.contains("hit"));
}

#[test]
fn test_density_histogram() {
    let mut hud = TimelineWindowHUD::new("test_session".to_string());

    let keyframes = vec![SeekPoint {
        display_idx: 0,
        byte_offset: 0,
        is_keyframe: true,
        pts: None,
    }];

    let mut window = IndexSessionWindow::new(
        "test_session".to_string(),
        100,
        IndexWindowPolicy::Fixed(50),
        keyframes,
    );

    // Materialize some frames
    let _ = window.materialize_frame(FrameMetadata {
        display_idx: 10,
        decode_idx: 10,
        byte_offset: 1000,
        size: 100,
        is_keyframe: false,
        pts: None,
        dts: None,
        frame_type: None,
    });

    hud.update_from_window(&window);

    let histogram = hud.format_density_histogram();
    assert!(histogram.contains("Density:"));
}

#[test]
fn test_format_text() {
    let mut hud = TimelineWindowHUD::new("test_session".to_string());

    let keyframes = vec![SeekPoint {
        display_idx: 0,
        byte_offset: 0,
        is_keyframe: true,
        pts: None,
    }];

    let window = IndexSessionWindow::new(
        "test_session".to_string(),
        100,
        IndexWindowPolicy::Fixed(20),
        keyframes,
    );

    hud.update_from_window(&window);
    hud.record_access(0, true, 100);

    let text = hud.format_text();
    assert!(text.contains("Timeline Window DevHUD"));
    assert!(text.contains("Window State"));
    assert!(text.contains("Materialization"));
    assert!(text.contains("Performance"));
}

#[test]
fn test_performance_percentiles() {
    let mut hud = TimelineWindowHUD::new("test_session".to_string());

    // Record various access times
    hud.record_access(0, true, 100);
    hud.record_access(1, true, 200);
    hud.record_access(2, true, 1000);
    hud.record_access(3, true, 10000);

    // Verify percentile calculations
    assert!(hud.performance().p95_access_time_us > hud.performance().avg_access_time_us as u64);
    assert!(hud.performance().p99_access_time_us > hud.performance().p95_access_time_us);
    assert!(hud.performance().p99_access_time_us <= hud.performance().max_access_time_us);
}

#[test]
fn test_window_adjust_latency_bounded() {
    let mut hud = TimelineWindowHUD::new("test_session".to_string());

    // Record 1500 window adjustments (should keep only last 1000)
    for _ in 0..1500 {
        hud.record_window_move(1000);
    }

    assert_eq!(hud.performance().window_adjust_latency_us.len(), 1000);
}

#[test]
fn test_coverage_calculation() {
    let mut hud = TimelineWindowHUD::new("test_session".to_string());

    let keyframes = vec![SeekPoint {
        display_idx: 0,
        byte_offset: 0,
        is_keyframe: true,
        pts: None,
    }];

    let mut window = IndexSessionWindow::new(
        "test_session".to_string(),
        100,
        IndexWindowPolicy::Fixed(10),
        keyframes,
    );

    // Materialize half the window
    for i in 0..5 {
        let _ = window.materialize_frame(FrameMetadata {
            display_idx: i,
            decode_idx: i,
            byte_offset: (i * 100) as u64,
            size: 100,
            is_keyframe: false,
            pts: None,
            dts: None,
            frame_type: None,
        });
    }

    hud.update_from_window(&window);

    // Coverage should be ~50%
    assert!((hud.window_viz().coverage_percent - 0.5).abs() < 0.1);
}
