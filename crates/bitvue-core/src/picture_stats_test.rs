// Picture Stats module tests

// ============================================================================
// Fixtures
// ============================================================================
fn create_test_frame(display_idx: usize, frame_type: &str, size_bytes: u64, marker: crate::timeline::FrameMarker) -> crate::timeline::TimelineFrame {
    crate::timeline::TimelineFrame {
        display_idx,
        frame_type: frame_type.to_string(),
        size_bytes,
        pts: Some(display_idx as u64 * 1000),
        dts: Some(display_idx as u64 * 1000),
        marker,
        is_selected: false,
    }
}

fn create_test_row(display_idx: usize) -> PictureStatsRow {
    PictureStatsRow {
        display_idx,
        frame_type: "I".to_string(),
        size_bytes: 50000,
        size_bits: 400000,
        bpp: Some(0.5),
        pts: Some(5000),
        dts: Some(5000),
        pts_dts_delta: Some(0),
        is_keyframe: true,
        has_error: false,
        has_bookmark: false,
        is_scene_change: false,
        qp_avg: Some(25.0),
        qp_min: Some(20),
        qp_max: Some(30),
    }
}

fn create_test_filter() -> PictureStatsFilter {
    PictureStatsFilter::default()
}

fn create_test_table(frame_count: usize) -> PictureStatsTable {
    let frames: Vec<crate::timeline::TimelineFrame> = (0..frame_count)
        .map(|i| create_test_frame(i, "I", 50000, crate::timeline::FrameMarker::Key))
        .collect();
    PictureStatsTable::new(&frames, 1920, 1080)
}

// ============================================================================
// PictureStatsRow Tests
// ============================================================================
#[cfg(test)]
mod row_tests {
    use super::*;

    #[test]
    fn test_from_timeline_frame_calculates_bpp() {
        let frame = create_test_frame(0, "I", 50000, crate::timeline::FrameMarker::Key);
        let row = PictureStatsRow::from_timeline_frame(&frame, 100, 100);
        assert_eq!(row.display_idx, 0);
        assert_eq!(row.size_bytes, 50000);
        assert!(row.bpp.is_some());
        // BPP = (50000 * 8) / (100 * 100) = 400000 / 10000 = 40
        assert!((row.bpp.unwrap() - 40.0).abs() < 0.01);
    }

    #[test]
    fn test_from_timeline_frame_zero_dimensions() {
        let frame = create_test_frame(0, "I", 50000, crate::timeline::FrameMarker::Key);
        let row = PictureStatsRow::from_timeline_frame(&frame, 0, 0);
        assert!(row.bpp.is_none());
    }

    #[test]
    fn test_with_qp_adds_qp_data() {
        let row = create_test_row(0).with_qp(20, 30);
        assert_eq!(row.qp_min, Some(20));
        assert_eq!(row.qp_max, Some(30));
        assert_eq!(row.qp_avg, Some(25.0));
    }

    #[test]
    fn test_from_timeline_frame_marker_detection() {
        let frame = create_test_frame(0, "I", 50000, crate::timeline::FrameMarker::Key);
        let row = PictureStatsRow::from_timeline_frame(&frame, 100, 100);
        assert!(row.is_keyframe);
        assert!(!row.has_error);
        assert!(!row.has_bookmark);
        assert!(!row.is_scene_change);
    }

    #[test]
    fn test_pts_dts_delta_calculation() {
        let mut frame = create_test_frame(0, "I", 50000, crate::timeline::FrameMarker::Key);
        frame.pts = Some(15000);
        frame.dts = Some(10000);
        let row = PictureStatsRow::from_timeline_frame(&frame, 100, 100);
        assert_eq!(row.pts_dts_delta, Some(5000));
    }
}

// ============================================================================
// SortDirection Tests
// ============================================================================
#[cfg(test)]
mod sort_direction_tests {
    use super::*;

    #[test]
    fn test_toggle_switches_direction() {
        assert_eq!(SortDirection::Ascending.toggle(), SortDirection::Descending);
        assert_eq!(SortDirection::Descending.toggle(), SortDirection::Ascending);
    }

    #[test]
    fn test_default_is_ascending() {
        assert_eq!(SortDirection::default(), SortDirection::Ascending);
    }
}

// ============================================================================
// PictureStatsFilter Tests
// ============================================================================
#[cfg(test)]
mod filter_tests {
    use super::*;

    #[test]
    fn test_default_matches_all() {
        let filter = create_test_filter();
        let row = create_test_row(0);
        assert!(filter.matches(&row));
    }

    #[test]
    fn test_frame_type_filter() {
        let mut filter = create_test_filter();
        filter.frame_types = vec!["I".to_string()];
        let row = create_test_row(0);
        assert!(filter.matches(&row));
    }

    #[test]
    fn test_frame_type_filter_rejects_non_matching() {
        let mut filter = create_test_filter();
        filter.frame_types = vec!["P".to_string()];
        let row = create_test_row(0); // I-frame
        assert!(!filter.matches(&row));
    }

    #[test]
    fn test_keyframes_only() {
        let mut filter = create_test_filter();
        filter.keyframes_only = true;
        let keyframe = create_test_row(0);
        assert!(filter.matches(&keyframe));
    }

    #[test]
    fn test_keyframes_only_rejects_non_keyframe() {
        let mut filter = create_test_filter();
        filter.keyframes_only = true;
        let mut non_keyframe = create_test_row(0);
        non_keyframe.is_keyframe = false;
        assert!(!filter.matches(&non_keyframe));
    }

    #[test]
    fn test_min_size_filter() {
        let mut filter = create_test_filter();
        filter.min_size_bytes = Some(40000);
        let row = create_test_row(0);
        assert!(filter.matches(&row));
    }

    #[test]
    fn test_min_size_filter_rejects_small() {
        let mut filter = create_test_filter();
        filter.min_size_bytes = Some(60000);
        let row = create_test_row(0); // 50000 bytes
        assert!(!filter.matches(&row));
    }

    #[test]
    fn test_max_size_filter() {
        let mut filter = create_test_filter();
        filter.max_size_bytes = Some(60000);
        let row = create_test_row(0);
        assert!(filter.matches(&row));
    }

    #[test]
    fn test_max_size_filter_rejects_large() {
        let mut filter = create_test_filter();
        filter.max_size_bytes = Some(40000);
        let row = create_test_row(0); // 50000 bytes
        assert!(!filter.matches(&row));
    }

    #[test]
    fn test_bpp_filter() {
        let mut filter = create_test_filter();
        filter.min_bpp = Some(0.3);
        filter.max_bpp = Some(0.7);
        let row = create_test_row(0); // bpp = 0.5
        assert!(filter.matches(&row));
    }

    #[test]
    fn test_bpp_filter_rejects_outside_range() {
        let mut filter = create_test_filter();
        filter.min_bpp = Some(1.0);
        let row = create_test_row(0); // bpp = 0.5
        assert!(!filter.matches(&row));
    }

    #[test]
    fn test_bpp_filter_handles_none() {
        let mut filter = create_test_filter();
        filter.min_bpp = Some(0.3);
        let mut row = create_test_row(0);
        row.bpp = None;
        assert!(!filter.matches(&row));
    }
}

// ============================================================================
// SequenceStats Tests
// ============================================================================
#[cfg(test)]
mod sequence_stats_tests {
    use super::*;

    #[test]
    fn test_from_rows_empty_returns_default() {
        let stats = SequenceStats::from_rows(&[]);
        assert_eq!(stats.total_frames, 0);
    }

    #[test]
    fn test_from_rows_calculates_frame_counts() {
        let rows = vec![
            create_test_row(0),
            create_test_row(1),
        ];
        let stats = SequenceStats::from_rows(&rows);
        assert_eq!(stats.total_frames, 2);
    }

    #[test]
    fn test_from_rows_calculates_type_distribution() {
        let mut rows = vec![create_test_row(0)];
        rows[0].frame_type = "I".to_string();
        let stats = SequenceStats::from_rows(&rows);
        assert_eq!(*stats.frame_type_counts.get("I").unwrap_or(&0), 1);
    }

    #[test]
    fn test_from_rows_calculates_size_stats() {
        let rows = vec![create_test_row(0)];
        let stats = SequenceStats::from_rows(&rows);
        assert_eq!(stats.total_size_bytes, 50000);
        assert_eq!(stats.avg_size_bytes, 50000.0);
        assert_eq!(stats.min_size_bytes, 50000);
        assert_eq!(stats.max_size_bytes, 50000);
    }

    #[test]
    fn test_from_rows_calculates_marker_counts() {
        let mut rows = vec![
            create_test_row(0),
            create_test_row(1),
        ];
        rows[1].is_keyframe = false;  // Only first row is keyframe
        rows[1].has_error = true;      // Second row has error
        let stats = SequenceStats::from_rows(&rows);
        assert_eq!(stats.keyframe_count, 1);
        assert_eq!(stats.error_count, 1);
    }

    #[test]
    fn test_from_rows_calculates_avg_bpp() {
        let rows = vec![create_test_row(0)];
        let stats = SequenceStats::from_rows(&rows);
        assert!(stats.avg_bpp.is_some());
        assert!((stats.avg_bpp.unwrap() - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_from_rows_handles_no_bpp() {
        let mut row = create_test_row(0);
        row.bpp = None;
        let stats = SequenceStats::from_rows(&[row]);
        assert!(stats.avg_bpp.is_none());
    }

    #[test]
    fn test_from_rows_calculates_reorder_depth() {
        let mut row = create_test_row(0);
        row.pts_dts_delta = Some(5);
        let stats = SequenceStats::from_rows(&[row]);
        assert_eq!(stats.max_reorder_depth, Some(5));
    }

    #[test]
    fn test_from_rows_calculates_gop_size() {
        let rows = vec![
            create_test_row(0),
            create_test_row(1),
            create_test_row(2),
        ];
        let mut stats = SequenceStats::from_rows(&rows);
        stats.keyframe_count = 3;
        stats.total_frames = 3;
        let avg_gop = stats.total_frames as f32 / stats.keyframe_count as f32;
        assert_eq!(stats.avg_gop_size, Some(avg_gop));
    }
}

// ============================================================================
// PictureStatsTable Tests
// ============================================================================
#[cfg(test)]
mod table_tests {
    use super::*;

    #[test]
    fn test_new_creates_table() {
        let table = create_test_table(5);
        assert_eq!(table.total_count(), 5);
        assert_eq!(table.filtered_count(), 5);
    }

    #[test]
    fn test_empty_creates_empty_table() {
        let table = PictureStatsTable::empty();
        assert_eq!(table.total_count(), 0);
    }

    #[test]
    fn test_get_view_returns_all_rows_unfiltered() {
        let table = create_test_table(5);
        let view = table.get_view();
        assert_eq!(view.len(), 5);
    }

    #[test]
    fn test_get_view_respects_filter() {
        let mut table = create_test_table(5);
        table.filter.errors_only = true;
        let view = table.get_view();
        assert_eq!(view.len(), 0); // No error frames
    }

    #[test]
    fn test_set_sort_toggles_same_column() {
        let mut table = create_test_table(5);
        table.set_sort(PictureStatsSortColumn::SizeBytes);
        table.set_sort(PictureStatsSortColumn::SizeBytes);
        assert_eq!(table.sort_direction, SortDirection::Descending);
    }

    #[test]
    fn test_set_sort_changes_column() {
        let mut table = create_test_table(5);
        table.set_sort(PictureStatsSortColumn::FrameType);
        assert_eq!(table.sort_column, PictureStatsSortColumn::FrameType);
        assert_eq!(table.sort_direction, SortDirection::Ascending);
    }

    #[test]
    fn test_get_row_by_display_idx() {
        let table = create_test_table(5);
        let row = table.get_row(3);
        assert!(row.is_some());
        assert_eq!(row.unwrap().display_idx, 3);
    }

    #[test]
    fn test_get_row_not_found() {
        let table = create_test_table(5);
        let row = table.get_row(10);
        assert!(row.is_none());
    }

    #[test]
    fn test_update_replaces_rows() {
        let mut table = create_test_table(5);
        table.update(&[create_test_frame(0, "P", 40000, crate::timeline::FrameMarker::Bookmark)]);
        assert_eq!(table.total_count(), 1);
    }

    #[test]
    fn test_row_to_csv() {
        let row = create_test_row(0);
        let csv = PictureStatsTable::row_to_csv(&row);
        assert!(csv.contains("0")); // display_idx
        assert!(csv.contains("I")); // frame_type
    }

    #[test]
    fn test_csv_header() {
        let header = PictureStatsTable::csv_header();
        assert!(header.contains("display_idx"));
        assert!(header.contains("frame_type"));
    }
}
