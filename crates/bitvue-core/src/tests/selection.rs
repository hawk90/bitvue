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
//! Tests for selection state management

use crate::selection::{
    DerivedCursor, SelectionAction, SelectionReducer, SelectionState, SpatialBlock,
    TemporalSelection,
};
use crate::StreamId;

#[test]
fn test_temporal_selection_precedence() {
    // Block has highest precedence
    let block = TemporalSelection::Block {
        frame_index: 0,
        block: SpatialBlock {
            x: 0,
            y: 0,
            w: 8,
            h: 8,
        },
    };
    let point = TemporalSelection::Point { frame_index: 0 };
    let range = TemporalSelection::Range { start: 0, end: 10 };
    let marker = TemporalSelection::Marker { frame_index: 0 };

    assert_eq!(block.precedence(), 4);
    assert_eq!(point.precedence(), 3);
    assert_eq!(range.precedence(), 2);
    assert_eq!(marker.precedence(), 1);

    assert!(block.precedence() > point.precedence());
    assert!(point.precedence() > range.precedence());
    assert!(range.precedence() > marker.precedence());
}

#[test]
fn test_derived_cursor() {
    // Point selection
    let point = TemporalSelection::Point { frame_index: 42 };
    let cursor = DerivedCursor::from_selection(&point);
    assert_eq!(cursor.frame_index, 42);
    assert_eq!(cursor.spatial_pos, None);

    // Block selection
    let block = TemporalSelection::Block {
        frame_index: 10,
        block: SpatialBlock {
            x: 16,
            y: 32,
            w: 8,
            h: 8,
        },
    };
    let cursor = DerivedCursor::from_selection(&block);
    assert_eq!(cursor.frame_index, 10);
    assert_eq!(cursor.spatial_pos, Some((16, 32)));
}

#[test]
fn test_selection_precedence_block_clears_others() {
    // Per SELECTION_PRECEDENCE_RULES.md:
    // "Click block -> any -> block selected, others cleared"
    let mut state = SelectionState::new(StreamId::A);

    // Start with range
    state.select_range(0, 10);
    assert!(matches!(
        state.temporal,
        Some(TemporalSelection::Range { .. })
    ));

    // Click block -> range cleared, block selected
    state.select_block(
        5,
        SpatialBlock {
            x: 0,
            y: 0,
            w: 8,
            h: 8,
        },
    );

    assert!(matches!(
        state.temporal,
        Some(TemporalSelection::Block { .. })
    ));
    assert_eq!(state.cursor.unwrap().frame_index, 5);
    assert_eq!(state.cursor.unwrap().spatial_pos, Some((0, 0)));
}

#[test]
fn test_selection_precedence_point_replaces_range() {
    // Per SELECTION_PRECEDENCE_RULES.md:
    // "Click frame | range | point replaces range"
    let mut state = SelectionState::new(StreamId::A);

    // Start with range
    state.select_range(0, 10);
    assert!(matches!(
        state.temporal,
        Some(TemporalSelection::Range { .. })
    ));

    // Click frame -> point replaces range
    state.select_point(5);

    assert!(matches!(
        state.temporal,
        Some(TemporalSelection::Point { .. })
    ));
    assert_eq!(state.cursor.unwrap().frame_index, 5);
}

#[test]
fn test_selection_precedence_range_replaces_point() {
    // Per SELECTION_PRECEDENCE_RULES.md:
    // "Drag range | any | range replaces point/marker"
    let mut state = SelectionState::new(StreamId::A);

    // Start with point
    state.select_point(5);
    assert!(matches!(
        state.temporal,
        Some(TemporalSelection::Point { .. })
    ));

    // Drag range -> range replaces point
    state.select_range(0, 10);

    assert!(matches!(
        state.temporal,
        Some(TemporalSelection::Range { .. })
    ));
    assert_eq!(state.temporal.as_ref().unwrap().frame_index(), 0);
}

#[test]
fn test_selection_precedence_marker_on_empty() {
    // Per SELECTION_PRECEDENCE_RULES.md:
    // "Click marker | none | point at marker"
    let mut state = SelectionState::new(StreamId::A);

    state.select_marker(42);

    // Marker on empty creates point
    assert!(matches!(
        state.temporal,
        Some(TemporalSelection::Point { .. })
    ));
    assert_eq!(state.cursor.unwrap().frame_index, 42);
}

#[test]
fn test_selection_precedence_marker_clears_range() {
    // Per SELECTION_PRECEDENCE_RULES.md:
    // "Click marker | range | range cleared, point set"
    let mut state = SelectionState::new(StreamId::A);

    // Start with range
    state.select_range(0, 10);
    assert!(matches!(
        state.temporal,
        Some(TemporalSelection::Range { .. })
    ));

    // Click marker -> range cleared, point at marker
    state.select_marker(5);

    assert!(matches!(
        state.temporal,
        Some(TemporalSelection::Point { .. })
    ));
    assert_eq!(state.cursor.unwrap().frame_index, 5);
}

#[test]
fn test_selection_only_one_active() {
    // Per SELECTION_PRECEDENCE_RULES.md:
    // "Only ONE selection type may be active."
    let mut state = SelectionState::new(StreamId::A);

    // Select point
    state.select_point(5);
    assert!(state.temporal.is_some());
    assert_eq!(state.temporal.as_ref().unwrap().precedence(), 3); // Point

    // Select range -> point cleared
    state.select_range(0, 10);
    assert!(state.temporal.is_some());
    assert_eq!(state.temporal.as_ref().unwrap().precedence(), 2); // Range

    // Select block -> range cleared
    state.select_block(
        7,
        SpatialBlock {
            x: 0,
            y: 0,
            w: 8,
            h: 8,
        },
    );
    assert!(state.temporal.is_some());
    assert_eq!(state.temporal.as_ref().unwrap().precedence(), 4); // Block
}

#[test]
fn test_selection_reducer_block() {
    let state = SelectionState::new(StreamId::A);
    let block = SpatialBlock {
        x: 16,
        y: 24,
        w: 8,
        h: 8,
    };

    let state = SelectionReducer::reduce(
        state,
        SelectionAction::SelectBlock {
            frame_index: 10,
            block,
        },
    );

    assert!(matches!(
        state.temporal,
        Some(TemporalSelection::Block { .. })
    ));
    assert_eq!(state.cursor.unwrap().frame_index, 10);
    assert_eq!(state.cursor.unwrap().spatial_pos, Some((16, 24)));
}

#[test]
fn test_selection_reducer_point() {
    let state = SelectionState::new(StreamId::A);

    let state = SelectionReducer::reduce(state, SelectionAction::SelectPoint { frame_index: 42 });

    assert!(matches!(
        state.temporal,
        Some(TemporalSelection::Point { .. })
    ));
    assert_eq!(state.cursor.unwrap().frame_index, 42);
}

#[test]
fn test_selection_reducer_range() {
    let state = SelectionState::new(StreamId::A);

    let state =
        SelectionReducer::reduce(state, SelectionAction::SelectRange { start: 10, end: 20 });

    assert!(matches!(
        state.temporal,
        Some(TemporalSelection::Range { .. })
    ));
    assert_eq!(
        state.temporal.as_ref().unwrap().range_bounds(),
        Some((10, 20))
    );
}

#[test]
fn test_selection_reducer_clear() {
    let mut state = SelectionState::new(StreamId::A);
    state.select_point(42);

    assert!(state.temporal.is_some());

    let state = SelectionReducer::reduce(state, SelectionAction::ClearTemporal);

    assert!(state.temporal.is_none());
    assert!(state.cursor.is_none());
}

#[test]
fn test_cursor_is_derived() {
    // Per SELECTION_PRECEDENCE_RULES.md:
    // "Selection is the source of truth. Cursor is derived."
    let mut state = SelectionState::new(StreamId::A);

    // No selection -> no cursor
    assert!(state.cursor.is_none());

    // Select point -> cursor derived
    state.select_point(10);
    assert!(state.cursor.is_some());
    assert_eq!(state.cursor.unwrap().frame_index, 10);

    // Change selection -> cursor updates automatically
    state.select_point(20);
    assert_eq!(state.cursor.unwrap().frame_index, 20);

    // Clear selection -> cursor cleared
    state.clear_temporal();
    assert!(state.cursor.is_none());
}

#[test]
fn test_temporal_selection_frame_index() {
    let block = TemporalSelection::Block {
        frame_index: 5,
        block: SpatialBlock {
            x: 0,
            y: 0,
            w: 8,
            h: 8,
        },
    };
    assert_eq!(block.frame_index(), 5);

    let point = TemporalSelection::Point { frame_index: 10 };
    assert_eq!(point.frame_index(), 10);

    let range = TemporalSelection::Range { start: 15, end: 25 };
    assert_eq!(range.frame_index(), 15); // Returns start

    let marker = TemporalSelection::Marker { frame_index: 30 };
    assert_eq!(marker.frame_index(), 30);
}

#[test]
fn test_temporal_selection_spatial_block() {
    let block_data = SpatialBlock {
        x: 16,
        y: 24,
        w: 8,
        h: 8,
    };

    let block = TemporalSelection::Block {
        frame_index: 5,
        block: block_data,
    };
    assert_eq!(block.spatial_block(), Some(block_data));

    let point = TemporalSelection::Point { frame_index: 10 };
    assert_eq!(point.spatial_block(), None);
}
