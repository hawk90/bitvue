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
use crate::indexing::*;
use crate::types::*;

#[test]
fn test_quick_index_creation() {
    let seek_points = vec![
        SeekPoint {
            display_idx: 0,
            byte_offset: 0,
            is_keyframe: true,
            pts: Some(0),
        },
        SeekPoint {
            display_idx: 30,
            byte_offset: 1024,
            is_keyframe: true,
            pts: Some(30000),
        },
    ];

    let quick = QuickIndex::new(seek_points, 10240);

    assert_eq!(quick.seek_points.len(), 2);
    assert_eq!(quick.file_size, 10240);
    assert!(!quick.is_empty());
}

#[test]
fn test_quick_index_find_nearest_keyframe() {
    let seek_points = vec![
        SeekPoint {
            display_idx: 0,
            byte_offset: 0,
            is_keyframe: true,
            pts: Some(0),
        },
        SeekPoint {
            display_idx: 30,
            byte_offset: 1024,
            is_keyframe: true,
            pts: Some(30000),
        },
        SeekPoint {
            display_idx: 60,
            byte_offset: 2048,
            is_keyframe: true,
            pts: Some(60000),
        },
    ];

    let quick = QuickIndex::new(seek_points, 10240);

    // Frame 0 -> keyframe 0
    let kf = quick.find_nearest_keyframe(0).unwrap();
    assert_eq!(kf.display_idx, 0);

    // Frame 25 -> keyframe 0
    let kf = quick.find_nearest_keyframe(25).unwrap();
    assert_eq!(kf.display_idx, 0);

    // Frame 35 -> keyframe 30
    let kf = quick.find_nearest_keyframe(35).unwrap();
    assert_eq!(kf.display_idx, 30);

    // Frame 100 -> keyframe 60
    let kf = quick.find_nearest_keyframe(100).unwrap();
    assert_eq!(kf.display_idx, 60);
}

#[test]
fn test_full_index_creation() {
    let frames = vec![
        FrameMetadata {
            display_idx: 0,
            decode_idx: 0,
            byte_offset: 0,
            size: 512,
            is_keyframe: true,
            pts: Some(0),
            dts: Some(0),
            frame_type: Some("I".to_string()),
        },
        FrameMetadata {
            display_idx: 1,
            decode_idx: 1,
            byte_offset: 512,
            size: 256,
            is_keyframe: false,
            pts: Some(1000),
            dts: Some(1000),
            frame_type: Some("P".to_string()),
        },
    ];

    let full = FullIndex::new(frames, 10240, true);

    assert_eq!(full.frame_count(), 2);
    assert!(full.is_complete);
    assert!(full.contains(0));
    assert!(full.contains(1));
    assert!(!full.contains(2));
}

#[test]
fn test_full_index_to_quick_index() {
    let frames = vec![
        FrameMetadata {
            display_idx: 0,
            decode_idx: 0,
            byte_offset: 0,
            size: 512,
            is_keyframe: true,
            pts: Some(0),
            dts: Some(0),
            frame_type: Some("I".to_string()),
        },
        FrameMetadata {
            display_idx: 1,
            decode_idx: 1,
            byte_offset: 512,
            size: 256,
            is_keyframe: false,
            pts: Some(1000),
            dts: Some(1000),
            frame_type: Some("P".to_string()),
        },
        FrameMetadata {
            display_idx: 2,
            decode_idx: 2,
            byte_offset: 768,
            size: 512,
            is_keyframe: true,
            pts: Some(2000),
            dts: Some(2000),
            frame_type: Some("I".to_string()),
        },
    ];

    let full = FullIndex::new(frames, 10240, true);
    let quick = full.to_quick_index();

    // Quick index should only have keyframes
    assert_eq!(quick.seek_points.len(), 2);
    assert_eq!(quick.seek_points[0].display_idx, 0);
    assert_eq!(quick.seek_points[1].display_idx, 2);
}

#[test]
fn test_index_progress() {
    let progress = IndexProgress::new();

    assert_eq!(progress.progress(), 0.0);
    assert!(!progress.is_complete());
    assert!(!progress.is_cancelled());

    progress.set_progress(0.5);
    assert_eq!(progress.progress(), 0.5);

    progress.mark_complete();
    assert_eq!(progress.progress(), 1.0);
    assert!(progress.is_complete());
    assert_eq!(progress.status(), "Complete");

    let progress2 = IndexProgress::new();
    progress2.cancel();
    assert!(progress2.is_cancelled());
    assert_eq!(progress2.status(), "Cancelled");
}

#[test]
fn test_index_progress_clamping() {
    let progress = IndexProgress::new();

    // Test clamping to [0.0, 1.0]
    progress.set_progress(-0.5);
    assert_eq!(progress.progress(), 0.0);

    progress.set_progress(1.5);
    assert_eq!(progress.progress(), 1.0);
}

#[test]
fn test_index_state_none() {
    let state = IndexState::None;
    assert!(state.quick().is_none());
    assert!(state.full().is_none());
    assert!(!state.has_full_index());
    assert!(!state.is_building());
    assert!(!state.can_access(0));
}

#[test]
fn test_index_state_quick() {
    let seek_points = vec![SeekPoint {
        display_idx: 0,
        byte_offset: 0,
        is_keyframe: true,
        pts: Some(0),
    }];

    let quick = QuickIndex::new(seek_points, 10240);
    let state = IndexState::Quick(quick);

    assert!(state.quick().is_some());
    assert!(state.full().is_none());
    assert!(!state.has_full_index());
    assert!(!state.is_building());
    assert!(state.can_access(0)); // Can access keyframe
}

#[test]
fn test_index_state_building() {
    let seek_points = vec![SeekPoint {
        display_idx: 0,
        byte_offset: 0,
        is_keyframe: true,
        pts: Some(0),
    }];

    let quick = QuickIndex::new(seek_points, 10240);

    let frames = vec![FrameMetadata {
        display_idx: 0,
        decode_idx: 0,
        byte_offset: 0,
        size: 512,
        is_keyframe: true,
        pts: Some(0),
        dts: Some(0),
        frame_type: Some("I".to_string()),
    }];

    let partial = FullIndex::new(frames, 10240, false);
    let progress = IndexProgress::new();

    let state = IndexState::Building {
        quick,
        partial,
        progress,
    };

    assert!(state.quick().is_some());
    assert!(state.full().is_none());
    assert!(!state.has_full_index());
    assert!(state.is_building());
    assert!(state.progress().is_some());
    assert!(state.can_access(0)); // Can access indexed frames
    assert!(!state.can_access(10)); // Cannot access unindexed frames
}

#[test]
fn test_index_state_full() {
    let frames = vec![FrameMetadata {
        display_idx: 0,
        decode_idx: 0,
        byte_offset: 0,
        size: 512,
        is_keyframe: true,
        pts: Some(0),
        dts: Some(0),
        frame_type: Some("I".to_string()),
    }];

    let full = FullIndex::new(frames, 10240, true);
    let state = IndexState::Full(full);

    assert!(state.full().is_some());
    assert!(state.has_full_index());
    assert!(!state.is_building());
    assert!(state.can_access(0));
}

#[test]
fn test_full_index_keyframe_indices() {
    let frames = vec![
        FrameMetadata {
            display_idx: 0,
            decode_idx: 0,
            byte_offset: 0,
            size: 512,
            is_keyframe: true,
            pts: Some(0),
            dts: Some(0),
            frame_type: Some("I".to_string()),
        },
        FrameMetadata {
            display_idx: 1,
            decode_idx: 1,
            byte_offset: 512,
            size: 256,
            is_keyframe: false,
            pts: Some(1000),
            dts: Some(1000),
            frame_type: Some("P".to_string()),
        },
        FrameMetadata {
            display_idx: 2,
            decode_idx: 2,
            byte_offset: 768,
            size: 512,
            is_keyframe: true,
            pts: Some(2000),
            dts: Some(2000),
            frame_type: Some("I".to_string()),
        },
        FrameMetadata {
            display_idx: 3,
            decode_idx: 3,
            byte_offset: 1280,
            size: 256,
            is_keyframe: false,
            pts: Some(3000),
            dts: Some(3000),
            frame_type: Some("P".to_string()),
        },
    ];

    let full = FullIndex::new(frames, 10240, true);
    let keyframes = full.keyframe_indices();

    assert_eq!(keyframes, vec![0, 2]);
}

// T1-2: Large File Open Fast-Path tests

#[test]
fn test_open_strategy_fast_path() {
    let fast_path = OpenFastPath::new(OpenStrategy::FastPath);

    // Fast path always uses quick index
    assert!(fast_path.should_use_quick_index(1000));
    assert!(fast_path.should_use_quick_index(100_000_000));
}

#[test]
fn test_open_strategy_full_path() {
    let full_path = OpenFastPath::new(OpenStrategy::FullPath);

    // Full path never uses quick index
    assert!(!full_path.should_use_quick_index(1000));
    assert!(!full_path.should_use_quick_index(100_000_000));
}

#[test]
fn test_open_strategy_adaptive() {
    let adaptive = OpenFastPath::new(OpenStrategy::Adaptive);

    // Default threshold: 10MB
    assert!(!adaptive.should_use_quick_index(5 * 1024 * 1024)); // 5MB -> full
    assert!(adaptive.should_use_quick_index(15 * 1024 * 1024)); // 15MB -> quick

    // Custom threshold: 20MB
    let adaptive_20mb =
        OpenFastPath::new(OpenStrategy::Adaptive).with_adaptive_threshold(20 * 1024 * 1024);

    assert!(!adaptive_20mb.should_use_quick_index(15 * 1024 * 1024)); // 15MB -> full
    assert!(adaptive_20mb.should_use_quick_index(25 * 1024 * 1024)); // 25MB -> quick
}

#[test]
fn test_open_fast_path_can_display_first_frame() {
    let fast_path = OpenFastPath::default();

    // No index -> cannot display
    let state_none = IndexState::None;
    assert!(!fast_path.can_display_first_frame(&state_none));

    // Quick index with first keyframe -> can display
    let seek_points = vec![SeekPoint {
        display_idx: 0,
        byte_offset: 0,
        is_keyframe: true,
        pts: Some(0),
    }];
    let state_quick = IndexState::Quick(QuickIndex::new(seek_points, 10240));
    assert!(fast_path.can_display_first_frame(&state_quick));
}

#[test]
fn test_open_fast_path_status_message() {
    let fast_path = OpenFastPath::default();

    // None
    let state_none = IndexState::None;
    assert_eq!(fast_path.status_message(&state_none), "No index");

    // Quick
    let seek_points = vec![SeekPoint {
        display_idx: 0,
        byte_offset: 0,
        is_keyframe: true,
        pts: Some(0),
    }];
    let state_quick = IndexState::Quick(QuickIndex::new(seek_points, 10240));
    assert_eq!(fast_path.status_message(&state_quick), "Quick index ready");

    // Building
    let quick = QuickIndex::new(vec![], 10240);
    let partial = FullIndex::new(vec![], 10240, false);
    let progress = IndexProgress::new();
    let state_building = IndexState::Building {
        quick,
        partial,
        progress,
    };
    assert_eq!(
        fast_path.status_message(&state_building),
        "Index building..."
    );

    // Full
    let full = FullIndex::new(vec![], 10240, true);
    let state_full = IndexState::Full(full);
    assert_eq!(fast_path.status_message(&state_full), "Index complete");
}

#[test]
fn test_index_ready_gate_can_access() {
    // Quick index with keyframe at 0
    // With quick index, any frame can be "accessed" by decoding from nearest keyframe
    let seek_points = vec![SeekPoint {
        display_idx: 0,
        byte_offset: 0,
        is_keyframe: true,
        pts: Some(0),
    }];
    let state = IndexState::Quick(QuickIndex::new(seek_points, 10240));
    let gate = IndexReadyGate::new(state);

    assert!(gate.can_access_frame(0)); // Can access keyframe
    assert!(gate.can_access_frame(10)); // Can access via nearest keyframe (0)
    assert!(!gate.is_full_index_ready());
    assert!(!gate.is_indexing());
}

#[test]
fn test_index_ready_gate_constraint_message() {
    // Building state with partial index
    let seek_points = vec![SeekPoint {
        display_idx: 0,
        byte_offset: 0,
        is_keyframe: true,
        pts: Some(0),
    }];
    let quick = QuickIndex::new(seek_points, 10240);

    let frames = vec![FrameMetadata {
        display_idx: 0,
        decode_idx: 0,
        byte_offset: 0,
        size: 512,
        is_keyframe: true,
        pts: Some(0),
        dts: Some(0),
        frame_type: Some("I".to_string()),
    }];
    let partial = FullIndex::new(frames, 10240, false);
    let progress = IndexProgress::new();

    let state = IndexState::Building {
        quick,
        partial,
        progress,
    };
    let gate = IndexReadyGate::new(state);

    // Frame 0 is indexed -> no constraint
    assert!(gate.constraint_message(0).is_none());

    // Frame 10 is not indexed -> constraint message
    let msg = gate.constraint_message(10).unwrap();
    assert_eq!(msg, "Frame not yet indexed. Index building in progress...");

    assert!(gate.is_indexing());
}

#[test]
fn test_index_ready_gate_accessible_range() {
    // Quick index with multiple keyframes
    let seek_points = vec![
        SeekPoint {
            display_idx: 0,
            byte_offset: 0,
            is_keyframe: true,
            pts: Some(0),
        },
        SeekPoint {
            display_idx: 30,
            byte_offset: 1024,
            is_keyframe: true,
            pts: Some(30000),
        },
    ];
    let state = IndexState::Quick(QuickIndex::new(seek_points, 10240));
    let gate = IndexReadyGate::new(state);

    let range = gate.accessible_range().unwrap();
    assert_eq!(range, (0, 30));
}

#[test]
fn test_index_ready_gate_full_index() {
    let frames = vec![
        FrameMetadata {
            display_idx: 0,
            decode_idx: 0,
            byte_offset: 0,
            size: 512,
            is_keyframe: true,
            pts: Some(0),
            dts: Some(0),
            frame_type: Some("I".to_string()),
        },
        FrameMetadata {
            display_idx: 1,
            decode_idx: 1,
            byte_offset: 512,
            size: 256,
            is_keyframe: false,
            pts: Some(1000),
            dts: Some(1000),
            frame_type: Some("P".to_string()),
        },
    ];
    let full = FullIndex::new(frames, 10240, true);
    let state = IndexState::Full(full);
    let gate = IndexReadyGate::new(state);

    assert!(gate.is_full_index_ready());
    assert!(!gate.is_indexing());
    assert!(gate.can_access_frame(0));
    assert!(gate.can_access_frame(1));
    assert!(!gate.can_access_frame(2));

    let range = gate.accessible_range().unwrap();
    assert_eq!(range, (0, 1));
}
