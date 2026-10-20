//! `TimelineEvidenceManager::create_frame_evidence`: the exact IDs, labels, links, metadata and
//! visual properties it writes into the evidence chain and the frame map.

use crate::timeline::TimelineFrame;
use crate::{
    BitRange, DecodeArtifactType, EvidenceChain, FrameMarker, SyntaxNodeType,
    TimelineEvidenceManager, VizElementType,
};

fn frame(idx: usize, marker: FrameMarker) -> TimelineFrame {
    TimelineFrame {
        display_idx: idx,
        size_bytes: 777,
        frame_type: "B".to_string(),
        marker,
        pts: Some(900),
        dts: Some(450),
        is_selected: false,
    }
}

fn manager() -> TimelineEvidenceManager {
    TimelineEvidenceManager::new(EvidenceChain::new())
}

fn s(map: &std::collections::HashMap<String, String>, k: &str) -> Option<String> {
    map.get(k).cloned()
}

#[test]
fn ids_are_allocated_in_stage_order() {
    let mut m = manager();
    let ev = m.create_frame_evidence(&frame(5, FrameMarker::None), BitRange::new(80, 240), 20);
    assert_eq!(ev.display_idx, 5);
    assert_eq!(ev.bit_offset_id, "timeline_ev_0");
    assert_eq!(ev.syntax_id, "timeline_ev_1");
    assert_eq!(ev.decode_id, "timeline_ev_2");
    assert_eq!(ev.timeline_viz_id, "timeline_ev_3");
}

#[test]
fn ids_keep_counting_across_frames() {
    let mut m = manager();
    m.create_frame_evidence(&frame(0, FrameMarker::None), BitRange::new(0, 8), 1);
    let second = m.create_frame_evidence(&frame(1, FrameMarker::None), BitRange::new(0, 8), 1);
    assert_eq!(second.bit_offset_id, "timeline_ev_4");
    assert_eq!(second.timeline_viz_id, "timeline_ev_7");
}

#[test]
fn one_record_is_added_to_each_of_the_four_indexes() {
    let mut m = manager();
    m.create_frame_evidence(&frame(5, FrameMarker::None), BitRange::new(0, 8), 1);
    let c = m.evidence_chain();
    assert_eq!(c.bit_offset_index.len(), 1);
    assert_eq!(c.syntax_index.len(), 1);
    assert_eq!(c.decode_index.len(), 1);
    assert_eq!(c.viz_index.len(), 1);
}

#[test]
fn bit_offset_evidence_has_range_provenance_and_syntax_link() {
    let mut m = manager();
    let ev = m.create_frame_evidence(&frame(5, FrameMarker::None), BitRange::new(80, 240), 20);
    let b = m
        .evidence_chain()
        .bit_offset_index
        .find_by_id(&ev.bit_offset_id)
        .unwrap();
    assert_eq!(b.bit_range, BitRange::new(80, 240));
    assert_eq!(b.provenance, "timeline_frame_5");
    assert_eq!(b.syntax_link.as_deref(), Some(ev.syntax_id.as_str()));
}

#[test]
fn syntax_evidence_is_a_frame_header_with_all_metadata() {
    let mut m = manager();
    let ev = m.create_frame_evidence(&frame(5, FrameMarker::Key), BitRange::new(80, 240), 20);
    let sy = m
        .evidence_chain()
        .syntax_index
        .find_by_id(&ev.syntax_id)
        .unwrap();
    assert_eq!(sy.node_type, SyntaxNodeType::FrameHeader);
    assert_eq!(sy.node_label, "frame_5_header");
    assert_eq!(sy.bit_range, BitRange::new(80, 240));
    assert_eq!(sy.bit_offset_link, ev.bit_offset_id);
    assert_eq!(sy.decode_link.as_deref(), Some(ev.decode_id.as_str()));
    assert!(sy.parent_link.is_none());
    assert!(sy.child_links.is_empty());
    assert_eq!(s(&sy.metadata, "frame_type").as_deref(), Some("B"));
    assert_eq!(s(&sy.metadata, "size_bytes").as_deref(), Some("20"));
    assert_eq!(s(&sy.metadata, "is_keyframe").as_deref(), Some("true"));
    assert_eq!(s(&sy.metadata, "display_idx").as_deref(), Some("5"));
    assert_eq!(s(&sy.metadata, "pts").as_deref(), Some("900"));
    assert_eq!(s(&sy.metadata, "dts").as_deref(), Some("450"));
    assert_eq!(sy.metadata.len(), 6);
}

#[test]
fn pts_and_dts_metadata_are_independent_and_omitted_when_unknown() {
    for (pts, dts) in [
        (None, None),
        (Some(7), None),
        (None, Some(8)),
        (Some(0), Some(0)),
    ] {
        let mut f = frame(1, FrameMarker::None);
        f.pts = pts;
        f.dts = dts;
        let mut m = manager();
        let ev = m.create_frame_evidence(&f, BitRange::new(0, 8), 1);
        let sy = m
            .evidence_chain()
            .syntax_index
            .find_by_id(&ev.syntax_id)
            .unwrap();
        assert_eq!(s(&sy.metadata, "pts"), pts.map(|v| v.to_string()));
        assert_eq!(s(&sy.metadata, "dts"), dts.map(|v| v.to_string()));
        assert_eq!(
            sy.metadata.len(),
            4 + pts.iter().count() + dts.iter().count()
        );
    }
}

#[test]
fn is_keyframe_is_true_only_for_the_key_marker() {
    for (marker, expected) in [
        (FrameMarker::Key, "true"),
        (FrameMarker::None, "false"),
        (FrameMarker::Error, "false"),
        (FrameMarker::Bookmark, "false"),
        (FrameMarker::SceneChange, "false"),
    ] {
        let mut m = manager();
        let ev = m.create_frame_evidence(&frame(1, marker), BitRange::new(0, 8), 1);
        let sy = m
            .evidence_chain()
            .syntax_index
            .find_by_id(&ev.syntax_id)
            .unwrap();
        let vz = m
            .evidence_chain()
            .viz_index
            .find_by_id(&ev.timeline_viz_id)
            .unwrap();
        assert_eq!(s(&sy.metadata, "is_keyframe").as_deref(), Some(expected));
        assert_eq!(
            s(&vz.visual_properties, "is_keyframe").as_deref(),
            Some(expected)
        );
    }
}

#[test]
fn decode_evidence_is_a_yuv_frame_linked_both_ways() {
    let mut m = manager();
    let ev = m.create_frame_evidence(&frame(5, FrameMarker::None), BitRange::new(80, 240), 20);
    let d = m
        .evidence_chain()
        .decode_index
        .find_by_id(&ev.decode_id)
        .unwrap();
    assert_eq!(d.artifact_type, DecodeArtifactType::YuvFrame);
    assert_eq!(d.artifact_label, "frame_5_yuv");
    assert_eq!(d.syntax_link, ev.syntax_id);
    assert_eq!(d.viz_link.as_deref(), Some(ev.timeline_viz_id.as_str()));
    assert_eq!((d.frame_idx, d.display_idx), (Some(5), Some(5)));
    assert_eq!(s(&d.metadata, "size_bytes").as_deref(), Some("20"));
    assert_eq!(d.metadata.len(), 1);
}

#[test]
fn viz_evidence_is_a_timeline_lane_linked_to_decode() {
    let mut m = manager();
    let ev = m.create_frame_evidence(&frame(5, FrameMarker::Bookmark), BitRange::new(0, 8), 20);
    let v = m
        .evidence_chain()
        .viz_index
        .find_by_id(&ev.timeline_viz_id)
        .unwrap();
    assert_eq!(v.element_type, VizElementType::TimelineLane);
    assert_eq!(v.element_label, "timeline_frame_5");
    assert_eq!(v.decode_link, ev.decode_id);
    assert_eq!((v.frame_idx, v.display_idx), (Some(5), Some(5)));
    assert_eq!(v.temporal_pos, Some(5.0));
    assert_eq!(s(&v.metadata, "display_idx").as_deref(), Some("5"));
    assert_eq!(v.metadata.len(), 1);
}

#[test]
fn viz_visual_properties_use_the_timeline_frames_own_size_not_the_argument() {
    let mut m = manager();
    let ev = m.create_frame_evidence(&frame(5, FrameMarker::Bookmark), BitRange::new(0, 8), 20);
    let v = m
        .evidence_chain()
        .viz_index
        .find_by_id(&ev.timeline_viz_id)
        .unwrap();
    let p = &v.visual_properties;
    assert_eq!(s(p, "frame_type").as_deref(), Some("B"));
    assert_eq!(s(p, "marker").as_deref(), Some("Bookmark"));
    assert_eq!(s(p, "is_keyframe").as_deref(), Some("false"));
    assert_eq!(s(p, "size_bytes").as_deref(), Some("777")); // not "20"
    assert_eq!(s(p, "display_idx").as_deref(), Some("5"));
    assert_eq!(p.len(), 5);
}

#[test]
fn returned_bundle_matches_the_stored_one() {
    let mut m = manager();
    let ev = m.create_frame_evidence(&frame(5, FrameMarker::None), BitRange::new(0, 8), 1);
    let stored = m.get_frame_evidence(5).unwrap();
    assert_eq!(stored.bit_offset_id, ev.bit_offset_id);
    assert_eq!(stored.syntax_id, ev.syntax_id);
    assert_eq!(stored.decode_id, ev.decode_id);
    assert_eq!(stored.timeline_viz_id, ev.timeline_viz_id);
    assert_eq!(m.frame_count(), 1);
    assert_eq!(m.get_frame_bit_range(5), Some(BitRange::new(0, 8)));
}

#[test]
fn recreating_a_frame_replaces_the_map_entry_but_keeps_both_records_in_the_chain() {
    let mut m = manager();
    m.create_frame_evidence(&frame(5, FrameMarker::None), BitRange::new(0, 8), 1);
    let second = m.create_frame_evidence(&frame(5, FrameMarker::None), BitRange::new(16, 32), 1);
    assert_eq!(m.frame_count(), 1);
    assert_eq!(
        m.get_frame_evidence(5).unwrap().bit_offset_id,
        second.bit_offset_id
    );
    assert_eq!(m.evidence_chain().bit_offset_index.len(), 2);
}

#[test]
fn clear_resets_ids_but_leaves_the_chain_untouched() {
    let mut m = manager();
    m.create_frame_evidence(&frame(5, FrameMarker::None), BitRange::new(0, 8), 1);
    m.clear();
    assert_eq!(m.frame_count(), 0);
    let again = m.create_frame_evidence(&frame(5, FrameMarker::None), BitRange::new(0, 8), 1);
    assert_eq!(again.bit_offset_id, "timeline_ev_0");
    assert_eq!(m.evidence_chain().bit_offset_index.len(), 2);
}

#[test]
fn existing_chain_contents_are_preserved() {
    let mut chain = EvidenceChain::new();
    chain.add_bit_offset(crate::BitOffsetEvidence::new(
        "pre".into(),
        BitRange::new(0, 8),
        "pre".into(),
    ));
    let mut m = TimelineEvidenceManager::new(chain);
    m.create_frame_evidence(&frame(0, FrameMarker::None), BitRange::new(0, 8), 1);
    assert_eq!(m.evidence_chain().bit_offset_index.len(), 2);
    assert!(m
        .evidence_chain()
        .bit_offset_index
        .find_by_id("pre")
        .is_some());
}

#[test]
fn extreme_values_are_stringified_verbatim() {
    let mut f = frame(usize::MAX, FrameMarker::None);
    f.size_bytes = u64::MAX;
    f.pts = Some(u64::MAX);
    let mut m = manager();
    let ev = m.create_frame_evidence(&f, BitRange::new(0, 8), usize::MAX);
    let sy = m
        .evidence_chain()
        .syntax_index
        .find_by_id(&ev.syntax_id)
        .unwrap();
    let v = m
        .evidence_chain()
        .viz_index
        .find_by_id(&ev.timeline_viz_id)
        .unwrap();
    assert_eq!(sy.metadata["size_bytes"], usize::MAX.to_string());
    assert_eq!(sy.metadata["pts"], u64::MAX.to_string());
    assert_eq!(v.visual_properties["size_bytes"], u64::MAX.to_string());
    assert_eq!(v.temporal_pos, Some(usize::MAX as f64));
}
