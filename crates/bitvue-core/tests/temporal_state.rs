#![allow(hidden_glob_reexports)]
#![allow(unreachable_code)]
#![allow(non_camel_case_types)]
#![allow(unused_assignments)]
#![allow(unused_parens)]
#![allow(unused_imports)]
#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unused_mut)]
#![allow(unused_comparisons)]
#![allow(unused_doc_comments)]
//! Tests for temporal state management

use bitvue_core::semantic_evidence::Codec;
use bitvue_core::temporal_state::{
    DpbStateSnapshot, ReferenceSlot, TemporalRefType, TemporalReferenceEdge, TemporalStateTimeline,
};

#[test]
fn test_reference_slot() {
    let mut slot = ReferenceSlot::new(0, 42, 40, TemporalRefType::Last);
    assert_eq!(slot.usage_count, 0);
    assert_eq!(slot.age, 0);

    slot.increment_usage();
    slot.increment_age();
    assert_eq!(slot.usage_count, 1);
    assert_eq!(slot.age, 1);
}

#[test]
fn test_dpb_snapshot() {
    let mut snapshot = DpbStateSnapshot::new(42, 40, Codec::Av1, 8);
    assert_eq!(snapshot.fullness_percent(), 0.0);

    let slot = ReferenceSlot::new(0, 40, 38, TemporalRefType::Last);
    snapshot.add_slot(slot);
    assert_eq!(snapshot.current_occupancy, 1);
    assert_eq!(snapshot.fullness_percent(), 12.5);

    let found = snapshot.find_slot_by_frame(40);
    assert!(found.is_some());
    assert_eq!(found.unwrap().ref_type, TemporalRefType::Last);

    let empty = snapshot.empty_slots();
    assert_eq!(empty.len(), 7);
    assert!(!empty.contains(&0));
}

#[test]
fn test_temporal_timeline() {
    let mut timeline = TemporalStateTimeline::new();

    // Add snapshots
    timeline.add_snapshot(DpbStateSnapshot::new(0, 0, Codec::Av1, 8));
    timeline.add_snapshot(DpbStateSnapshot::new(1, 1, Codec::Av1, 8));
    timeline.add_snapshot(DpbStateSnapshot::new(2, 3, Codec::Av1, 8));

    // Add reference edges
    timeline.add_reference_edge(TemporalReferenceEdge {
        from_display_idx: 1,
        to_display_idx: 0,
        ref_type: TemporalRefType::Last,
        weight: None,
    });
    timeline.add_reference_edge(TemporalReferenceEdge {
        from_display_idx: 2,
        to_display_idx: 1,
        ref_type: TemporalRefType::Last,
        weight: None,
    });
    timeline.add_reference_edge(TemporalReferenceEdge {
        from_display_idx: 2,
        to_display_idx: 0,
        ref_type: TemporalRefType::Golden,
        weight: None,
    });

    assert_eq!(timeline.len(), 3);

    // Test snapshot lookup
    let snap = timeline.get_snapshot(1);
    assert!(snap.is_some());

    // Test reference chain
    let chain = timeline.get_reference_chain(2, 5);
    assert_eq!(chain.len(), 2); // Frame 2 refs frames 1 and 0

    // Test dependent chain
    let deps = timeline.get_dependent_chain(0, 5);
    assert_eq!(deps.len(), 2); // Frames 1 and 2 depend on frame 0
}
