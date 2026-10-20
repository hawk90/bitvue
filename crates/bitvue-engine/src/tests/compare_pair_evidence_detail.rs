//! `CompareEvidenceManager::create_pair_evidence`: the exact IDs, labels, links and metadata it
//! writes into the two evidence chains and the pair map.

use crate::{
    BitRange, CompareEvidenceManager, DecodeArtifactType, EvidenceChain, FramePair, SyntaxNodeType,
    VizElementType,
};

fn pair(a: Option<usize>, b: Option<usize>, pts_delta: Option<i64>) -> FramePair {
    FramePair {
        stream_a_idx: a,
        stream_b_idx: b,
        pts_delta,
        has_gap: a.is_none() || b.is_none(),
    }
}

fn manager() -> CompareEvidenceManager {
    CompareEvidenceManager::new(EvidenceChain::new(), EvidenceChain::new())
}

fn create(m: &mut CompareEvidenceManager, a: usize, b: usize) -> crate::ComparePairEvidence {
    m.create_pair_evidence(
        &pair(Some(a), Some(b), None),
        BitRange::new(80, 240),
        BitRange::new(16, 100),
        20,
        11,
    )
    .unwrap()
}

#[test]
fn ids_are_allocated_a_then_b_then_diff() {
    let mut m = manager();
    let ev = create(&mut m, 3, 4);
    assert_eq!(ev.stream_a.bit_offset_id, "compare_ev_0");
    assert_eq!(ev.stream_a.syntax_id, "compare_ev_1");
    assert_eq!(ev.stream_a.decode_id, "compare_ev_2");
    assert_eq!(ev.stream_b.bit_offset_id, "compare_ev_3");
    assert_eq!(ev.stream_b.syntax_id, "compare_ev_4");
    assert_eq!(ev.stream_b.decode_id, "compare_ev_5");
    assert_eq!(ev.diff_viz_id, "compare_ev_6");
}

#[test]
fn ids_keep_counting_across_pairs() {
    let mut m = manager();
    create(&mut m, 0, 0);
    let second = create(&mut m, 1, 1);
    assert_eq!(second.stream_a.bit_offset_id, "compare_ev_7");
    assert_eq!(second.diff_viz_id, "compare_ev_13");
}

#[test]
fn a_pair_with_a_gap_returns_none_and_consumes_no_ids() {
    let mut m = manager();
    for p in [
        pair(None, Some(1), None),
        pair(Some(1), None, None),
        pair(None, None, None),
    ] {
        assert!(m
            .create_pair_evidence(&p, BitRange::new(0, 8), BitRange::new(0, 8), 1, 1)
            .is_none());
    }
    assert_eq!(m.pair_count(), 0);
    assert!(m.evidence_chain_a().bit_offset_index.is_empty());
    assert!(m.evidence_chain_b().bit_offset_index.is_empty());
    assert_eq!(create(&mut m, 0, 0).stream_a.bit_offset_id, "compare_ev_0");
}

#[test]
fn has_gap_flag_is_ignored_only_the_indices_decide() {
    let mut m = manager();
    let mut p = pair(Some(2), Some(2), None);
    p.has_gap = true;
    assert!(m
        .create_pair_evidence(&p, BitRange::new(0, 8), BitRange::new(0, 8), 1, 1)
        .is_some());
}

#[test]
fn bit_offset_evidence_carries_range_and_provenance_per_stream() {
    let mut m = manager();
    let ev = create(&mut m, 3, 4);
    let a = m
        .evidence_chain_a()
        .bit_offset_index
        .find_by_id(&ev.stream_a.bit_offset_id)
        .unwrap();
    assert_eq!(a.bit_range, BitRange::new(80, 240));
    assert_eq!(a.provenance, "compare_A_3");
    let b = m
        .evidence_chain_b()
        .bit_offset_index
        .find_by_id(&ev.stream_b.bit_offset_id)
        .unwrap();
    assert_eq!(b.bit_range, BitRange::new(16, 100));
    assert_eq!(b.provenance, "compare_B_4");
}

#[test]
fn each_chain_receives_only_its_own_streams_evidence() {
    let mut m = manager();
    let ev = create(&mut m, 3, 4);
    let (ca, cb) = (m.evidence_chain_a(), m.evidence_chain_b());
    assert_eq!(ca.bit_offset_index.len(), 1);
    assert_eq!(ca.syntax_index.len(), 1);
    assert_eq!(ca.decode_index.len(), 1);
    assert_eq!(cb.bit_offset_index.len(), 1);
    assert_eq!(cb.syntax_index.len(), 1);
    assert_eq!(cb.decode_index.len(), 1);
    assert!(ca
        .bit_offset_index
        .find_by_id(&ev.stream_b.bit_offset_id)
        .is_none());
    assert!(cb
        .bit_offset_index
        .find_by_id(&ev.stream_a.bit_offset_id)
        .is_none());
}

#[test]
fn syntax_evidence_is_a_frame_header_linked_to_its_bit_offset() {
    let mut m = manager();
    let ev = create(&mut m, 3, 4);
    let a = m
        .evidence_chain_a()
        .syntax_index
        .find_by_id(&ev.stream_a.syntax_id)
        .unwrap();
    assert_eq!(a.node_type, SyntaxNodeType::FrameHeader);
    assert_eq!(a.node_label, "frame_3_header_A");
    assert_eq!(a.bit_range, BitRange::new(80, 240));
    assert_eq!(a.bit_offset_link, ev.stream_a.bit_offset_id);
    assert_eq!(a.metadata.get("stream").map(String::as_str), Some("A"));
    assert_eq!(a.metadata.get("frame_size").map(String::as_str), Some("20"));
    assert_eq!(a.metadata.len(), 2);
    let b = m
        .evidence_chain_b()
        .syntax_index
        .find_by_id(&ev.stream_b.syntax_id)
        .unwrap();
    assert_eq!(b.node_label, "frame_4_header_B");
    assert_eq!(b.bit_range, BitRange::new(16, 100));
    assert_eq!(b.bit_offset_link, ev.stream_b.bit_offset_id);
    assert_eq!(b.metadata.get("stream").map(String::as_str), Some("B"));
    assert_eq!(b.metadata.get("frame_size").map(String::as_str), Some("11"));
}

#[test]
fn syntax_evidence_has_no_parent_children_or_decode_link() {
    // The syntax -> decode link is not set even though a decode record points back at it.
    let mut m = manager();
    let ev = create(&mut m, 0, 0);
    let a = m
        .evidence_chain_a()
        .syntax_index
        .find_by_id(&ev.stream_a.syntax_id)
        .unwrap();
    assert!(a.parent_link.is_none());
    assert!(a.child_links.is_empty());
    assert!(a.decode_link.is_none());
}

#[test]
fn decode_evidence_is_a_yuv_frame_linked_to_its_syntax_node() {
    let mut m = manager();
    let ev = create(&mut m, 3, 4);
    let a = m
        .evidence_chain_a()
        .decode_index
        .find_by_id(&ev.stream_a.decode_id)
        .unwrap();
    assert_eq!(a.artifact_type, DecodeArtifactType::YuvFrame);
    assert_eq!(a.artifact_label, "frame_3_yuv_A");
    assert_eq!(a.syntax_link, ev.stream_a.syntax_id);
    assert_eq!((a.frame_idx, a.display_idx), (Some(3), Some(3)));
    assert_eq!(a.metadata.get("stream").map(String::as_str), Some("A"));
    assert_eq!(a.metadata.len(), 1);
    assert!(a.viz_link.is_none());
    let b = m
        .evidence_chain_b()
        .decode_index
        .find_by_id(&ev.stream_b.decode_id)
        .unwrap();
    assert_eq!(b.artifact_label, "frame_4_yuv_B");
    assert_eq!(b.syntax_link, ev.stream_b.syntax_id);
    assert_eq!((b.frame_idx, b.display_idx), (Some(4), Some(4)));
    assert_eq!(b.metadata.get("stream").map(String::as_str), Some("B"));
}

#[test]
fn diff_heatmap_lives_only_in_chain_a_and_links_to_decode_a() {
    let mut m = manager();
    let ev = create(&mut m, 3, 4);
    assert!(m.evidence_chain_b().viz_index.is_empty());
    assert_eq!(m.evidence_chain_a().viz_index.len(), 1);
    let viz = m
        .evidence_chain_a()
        .viz_index
        .find_by_id(&ev.diff_viz_id)
        .unwrap();
    assert_eq!(viz.element_type, VizElementType::DiffHeatmap);
    assert_eq!(viz.element_label, "diff_3_4");
    assert_eq!(viz.decode_link, ev.stream_a.decode_id);
}

#[test]
fn diff_heatmap_records_both_frame_indices_and_both_decode_ids() {
    let mut m = manager();
    let ev = create(&mut m, 3, 4);
    let viz = m
        .evidence_chain_a()
        .viz_index
        .find_by_id(&ev.diff_viz_id)
        .unwrap();
    assert_eq!(
        viz.visual_properties.get("frame_a_idx").map(String::as_str),
        Some("3")
    );
    assert_eq!(
        viz.visual_properties.get("frame_b_idx").map(String::as_str),
        Some("4")
    );
    assert_eq!(
        viz.metadata.get("stream_a_decode"),
        Some(&ev.stream_a.decode_id)
    );
    assert_eq!(
        viz.metadata.get("stream_b_decode"),
        Some(&ev.stream_b.decode_id)
    );
    // Unlike the decode records, the viz element's own frame indices are never set.
    assert_eq!((viz.frame_idx, viz.display_idx), (None, None));
    assert_eq!(viz.visual_properties.len(), 2);
}

#[test]
fn pts_delta_is_recorded_when_present_including_zero_and_negative() {
    for (delta, expected) in [(90_000i64, "90000"), (0, "0"), (-33, "-33")] {
        let mut m = manager();
        let ev = m
            .create_pair_evidence(
                &pair(Some(1), Some(1), Some(delta)),
                BitRange::new(0, 8),
                BitRange::new(0, 8),
                1,
                1,
            )
            .unwrap();
        let viz = m
            .evidence_chain_a()
            .viz_index
            .find_by_id(&ev.diff_viz_id)
            .unwrap();
        assert_eq!(
            viz.visual_properties.get("pts_delta").map(String::as_str),
            Some(expected)
        );
        assert_eq!(viz.visual_properties.len(), 3);
    }
}

#[test]
fn pts_delta_property_is_absent_when_unknown() {
    let mut m = manager();
    let ev = create(&mut m, 1, 1);
    let viz = m
        .evidence_chain_a()
        .viz_index
        .find_by_id(&ev.diff_viz_id)
        .unwrap();
    assert!(!viz.visual_properties.contains_key("pts_delta"));
}

#[test]
fn returned_evidence_matches_the_stored_copy() {
    let mut m = manager();
    let ev = create(&mut m, 3, 4);
    assert_eq!((ev.frame_a_idx, ev.frame_b_idx), (3, 4));
    let stored = m.get_pair_evidence(3, 4).unwrap();
    assert_eq!(stored.diff_viz_id, ev.diff_viz_id);
    assert_eq!(stored.stream_a.syntax_id, ev.stream_a.syntax_id);
    assert_eq!(stored.stream_b.decode_id, ev.stream_b.decode_id);
    assert!(m.get_pair_evidence(4, 3).is_none());
}

#[test]
fn recreating_a_pair_replaces_the_map_entry_but_keeps_both_records_in_the_chains() {
    let mut m = manager();
    let first = create(&mut m, 1, 1);
    let second = create(&mut m, 1, 1);
    assert_eq!(m.pair_count(), 1);
    assert_eq!(
        m.get_pair_evidence(1, 1).unwrap().diff_viz_id,
        second.diff_viz_id
    );
    assert_ne!(first.diff_viz_id, second.diff_viz_id);
    assert_eq!(m.evidence_chain_a().viz_index.len(), 2);
    assert_eq!(m.evidence_chain_a().bit_offset_index.len(), 2);
}

#[test]
fn clear_resets_ids_but_leaves_the_chains_untouched() {
    let mut m = manager();
    create(&mut m, 1, 1);
    m.clear();
    assert_eq!(m.pair_count(), 0);
    assert_eq!(m.evidence_chain_a().bit_offset_index.len(), 1);
    let again = create(&mut m, 1, 1);
    assert_eq!(again.stream_a.bit_offset_id, "compare_ev_0");
    // The reused ID now exists twice in chain A.
    assert_eq!(m.evidence_chain_a().bit_offset_index.len(), 2);
}

#[test]
fn existing_chain_contents_are_preserved_and_appended_to() {
    let mut pre = EvidenceChain::new();
    pre.add_bit_offset(crate::BitOffsetEvidence::new(
        "pre".into(),
        BitRange::new(0, 8),
        "pre".into(),
    ));
    let mut m = CompareEvidenceManager::new(pre, EvidenceChain::new());
    create(&mut m, 0, 0);
    assert_eq!(m.evidence_chain_a().bit_offset_index.len(), 2);
    assert!(m
        .evidence_chain_a()
        .bit_offset_index
        .find_by_id("pre")
        .is_some());
}

#[test]
fn frame_sizes_are_stringified_verbatim_even_when_extreme() {
    let mut m = manager();
    let ev = m
        .create_pair_evidence(
            &pair(Some(0), Some(0), None),
            BitRange::new(0, 8),
            BitRange::new(0, 8),
            0,
            usize::MAX,
        )
        .unwrap();
    let a = m
        .evidence_chain_a()
        .syntax_index
        .find_by_id(&ev.stream_a.syntax_id)
        .unwrap();
    let b = m
        .evidence_chain_b()
        .syntax_index
        .find_by_id(&ev.stream_b.syntax_id)
        .unwrap();
    assert_eq!(a.metadata["frame_size"], "0");
    assert_eq!(b.metadata["frame_size"], usize::MAX.to_string());
}

#[test]
fn an_empty_bit_range_is_accepted_without_validation() {
    let mut m = manager();
    let ev = m
        .create_pair_evidence(
            &pair(Some(0), Some(0), None),
            BitRange::new(5, 5),
            BitRange::new(9, 9),
            0,
            0,
        )
        .unwrap();
    let a = m
        .evidence_chain_a()
        .bit_offset_index
        .find_by_id(&ev.stream_a.bit_offset_id)
        .unwrap();
    assert_eq!(a.bit_range, BitRange::new(5, 5));
    assert_eq!(a.size_bytes, 0);
}
