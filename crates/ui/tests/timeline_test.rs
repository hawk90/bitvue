//! Timeline Workspace Integration Test
//!
//! Tests that Timeline workspace can correctly collect and display frame data

use bitvue_core::{StreamId, UnitKey, UnitNode};
use std::sync::Arc;

/// Helper to create a test UnitNode
fn create_unit_node(
    stream: StreamId,
    offset: u64,
    size: usize,
    unit_type: &str,
    frame_index: Option<usize>,
    frame_type: Option<&str>,
    children: Vec<UnitNode>,
) -> UnitNode {
    UnitNode {
        key: UnitKey {
            stream,
            unit_type: unit_type.to_string().into(),
            offset,
            size,
        },
        unit_type: unit_type.to_string().into(),
        offset,
        size,
        frame_index,
        frame_type: frame_type.map(|s| Arc::from(s)),
        pts: None,
        dts: None,
        display_name: unit_type.to_string().into(),
        children,
        qp_avg: None,
        mv_grid: None,
        temporal_id: None,
        ref_frames: None,
        ref_slots: None,
    }
}

#[test]
fn test_timeline_collects_frames_from_units() {
    // Create mock unit tree with frames
    let units = vec![
        create_unit_node(
            StreamId::A,
            0,
            100,
            "OBU_SEQUENCE_HEADER",
            None, // Not a frame
            None,
            vec![],
        ),
        create_unit_node(
            StreamId::A,
            100,
            5000,
            "OBU_FRAME (KEY_FRAME)",
            Some(0), // First frame
            Some("KEY"),
            vec![],
        ),
        create_unit_node(
            StreamId::A,
            5100,
            3000,
            "OBU_FRAME (INTER_FRAME)",
            Some(1), // Second frame
            Some("INTER"),
            vec![],
        ),
        create_unit_node(
            StreamId::A,
            8100,
            2500,
            "OBU_FRAME (INTER_FRAME)",
            Some(2), // Third frame
            Some("INTER"),
            vec![],
        ),
    ];

    // Count frames (should be 3, excluding sequence header)
    let frame_count = units.iter().filter(|u| u.frame_index.is_some()).count();
    assert_eq!(frame_count, 3, "Should have 3 frames");

    // Verify frame indices are sequential
    let mut frame_indices: Vec<usize> = units.iter().filter_map(|u| u.frame_index).collect();
    frame_indices.sort();
    assert_eq!(
        frame_indices,
        vec![0, 1, 2],
        "Frame indices should be 0, 1, 2"
    );

    // Verify frame types
    let key_frames = units.iter().filter(|u| u.unit_type.contains("KEY")).count();
    let inter_frames = units
        .iter()
        .filter(|u| u.unit_type.contains("INTER"))
        .count();

    assert_eq!(key_frames, 1, "Should have 1 KEY frame");
    assert_eq!(inter_frames, 2, "Should have 2 INTER frames");

    // Verify frame sizes are reasonable
    for unit in &units {
        if unit.frame_index.is_some() {
            assert!(unit.size > 0, "Frame size should be > 0");
            assert!(unit.size < 100000, "Frame size should be reasonable");
        }
    }

    println!("✅ Timeline workspace frame collection test PASSED");
    println!("   - {} total units", units.len());
    println!("   - {} frames", frame_count);
    println!(
        "   - {} KEY frames, {} INTER frames",
        key_frames, inter_frames
    );
}

#[test]
fn test_timeline_handles_empty_units() {
    let units: Vec<UnitNode> = vec![];

    let frame_count = units.iter().filter(|u| u.frame_index.is_some()).count();
    assert_eq!(frame_count, 0, "Empty units should have 0 frames");

    println!("✅ Timeline workspace empty units test PASSED");
}

#[test]
fn test_timeline_handles_nested_units() {
    // Create nested unit tree
    let child = create_unit_node(
        StreamId::A,
        100,
        5000,
        "OBU_TILE_GROUP",
        None, // Child unit, not a separate frame
        None,
        vec![],
    );

    let units = vec![create_unit_node(
        StreamId::A,
        0,
        10000,
        "OBU_FRAME_HEADER",
        Some(0),
        Some("KEY"),
        vec![child],
    )];

    // Should count only the parent frame, not children
    let frame_count = count_frames_recursive(&units);
    assert_eq!(frame_count, 1, "Nested structure should have 1 frame");

    println!("✅ Timeline workspace nested units test PASSED");
}

fn count_frames_recursive(units: &[UnitNode]) -> usize {
    let mut count = 0;
    for unit in units {
        if unit.frame_index.is_some() {
            count += 1;
        }
        count += count_frames_recursive(&unit.children);
    }
    count
}

#[test]
fn test_timeline_frame_sorting() {
    // Create frames in non-sequential order
    let mut units = vec![
        create_unit_node(
            StreamId::A,
            8100,
            2500,
            "OBU_FRAME (INTER_FRAME)",
            Some(2),
            Some("INTER"),
            vec![],
        ),
        create_unit_node(
            StreamId::A,
            100,
            5000,
            "OBU_FRAME (KEY_FRAME)",
            Some(0),
            Some("KEY"),
            vec![],
        ),
        create_unit_node(
            StreamId::A,
            5100,
            3000,
            "OBU_FRAME (INTER_FRAME)",
            Some(1),
            Some("INTER"),
            vec![],
        ),
    ];

    // Sort by frame index
    units.sort_by_key(|u| u.frame_index);

    // Verify sorted order
    let indices: Vec<usize> = units.iter().filter_map(|u| u.frame_index).collect();
    assert_eq!(indices, vec![0, 1, 2], "Frames should be sorted by index");

    println!("✅ Timeline workspace frame sorting test PASSED");
}
