// Indexing module tests
//
// Per generate-tests skill: Arrange-Act-Assert pattern with fixtures
// and edge case coverage.

use super::*;

// ============================================================================
// Fixtures
// ============================================================================

/// Create test seek points
fn create_test_seek_points(count: usize, start_pts: u64, pts_delta: u64) -> Vec<SeekPoint> {
    (0..count)
        .map(|i| SeekPoint {
            display_idx: i * 10, // Every 10th frame is keyframe
            byte_offset: i as u64 * 1000,
            is_keyframe: true,
            pts: Some(start_pts + i as u64 * pts_delta),
        })
        .collect()
}

/// Create test FrameMetadata with PTS
fn create_test_frames_with_pts(count: usize, start_pts: u64, pts_delta: u64) -> Vec<FrameMetadata> {
    (0..count)
        .map(|i| FrameMetadata {
            display_idx: i,
            decode_idx: i,
            byte_offset: i as u64 * 100,
            size: 100,
            is_keyframe: i % 10 == 0, // Every 10th frame is keyframe
            pts: Some(start_pts + i as u64 * pts_delta),
            dts: Some(i as u64 * pts_delta),
            frame_type: None,
        })
        .collect()
}

/// Create test FrameMetadata without PTS
fn create_test_frames_no_pts(count: usize) -> Vec<FrameMetadata> {
    (0..count)
        .map(|i| FrameMetadata {
            display_idx: i,
            decode_idx: i,
            byte_offset: i as u64 * 100,
            size: 100,
            is_keyframe: i % 10 == 0,
            pts: None,
            dts: Some(i as u64 * 33),
            frame_type: None,
        })
        .collect()
}

/// Create test QuickIndex
fn create_test_quick_index(count: usize) -> QuickIndex {
    let seek_points = create_test_seek_points(count, 1000, 33);
    QuickIndex::new(seek_points, count as u64 * 1000)
}

/// Create test FullIndex
fn create_test_full_index(count: usize) -> FullIndex {
    let frames = create_test_frames_with_pts(count, 1000, 33);
    let file_size = count as u64 * 100;
    FullIndex::new(frames, file_size, true)
}

// ============================================================================
// SeekPoint Tests
// ============================================================================

#[cfg(test)]
mod seek_point_tests {
    use super::*;

    #[test]
    fn test_seek_point_creation() {
        // Arrange & Act
        let point = SeekPoint {
            display_idx: 100,
            byte_offset: 50000,
            is_keyframe: true,
            pts: Some(3300),
        };

        // Assert
        assert_eq!(point.display_idx, 100);
        assert_eq!(point.byte_offset, 50000);
        assert!(point.is_keyframe);
        assert_eq!(point.pts, Some(3300));
    }

    #[test]
    fn test_seek_point_without_pts() {
        // Arrange & Act
        let point = SeekPoint {
            display_idx: 0,
            byte_offset: 0,
            is_keyframe: true,
            pts: None,
        };

        // Assert
        assert_eq!(point.pts, None);
    }

    #[test]
    fn test_seek_point_non_keyframe() {
        // Arrange & Act
        let point = SeekPoint {
            display_idx: 50,
            byte_offset: 25000,
            is_keyframe: false,
            pts: Some(1650),
        };

        // Assert
        assert!(!point.is_keyframe);
    }
}

// ============================================================================
// QuickIndex Tests
// ============================================================================

#[cfg(test)]
mod quick_index_tests {
    use super::*;

    #[test]
    fn test_quick_index_new() {
        // Arrange
        let seek_points = create_test_seek_points(10, 1000, 33);

        // Act
        let index = QuickIndex::new(seek_points.clone(), 10000);

        // Assert
        assert_eq!(index.seek_points.len(), 10);
        assert_eq!(index.file_size, 10000);
    }

    #[test]
    fn test_quick_index_empty() {
        // Arrange & Act
        let index = QuickIndex::new(vec![], 0);

        // Assert
        assert!(index.seek_points.is_empty());
        assert_eq!(index.file_size, 0);
    }

    #[test]
    fn test_find_nearest_keyframe_exact_match() {
        // Arrange
        let index = create_test_quick_index(10);

        // Act
        let result = index.find_nearest_keyframe(50);

        // Assert
        assert!(result.is_some());
        let point = result.unwrap();
        assert_eq!(point.display_idx, 50);
    }

    #[test]
    fn test_find_nearest_keyframe_between_keyframes() {
        // Arrange
        let index = create_test_quick_index(10);

        // Act - Between keyframe at 0 and 10
        let result = index.find_nearest_keyframe(5);

        // Assert - Should return keyframe 0
        assert!(result.is_some());
        let point = result.unwrap();
        assert_eq!(point.display_idx, 0);
    }

    #[test]
    fn test_find_nearest_keyframe_before_first() {
        // Arrange
        let index = create_test_quick_index(10); // First keyframe at 0

        // Act - Negative index would be 0
        let result = index.find_nearest_keyframe(0);

        // Assert
        assert!(result.is_some());
        assert_eq!(result.unwrap().display_idx, 0);
    }

    #[test]
    fn test_find_nearest_keyframe_after_last() {
        // Arrange
        let index = create_test_quick_index(10); // Last keyframe at 90

        // Act
        let result = index.find_nearest_keyframe(100);

        // Assert - Should return last keyframe
        assert!(result.is_some());
        assert_eq!(result.unwrap().display_idx, 90);
    }

    #[test]
    fn test_find_nearest_keyframe_empty_index() {
        // Arrange
        let index = QuickIndex::new(vec![], 0);

        // Act
        let result = index.find_nearest_keyframe(50);

        // Assert
        assert!(result.is_none());
    }

    #[test]
    fn test_quick_indexable_trait_on_full_index() {
        // Arrange
        let frames = create_test_frames_with_pts(100, 1000, 33);
        let full_index = FullIndex::new(frames, 10000, true);

        // Act
        let quick_index = full_index.to_quick_index();

        // Assert - Should create a quick index with keyframes
        assert!(!quick_index.seek_points.is_empty());
    }
}

// ============================================================================
// IndexProgress Tests
// ============================================================================

#[cfg(test)]
mod index_progress_tests {
    use super::*;

    #[test]
    fn test_index_progress_new() {
        // Arrange & Act
        let progress = IndexProgress::new();

        // Assert
        assert_eq!(progress.progress(), 0.0);
        assert!(!progress.is_complete());
        assert!(!progress.is_cancelled());
        assert_eq!(progress.status(), "Starting...");
    }

    #[test]
    fn test_index_progress_default() {
        // Arrange & Act
        let progress = IndexProgress::default();

        // Assert
        assert_eq!(progress.progress(), 0.0);
    }

    #[test]
    fn test_set_progress() {
        // Arrange
        let progress = IndexProgress::new();

        // Act
        progress.set_progress(0.5);

        // Assert
        assert_eq!(progress.progress(), 0.5);
    }

    #[test]
    fn test_set_progress_clamps_to_one() {
        // Arrange
        let progress = IndexProgress::new();

        // Act
        progress.set_progress(1.5);

        // Assert
        assert_eq!(progress.progress(), 1.0);
    }

    #[test]
    fn test_set_progress_clamps_to_zero() {
        // Arrange
        let progress = IndexProgress::new();

        // Act
        progress.set_progress(-0.5);

        // Assert
        assert_eq!(progress.progress(), 0.0);
    }

    #[test]
    fn test_mark_complete() {
        // Arrange
        let progress = IndexProgress::new();
        progress.set_progress(0.75);

        // Act
        progress.mark_complete();

        // Assert
        assert_eq!(progress.progress(), 1.0);
        assert!(progress.is_complete());
        assert_eq!(progress.status(), "Complete");
    }

    #[test]
    fn test_cancel() {
        // Arrange
        let progress = IndexProgress::new();

        // Act
        progress.cancel();

        // Assert
        assert!(progress.is_cancelled());
        assert_eq!(progress.status(), "Cancelled");
    }

    #[test]
    fn test_set_status() {
        // Arrange
        let progress = IndexProgress::new();

        // Act
        progress.set_status("Processing frame 100...");

        // Assert
        assert_eq!(progress.status(), "Processing frame 100...");
    }

    #[test]
    fn test_multiple_progress_updates() {
        // Arrange
        let progress = IndexProgress::new();

        // Act
        for i in 0..=10 {
            progress.set_progress(i as f64 / 10.0);
        }

        // Assert
        assert_eq!(progress.progress(), 1.0);
    }
}

// ============================================================================
// IndexState Tests
// ============================================================================

#[cfg(test)]
mod index_state_tests {
    use super::*;

    #[test]
    fn test_index_state_none() {
        // Arrange & Act
        let state = IndexState::None;

        // Assert
        assert!(state.quick().is_none());
        assert!(state.full().is_none());
        assert!(!state.has_full_index());
        assert!(!state.is_building());
        assert!(state.progress().is_none());
    }

    #[test]
    fn test_index_state_quick() {
        // Arrange
        let quick = create_test_quick_index(10);

        // Act
        let state = IndexState::Quick(quick);

        // Assert
        assert!(state.quick().is_some());
        assert!(state.full().is_none());
        assert!(!state.has_full_index());
        assert!(!state.is_building());
        assert!(state.progress().is_none());
    }

    #[test]
    fn test_index_state_full() {
        // Arrange
        let full = create_test_full_index(100);

        // Act
        let state = IndexState::Full(full);

        // Assert
        assert!(state.quick().is_none()); // Full doesn't expose quick
        assert!(state.full().is_some());
        assert!(state.has_full_index());
        assert!(!state.is_building());
        assert!(state.progress().is_none());
    }

    #[test]
    fn test_index_state_building() {
        // Arrange
        let quick = create_test_quick_index(10);
        let partial = create_test_full_index(50);
        let progress = IndexProgress::new();

        // Act
        let state = IndexState::Building {
            quick,
            partial,
            progress,
        };

        // Assert
        assert!(state.quick().is_some());
        assert!(state.full().is_none()); // Partial not exposed as full
        assert!(!state.has_full_index());
        assert!(state.is_building());
        assert!(state.progress().is_some());
    }

    #[test]
    fn test_can_access_none_state() {
        // Arrange
        let state = IndexState::None;

        // Act & Assert
        assert!(!state.can_access(0));
        assert!(!state.can_access(100));
    }

    #[test]
    fn test_can_access_quick_state() {
        // Arrange
        let quick = create_test_quick_index(10);
        let state = IndexState::Quick(quick);

        // Act & Assert - Can access keyframes (0, 10, 20, ...)
        assert!(state.can_access(0));
        assert!(state.can_access(10));
        assert!(state.can_access(50));
        // Non-keyframes in range should also work via nearest
        assert!(state.can_access(5));
    }

    #[test]
    fn test_can_access_building_state() {
        // Arrange
        let quick = create_test_quick_index(10);
        let partial = create_test_full_index(50); // 50 frames indexed
        let progress = IndexProgress::new();
        let state = IndexState::Building {
            quick,
            partial,
            progress,
        };

        // Act & Assert
        assert!(state.can_access(0));
        assert!(state.can_access(25));
        assert!(state.can_access(49));
        assert!(!state.can_access(50)); // Out of range
    }

    #[test]
    fn test_can_access_full_state() {
        // Arrange
        let full = create_test_full_index(100);
        let state = IndexState::Full(full);

        // Act & Assert
        assert!(state.can_access(0));
        assert!(state.can_access(99));
        assert!(!state.can_access(100)); // Out of range
    }
}

// ============================================================================
// OpenStrategy Tests
// ============================================================================

#[cfg(test)]
mod open_strategy_tests {
    use super::*;

    #[test]
    fn test_open_strategy_fast_path() {
        // Arrange & Act
        let fast_path = OpenFastPath::new(OpenStrategy::FastPath);

        // Assert - Always use quick index regardless of file size
        assert!(fast_path.should_use_quick_index(0));
        assert!(fast_path.should_use_quick_index(1_000_000));
        assert!(fast_path.should_use_quick_index(u64::MAX));
    }

    #[test]
    fn test_open_strategy_full_path() {
        // Arrange & Act
        let fast_path = OpenFastPath::new(OpenStrategy::FullPath);

        // Assert - Never use quick index
        assert!(!fast_path.should_use_quick_index(0));
        assert!(!fast_path.should_use_quick_index(1_000_000));
        assert!(!fast_path.should_use_quick_index(u64::MAX));
    }

    #[test]
    fn test_open_strategy_adaptive_small_file() {
        // Arrange
        let fast_path = OpenFastPath::new(OpenStrategy::Adaptive);

        // Act - Small file (< 10MB)
        let use_quick = fast_path.should_use_quick_index(5 * 1024 * 1024);

        // Assert - Don't use quick index for small files
        assert!(!use_quick);
    }

    #[test]
    fn test_open_strategy_adaptive_large_file() {
        // Arrange
        let fast_path = OpenFastPath::new(OpenStrategy::Adaptive);

        // Act - Large file (>= 10MB)
        let use_quick = fast_path.should_use_quick_index(15 * 1024 * 1024);

        // Assert - Use quick index for large files
        assert!(use_quick);
    }

    #[test]
    fn test_open_strategy_adaptive_threshold() {
        // Arrange
        let fast_path = OpenFastPath::new(OpenStrategy::Adaptive);

        // Act - Exactly at threshold (10MB)
        let use_quick = fast_path.should_use_quick_index(10 * 1024 * 1024);

        // Assert - Should use quick index at threshold
        assert!(use_quick);
    }

    #[test]
    fn test_open_strategy_with_custom_threshold() {
        // Arrange
        let fast_path = OpenFastPath::new(OpenStrategy::Adaptive)
            .with_adaptive_threshold(5 * 1024 * 1024); // 5MB threshold

        // Act - File at new threshold
        let use_quick = fast_path.should_use_quick_index(5 * 1024 * 1024);

        // Assert
        assert!(use_quick);
    }

    #[test]
    fn test_open_strategy_default() {
        // Arrange & Act
        let fast_path = OpenFastPath::default();

        // Assert - Default is Adaptive
        assert!(fast_path.should_use_quick_index(20 * 1024 * 1024));
        assert!(!fast_path.should_use_quick_index(5 * 1024 * 1024));
    }
}

// ============================================================================
// OpenFastPath Tests
// ============================================================================

#[cfg(test)]
mod open_fast_path_tests {
    use super::*;

    #[test]
    fn test_status_message_none() {
        // Arrange
        let fast_path = OpenFastPath::new(OpenStrategy::FastPath);
        let state = IndexState::None;

        // Act
        let msg = fast_path.status_message(&state);

        // Assert
        assert_eq!(msg, "No index");
    }

    #[test]
    fn test_status_message_quick() {
        // Arrange
        let fast_path = OpenFastPath::new(OpenStrategy::FastPath);
        let quick = create_test_quick_index(10);
        let state = IndexState::Quick(quick);

        // Act
        let msg = fast_path.status_message(&state);

        // Assert
        assert_eq!(msg, "Quick index ready");
    }

    #[test]
    fn test_status_message_building() {
        // Arrange
        let fast_path = OpenFastPath::new(OpenStrategy::FastPath);
        let quick = create_test_quick_index(10);
        let partial = create_test_full_index(50);
        let progress = IndexProgress::new();
        let state = IndexState::Building {
            quick,
            partial,
            progress,
        };

        // Act
        let msg = fast_path.status_message(&state);

        // Assert
        assert_eq!(msg, "Index building...");
    }

    #[test]
    fn test_status_message_full() {
        // Arrange
        let fast_path = OpenFastPath::new(OpenStrategy::FastPath);
        let full = create_test_full_index(100);
        let state = IndexState::Full(full);

        // Act
        let msg = fast_path.status_message(&state);

        // Assert
        assert_eq!(msg, "Index complete");
    }

    #[test]
    fn test_can_display_first_frame_none() {
        // Arrange
        let fast_path = OpenFastPath::new(OpenStrategy::FastPath);
        let state = IndexState::None;

        // Act
        let can_display = fast_path.can_display_first_frame(&state);

        // Assert
        assert!(!can_display);
    }

    #[test]
    fn test_can_display_first_frame_quick() {
        // Arrange
        let fast_path = OpenFastPath::new(OpenStrategy::FastPath);
        let quick = create_test_quick_index(10);
        let state = IndexState::Quick(quick);

        // Act
        let can_display = fast_path.can_display_first_frame(&state);

        // Assert
        assert!(can_display); // Frame 0 is a keyframe
    }

    #[test]
    fn test_can_display_first_frame_building() {
        // Arrange
        let fast_path = OpenFastPath::new(OpenStrategy::FastPath);
        let quick = create_test_quick_index(10);
        let partial = create_test_full_index(10); // Has frame 0
        let progress = IndexProgress::new();
        let state = IndexState::Building {
            quick,
            partial,
            progress,
        };

        // Act
        let can_display = fast_path.can_display_first_frame(&state);

        // Assert
        assert!(can_display);
    }
}

// ============================================================================
// IndexReadyGate Tests
// ============================================================================

#[cfg(test)]
mod index_ready_gate_tests {
    use super::*;

    #[test]
    fn test_gate_can_access_frame_none() {
        // Arrange
        let gate = IndexReadyGate::new(IndexState::None);

        // Act & Assert
        assert!(!gate.can_access_frame(0));
        assert!(!gate.can_access_frame(100));
    }

    #[test]
    fn test_gate_can_access_frame_quick() {
        // Arrange
        let quick = create_test_quick_index(10);
        let gate = IndexReadyGate::new(IndexState::Quick(quick));

        // Act & Assert
        assert!(gate.can_access_frame(0)); // Keyframe
        assert!(gate.can_access_frame(50)); // Has keyframe nearby
    }

    #[test]
    fn test_gate_can_access_frame_full() {
        // Arrange
        let full = create_test_full_index(100);
        let gate = IndexReadyGate::new(IndexState::Full(full));

        // Act & Assert
        assert!(gate.can_access_frame(0));
        assert!(gate.can_access_frame(99));
        assert!(!gate.can_access_frame(100)); // Out of range
    }

    #[test]
    fn test_gate_is_full_index_ready_none() {
        // Arrange
        let gate = IndexReadyGate::new(IndexState::None);

        // Act
        let ready = gate.is_full_index_ready();

        // Assert
        assert!(!ready);
    }

    #[test]
    fn test_gate_is_full_index_ready_quick() {
        // Arrange
        let quick = create_test_quick_index(10);
        let gate = IndexReadyGate::new(IndexState::Quick(quick));

        // Act
        let ready = gate.is_full_index_ready();

        // Assert
        assert!(!ready);
    }

    #[test]
    fn test_gate_is_full_index_ready_full() {
        // Arrange
        let full = create_test_full_index(100);
        let gate = IndexReadyGate::new(IndexState::Full(full));

        // Act
        let ready = gate.is_full_index_ready();

        // Assert
        assert!(ready);
    }

    #[test]
    fn test_gate_is_indexing() {
        // Arrange
        let quick = create_test_quick_index(10);
        let partial = create_test_full_index(50);
        let progress = IndexProgress::new();
        let state = IndexState::Building {
            quick,
            partial,
            progress,
        };
        let gate = IndexReadyGate::new(state);

        // Act
        let indexing = gate.is_indexing();

        // Assert
        assert!(indexing);
    }

    #[test]
    fn test_gate_constraint_message_not_indexing() {
        // Arrange
        let gate = IndexReadyGate::new(IndexState::None);

        // Act
        let msg = gate.constraint_message(100);

        // Assert
        assert!(msg.is_some());
        assert_eq!(msg.unwrap(), "Frame not accessible with current index");
    }

    #[test]
    fn test_gate_constraint_message_indexing() {
        // Arrange
        let quick = create_test_quick_index(10);
        let partial = create_test_full_index(10);
        let progress = IndexProgress::new();
        let state = IndexState::Building {
            quick,
            partial,
            progress,
        };
        let gate = IndexReadyGate::new(state);

        // Act - Frame outside partial range
        let msg = gate.constraint_message(50);

        // Assert
        assert!(msg.is_some());
        assert!(msg.unwrap().contains("Index building in progress"));
    }

    #[test]
    fn test_gate_constraint_message_accessible() {
        // Arrange
        let full = create_test_full_index(100);
        let gate = IndexReadyGate::new(IndexState::Full(full));

        // Act
        let msg = gate.constraint_message(50);

        // Assert
        assert!(msg.is_none()); // Frame is accessible
    }

    #[test]
    fn test_gate_accessible_range_none() {
        // Arrange
        let gate = IndexReadyGate::new(IndexState::None);

        // Act
        let range = gate.accessible_range();

        // Assert
        assert!(range.is_none());
    }

    #[test]
    fn test_gate_accessible_range_quick() {
        // Arrange
        let quick = create_test_quick_index(10); // 0 to 90
        let gate = IndexReadyGate::new(IndexState::Quick(quick));

        // Act
        let range = gate.accessible_range();

        // Assert
        assert!(range.is_some());
        assert_eq!(range.unwrap(), (0, 90));
    }

    #[test]
    fn test_gate_accessible_range_building() {
        // Arrange
        let quick = create_test_quick_index(10);
        let partial = create_test_full_index(50); // 50 frames
        let progress = IndexProgress::new();
        let state = IndexState::Building {
            quick,
            partial,
            progress,
        };
        let gate = IndexReadyGate::new(state);

        // Act
        let range = gate.accessible_range();

        // Assert
        assert!(range.is_some());
        assert_eq!(range.unwrap(), (0, 49));
    }

    #[test]
    fn test_gate_accessible_range_full() {
        // Arrange
        let full = create_test_full_index(100);
        let gate = IndexReadyGate::new(IndexState::Full(full));

        // Act
        let range = gate.accessible_range();

        // Assert
        assert!(range.is_some());
        assert_eq!(range.unwrap(), (0, 99));
    }

    #[test]
    fn test_gate_accessible_range_empty_full_index() {
        // Arrange
        let full = FullIndex::new(vec![], 0, true);
        let gate = IndexReadyGate::new(IndexState::Full(full));

        // Act
        let range = gate.accessible_range();

        // Assert
        assert!(range.is_none());
    }
}

// ============================================================================
// Edge Cases
// ============================================================================

#[cfg(test)]
mod edge_case_tests {
    use super::*;

    #[test]
    fn test_quick_index_single_seek_point() {
        // Arrange
        let seek_points = vec![SeekPoint {
            display_idx: 0,
            byte_offset: 0,
            is_keyframe: true,
            pts: Some(0),
        }];

        // Act
        let index = QuickIndex::new(seek_points, 1000);

        // Assert
        assert_eq!(index.seek_points.len(), 1);
        assert!(index.find_nearest_keyframe(0).is_some());
        assert!(index.find_nearest_keyframe(100).is_some()); // Returns the only point
    }

    #[test]
    fn test_full_index_empty() {
        // Arrange & Act
        let index = FullIndex::new(vec![], 0, true);

        // Assert
        assert!(index.frames.is_empty());
        assert!(!index.contains(0));
    }

    #[test]
    fn test_full_index_single_frame() {
        // Arrange
        let frames = create_test_frames_with_pts(1, 1000, 33);

        // Act
        let index = FullIndex::new(frames, 100, true);

        // Assert
        assert_eq!(index.frames.len(), 1);
        assert!(index.contains(0));
        assert!(!index.contains(1));
    }

    #[test]
    fn test_index_progress_concurrent_updates() {
        // Arrange - Create two progress trackers
        let progress1 = IndexProgress::new();
        let progress2 = IndexProgress::new();

        // Act - Update both independently
        progress1.set_progress(0.5);
        progress2.set_progress(0.75);

        // Assert - Each should have its own value (not shared Arc clones)
        assert_eq!(progress1.progress(), 0.5);
        assert_eq!(progress2.progress(), 0.75);
    }

    #[test]
    fn test_index_state_quick_index_nearest_keyframe_at_boundary() {
        // Arrange
        let seek_points = vec![
            SeekPoint {
                display_idx: 0,
                byte_offset: 0,
                is_keyframe: true,
                pts: Some(0),
            },
            SeekPoint {
                display_idx: 100,
                byte_offset: 10000,
                is_keyframe: true,
                pts: Some(3300),
            },
        ];
        let quick = QuickIndex::new(seek_points, 20000);

        // Act
        let state = IndexState::Quick(quick);

        // Assert - Boundary cases
        assert!(state.can_access(0));   // Exact keyframe
        assert!(state.can_access(99));  // Should find keyframe at 0
        assert!(state.can_access(100)); // Exact keyframe
    }

    #[test]
    fn test_open_fast_path_zero_file_size() {
        // Arrange
        let fast_path = OpenFastPath::new(OpenStrategy::Adaptive);

        // Act - Zero byte file
        let use_quick = fast_path.should_use_quick_index(0);

        // Assert - Should use full path for empty files
        assert!(!use_quick);
    }

    #[test]
    fn test_gate_constraint_message_frame_zero_none_state() {
        // Arrange
        let gate = IndexReadyGate::new(IndexState::None);

        // Act
        let msg = gate.constraint_message(0);

        // Assert - Even frame 0 is not accessible without index
        assert!(msg.is_some());
    }
}
