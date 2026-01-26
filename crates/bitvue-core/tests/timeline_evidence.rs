//! Tests for timeline evidence manager

use bitvue_core::evidence::{DecodeArtifactType, VizElementType};
use bitvue_core::timeline::{FrameMarker, TimelineFrame};
use bitvue_core::timeline_evidence::TimelineEvidenceManager;
use bitvue_core::BitRange;

fn create_test_frame(display_idx: usize) -> TimelineFrame {
    TimelineFrame::new(
        display_idx,
        1024,
        if display_idx % 30 == 0 { "I" } else { "P" }.to_string(),
    )
    .with_marker(if display_idx % 30 == 0 {
        FrameMarker::Key
    } else {
        FrameMarker::None
    })
    .with_pts(display_idx as u64 * 1000)
    .with_dts(display_idx as u64 * 1000)
}

// Helper for creating custom timeline frames for UX tests
fn create_test_timeline_frame(
    display_idx: usize,
    _byte_offset: u64,
    size_bytes: u64,
    is_keyframe: bool,
    pts: Option<u64>,
    dts: Option<u64>,
) -> TimelineFrame {
    let mut frame = TimelineFrame::new(
        display_idx,
        size_bytes,
        if is_keyframe { "I" } else { "P" }.to_string(),
    )
    .with_marker(if is_keyframe {
        FrameMarker::Key
    } else {
        FrameMarker::None
    });

    if let Some(pts_val) = pts {
        frame = frame.with_pts(pts_val);
    }
    if let Some(dts_val) = dts {
        frame = frame.with_dts(dts_val);
    }

    frame
}

#[test]
fn test_create_frame_evidence() {
    let mut manager = TimelineEvidenceManager::default();

    let frame = create_test_frame(0);
    let bit_range = BitRange::new(0, 1024 * 8);

    let evidence = manager.create_frame_evidence(&frame, bit_range, 1024);

    assert_eq!(evidence.display_idx, 0);
    assert!(!evidence.bit_offset_id.is_empty());
    assert!(!evidence.syntax_id.is_empty());
    assert!(!evidence.decode_id.is_empty());
    assert!(!evidence.timeline_viz_id.is_empty());
}

#[test]
fn test_get_frame_evidence() {
    let mut manager = TimelineEvidenceManager::default();

    let frame = create_test_frame(5);
    let bit_range = BitRange::new(5000, 6000);

    manager.create_frame_evidence(&frame, bit_range, 1024);

    let retrieved = manager.get_frame_evidence(5);
    assert!(retrieved.is_some());
    assert_eq!(retrieved.unwrap().display_idx, 5);

    let missing = manager.get_frame_evidence(10);
    assert!(missing.is_none());
}

#[test]
fn test_get_frame_bit_range() {
    let mut manager = TimelineEvidenceManager::default();

    let frame = create_test_frame(0);
    let bit_range = BitRange::new(1000, 2000);

    manager.create_frame_evidence(&frame, bit_range.clone(), 1024);

    let retrieved_range = manager.get_frame_bit_range(0);
    assert!(retrieved_range.is_some());

    let range = retrieved_range.unwrap();
    assert_eq!(range.start_bit, 1000);
    assert_eq!(range.end_bit, 2000);
}

#[test]
fn test_find_keyframes() {
    let mut manager = TimelineEvidenceManager::default();

    // Create frames: 0, 30, 60 are keyframes
    for i in 0..90 {
        let frame = create_test_frame(i);
        let bit_range = BitRange::new(i as u64 * 1000, (i as u64 + 1) * 1000);
        manager.create_frame_evidence(&frame, bit_range, 1024);
    }

    let keyframes = manager.get_keyframe_indices();
    assert_eq!(keyframes.len(), 3);
    assert!(keyframes.contains(&0));
    assert!(keyframes.contains(&30));
    assert!(keyframes.contains(&60));
}

#[test]
fn test_find_frame_at_temporal_pos() {
    let manager = TimelineEvidenceManager::default();

    // Test with 100 frames
    assert_eq!(manager.find_frame_at_temporal_pos(0.0, 100), Some(0));
    assert_eq!(manager.find_frame_at_temporal_pos(0.5, 100), Some(50)); // Rounded from 49.5
    assert_eq!(manager.find_frame_at_temporal_pos(1.0, 100), Some(99));

    // Test empty
    assert_eq!(manager.find_frame_at_temporal_pos(0.5, 0), None);
}

#[test]
fn test_evidence_chain_traversal() {
    let mut manager = TimelineEvidenceManager::default();

    let frame = create_test_frame(0);
    let bit_range = BitRange::new(0, 1000);

    let evidence = manager.create_frame_evidence(&frame, bit_range, 1024);

    // Forward traversal: bit_offset → syntax
    let syntax_ev = manager
        .evidence_chain()
        .syntax_index
        .find_by_id(&evidence.syntax_id);
    assert!(syntax_ev.is_some());
    assert_eq!(syntax_ev.unwrap().bit_offset_link, evidence.bit_offset_id);

    // syntax → decode
    let decode_ev = manager
        .evidence_chain()
        .decode_index
        .find_by_id(&evidence.decode_id);
    assert!(decode_ev.is_some());
    assert_eq!(decode_ev.unwrap().syntax_link, evidence.syntax_id);

    // decode → viz
    let viz_ev = manager
        .evidence_chain()
        .viz_index
        .find_by_id(&evidence.timeline_viz_id);
    assert!(viz_ev.is_some());
    assert_eq!(viz_ev.unwrap().decode_link, evidence.decode_id);
}

#[test]
fn test_clear_evidence() {
    let mut manager = TimelineEvidenceManager::default();

    // Add frames
    for i in 0..10 {
        let frame = create_test_frame(i);
        let bit_range = BitRange::new(i as u64 * 1000, (i as u64 + 1) * 1000);
        manager.create_frame_evidence(&frame, bit_range, 1024);
    }

    assert_eq!(manager.frame_count(), 10);

    manager.clear();

    assert_eq!(manager.frame_count(), 0);
    assert!(manager.get_frame_evidence(0).is_none());
}

#[test]
fn test_frame_metadata_in_evidence() {
    let mut manager = TimelineEvidenceManager::default();

    let frame = create_test_frame(30); // Keyframe
    let bit_range = BitRange::new(30000, 31000);

    let evidence = manager.create_frame_evidence(&frame, bit_range, 1024);

    // Check syntax evidence has metadata
    let syntax_ev = manager
        .evidence_chain()
        .syntax_index
        .find_by_id(&evidence.syntax_id)
        .unwrap();

    assert_eq!(syntax_ev.metadata.get("frame_type"), Some(&"I".to_string()));
    assert_eq!(
        syntax_ev.metadata.get("size_bytes"),
        Some(&"1024".to_string())
    );
    assert_eq!(syntax_ev.metadata.get("pts"), Some(&"30000".to_string()));

    // Check viz evidence has visual properties
    let viz_ev = manager
        .evidence_chain()
        .viz_index
        .find_by_id(&evidence.timeline_viz_id)
        .unwrap();

    assert_eq!(
        viz_ev.visual_properties.get("frame_type"),
        Some(&"I".to_string())
    );
    assert_eq!(
        viz_ev.visual_properties.get("is_keyframe"),
        Some(&"true".to_string())
    );
}

// UX Core evidence chain integration tests
// Deliverable: evidence_chain_01_bit_offset:UX:Core:ALL:evidence_chain

#[test]
fn test_ux_timeline_interaction_evidence_trace() {
    let mut manager = TimelineEvidenceManager::default();

    // Simulate user clicking on timeline at display_idx 5
    let timeline_frame = create_test_timeline_frame(5, 5000, 2048, true, Some(50000), Some(50000));
    let bit_range = BitRange::new(5000 * 8, (5000 + 2048) * 8);
    let evidence = manager.create_frame_evidence(&timeline_frame, bit_range, 2048);

    // UX Core: Trace click event back to bit offset
    let bit_range = manager.trace_to_bit_offset(5).unwrap();
    assert_eq!(bit_range.byte_offset(), 5000);

    // UX Core: Verify full evidence chain linkage
    let chain = manager.evidence_chain();

    // Bit offset → Syntax
    let bit_ev = chain
        .bit_offset_index
        .find_by_id(&evidence.bit_offset_id)
        .unwrap();
    assert_eq!(bit_ev.syntax_link.as_ref().unwrap(), &evidence.syntax_id);

    // Syntax → Decode
    let syntax_ev = chain.syntax_index.find_by_id(&evidence.syntax_id).unwrap();
    assert_eq!(syntax_ev.decode_link.as_ref().unwrap(), &evidence.decode_id);

    // Decode → Viz (Timeline)
    let decode_ev = chain.decode_index.find_by_id(&evidence.decode_id).unwrap();
    assert_eq!(
        decode_ev.viz_link.as_ref().unwrap(),
        &evidence.timeline_viz_id
    );
}

#[test]
fn test_ux_multi_frame_selection_evidence() {
    let mut manager = TimelineEvidenceManager::default();

    // Simulate user selecting multiple frames on timeline
    for i in 0..5 {
        let frame = create_test_timeline_frame(
            i,
            i as u64 * 1000,
            1024,
            i % 2 == 0,
            Some(i as u64 * 33333),
            Some(i as u64 * 33333),
        );
        let bit_range = BitRange::new(i as u64 * 1000 * 8, (i as u64 + 1) * 1000 * 8);
        manager.create_frame_evidence(&frame, bit_range, 1024);
    }

    // UX Core: Verify all selections have evidence trails
    for i in 0..5 {
        let bit_range = manager.trace_to_bit_offset(i).unwrap();
        assert_eq!(bit_range.byte_offset(), i as u64 * 1000);

        // Verify bidirectional lookup
        let display_idx = manager.trace_to_display_idx(i as u64 * 1000).unwrap();
        assert_eq!(display_idx, i);
    }
}

#[test]
fn test_ux_evidence_metadata_preservation() {
    let mut manager = TimelineEvidenceManager::default();

    let frame = create_test_timeline_frame(0, 0, 4096, true, Some(100000), Some(95000));
    let bit_range = BitRange::new(0, 4096 * 8);
    let evidence = manager.create_frame_evidence(&frame, bit_range, 4096);

    // UX Core: Verify all metadata is preserved through evidence chain
    let chain = manager.evidence_chain();

    // Check syntax metadata
    let syntax_ev = chain.syntax_index.find_by_id(&evidence.syntax_id).unwrap();
    assert_eq!(
        syntax_ev.metadata.get("is_keyframe"),
        Some(&"true".to_string())
    );
    assert_eq!(
        syntax_ev.metadata.get("size_bytes"),
        Some(&"4096".to_string())
    );
    assert_eq!(syntax_ev.metadata.get("pts"), Some(&"100000".to_string()));

    // Check viz metadata
    let viz_ev = chain
        .viz_index
        .find_by_id(&evidence.timeline_viz_id)
        .unwrap();
    assert_eq!(
        viz_ev.visual_properties.get("is_keyframe"),
        Some(&"true".to_string())
    );
    assert_eq!(viz_ev.element_type, VizElementType::TimelineLane);
}

// UX Timeline evidence chain integration tests
// Deliverable: evidence_chain_01_bit_offset:UX:Timeline:ALL:evidence_chain

#[test]
fn test_ux_timeline_scrub_evidence_tracking() {
    let mut manager = TimelineEvidenceManager::default();

    // UX Timeline: User scrubs through frames 0-9 rapidly
    for i in 0..10 {
        let frame = create_test_frame(i);
        let bit_range = BitRange::new(i as u64 * 4096 * 8, (i as u64 + 1) * 4096 * 8);
        manager.create_frame_evidence(&frame, bit_range, 4096);
    }

    // UX Timeline: Verify evidence exists for each scrubbed frame
    for i in 0..10 {
        let evidence = manager.get_frame_evidence(i).unwrap();
        assert_eq!(evidence.display_idx, i);

        // UX Timeline: Verify evidence chain linkage
        let bit_range = manager.trace_to_bit_offset(i).unwrap();
        assert_eq!(bit_range.byte_offset(), i as u64 * 4096);
    }
}

#[test]
fn test_ux_timeline_keyframe_jump_evidence() {
    let mut manager = TimelineEvidenceManager::default();

    // UX Timeline: Add keyframes at indices 0, 30, 60, 90
    for i in &[0, 30, 60, 90] {
        let frame = create_test_frame(*i);
        let bit_range = BitRange::new(*i as u64 * 1000 * 8, (*i as u64 + 1) * 1000 * 8);
        manager.create_frame_evidence(&frame, bit_range, 1000);
    }

    // UX Timeline: User jumps to keyframe 60
    let evidence = manager.get_frame_evidence(60).unwrap();
    assert_eq!(evidence.display_idx, 60);

    // UX Timeline: Verify can trace back to bit offset
    let bit_range = manager.trace_to_bit_offset(60).unwrap();
    assert_eq!(bit_range.byte_offset(), 60000);

    // UX Timeline: Verify keyframe metadata preserved
    let chain = manager.evidence_chain();
    let syntax_ev = chain.syntax_index.find_by_id(&evidence.syntax_id).unwrap();
    assert_eq!(
        syntax_ev.metadata.get("is_keyframe"),
        Some(&"true".to_string())
    );
}

#[test]
fn test_ux_timeline_error_marker_evidence() {
    let mut manager = TimelineEvidenceManager::default();

    // UX Timeline: Frame 5 has decode error
    let mut frame = create_test_frame(5);
    frame.marker = FrameMarker::Error;

    let bit_range = BitRange::new(5000 * 8, 6000 * 8);
    let evidence = manager.create_frame_evidence(&frame, bit_range, 1000);

    // UX Timeline: Verify error marker visible in evidence chain
    let chain = manager.evidence_chain();
    let viz_ev = chain
        .viz_index
        .find_by_id(&evidence.timeline_viz_id)
        .unwrap();
    assert_eq!(
        viz_ev.visual_properties.get("marker"),
        Some(&"Error".to_string())
    );
}

#[test]
fn test_ux_timeline_bookmark_evidence() {
    let mut manager = TimelineEvidenceManager::default();

    // UX Timeline: User bookmarks frame 42
    let mut frame = create_test_frame(42);
    frame.marker = FrameMarker::Bookmark;

    let bit_range = BitRange::new(42000 * 8, 43000 * 8);
    let evidence = manager.create_frame_evidence(&frame, bit_range, 1000);

    // UX Timeline: Verify bookmark preserved in evidence
    let chain = manager.evidence_chain();
    let viz_ev = chain
        .viz_index
        .find_by_id(&evidence.timeline_viz_id)
        .unwrap();
    assert_eq!(
        viz_ev.visual_properties.get("marker"),
        Some(&"Bookmark".to_string())
    );

    // UX Timeline: Can navigate from bookmark back to bitstream
    let bit_range = manager.trace_to_bit_offset(42).unwrap();
    assert_eq!(bit_range.byte_offset(), 42000);
}

#[test]
fn test_ux_timeline_range_selection_evidence() {
    let mut manager = TimelineEvidenceManager::default();

    // UX Timeline: User selects frame range [10..15] for batch export
    for i in 10..15 {
        let frame = create_test_frame(i);
        let bit_range = BitRange::new(i as u64 * 2000 * 8, (i as u64 + 1) * 2000 * 8);
        manager.create_frame_evidence(&frame, bit_range, 2000);
    }

    // UX Timeline: Verify each selected frame has evidence trail
    for i in 10..15 {
        let evidence = manager.get_frame_evidence(i).unwrap();
        assert_eq!(evidence.display_idx, i);

        // UX Timeline: Bidirectional navigation works
        let bit_range = manager.trace_to_bit_offset(i).unwrap();
        let display_idx = manager
            .trace_to_display_idx(bit_range.byte_offset())
            .unwrap();
        assert_eq!(display_idx, i);
    }
}

// AV1 Timeline evidence chain test - Task 16 (S.T4-2.AV1.Timeline.Timeline.impl.evidence_chain.001)

#[test]
fn test_av1_timeline_keyframe_obu_evidence() {
    // AV1 Timeline: User clicks on KEY_FRAME in timeline to trace to OBU
    let mut manager = TimelineEvidenceManager::default();

    // AV1 Timeline: Create evidence for KEY_FRAME with OBU-level detail
    let mut key_frame = create_test_frame(0);
    key_frame.marker = FrameMarker::Key;

    // AV1 Timeline: KEY_FRAME OBU at byte offset 0, size 18000 bytes
    let key_bit_range = BitRange::new(0, 18000 * 8);
    let key_evidence = manager.create_frame_evidence(&key_frame, key_bit_range, 18000);

    // AV1 Timeline: INTER_FRAME at byte offset 18000
    let inter_frame = create_test_frame(1);
    let inter_bit_range = BitRange::new(18000 * 8, 24000 * 8);
    let inter_evidence = manager.create_frame_evidence(&inter_frame, inter_bit_range, 6000);

    // AV1 Timeline: User clicks on KEY_FRAME in timeline
    let chain = manager.evidence_chain();

    // AV1 Timeline: Viz layer shows frame type
    let key_viz = chain
        .viz_index
        .find_by_id(&key_evidence.timeline_viz_id)
        .unwrap();
    assert_eq!(key_viz.element_type, VizElementType::TimelineLane);
    assert_eq!(
        key_viz.visual_properties.get("marker"),
        Some(&"Key".to_string())
    );

    // AV1 Timeline: Decode layer identifies frame
    let key_decode = chain
        .decode_index
        .find_by_id(&key_evidence.decode_id)
        .unwrap();
    assert_eq!(key_decode.display_idx, Some(0));
    assert_eq!(key_decode.artifact_type, DecodeArtifactType::YuvFrame);

    // AV1 Timeline: Syntax layer points to OBU
    let key_syntax = chain
        .syntax_index
        .find_by_id(&key_evidence.syntax_id)
        .unwrap();
    assert_eq!(
        key_syntax.node_type,
        bitvue_core::evidence::SyntaxNodeType::FrameHeader
    );
    assert_eq!(
        key_syntax.metadata.get("display_idx"),
        Some(&"0".to_string())
    );

    // AV1 Timeline: Bit offset layer shows OBU location in file
    let key_bit_offset = chain
        .bit_offset_index
        .find_by_id(&key_evidence.bit_offset_id)
        .unwrap();
    assert_eq!(key_bit_offset.bit_range.byte_offset(), 0);
    assert_eq!(key_bit_offset.bit_range.size_bits(), 18000 * 8);

    // AV1 Timeline: Verify bidirectional navigation works
    // From timeline frame 0 → OBU byte offset 0
    let traced_bit_range = manager.trace_to_bit_offset(0).unwrap();
    assert_eq!(traced_bit_range.byte_offset(), 0);

    // From OBU byte offset 0 → timeline frame 0
    let traced_display_idx = manager.trace_to_display_idx(0).unwrap();
    assert_eq!(traced_display_idx, 0);

    // AV1 Timeline: INTER_FRAME has different offset
    let inter_viz = chain
        .viz_index
        .find_by_id(&inter_evidence.timeline_viz_id)
        .unwrap();
    assert_eq!(
        inter_viz.visual_properties.get("display_idx"),
        Some(&"1".to_string())
    );

    let inter_bit_offset = chain
        .bit_offset_index
        .find_by_id(&inter_evidence.bit_offset_id)
        .unwrap();
    assert_eq!(inter_bit_offset.bit_range.byte_offset(), 18000);
}

// AV1 TimelineLanes evidence chain test - Task 18 (S.T4-2.AV1.Timeline.TimelineLanes.impl.evidence_chain.001)

#[test]
fn test_av1_timeline_lanes_metrics_evidence() {
    // AV1 TimelineLanes: User clicks on lane data point to trace to frame OBU
    let mut manager = TimelineEvidenceManager::default();

    // AV1 TimelineLanes: Create evidence for frames with QP/BPP metrics
    let mut key_frame = create_test_frame(0);
    key_frame.marker = FrameMarker::Key;
    let key_bit_range = BitRange::new(0, 18000 * 8);
    let key_evidence = manager.create_frame_evidence(&key_frame, key_bit_range, 18000);

    let inter_frame = create_test_frame(1);
    let inter_bit_range = BitRange::new(18000 * 8, 24000 * 8);
    let inter_evidence = manager.create_frame_evidence(&inter_frame, inter_bit_range, 6000);

    // AV1 TimelineLanes: User views QP lane (KEY_FRAME QP=28, INTER_FRAME QP=24)
    // Each lane data point links to frame evidence
    let chain = manager.evidence_chain();

    // AV1 TimelineLanes: QP lane point for frame 0 links to KEY_FRAME evidence
    let key_viz = chain
        .viz_index
        .find_by_id(&key_evidence.timeline_viz_id)
        .unwrap();
    assert_eq!(key_viz.element_type, VizElementType::TimelineLane);
    assert_eq!(key_viz.display_idx, Some(0));
    assert_eq!(
        key_viz.visual_properties.get("marker"),
        Some(&"Key".to_string())
    );

    // AV1 TimelineLanes: Trace from lane data point (frame 0) → decode → syntax → bit offset
    let key_decode = chain
        .decode_index
        .find_by_id(&key_evidence.decode_id)
        .unwrap();
    assert_eq!(key_decode.display_idx, Some(0));

    let key_syntax = chain
        .syntax_index
        .find_by_id(&key_evidence.syntax_id)
        .unwrap();
    assert_eq!(
        key_syntax.metadata.get("display_idx"),
        Some(&"0".to_string())
    );

    let key_bit_offset = chain
        .bit_offset_index
        .find_by_id(&key_evidence.bit_offset_id)
        .unwrap();
    assert_eq!(key_bit_offset.bit_range.byte_offset(), 0);
    assert_eq!(key_bit_offset.bit_range.size_bits(), 18000 * 8);

    // AV1 TimelineLanes: QP lane point for frame 1 links to INTER_FRAME evidence
    let inter_viz = chain
        .viz_index
        .find_by_id(&inter_evidence.timeline_viz_id)
        .unwrap();
    assert_eq!(inter_viz.element_type, VizElementType::TimelineLane);
    assert_eq!(inter_viz.display_idx, Some(1));

    // AV1 TimelineLanes: Trace from lane data point (frame 1) → bit offset
    let inter_bit_offset = chain
        .bit_offset_index
        .find_by_id(&inter_evidence.bit_offset_id)
        .unwrap();
    assert_eq!(inter_bit_offset.bit_range.byte_offset(), 18000);

    // AV1 TimelineLanes: Bidirectional navigation
    // From lane data point (frame 0) → OBU byte offset 0
    let traced_bit_range = manager.trace_to_bit_offset(0).unwrap();
    assert_eq!(traced_bit_range.byte_offset(), 0);

    // From OBU byte offset 0 → lane data point (frame 0)
    let traced_display_idx = manager.trace_to_display_idx(0).unwrap();
    assert_eq!(traced_display_idx, 0);

    // AV1 TimelineLanes: From lane data point (frame 1) → OBU byte offset 18000
    let traced_bit_range = manager.trace_to_bit_offset(1).unwrap();
    assert_eq!(traced_bit_range.byte_offset(), 18000);

    // From OBU byte offset 18000 → lane data point (frame 1)
    let traced_display_idx = manager.trace_to_display_idx(18000).unwrap();
    assert_eq!(traced_display_idx, 1);
}

// AV1 TimelineBands evidence chain test - Task 20 (S.T4-2.AV1.Timeline.TimelineBands.impl.evidence_chain.001)

#[test]
fn test_av1_timeline_bands_diagnostic_evidence() {
    // AV1 TimelineBands: User clicks on diagnostic band entry to trace to frame
    let mut manager = TimelineEvidenceManager::default();

    // AV1 TimelineBands: Create evidence for frames with diagnostic information
    // Frame 0: KEY_FRAME with scene change
    let mut key_frame = create_test_frame(0);
    key_frame.marker = FrameMarker::Key;
    let key_bit_range = BitRange::new(0, 18000 * 8);
    let key_evidence = manager.create_frame_evidence(&key_frame, key_bit_range, 18000);

    // Frame 1: B-frame with reorder mismatch (PTS=1000, DTS=3000)
    let reorder_frame = create_test_frame(1);
    let reorder_bit_range = BitRange::new(18000 * 8, 24000 * 8);
    let reorder_evidence = manager.create_frame_evidence(&reorder_frame, reorder_bit_range, 6000);

    // Frame 10: Frame with error burst
    let error_frame = create_test_frame(10);
    let error_bit_range = BitRange::new(200000 * 8, 205500 * 8);
    let error_evidence = manager.create_frame_evidence(&error_frame, error_bit_range, 5500);

    // AV1 TimelineBands: User clicks on scene change marker at frame 0
    let chain = manager.evidence_chain();

    // AV1 TimelineBands: Scene change diagnostic links to KEY_FRAME evidence
    let key_viz = chain
        .viz_index
        .find_by_id(&key_evidence.timeline_viz_id)
        .unwrap();
    assert_eq!(key_viz.element_type, VizElementType::TimelineLane);
    assert_eq!(key_viz.display_idx, Some(0));
    assert_eq!(
        key_viz.visual_properties.get("marker"),
        Some(&"Key".to_string())
    );

    // AV1 TimelineBands: Trace from scene change → syntax → bit offset
    let key_syntax = chain
        .syntax_index
        .find_by_id(&key_evidence.syntax_id)
        .unwrap();
    assert_eq!(
        key_syntax.metadata.get("display_idx"),
        Some(&"0".to_string())
    );
    assert_eq!(
        key_syntax.metadata.get("is_keyframe"),
        Some(&"true".to_string())
    );

    let key_bit_offset = chain
        .bit_offset_index
        .find_by_id(&key_evidence.bit_offset_id)
        .unwrap();
    assert_eq!(key_bit_offset.bit_range.byte_offset(), 0);

    // AV1 TimelineBands: User clicks on reorder mismatch marker at frame 1
    let reorder_viz = chain
        .viz_index
        .find_by_id(&reorder_evidence.timeline_viz_id)
        .unwrap();
    assert_eq!(reorder_viz.element_type, VizElementType::TimelineLane);
    assert_eq!(reorder_viz.display_idx, Some(1));

    // AV1 TimelineBands: Trace from reorder mismatch → decode → syntax → bit offset
    let reorder_decode = chain
        .decode_index
        .find_by_id(&reorder_evidence.decode_id)
        .unwrap();
    assert_eq!(reorder_decode.display_idx, Some(1));

    let reorder_bit_offset = chain
        .bit_offset_index
        .find_by_id(&reorder_evidence.bit_offset_id)
        .unwrap();
    assert_eq!(reorder_bit_offset.bit_range.byte_offset(), 18000);

    // AV1 TimelineBands: User clicks on error burst marker at frame 10
    let error_viz = chain
        .viz_index
        .find_by_id(&error_evidence.timeline_viz_id)
        .unwrap();
    assert_eq!(error_viz.element_type, VizElementType::TimelineLane);
    assert_eq!(error_viz.display_idx, Some(10));

    // AV1 TimelineBands: Trace from error burst → bit offset to find corrupted OBU
    let error_bit_offset = chain
        .bit_offset_index
        .find_by_id(&error_evidence.bit_offset_id)
        .unwrap();
    assert_eq!(error_bit_offset.bit_range.byte_offset(), 200000);
    assert_eq!(error_bit_offset.bit_range.size_bits(), 5500 * 8);

    // AV1 TimelineBands: Bidirectional navigation from diagnostic bands
    // From scene change (frame 0) → OBU byte offset 0
    let traced_bit_range = manager.trace_to_bit_offset(0).unwrap();
    assert_eq!(traced_bit_range.byte_offset(), 0);

    // From reorder mismatch (frame 1) → OBU byte offset 18000
    let traced_bit_range = manager.trace_to_bit_offset(1).unwrap();
    assert_eq!(traced_bit_range.byte_offset(), 18000);

    // From error burst (frame 10) → OBU byte offset 200000
    let traced_bit_range = manager.trace_to_bit_offset(10).unwrap();
    assert_eq!(traced_bit_range.byte_offset(), 200000);

    // AV1 TimelineBands: Reverse navigation from bit offset → diagnostic band
    // From OBU byte offset 0 → scene change at frame 0
    let traced_display_idx = manager.trace_to_display_idx(0).unwrap();
    assert_eq!(traced_display_idx, 0);

    // From OBU byte offset 200000 → error burst at frame 10
    let traced_display_idx = manager.trace_to_display_idx(200000).unwrap();
    assert_eq!(traced_display_idx, 10);
}

// AV1 Metrics TimelineLanes evidence chain test - Task 25 (S.T4-3.AV1.Metrics.TimelineLanes.impl.evidence_chain.001)

#[test]
fn test_av1_metrics_timeline_lanes_evidence() {
    // AV1 Metrics: User clicks metrics lane point to trace to frame
    let mut manager = TimelineEvidenceManager::default();

    // Frame 0: KEY_FRAME with PSNR=42.5
    let key_frame = create_test_frame(0);
    let key_evidence = manager.create_frame_evidence(&key_frame, BitRange::new(0, 144000), 18000);

    // Frame 1: INTER_FRAME with PSNR=38.5
    let inter_frame = create_test_frame(1);
    let inter_evidence =
        manager.create_frame_evidence(&inter_frame, BitRange::new(144000, 192000), 6000);

    let chain = manager.evidence_chain();

    // Verify metrics lane evidence links
    let key_viz = chain
        .viz_index
        .find_by_id(&key_evidence.timeline_viz_id)
        .unwrap();
    assert_eq!(key_viz.display_idx, Some(0));

    let inter_viz = chain
        .viz_index
        .find_by_id(&inter_evidence.timeline_viz_id)
        .unwrap();
    assert_eq!(inter_viz.display_idx, Some(1));

    // Bidirectional navigation
    assert_eq!(manager.trace_to_bit_offset(0).unwrap().byte_offset(), 0);
    assert_eq!(manager.trace_to_bit_offset(1).unwrap().byte_offset(), 18000);
}
