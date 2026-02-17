#![allow(dead_code)]
//! Tests for timeline_lanes module

use bitvue_core::timeline::{FrameMarker, TimelineFrame};
use bitvue_core::timeline_lanes::{
    calculate_bpp, estimate_qp_avg, FrameQpStats, FrameSliceStats, Lane, LaneType, MarkerCluster,
    MarkerClustering, TimelineLaneSystem,
};

#[test]
fn test_lane_type_name() {
    assert_eq!(LaneType::QpAvg.name(), "QP Average");
    assert_eq!(LaneType::BitsPerPixel.name(), "Bits per Pixel");
}

#[test]
fn test_lane_type_color() {
    assert_eq!(LaneType::QpAvg.color_hint(), "cyan");
    assert_eq!(LaneType::BitsPerPixel.color_hint(), "magenta");
}

#[test]
fn test_lane_type_secondary_axis() {
    assert!(LaneType::QpAvg.uses_secondary_axis());
    assert!(LaneType::BitsPerPixel.uses_secondary_axis());
    assert!(!LaneType::SliceCount.uses_secondary_axis());
}

#[test]
fn test_lane_creation() {
    let lane = Lane::new(LaneType::QpAvg);
    assert_eq!(lane.lane_type, LaneType::QpAvg);
    assert!(lane.enabled);
    assert_eq!(lane.opacity, 1.0);
}

#[test]
fn test_lane_add_point() {
    let mut lane = Lane::new(LaneType::QpAvg);
    lane.add_point(0, 24.0);
    lane.add_point(1, 25.0);

    assert_eq!(lane.data.len(), 2);
    assert_eq!(lane.get_value(0), Some(24.0));
    assert_eq!(lane.get_value(1), Some(25.0));
    assert_eq!(lane.get_value(2), None);
}

#[test]
fn test_lane_value_range() {
    let mut lane = Lane::new(LaneType::QpAvg);
    lane.add_point(0, 24.0);
    lane.add_point(1, 30.0);
    lane.add_point(2, 20.0);

    let (min, max) = lane.value_range();
    assert_eq!(min, 20.0);
    assert_eq!(max, 30.0);
}

#[test]
fn test_lane_value_range_empty() {
    let lane = Lane::new(LaneType::QpAvg);
    let (min, max) = lane.value_range();
    assert_eq!(min, 0.0);
    assert_eq!(max, 0.0);
}

#[test]
fn test_marker_cluster_single() {
    let cluster = MarkerCluster::single(10, FrameMarker::Key);
    assert_eq!(cluster.center_idx, 10);
    assert_eq!(cluster.start_idx, 10);
    assert_eq!(cluster.end_idx, 10);
    assert_eq!(cluster.count, 1);
}

#[test]
fn test_marker_cluster_can_merge() {
    let cluster = MarkerCluster::single(10, FrameMarker::Key);

    // Within threshold
    assert!(cluster.can_merge(12, 5));
    assert!(cluster.can_merge(8, 5));

    // Outside threshold
    assert!(!cluster.can_merge(20, 5));
    assert!(!cluster.can_merge(2, 5));
}

#[test]
fn test_marker_cluster_merge() {
    let mut cluster = MarkerCluster::single(10, FrameMarker::Bookmark);

    cluster.merge(12, FrameMarker::Bookmark);
    assert_eq!(cluster.count, 2);
    assert_eq!(cluster.start_idx, 10);
    assert_eq!(cluster.end_idx, 12);
    assert_eq!(cluster.center_idx, 11);

    cluster.merge(8, FrameMarker::Bookmark);
    assert_eq!(cluster.count, 3);
    assert_eq!(cluster.start_idx, 8);
    assert_eq!(cluster.end_idx, 12);
    assert_eq!(cluster.center_idx, 10);
}

#[test]
fn test_marker_cluster_merge_critical() {
    let mut cluster = MarkerCluster::single(10, FrameMarker::Bookmark);
    assert_eq!(cluster.primary_type, FrameMarker::Bookmark);

    // Merge critical marker
    cluster.merge(12, FrameMarker::Error);
    assert_eq!(cluster.primary_type, FrameMarker::Error); // Critical wins
}

#[test]
fn test_marker_clustering_no_markers() {
    let markers: Vec<(usize, FrameMarker)> = vec![];
    let clusters = MarkerClustering::cluster(&markers, 5);
    assert_eq!(clusters.len(), 0);
}

#[test]
fn test_marker_clustering_single_marker() {
    let markers = vec![(10, FrameMarker::Key)];
    let clusters = MarkerClustering::cluster(&markers, 5);

    assert_eq!(clusters.len(), 1);
    assert_eq!(clusters[0].count, 1);
    assert_eq!(clusters[0].center_idx, 10);
}

#[test]
fn test_marker_clustering_nearby_markers() {
    let markers = vec![
        (10, FrameMarker::Key),
        (12, FrameMarker::Bookmark),
        (14, FrameMarker::Bookmark),
    ];
    let clusters = MarkerClustering::cluster(&markers, 5);

    // All should be in one cluster (within threshold)
    assert_eq!(clusters.len(), 1);
    assert_eq!(clusters[0].count, 3);
    assert_eq!(clusters[0].start_idx, 10);
    assert_eq!(clusters[0].end_idx, 14);
}

#[test]
fn test_marker_clustering_distant_markers() {
    let markers = vec![
        (10, FrameMarker::Key),
        (50, FrameMarker::Key),
        (100, FrameMarker::Error),
    ];
    let clusters = MarkerClustering::cluster(&markers, 5);

    // Should create 3 separate clusters (too far apart)
    assert_eq!(clusters.len(), 3);
}

#[test]
fn test_marker_clustering_skip_none() {
    let markers = vec![
        (10, FrameMarker::Key),
        (12, FrameMarker::None), // Should be skipped
        (14, FrameMarker::Bookmark),
    ];
    let clusters = MarkerClustering::cluster(&markers, 5);

    // Only 2 markers (None is skipped)
    assert_eq!(clusters.len(), 1);
    assert_eq!(clusters[0].count, 2);
}

#[test]
fn test_marker_clustering_calculate_threshold() {
    let threshold = MarkerClustering::calculate_threshold(1.0, 1000);
    assert_eq!(threshold, 10); // 1% of 1000

    let threshold = MarkerClustering::calculate_threshold(2.0, 1000);
    assert_eq!(threshold, 5); // 1% / 2

    let threshold = MarkerClustering::calculate_threshold(10.0, 1000);
    assert_eq!(threshold, 1); // 1% / 10
}

#[test]
fn test_timeline_lane_system_creation() {
    let system = TimelineLaneSystem::new(1000);
    assert_eq!(system.total_frames, 1000);
    assert_eq!(system.zoom_level, 1.0);
    assert_eq!(system.lanes.len(), 0);
}

#[test]
fn test_timeline_lane_system_add_lane() {
    let mut system = TimelineLaneSystem::new(1000);
    let lane = Lane::new(LaneType::QpAvg);
    system.add_lane(lane);

    assert_eq!(system.lanes.len(), 1);
    assert!(system.get_lane(LaneType::QpAvg).is_some());
}

#[test]
fn test_timeline_lane_system_toggle() {
    let mut system = TimelineLaneSystem::new(1000);
    let lane = Lane::new(LaneType::QpAvg);
    system.add_lane(lane);

    assert!(system.get_lane(LaneType::QpAvg).unwrap().enabled);

    system.toggle_lane(LaneType::QpAvg);
    assert!(!system.get_lane(LaneType::QpAvg).unwrap().enabled);

    system.toggle_lane(LaneType::QpAvg);
    assert!(system.get_lane(LaneType::QpAvg).unwrap().enabled);
}

#[test]
fn test_timeline_lane_system_enabled_lanes() {
    let mut system = TimelineLaneSystem::new(1000);
    system.add_lane(Lane::new(LaneType::QpAvg));
    system.add_lane(Lane::new(LaneType::BitsPerPixel));

    assert_eq!(system.enabled_count(), 2);

    system.set_lane_enabled(LaneType::QpAvg, false);
    assert_eq!(system.enabled_count(), 1);

    let enabled = system.enabled_lanes();
    assert_eq!(enabled.len(), 1);
    assert_eq!(enabled[0].lane_type, LaneType::BitsPerPixel);
}

#[test]
fn test_timeline_lane_system_marker_clusters() {
    let mut system = TimelineLaneSystem::new(1000);

    let markers = vec![
        (10, FrameMarker::Key),
        (50, FrameMarker::Key),
        (100, FrameMarker::Error),
    ];

    system.set_zoom_level(1.0, &markers);

    // At zoom 1.0, threshold should cluster some markers
    assert!(system.marker_clusters.len() > 0);
}

#[test]
fn test_timeline_lane_system_get_cluster_at() {
    let mut system = TimelineLaneSystem::new(1000);

    let markers = vec![(10, FrameMarker::Key), (12, FrameMarker::Bookmark)];

    system.set_zoom_level(1.0, &markers);

    // Should find cluster containing frame 11
    let cluster = system.get_cluster_at(11);
    assert!(cluster.is_some());
    let cluster = cluster.unwrap();
    assert!(cluster.start_idx <= 11 && cluster.end_idx >= 11);
}

// AV1 TimelineLanes viz_core test - Task 17 (S.T4-2.AV1.Timeline.TimelineLanes.impl.viz_core.001)

#[test]
fn test_av1_timeline_lanes_frame_metrics() {
    // AV1 TimelineLanes: User views QP avg and bits-per-pixel lanes for AV1 stream
    let mut system = TimelineLaneSystem::new(10);

    // AV1 TimelineLanes: Create lanes for AV1 metrics
    let mut qp_lane = Lane::new(LaneType::QpAvg);
    let mut bpp_lane = Lane::new(LaneType::BitsPerPixel);

    // AV1 TimelineLanes: KEY_FRAME at idx 0 (18000 bytes, 640x360 = 230400 pixels)
    // QP for KEY_FRAME is typically higher
    qp_lane.add_point(0, 28.0);
    bpp_lane.add_point(0, (18000.0 * 8.0) / 230400.0); // ~0.626 bpp

    // AV1 TimelineLanes: INTER_FRAME at idx 1 (6000 bytes)
    qp_lane.add_point(1, 24.0);
    bpp_lane.add_point(1, (6000.0 * 8.0) / 230400.0); // ~0.208 bpp

    // AV1 TimelineLanes: INTER_FRAME at idx 2 (5500 bytes)
    qp_lane.add_point(2, 25.0);
    bpp_lane.add_point(2, (5500.0 * 8.0) / 230400.0); // ~0.191 bpp

    // AV1 TimelineLanes: INTER_FRAME at idx 3 (6200 bytes)
    qp_lane.add_point(3, 24.5);
    bpp_lane.add_point(3, (6200.0 * 8.0) / 230400.0); // ~0.215 bpp

    system.add_lane(qp_lane);
    system.add_lane(bpp_lane);

    // AV1 TimelineLanes: Verify QP lane configuration
    let qp = system.get_lane(LaneType::QpAvg).unwrap();
    assert_eq!(qp.lane_type, LaneType::QpAvg);
    assert_eq!(qp.lane_type.name(), "QP Average");
    assert_eq!(qp.lane_type.color_hint(), "cyan");
    assert!(qp.lane_type.uses_secondary_axis());
    assert!(qp.enabled);

    // AV1 TimelineLanes: Verify KEY_FRAME has higher QP than INTER frames
    assert_eq!(qp.get_value(0), Some(28.0)); // KEY_FRAME
    assert_eq!(qp.get_value(1), Some(24.0)); // INTER_FRAME
    assert!(qp.get_value(0).unwrap() > qp.get_value(1).unwrap());

    // AV1 TimelineLanes: Verify QP range
    let (qp_min, qp_max) = qp.value_range();
    assert_eq!(qp_min, 24.0);
    assert_eq!(qp_max, 28.0);

    // AV1 TimelineLanes: Verify bits-per-pixel lane
    let bpp = system.get_lane(LaneType::BitsPerPixel).unwrap();
    assert_eq!(bpp.lane_type, LaneType::BitsPerPixel);
    assert_eq!(bpp.lane_type.name(), "Bits per Pixel");
    assert_eq!(bpp.lane_type.color_hint(), "magenta");
    assert!(bpp.lane_type.uses_secondary_axis());

    // AV1 TimelineLanes: Verify KEY_FRAME has higher bpp than INTER frames
    let key_bpp = bpp.get_value(0).unwrap();
    let inter_bpp = bpp.get_value(1).unwrap();
    assert!(key_bpp > inter_bpp);
    assert!(key_bpp > 0.6); // KEY_FRAME > 0.6 bpp
    assert!(inter_bpp < 0.3); // INTER_FRAME < 0.3 bpp

    // AV1 TimelineLanes: Verify both lanes are enabled
    assert_eq!(system.enabled_count(), 2);
    let enabled = system.enabled_lanes();
    assert_eq!(enabled.len(), 2);

    // AV1 TimelineLanes: Toggle lanes
    system.toggle_lane(LaneType::QpAvg);
    assert_eq!(system.enabled_count(), 1);
    system.toggle_lane(LaneType::QpAvg);
    assert_eq!(system.enabled_count(), 2);
}

// AV1 Metrics TimelineLanes viz_core test - Task 24 (S.T4-3.AV1.Metrics.TimelineLanes.impl.viz_core.001)

#[test]
fn test_av1_metrics_timeline_lanes() {
    // AV1 Metrics: User views PSNR-Y lane overlaid on timeline
    let mut system = TimelineLaneSystem::new(10);

    // AV1 Metrics: Create PSNR lane (using QpAvg as proxy)
    let mut psnr_lane = Lane::new(LaneType::QpAvg);

    // KEY_FRAME has best quality
    psnr_lane.add_point(0, 22.0);

    // INTER_FRAMEs have varying quality
    for i in 1..10 {
        psnr_lane.add_point(i, 24.0 + (i as f32 * 0.4));
    }

    system.add_lane(psnr_lane);

    // Verify lane properties
    let lane = system.get_lane(LaneType::QpAvg).unwrap();
    assert_eq!(lane.data.len(), 10);
    assert_eq!(lane.get_value(0), Some(22.0));

    let (min, max) = lane.value_range();
    assert_eq!(min, 22.0);
    assert!(max > 27.0);
}

#[test]
fn test_calculate_bpp() {
    // 1920x1080 = 2,073,600 pixels
    // 10,000 bytes = 80,000 bits
    // BPP = 80,000 / 2,073,600 ≈ 0.0386
    let bpp = calculate_bpp(10_000, 1920, 1080);
    assert!(bpp > 0.038 && bpp < 0.039);

    // Edge case: zero dimensions
    assert_eq!(calculate_bpp(1000, 0, 0), 0.0);
    assert_eq!(calculate_bpp(1000, 100, 0), 0.0);
}

#[test]
fn test_estimate_qp_avg() {
    assert_eq!(estimate_qp_avg(20, 30), 25.0);
    assert_eq!(estimate_qp_avg(0, 51), 25.5);
    assert_eq!(estimate_qp_avg(22, 22), 22.0);
}

#[test]
fn test_populate_bpp_lane() {
    let frames = vec![
        TimelineFrame {
            display_idx: 0,
            size_bytes: 10_000,
            frame_type: "I".to_string(),
            marker: FrameMarker::Key,
            pts: Some(0),
            dts: Some(0),
            is_selected: false,
        },
        TimelineFrame {
            display_idx: 1,
            size_bytes: 2_000,
            frame_type: "P".to_string(),
            marker: FrameMarker::None,
            pts: Some(1000),
            dts: Some(1000),
            is_selected: false,
        },
    ];

    let mut system = TimelineLaneSystem::new(2);
    system.populate_bpp_lane(&frames, 1920, 1080);

    let lane = system.get_lane(LaneType::BitsPerPixel).unwrap();
    assert_eq!(lane.data.len(), 2);

    // First frame (I-frame) should have higher BPP
    let bpp_0 = lane.get_value(0).unwrap();
    let bpp_1 = lane.get_value(1).unwrap();
    assert!(bpp_0 > bpp_1);
}

#[test]
fn test_populate_reorder_lane() {
    let frames = vec![
        TimelineFrame {
            display_idx: 0,
            size_bytes: 10_000,
            frame_type: "I".to_string(),
            marker: FrameMarker::Key,
            pts: Some(0),
            dts: Some(0),
            is_selected: false,
        },
        TimelineFrame {
            display_idx: 1,
            size_bytes: 2_000,
            frame_type: "B".to_string(),
            marker: FrameMarker::None,
            pts: Some(2000), // PTS > DTS = reordering
            dts: Some(1000),
            is_selected: false,
        },
        TimelineFrame {
            display_idx: 2,
            size_bytes: 3_000,
            frame_type: "P".to_string(),
            marker: FrameMarker::None,
            pts: Some(1000),
            dts: Some(2000),
            is_selected: false,
        },
    ];

    let mut system = TimelineLaneSystem::new(3);
    system.populate_reorder_lane(&frames);

    let lane = system.get_lane(LaneType::ReorderMismatch).unwrap();
    assert_eq!(lane.data.len(), 3);

    // Frame 0: PTS == DTS, no mismatch
    assert_eq!(lane.get_value(0), Some(0.0));
    // Frame 1: PTS != DTS, has mismatch
    assert_eq!(lane.get_value(1), Some(1.0));
    // Frame 2: PTS != DTS, has mismatch
    assert_eq!(lane.get_value(2), Some(1.0));
}

// ═══════════════════════════════════════════════════════════════════════════
// QP Lane Tests
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_frame_qp_stats_new() {
    let stats = FrameQpStats::new(5, 24.5);
    assert_eq!(stats.display_idx, 5);
    assert_eq!(stats.qp_avg, 24.5);
    assert_eq!(stats.qp_min, None);
    assert_eq!(stats.qp_max, None);
}

#[test]
fn test_frame_qp_stats_with_range() {
    let stats = FrameQpStats::with_range(3, 26.0, 20, 32);
    assert_eq!(stats.display_idx, 3);
    assert_eq!(stats.qp_avg, 26.0);
    assert_eq!(stats.qp_min, Some(20));
    assert_eq!(stats.qp_max, Some(32));
}

#[test]
fn test_frame_qp_stats_from_range() {
    let stats = FrameQpStats::from_range(7, 20, 30);
    assert_eq!(stats.display_idx, 7);
    assert_eq!(stats.qp_avg, 25.0); // Estimated from min/max
    assert_eq!(stats.qp_min, Some(20));
    assert_eq!(stats.qp_max, Some(30));
}

#[test]
fn test_populate_qp_lane() {
    let qp_data = vec![
        FrameQpStats::new(0, 28.0), // KEY_FRAME (higher QP)
        FrameQpStats::new(1, 24.0), // P-frame
        FrameQpStats::new(2, 26.0), // B-frame
        FrameQpStats::new(3, 25.0), // P-frame
    ];

    let mut system = TimelineLaneSystem::new(4);
    system.populate_qp_lane(&qp_data);

    let lane = system.get_lane(LaneType::QpAvg).unwrap();
    assert_eq!(lane.data.len(), 4);
    assert_eq!(lane.get_value(0), Some(28.0));
    assert_eq!(lane.get_value(1), Some(24.0));
    assert_eq!(lane.get_value(2), Some(26.0));
    assert_eq!(lane.get_value(3), Some(25.0));

    let (min, max) = lane.value_range();
    assert_eq!(min, 24.0);
    assert_eq!(max, 28.0);
}

#[test]
fn test_qp_statistics() {
    let qp_data = vec![
        FrameQpStats::new(0, 20.0),
        FrameQpStats::new(1, 24.0),
        FrameQpStats::new(2, 28.0),
        FrameQpStats::new(3, 28.0),
    ];

    let mut system = TimelineLaneSystem::new(4);
    system.populate_qp_lane(&qp_data);

    let stats = system.qp_statistics().unwrap();
    assert_eq!(stats.min_qp, 20.0);
    assert_eq!(stats.max_qp, 28.0);
    assert_eq!(stats.avg_qp, 25.0); // (20+24+28+28)/4 = 25
    assert_eq!(stats.frame_count, 4);
    assert!(stats.std_dev > 0.0);
}

#[test]
fn test_qp_statistics_summary() {
    let qp_data = vec![
        FrameQpStats::new(0, 22.0),
        FrameQpStats::new(1, 24.0),
        FrameQpStats::new(2, 26.0),
    ];

    let mut system = TimelineLaneSystem::new(3);
    system.populate_qp_lane(&qp_data);

    let stats = system.qp_statistics().unwrap();
    let summary = stats.summary_text();
    assert!(summary.contains("min 22.0"));
    assert!(summary.contains("max 26.0"));
    assert!(summary.contains("avg 24.0"));
    assert!(summary.contains("3 frames"));
}

// ═══════════════════════════════════════════════════════════════════════════
// Slice Lane Tests
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_frame_slice_stats_new() {
    let stats = FrameSliceStats::new(2, 4);
    assert_eq!(stats.display_idx, 2);
    assert_eq!(stats.slice_count, 4);
    assert_eq!(stats.tile_cols, None);
    assert_eq!(stats.tile_rows, None);
}

#[test]
fn test_frame_slice_stats_with_tile_grid() {
    let stats = FrameSliceStats::with_tile_grid(5, 12, 4, 3);
    assert_eq!(stats.display_idx, 5);
    assert_eq!(stats.slice_count, 12);
    assert_eq!(stats.tile_cols, Some(4));
    assert_eq!(stats.tile_rows, Some(3));
}

#[test]
fn test_populate_slice_lane() {
    let slice_data = vec![
        FrameSliceStats::new(0, 1), // Single slice
        FrameSliceStats::new(1, 4), // 4 slices
        FrameSliceStats::new(2, 8), // 8 slices
        FrameSliceStats::new(3, 4), // 4 slices
    ];

    let mut system = TimelineLaneSystem::new(4);
    system.populate_slice_lane(&slice_data);

    let lane = system.get_lane(LaneType::SliceCount).unwrap();
    assert_eq!(lane.data.len(), 4);
    assert_eq!(lane.get_value(0), Some(1.0));
    assert_eq!(lane.get_value(1), Some(4.0));
    assert_eq!(lane.get_value(2), Some(8.0));
    assert_eq!(lane.get_value(3), Some(4.0));
}

#[test]
fn test_slice_statistics() {
    let slice_data = vec![
        FrameSliceStats::new(0, 1), // Single slice
        FrameSliceStats::new(1, 4), // Multi slice
        FrameSliceStats::new(2, 8), // Multi slice
        FrameSliceStats::new(3, 1), // Single slice
    ];

    let mut system = TimelineLaneSystem::new(4);
    system.populate_slice_lane(&slice_data);

    let stats = system.slice_statistics().unwrap();
    assert_eq!(stats.min_slices, 1);
    assert_eq!(stats.max_slices, 8);
    assert!((stats.avg_slices - 3.5).abs() < 0.01); // (1+4+8+1)/4 = 3.5
    assert_eq!(stats.multi_slice_frame_count, 2);
    assert_eq!(stats.frame_count, 4);
}

#[test]
fn test_slice_statistics_summary() {
    let slice_data = vec![
        FrameSliceStats::new(0, 2),
        FrameSliceStats::new(1, 4),
        FrameSliceStats::new(2, 2),
    ];

    let mut system = TimelineLaneSystem::new(3);
    system.populate_slice_lane(&slice_data);

    let stats = system.slice_statistics().unwrap();
    let summary = stats.summary_text();
    assert!(summary.contains("min 2"));
    assert!(summary.contains("max 4"));
    assert!(summary.contains("multi-slice"));
}

#[test]
fn test_qp_lane_replaces_existing() {
    let mut system = TimelineLaneSystem::new(2);

    // First population
    system.populate_qp_lane(&[FrameQpStats::new(0, 20.0)]);
    assert_eq!(system.get_lane(LaneType::QpAvg).unwrap().data.len(), 1);

    // Second population should replace
    system.populate_qp_lane(&[FrameQpStats::new(0, 22.0), FrameQpStats::new(1, 24.0)]);

    let lane = system.get_lane(LaneType::QpAvg).unwrap();
    assert_eq!(lane.data.len(), 2);
    assert_eq!(lane.get_value(0), Some(22.0));
}

#[test]
fn test_slice_lane_replaces_existing() {
    let mut system = TimelineLaneSystem::new(2);

    // First population
    system.populate_slice_lane(&[FrameSliceStats::new(0, 1)]);
    assert_eq!(system.get_lane(LaneType::SliceCount).unwrap().data.len(), 1);

    // Second population should replace
    system.populate_slice_lane(&[FrameSliceStats::new(0, 4), FrameSliceStats::new(1, 8)]);

    let lane = system.get_lane(LaneType::SliceCount).unwrap();
    assert_eq!(lane.data.len(), 2);
    assert_eq!(lane.get_value(0), Some(4.0));
}
