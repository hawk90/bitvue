//! `TimelineWindowHUD::format_text`: the exact report layout, which sections appear when, and
//! the figures printed for each section.

use crate::indexing::{FrameMetadata, SeekPoint};
use crate::{IndexSessionWindow, IndexWindowPolicy, TimelineWindowHUD};

fn keyframe(display_idx: usize) -> SeekPoint {
    SeekPoint {
        display_idx,
        byte_offset: 0,
        is_keyframe: true,
        pts: None,
    }
}

fn frame(i: usize) -> FrameMetadata {
    FrameMetadata {
        display_idx: i,
        decode_idx: i,
        byte_offset: i as u64 * 1000,
        size: 100,
        is_keyframe: false,
        pts: None,
        dts: None,
        frame_type: None,
    }
}

fn window(total: usize, size: usize, materialized: usize) -> IndexSessionWindow {
    let mut w = IndexSessionWindow::new(
        "s".to_string(),
        total,
        IndexWindowPolicy::Fixed(size),
        vec![keyframe(0), keyframe(total / 2)],
    );
    for i in 0..materialized {
        w.materialize_frame(frame(i));
    }
    w
}

fn hud() -> TimelineWindowHUD {
    TimelineWindowHUD::new("s".to_string())
}

fn lines(text: &str) -> Vec<&str> {
    text.lines().collect()
}

fn position(text: &str, needle: &str) -> usize {
    text.find(needle)
        .unwrap_or_else(|| panic!("{needle:?} not found in:\n{text}"))
}

#[test]
fn fresh_hud_golden_output() {
    let expected = "\
=== Timeline Window DevHUD: s ===
Updates: 0

--- Window State ---
Total Frames: 0
Window: 0-0 (size=0)
Position: 0
Materialized: 0 (0.0% coverage)
Sparse Keyframes: 0

--- Timeline Visualization ---
[No frames]
--- Materialization ---
Total Requests: 0
Cache Hits: 0 (0.0%)
Cache Misses: 0
Evictions: 0
Window Moves: 0

--- Performance (UI Freeze Prevention) ---
Avg Access Time: 0.0 μs
Max Access Time: 0 μs
P95 Access Time: 0 μs
P99 Access Time: 0 μs
Blocking Ops (>16ms): 0 / 0 (0.0%)
";
    assert_eq!(hud().format_text(), expected);
}

#[test]
fn populated_hud_golden_output() {
    let mut hud = hud();
    hud.update_from_window(&window(1000, 100, 10));
    hud.record_access(0, true, 100);
    hud.record_access(1, false, 20_000);
    hud.record_access(2, true, 300);
    hud.record_window_move(500);
    hud.record_window_move(1500);
    hud.record_eviction(3);

    let expected = "\
=== Timeline Window DevHUD: s ===
Updates: 1

--- Window State ---
Total Frames: 1000
Window: 0-100 (size=100)
Position: 0
Materialized: 10 (10.0% coverage)
Sparse Keyframes: 2

--- Timeline Visualization ---
[▲░░░░░                       ·                              ]
  ░=window █=materialized ▲=position ·=keyframe

--- Materialization ---
Total Requests: 3
Cache Hits: 2 (66.7%)
Cache Misses: 1
Evictions: 1
Window Moves: 2

--- Access Pattern ---
Recent: [HMHWWE]
  H=hit M=miss W=window-move E=eviction

--- Materialization Density ---
Density: ▇_________

--- Performance (UI Freeze Prevention) ---
Avg Access Time: 6800.0 μs
Max Access Time: 20000 μs
P95 Access Time: 17360 μs
P99 Access Time: 18680 μs
Blocking Ops (>16ms): 1 / 3 (33.3%)
Avg Window Adjust: 1000.0 μs
";
    assert_eq!(hud.format_text(), expected);
}

#[test]
fn sections_appear_in_report_order() {
    let mut hud = hud();
    hud.update_from_window(&window(1000, 100, 10));
    hud.record_access(0, true, 100);
    let text = hud.format_text();

    let order = [
        "=== Timeline Window DevHUD",
        "--- Window State ---",
        "--- Timeline Visualization ---",
        "--- Materialization ---",
        "--- Access Pattern ---",
        "--- Materialization Density ---",
        "--- Performance (UI Freeze Prevention) ---",
    ];
    let positions: Vec<_> = order.iter().map(|h| position(&text, h)).collect();
    assert!(positions.windows(2).all(|w| w[0] < w[1]), "{positions:?}");
}

#[test]
fn header_carries_session_id_and_update_count() {
    let mut hud = TimelineWindowHUD::new("session #7 — ü".to_string());
    let w = window(100, 10, 0);
    hud.update_from_window(&w);
    hud.update_from_window(&w);
    hud.update_from_window(&w);
    let text = hud.format_text();
    let l = lines(&text);
    assert_eq!(l[0], "=== Timeline Window DevHUD: session #7 — ü ===");
    assert_eq!(l[1], "Updates: 3");
    assert_eq!(l[2], "");
}

#[test]
fn window_state_follows_the_window_it_was_updated_from() {
    let mut w = window(1000, 100, 0);
    w.set_position(500);
    let mut hud = hud();
    hud.update_from_window(&w);
    let text = hud.format_text();
    let (start, end) = (w.window_start, w.window_start + w.window_size);
    assert!(
        start > 0,
        "the window must have moved for this test to mean anything"
    );
    assert!(text.contains("Total Frames: 1000\n"));
    assert!(text.contains(&format!("Window: {start}-{end} (size=100)\n")));
    assert!(text.contains("Position: 500\n"));
}

#[test]
fn coverage_is_materialized_over_window_size_as_a_percentage() {
    let mut hud = hud();
    hud.update_from_window(&window(1000, 100, 25));
    assert!(hud
        .format_text()
        .contains("Materialized: 25 (25.0% coverage)\n"));
}

#[test]
fn sparse_keyframe_count_is_listed() {
    let mut hud = hud();
    hud.update_from_window(&window(1000, 100, 0));
    assert!(hud.format_text().contains("Sparse Keyframes: 2\n"));
}

#[test]
fn timeline_visualization_is_drawn_sixty_columns_wide() {
    let mut hud = hud();
    hud.update_from_window(&window(1000, 100, 10));
    let text = hud.format_text();
    let bar = lines(&text)
        .into_iter()
        .find(|l| l.starts_with('['))
        .expect("a timeline bar");
    assert_eq!(bar.chars().count(), 62); // '[' + 60 columns + ']'
}

#[test]
fn no_frames_placeholder_runs_straight_into_the_next_header() {
    let text = hud().format_text();
    assert!(text.contains("[No frames]\n--- Materialization ---\n"));
}

#[test]
fn cache_hit_rate_is_hits_over_requests() {
    let mut hud = hud();
    hud.record_access(0, true, 1);
    hud.record_access(1, false, 1);
    hud.record_access(2, false, 1);
    hud.record_access(3, false, 1);
    let text = hud.format_text();
    assert!(text.contains("Total Requests: 4\n"));
    assert!(text.contains("Cache Hits: 1 (25.0%)\n"));
    assert!(text.contains("Cache Misses: 3\n"));
}

#[test]
fn hit_rate_is_zero_not_nan_without_requests() {
    let text = hud().format_text();
    assert!(text.contains("Cache Hits: 0 (0.0%)\n"));
    assert!(!text.contains("NaN"));
}

#[test]
fn all_hits_is_one_hundred_percent() {
    let mut hud = hud();
    hud.record_access(0, true, 1);
    hud.record_access(1, true, 1);
    assert!(hud.format_text().contains("Cache Hits: 2 (100.0%)\n"));
}

#[test]
fn evictions_and_window_moves_are_counted() {
    let mut hud = hud();
    hud.record_eviction(0);
    hud.record_eviction(1);
    hud.record_window_move(10);
    let text = hud.format_text();
    assert!(text.contains("Evictions: 2\n"));
    assert!(text.contains("Window Moves: 1\n"));
}

#[test]
fn access_pattern_section_needs_recorded_events() {
    let mut hud = hud();
    hud.update_from_window(&window(1000, 100, 10));
    assert!(!hud.format_text().contains("--- Access Pattern ---"));

    hud.record_access(0, true, 1);
    let text = hud.format_text();
    assert!(text.contains("--- Access Pattern ---\nRecent: [H]\n"));
}

#[test]
fn a_window_move_alone_is_enough_to_show_the_access_pattern() {
    let mut hud = hud();
    hud.record_window_move(10);
    assert!(hud
        .format_text()
        .contains("--- Access Pattern ---\nRecent: [W]\n"));
}

#[test]
fn density_section_needs_an_update_from_a_window() {
    let mut hud = hud();
    assert!(!hud
        .format_text()
        .contains("--- Materialization Density ---"));

    hud.update_from_window(&window(1000, 100, 10));
    assert!(hud
        .format_text()
        .contains("--- Materialization Density ---\nDensity: ▇_________\n"));
}

#[test]
fn density_section_is_shown_even_for_a_stream_with_no_frames() {
    let mut hud = hud();
    hud.update_from_window(&window(0, 10, 0));
    assert!(hud
        .format_text()
        .contains("--- Materialization Density ---\nDensity: __________\n"));
}

#[test]
fn performance_figures_follow_the_recorded_accesses() {
    let mut hud = hud();
    hud.record_access(0, true, 100);
    hud.record_access(1, true, 300);
    let text = hud.format_text();
    assert!(text.contains("Avg Access Time: 200.0 μs\n"));
    assert!(text.contains("Max Access Time: 300 μs\n"));
    assert!(text.contains("P95 Access Time: 280 μs\n"));
    assert!(text.contains("P99 Access Time: 290 μs\n"));
}

#[test]
fn blocking_operations_are_those_over_sixteen_milliseconds() {
    let mut hud = hud();
    hud.record_access(0, true, 16_000); // exactly 16 ms is not blocking
    hud.record_access(1, true, 16_001);
    let text = hud.format_text();
    assert!(text.contains("Blocking Ops (>16ms): 1 / 2 (50.0%)\n"));
}

#[test]
fn blocking_percentage_is_zero_not_nan_without_operations() {
    let text = hud().format_text();
    assert!(text.contains("Blocking Ops (>16ms): 0 / 0 (0.0%)\n"));
    assert!(!text.contains("NaN"));
}

#[test]
fn window_adjust_average_needs_at_least_one_move() {
    let mut hud = hud();
    assert!(!hud.format_text().contains("Avg Window Adjust"));

    hud.record_window_move(100);
    hud.record_window_move(300);
    hud.record_window_move(500);
    assert!(hud.format_text().ends_with("Avg Window Adjust: 300.0 μs\n"));
}

#[test]
fn window_adjust_average_is_the_last_line_of_the_report() {
    let mut hud = hud();
    hud.record_window_move(10);
    let text = hud.format_text();
    assert_eq!(
        lines(&text).last().copied(),
        Some("Avg Window Adjust: 10.0 μs")
    );
}

#[test]
fn window_adjust_average_only_covers_the_last_thousand_moves() {
    let mut hud = hud();
    for _ in 0..1000 {
        hud.record_window_move(0);
    }
    for _ in 0..1000 {
        hud.record_window_move(10);
    }
    assert!(hud.format_text().ends_with("Avg Window Adjust: 10.0 μs\n"));
}

#[test]
fn report_always_ends_with_a_newline() {
    let mut hud = hud();
    assert!(hud.format_text().ends_with('\n'));
    hud.record_window_move(1);
    assert!(hud.format_text().ends_with('\n'));
}
