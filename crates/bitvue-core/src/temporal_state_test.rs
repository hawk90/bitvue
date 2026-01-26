// Temporal State module tests
use super::*;

// ============================================================================
// Fixtures
// ============================================================================
fn create_test_reference_slot() -> ReferenceSlot {
    ReferenceSlot::new(0, 100, 100, TemporalRefType::Last)
}

fn create_test_dp_state() -> DpbStateSnapshot {
    DpbStateSnapshot::new(0, 0, Codec::Av1, 8)
}

fn create_test_timeline() -> TemporalStateTimeline {
    TemporalStateTimeline::new()
}

// ============================================================================
// TemporalRefType Tests
// ============================================================================
#[cfg(test)]
mod ref_type_tests {
    use super::*;

    #[test]
    fn test_is_forward_for_forward_types() {
        assert!(TemporalRefType::Last.is_forward());
        assert!(TemporalRefType::Golden.is_forward());
        assert!(TemporalRefType::ShortTerm.is_forward());
        assert!(TemporalRefType::L0.is_forward());
    }

    #[test]
    fn test_is_backward_for_backward_types() {
        assert!(TemporalRefType::Altref.is_backward());
        assert!(TemporalRefType::Bwdref.is_backward());
        assert!(TemporalRefType::L1.is_backward());
    }

    #[test]
    fn test_display_name() {
        assert_eq!(TemporalRefType::Last.display_name(), "LAST");
        assert_eq!(TemporalRefType::Golden.display_name(), "GOLDEN");
        assert_eq!(TemporalRefType::Altref.display_name(), "ALTREF");
        assert_eq!(TemporalRefType::L0.display_name(), "L0");
    }
}

// ============================================================================
// ReferenceSlot Tests
// ============================================================================
#[cfg(test)]
mod reference_slot_tests {
    use super::*;

    #[test]
    fn test_new_creates_slot() {
        let slot = create_test_reference_slot();
        assert_eq!(slot.slot_idx, 0);
        assert_eq!(slot.frame_display_idx, 100);
        assert_eq!(slot.ref_type, TemporalRefType::Last);
    }

    #[test]
    fn test_increment_usage() {
        let mut slot = create_test_reference_slot();
        slot.increment_usage();
        assert_eq!(slot.usage_count, 1);
    }

    #[test]
    fn test_increment_age() {
        let mut slot = create_test_reference_slot();
        slot.increment_age();
        assert_eq!(slot.age, 1);
    }

    #[test]
    fn test_default_is_long_term_false() {
        let slot = create_test_reference_slot();
        assert!(!slot.is_long_term);
    }
}

// ============================================================================
// EvictionReason Tests
// ============================================================================
#[cfg(test)]
mod eviction_reason_tests {
    use super::*;

    #[test]
    fn test_description_sliding_window() {
        assert!(EvictionReason::SlidingWindow.description().contains("Sliding window"));
    }

    #[test]
    fn test_description_replaced() {
        let reason = EvictionReason::Replaced { by_frame: 50 };
        assert!(reason.description().contains("50"));
    }

    #[test]
    fn test_description_idr_flush() {
        assert!(EvictionReason::IdrFlush.description().contains("IDR"));
    }
}

// ============================================================================
// DpbStateSnapshot Tests
// ============================================================================
#[cfg(test)]
mod dpb_state_tests {
    use super::*;

    #[test]
    fn test_new_creates_snapshot() {
        let state = create_test_dp_state();
        assert_eq!(state.at_display_idx, 0);
        assert_eq!(state.max_capacity, 8);
        assert_eq!(state.current_occupancy, 0);
    }

    #[test]
    fn test_fullness_percent_empty() {
        let state = create_test_dp_state();
        assert_eq!(state.fullness_percent(), 0.0);
    }

    #[test]
    fn test_fullness_percent_half() {
        let mut state = create_test_dp_state();
        state.current_occupancy = 4;
        assert_eq!(state.fullness_percent(), 50.0);
    }

    #[test]
    fn test_fullness_percent_zero_capacity() {
        let mut state = create_test_dp_state();
        state.max_capacity = 0;
        assert_eq!(state.fullness_percent(), 0.0);
    }

    #[test]
    fn test_add_slot_increments_occupancy() {
        let mut state = create_test_dp_state();
        state.add_slot(create_test_reference_slot());
        assert_eq!(state.current_occupancy, 1);
    }

    #[test]
    fn test_remove_slot_decrements_occupancy() {
        let mut state = create_test_dp_state();
        state.add_slot(create_test_reference_slot());
        let removed = state.remove_slot(0);
        assert!(removed.is_some());
        assert_eq!(state.current_occupancy, 0);
    }

    #[test]
    fn test_remove_slot_not_found() {
        let mut state = create_test_dp_state();
        let result = state.remove_slot(99);
        assert!(result.is_none());
    }

    #[test]
    fn test_find_slot_by_frame() {
        let mut state = create_test_dp_state();
        state.add_slot(create_test_reference_slot());
        let result = state.find_slot_by_frame(100);
        assert!(result.is_some());
    }

    #[test]
    fn test_find_slot_by_type() {
        let mut state = create_test_dp_state();
        state.add_slot(create_test_reference_slot());
        let result = state.find_slot_by_type(TemporalRefType::Last);
        assert!(result.is_some());
    }

    #[test]
    fn test_empty_slots() {
        let mut state = create_test_dp_state();
        state.add_slot(ReferenceSlot::new(0, 100, 100, TemporalRefType::Last));
        state.add_slot(ReferenceSlot::new(3, 100, 100, TemporalRefType::Golden));
        let empty = state.empty_slots();
        assert_eq!(empty.len(), 6); // 8 total - 2 occupied
    }

    #[test]
    fn test_add_event() {
        let mut state = create_test_dp_state();
        state.add_event(DpbEvent::Inserted {
            slot_idx: 0,
            frame_display_idx: 100,
            ref_type: TemporalRefType::Last,
            reason: "Test".to_string(),
        });
        assert_eq!(state.events.len(), 1);
    }
}

// ============================================================================
// TemporalStateTimeline Tests
// ============================================================================
#[cfg(test)]
mod timeline_tests {
    use super::*;

    #[test]
    fn test_new_creates_empty_timeline() {
        let timeline = create_test_timeline();
        assert!(timeline.is_empty());
    }

    #[test]
    fn test_add_snapshot_increments_length() {
        let mut timeline = create_test_timeline();
        timeline.add_snapshot(create_test_dp_state());
        assert_eq!(timeline.len(), 1);
    }

    #[test]
    fn test_add_snapshot_sorts_by_display_idx() {
        let mut timeline = create_test_timeline();
        let mut state2 = create_test_dp_state();
        state2.at_display_idx = 200;
        let mut state1 = create_test_dp_state();
        state1.at_display_idx = 100;
        timeline.add_snapshot(state2);
        timeline.add_snapshot(state1);
        assert_eq!(timeline.all_snapshots()[0].at_display_idx, 100);
    }

    #[test]
    fn test_get_snapshot() {
        let mut timeline = create_test_timeline();
        let mut state = create_test_dp_state();
        state.at_display_idx = 100;
        timeline.add_snapshot(state);
        let result = timeline.get_snapshot(100);
        assert!(result.is_some());
    }

    #[test]
    fn test_get_snapshot_not_found() {
        let timeline = create_test_timeline();
        let result = timeline.get_snapshot(999);
        assert!(result.is_none());
    }

    #[test]
    fn test_get_snapshot_at_or_before() {
        let mut timeline = create_test_timeline();
        let mut state1 = create_test_dp_state();
        state1.at_display_idx = 100;
        let mut state2 = create_test_dp_state();
        state2.at_display_idx = 200;
        timeline.add_snapshot(state1);
        timeline.add_snapshot(state2);
        let result = timeline.get_snapshot_at_or_before(150);
        assert!(result.is_some());
        assert_eq!(result.unwrap().at_display_idx, 100);
    }

    #[test]
    fn test_get_snapshot_at_or_before_exact() {
        let mut timeline = create_test_timeline();
        let mut state = create_test_dp_state();
        state.at_display_idx = 100;
        timeline.add_snapshot(state);
        let result = timeline.get_snapshot_at_or_before(100);
        assert_eq!(result.unwrap().at_display_idx, 100);
    }

    #[test]
    fn test_get_snapshots_in_range() {
        let mut timeline = create_test_timeline();
        timeline.add_snapshot({
            let mut s = create_test_dp_state();
            s.at_display_idx = 100;
            s
        });
        timeline.add_snapshot({
            let mut s = create_test_dp_state();
            s.at_display_idx = 200;
            s
        });
        timeline.add_snapshot({
            let mut s = create_test_dp_state();
            s.at_display_idx = 300;
            s
        });
        let results = timeline.get_snapshots_in_range(150, 250);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].at_display_idx, 200);
    }

    #[test]
    fn test_add_reference_edge() {
        let mut timeline = create_test_timeline();
        timeline.add_reference_edge(TemporalReferenceEdge {
            from_display_idx: 100,
            to_display_idx: 50,
            ref_type: TemporalRefType::Last,
            weight: None,
        });
        assert_eq!(timeline.all_edges().len(), 1);
    }

    #[test]
    fn test_get_outgoing_refs() {
        let mut timeline = create_test_timeline();
        timeline.add_reference_edge(TemporalReferenceEdge {
            from_display_idx: 100,
            to_display_idx: 50,
            ref_type: TemporalRefType::Last,
            weight: None,
        });
        let refs = timeline.get_outgoing_refs(100);
        assert_eq!(refs.len(), 1);
    }

    #[test]
    fn test_get_incoming_refs() {
        let mut timeline = create_test_timeline();
        timeline.add_reference_edge(TemporalReferenceEdge {
            from_display_idx: 100,
            to_display_idx: 50,
            ref_type: TemporalRefType::Last,
            weight: None,
        });
        let refs = timeline.get_incoming_refs(50);
        assert_eq!(refs.len(), 1);
    }

    #[test]
    fn test_get_reference_chain() {
        let mut timeline = create_test_timeline();
        timeline.add_reference_edge(TemporalReferenceEdge {
            from_display_idx: 100,
            to_display_idx: 50,
            ref_type: TemporalRefType::Last,
            weight: None,
        });
        timeline.add_reference_edge(TemporalReferenceEdge {
            from_display_idx: 50,
            to_display_idx: 25,
            ref_type: TemporalRefType::Last,
            weight: None,
        });
        let chain = timeline.get_reference_chain(100, 10);
        assert!(chain.contains(&50));
        assert!(chain.contains(&25));
    }

    #[test]
    fn test_get_reference_chain_respects_max_depth() {
        let mut timeline = create_test_timeline();
        timeline.add_reference_edge(TemporalReferenceEdge {
            from_display_idx: 100,
            to_display_idx: 50,
            ref_type: TemporalRefType::Last,
            weight: None,
        });
        let chain = timeline.get_reference_chain(100, 0);
        // Note: max_depth=0 means "skip references at depth > 0", but direct references
        // (depth 1 from source) are still included. The chain contains [50].
        assert_eq!(chain.len(), 1);
    }

    #[test]
    fn test_get_dependent_chain() {
        let mut timeline = create_test_timeline();
        timeline.add_reference_edge(TemporalReferenceEdge {
            from_display_idx: 50,
            to_display_idx: 100,
            ref_type: TemporalRefType::Last,
            weight: None,
        });
        timeline.add_reference_edge(TemporalReferenceEdge {
            from_display_idx: 25,
            to_display_idx: 50,
            ref_type: TemporalRefType::Last,
            weight: None,
        });
        let chain = timeline.get_dependent_chain(100, 10);
        assert!(chain.contains(&50));
        assert!(chain.contains(&25));
    }
}

// ============================================================================
// TemporalStateEvidence Tests
// ============================================================================
#[cfg(test)]
mod evidence_tests {
    use super::*;

    #[test]
    fn test_new_creates_evidence() {
        let evidence = TemporalStateEvidence::new(
            "test_id".to_string(),
            100,
            "decode_1".to_string(),
            create_test_dp_state(),
        );
        assert_eq!(evidence.id, "test_id");
        assert_eq!(evidence.display_idx, 100);
        assert!(evidence.semantic_link.is_none());
    }
}

// ============================================================================
// TemporalStateIndex Tests
// ============================================================================
#[cfg(test)]
mod state_index_tests {
    use super::*;

    #[test]
    fn test_new_creates_empty_index() {
        let index = TemporalStateIndex::new();
        assert!(index.is_empty());
    }

    #[test]
    fn test_add_increments_length() {
        let mut index = TemporalStateIndex::new();
        index.add(TemporalStateEvidence::new(
            "test_id".to_string(),
            100,
            "decode_1".to_string(),
            create_test_dp_state(),
        ));
        assert_eq!(index.len(), 1);
    }

    #[test]
    fn test_add_sorts_by_display_idx() {
        let mut index = TemporalStateIndex::new();
        index.add(TemporalStateEvidence::new(
            "id2".to_string(),
            200,
            "decode_1".to_string(),
            create_test_dp_state(),
        ));
        index.add(TemporalStateEvidence::new(
            "id1".to_string(),
            100,
            "decode_1".to_string(),
            create_test_dp_state(),
        ));
        assert_eq!(index.all()[0].display_idx, 100);
    }

    #[test]
    fn test_find_by_id() {
        let mut index = TemporalStateIndex::new();
        index.add(TemporalStateEvidence::new(
            "test_id".to_string(),
            100,
            "decode_1".to_string(),
            create_test_dp_state(),
        ));
        let result = index.find_by_id("test_id");
        assert!(result.is_some());
    }

    #[test]
    fn test_find_by_display_idx() {
        let mut index = TemporalStateIndex::new();
        index.add(TemporalStateEvidence::new(
            "test_id".to_string(),
            100,
            "decode_1".to_string(),
            create_test_dp_state(),
        ));
        let result = index.find_by_display_idx(100);
        assert!(result.is_some());
    }

    #[test]
    fn test_find_by_decode_link() {
        let mut index = TemporalStateIndex::new();
        index.add(TemporalStateEvidence::new(
            "test_id".to_string(),
            100,
            "decode_1".to_string(),
            create_test_dp_state(),
        ));
        let results = index.find_by_decode_link("decode_1");
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_all_returns_all() {
        let mut index = TemporalStateIndex::new();
        index.add(TemporalStateEvidence::new(
            "id1".to_string(),
            100,
            "decode_1".to_string(),
            create_test_dp_state(),
        ));
        index.add(TemporalStateEvidence::new(
            "id2".to_string(),
            200,
            "decode_2".to_string(),
            create_test_dp_state(),
        ));
        assert_eq!(index.all().len(), 2);
    }
}
