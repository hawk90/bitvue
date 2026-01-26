//! Tests for picture statistics table

use bitvue_core::picture_stats::{
    PictureStatsFilter, PictureStatsRow, PictureStatsSortColumn, PictureStatsTable,
};
use bitvue_core::timeline::{FrameMarker, TimelineFrame};

fn create_test_frames() -> Vec<TimelineFrame> {
    vec![
        TimelineFrame::new(0, 50000, "I".to_string())
            .with_marker(FrameMarker::Key)
            .with_pts(0)
            .with_dts(0),
        TimelineFrame::new(1, 8000, "P".to_string())
            .with_pts(33)
            .with_dts(33),
        TimelineFrame::new(2, 4000, "B".to_string())
            .with_pts(66)
            .with_dts(66),
        TimelineFrame::new(3, 5000, "B".to_string())
            .with_pts(99)
            .with_dts(100), // Reorder
        TimelineFrame::new(4, 10000, "P".to_string())
            .with_pts(132)
            .with_dts(132),
        TimelineFrame::new(5, 55000, "I".to_string())
            .with_marker(FrameMarker::Key)
            .with_pts(165)
            .with_dts(165),
    ]
}

#[test]
fn test_picture_stats_row_creation() {
    let frame = TimelineFrame::new(0, 50000, "I".to_string())
        .with_marker(FrameMarker::Key)
        .with_pts(0)
        .with_dts(0);

    let row = PictureStatsRow::from_timeline_frame(&frame, 1920, 1080);

    assert_eq!(row.display_idx, 0);
    assert_eq!(row.frame_type, "I");
    assert_eq!(row.size_bytes, 50000);
    assert_eq!(row.size_bits, 400000);
    assert!(row.is_keyframe);
    assert!(!row.has_error);
    assert!(row.bpp.is_some());

    // BPP = 400000 bits / (1920 * 1080) pixels ≈ 0.193
    let bpp = row.bpp.unwrap();
    assert!(bpp > 0.19 && bpp < 0.20);
}

#[test]
fn test_picture_stats_table_creation() {
    let frames = create_test_frames();
    let table = PictureStatsTable::new(&frames, 1920, 1080);

    assert_eq!(table.total_count(), 6);
    assert_eq!(table.filtered_count(), 6);
}

#[test]
fn test_sequence_stats() {
    let frames = create_test_frames();
    let table = PictureStatsTable::new(&frames, 1920, 1080);

    let stats = &table.sequence_stats;
    assert_eq!(stats.total_frames, 6);
    assert_eq!(stats.keyframe_count, 2);
    assert_eq!(*stats.frame_type_counts.get("I").unwrap_or(&0), 2);
    assert_eq!(*stats.frame_type_counts.get("P").unwrap_or(&0), 2);
    assert_eq!(*stats.frame_type_counts.get("B").unwrap_or(&0), 2);

    // Total size = 50000 + 8000 + 4000 + 5000 + 10000 + 55000 = 132000
    assert_eq!(stats.total_size_bytes, 132000);

    // GOP size = 6 frames / 2 keyframes = 3.0
    assert!(stats.avg_gop_size.is_some());
    assert!((stats.avg_gop_size.unwrap() - 3.0).abs() < 0.01);
}

#[test]
fn test_filter_keyframes_only() {
    let frames = create_test_frames();
    let mut table = PictureStatsTable::new(&frames, 1920, 1080);

    table.filter.keyframes_only = true;
    let view = table.get_view();

    assert_eq!(view.len(), 2);
    assert!(view.iter().all(|r| r.is_keyframe));
}

#[test]
fn test_filter_by_frame_type() {
    let frames = create_test_frames();
    let mut table = PictureStatsTable::new(&frames, 1920, 1080);

    table.filter.frame_types = vec!["B".to_string()];
    let view = table.get_view();

    assert_eq!(view.len(), 2);
    assert!(view.iter().all(|r| r.frame_type == "B"));
}

#[test]
fn test_filter_by_size() {
    let frames = create_test_frames();
    let mut table = PictureStatsTable::new(&frames, 1920, 1080);

    table.filter.min_size_bytes = Some(10000);
    let view = table.get_view();

    assert_eq!(view.len(), 3); // I(50000), P(10000), I(55000)
}

#[test]
fn test_sort_by_size() {
    let frames = create_test_frames();
    let mut table = PictureStatsTable::new(&frames, 1920, 1080);

    table.set_sort(PictureStatsSortColumn::SizeBytes);
    let view = table.get_view();

    // Ascending: B(4000), B(5000), P(8000), P(10000), I(50000), I(55000)
    assert_eq!(view[0].size_bytes, 4000);
    assert_eq!(view[5].size_bytes, 55000);

    // Toggle to descending
    table.set_sort(PictureStatsSortColumn::SizeBytes);
    let view = table.get_view();

    assert_eq!(view[0].size_bytes, 55000);
    assert_eq!(view[5].size_bytes, 4000);
}

#[test]
fn test_reorder_depth() {
    let frames = create_test_frames();
    let table = PictureStatsTable::new(&frames, 1920, 1080);

    // Frame 3 has pts=99, dts=100, delta=-1
    let stats = &table.sequence_stats;
    assert!(stats.max_reorder_depth.is_some());
    assert_eq!(stats.max_reorder_depth.unwrap(), 1);
}

#[test]
fn test_csv_export() {
    let frame = TimelineFrame::new(0, 50000, "I".to_string())
        .with_marker(FrameMarker::Key)
        .with_pts(0)
        .with_dts(0);

    let row = PictureStatsRow::from_timeline_frame(&frame, 1920, 1080);
    let csv = PictureStatsTable::row_to_csv(&row);

    assert!(csv.starts_with("0,I,50000,400000,"));
    assert!(csv.contains("true,false,false,false")); // is_keyframe, has_error, has_bookmark, is_scene_change
}

#[test]
fn test_empty_table() {
    let table = PictureStatsTable::empty();

    assert_eq!(table.total_count(), 0);
    assert_eq!(table.filtered_count(), 0);
    assert!(table.get_view().is_empty());
}

#[test]
fn test_per_type_averages() {
    let frames = create_test_frames();
    let table = PictureStatsTable::new(&frames, 1920, 1080);

    let stats = &table.sequence_stats;

    // I-frame avg: (50000 + 55000) / 2 = 52500
    assert!(stats.i_frame_avg_size.is_some());
    assert!((stats.i_frame_avg_size.unwrap() - 52500.0).abs() < 0.1);

    // P-frame avg: (8000 + 10000) / 2 = 9000
    assert!(stats.p_frame_avg_size.is_some());
    assert!((stats.p_frame_avg_size.unwrap() - 9000.0).abs() < 0.1);

    // B-frame avg: (4000 + 5000) / 2 = 4500
    assert!(stats.b_frame_avg_size.is_some());
    assert!((stats.b_frame_avg_size.unwrap() - 4500.0).abs() < 0.1);
}
