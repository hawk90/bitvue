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
//! Tests for index session evidence integration

use bitvue_core::{
    indexing::{FrameMetadata, SeekPoint},
    IndexExtractorEvidenceManager, IndexSessionEvidenceManager, IndexingPhase, IndexingState,
    SessionOperation,
};
use std::sync::{Arc, Mutex};

fn create_test_frame_evidence_manager() -> Arc<Mutex<IndexExtractorEvidenceManager>> {
    Arc::new(Mutex::new(IndexExtractorEvidenceManager::new_empty()))
}

fn create_test_seekpoint(display_idx: usize) -> SeekPoint {
    SeekPoint {
        display_idx,
        byte_offset: display_idx as u64 * 1024,
        is_keyframe: true,
        pts: Some(display_idx as u64 * 1000),
    }
}

fn create_test_frame_metadata(display_idx: usize) -> FrameMetadata {
    FrameMetadata {
        display_idx,
        decode_idx: display_idx,
        byte_offset: display_idx as u64 * 1024,
        size: 512,
        is_keyframe: display_idx % 5 == 0,
        pts: Some(display_idx as u64 * 1000),
        dts: Some(display_idx as u64 * 1000),
        frame_type: Some(if display_idx % 5 == 0 {
            "I".to_string()
        } else {
            "P".to_string()
        }),
    }
}

#[test]
fn test_session_evidence_creation() {
    let frame_mgr = create_test_frame_evidence_manager();
    let session_mgr = IndexSessionEvidenceManager::new("test_session".to_string(), frame_mgr);

    assert_eq!(session_mgr.session_id(), "test_session");
    assert_eq!(session_mgr.operations().len(), 0);
}

#[test]
fn test_record_quick_index_operation() {
    let frame_mgr = create_test_frame_evidence_manager();

    // Add some seek point evidence
    {
        let mut mgr = frame_mgr.lock().unwrap();
        for i in 0..3 {
            mgr.create_seekpoint_evidence(&create_test_seekpoint(i * 5));
        }
    }

    let mut session_mgr = IndexSessionEvidenceManager::new("test_session".to_string(), frame_mgr);

    let op_id = session_mgr.record_operation(
        SessionOperation::QuickIndexComplete { keyframe_count: 3 },
        IndexingState::QuickIndexing,
        IndexingState::QuickComplete,
        vec![0, 5, 10],
    );

    assert_eq!(session_mgr.operations().len(), 1);
    let op = session_mgr.get_operation(&op_id).unwrap();
    assert_eq!(op.state_before, IndexingState::QuickIndexing);
    assert_eq!(op.state_after, IndexingState::QuickComplete);
    assert_eq!(op.frame_evidence_ids.len(), 3);
}

#[test]
fn test_record_full_index_operation() {
    let frame_mgr = create_test_frame_evidence_manager();

    // Add full frame metadata
    {
        let mut mgr = frame_mgr.lock().unwrap();
        for i in 0..10 {
            mgr.create_frame_metadata_evidence(&create_test_frame_metadata(i));
        }
    }

    let mut session_mgr = IndexSessionEvidenceManager::new("test_session".to_string(), frame_mgr);

    let op_id = session_mgr.record_operation(
        SessionOperation::FullIndexComplete { total_frames: 10 },
        IndexingState::FullIndexing,
        IndexingState::FullComplete,
        (0..10).collect(),
    );

    assert_eq!(session_mgr.operations().len(), 1);
    let op = session_mgr.get_operation(&op_id).unwrap();
    assert_eq!(op.frame_evidence_ids.len(), 10);
}

#[test]
fn test_trace_operation_to_frames() {
    let frame_mgr = create_test_frame_evidence_manager();

    {
        let mut mgr = frame_mgr.lock().unwrap();
        for i in 0..5 {
            mgr.create_seekpoint_evidence(&create_test_seekpoint(i));
        }
    }

    let mut session_mgr = IndexSessionEvidenceManager::new("test_session".to_string(), frame_mgr);

    let op_id = session_mgr.record_operation(
        SessionOperation::QuickIndexComplete { keyframe_count: 5 },
        IndexingState::QuickIndexing,
        IndexingState::QuickComplete,
        vec![0, 1, 2, 3, 4],
    );

    let frames = session_mgr.trace_operation_to_frames(&op_id);
    assert_eq!(frames.len(), 5);
    assert_eq!(frames, vec![0, 1, 2, 3, 4]);
}

#[test]
fn test_trace_frame_to_operations() {
    let frame_mgr = create_test_frame_evidence_manager();

    {
        let mut mgr = frame_mgr.lock().unwrap();
        mgr.create_seekpoint_evidence(&create_test_seekpoint(0));
    }

    let mut session_mgr = IndexSessionEvidenceManager::new("test_session".to_string(), frame_mgr);

    session_mgr.record_operation(
        SessionOperation::QuickIndexComplete { keyframe_count: 1 },
        IndexingState::QuickIndexing,
        IndexingState::QuickComplete,
        vec![0],
    );

    let ops = session_mgr.trace_frame_to_operations(0);
    assert_eq!(ops.len(), 1);
    assert!(matches!(
        ops[0].operation,
        SessionOperation::QuickIndexComplete { .. }
    ));
}

#[test]
fn test_multiple_operations() {
    let frame_mgr = create_test_frame_evidence_manager();
    let mut session_mgr = IndexSessionEvidenceManager::new("test_session".to_string(), frame_mgr);

    // Record multiple operations
    session_mgr.record_operation(
        SessionOperation::QuickIndexStart,
        IndexingState::Idle,
        IndexingState::QuickIndexing,
        vec![],
    );

    session_mgr.record_operation(
        SessionOperation::QuickIndexComplete { keyframe_count: 5 },
        IndexingState::QuickIndexing,
        IndexingState::QuickComplete,
        vec![],
    );

    session_mgr.record_operation(
        SessionOperation::FullIndexStart,
        IndexingState::QuickComplete,
        IndexingState::FullIndexing,
        vec![],
    );

    assert_eq!(session_mgr.operations().len(), 3);

    // Check last operation
    let last_op = session_mgr.last_operation().unwrap();
    assert!(matches!(
        last_op.operation,
        SessionOperation::FullIndexStart
    ));
}

#[test]
fn test_session_stats() {
    let frame_mgr = create_test_frame_evidence_manager();
    let mut session_mgr = IndexSessionEvidenceManager::new("test_session".to_string(), frame_mgr);

    session_mgr.record_operation(
        SessionOperation::QuickIndexComplete { keyframe_count: 3 },
        IndexingState::QuickIndexing,
        IndexingState::QuickComplete,
        vec![],
    );

    session_mgr.record_operation(
        SessionOperation::FullIndexComplete { total_frames: 15 },
        IndexingState::FullIndexing,
        IndexingState::FullComplete,
        vec![],
    );

    session_mgr.record_operation(
        SessionOperation::OperationCancelled {
            phase: IndexingPhase::Full,
        },
        IndexingState::FullIndexing,
        IndexingState::Cancelled,
        vec![],
    );

    let stats = session_mgr.session_stats();
    assert_eq!(stats.total_operations, 3);
    assert_eq!(stats.quick_index_count, 1);
    assert_eq!(stats.full_index_count, 1);
    assert_eq!(stats.total_keyframes_indexed, 3);
    assert_eq!(stats.total_frames_indexed, 15);
    assert_eq!(stats.cancelled_operations, 1);
}

#[test]
fn test_get_operations_by_type() {
    let frame_mgr = create_test_frame_evidence_manager();
    let mut session_mgr = IndexSessionEvidenceManager::new("test_session".to_string(), frame_mgr);

    session_mgr.record_operation(
        SessionOperation::QuickIndexStart,
        IndexingState::Idle,
        IndexingState::QuickIndexing,
        vec![],
    );

    session_mgr.record_operation(
        SessionOperation::QuickIndexComplete { keyframe_count: 5 },
        IndexingState::QuickIndexing,
        IndexingState::QuickComplete,
        vec![],
    );

    session_mgr.record_operation(
        SessionOperation::QuickIndexStart,
        IndexingState::Idle,
        IndexingState::QuickIndexing,
        vec![],
    );

    let start_ops = session_mgr.get_operations_by_type(&SessionOperation::QuickIndexStart);
    assert_eq!(start_ops.len(), 2);
}

#[test]
fn test_clear() {
    let frame_mgr = create_test_frame_evidence_manager();
    let mut session_mgr = IndexSessionEvidenceManager::new("test_session".to_string(), frame_mgr);

    session_mgr.record_operation(
        SessionOperation::QuickIndexComplete { keyframe_count: 3 },
        IndexingState::QuickIndexing,
        IndexingState::QuickComplete,
        vec![],
    );

    assert_eq!(session_mgr.operations().len(), 1);

    session_mgr.clear();

    assert_eq!(session_mgr.operations().len(), 0);
}

#[test]
fn test_error_operation() {
    let frame_mgr = create_test_frame_evidence_manager();
    let mut session_mgr = IndexSessionEvidenceManager::new("test_session".to_string(), frame_mgr);

    session_mgr.record_operation(
        SessionOperation::OperationError {
            phase: IndexingPhase::Quick,
            error_message: "Test error".to_string(),
        },
        IndexingState::QuickIndexing,
        IndexingState::Error,
        vec![],
    );

    let stats = session_mgr.session_stats();
    assert_eq!(stats.error_operations, 1);
}
