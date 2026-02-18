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
//! Tests for index session management

use bitvue_core::{
    Av1IndexExtractor, H264IndexExtractor, IndexSession, IndexingProgress, IndexingState,
};
use std::io::Cursor;

fn create_minimal_av1_stream() -> Vec<u8> {
    // Minimal AV1 stream with one sequence header OBU
    vec![
        0x0A, // OBU header: type=1 (sequence header), has_size=1
        0x05, // Size = 5 bytes
        0x00, 0x00, 0x00, 0x00, 0x00, // Dummy sequence header data
    ]
}

#[test]
fn test_session_initial_state() {
    let session = IndexSession::new();
    assert_eq!(session.state(), IndexingState::Idle);
    assert!(!session.is_quick_complete());
    assert!(!session.is_full_complete());
    assert!(session.quick_index().is_none());
    assert!(session.full_index().is_none());
}

#[test]
fn test_quick_index_execution() {
    let session = IndexSession::new();
    let extractor = Av1IndexExtractor::new();
    let data = create_minimal_av1_stream();
    let mut cursor = Cursor::new(data);

    let result = session.execute_quick_index(&extractor, &mut cursor, None::<fn(IndexingProgress)>);

    assert!(result.is_ok());
    assert_eq!(session.state(), IndexingState::QuickComplete);
    assert!(session.is_quick_complete());
    assert!(session.quick_index().is_some());

    let quick_idx = session.quick_index().unwrap();
    assert_eq!(quick_idx.seek_points.len(), 1);
}

#[test]
fn test_quick_index_with_progress() {
    use std::cell::RefCell;

    let session = IndexSession::new();
    let extractor = Av1IndexExtractor::new();
    let data = create_minimal_av1_stream();
    let mut cursor = Cursor::new(data);

    let progress_updates = RefCell::new(Vec::new());
    let result = session.execute_quick_index(
        &extractor,
        &mut cursor,
        Some(|p: IndexingProgress| {
            progress_updates.borrow_mut().push(p);
        }),
    );

    assert!(result.is_ok());
    let updates = progress_updates.borrow();
    assert!(!updates.is_empty());

    // Should have start and completion updates
    assert!(updates.iter().any(|p| p.progress == 0.0));
    assert!(updates.iter().any(|p| p.progress == 1.0));
}

#[test]
fn test_full_index_requires_quick_first() {
    let session = IndexSession::new();
    let extractor = Av1IndexExtractor::new();
    let data = create_minimal_av1_stream();
    let mut cursor = Cursor::new(data);

    // Try to run full index without quick index first
    let result = session.execute_full_index(&extractor, &mut cursor, None::<fn(IndexingProgress)>);

    assert!(result.is_err());
    assert_eq!(session.state(), IndexingState::Idle);
}

#[test]
fn test_full_workflow() {
    let session = IndexSession::new();
    let extractor = Av1IndexExtractor::new();
    let data = create_minimal_av1_stream();
    let mut cursor = Cursor::new(data);

    let result =
        session.execute_full_workflow(&extractor, &mut cursor, None::<fn(IndexingProgress)>);

    assert!(result.is_ok());
    assert_eq!(session.state(), IndexingState::FullComplete);
    assert!(session.is_quick_complete());
    assert!(session.is_full_complete());

    let (quick_idx, full_idx) = result.unwrap();
    assert_eq!(quick_idx.seek_points.len(), 1);
    assert_eq!(full_idx.frames.len(), 1);
    assert!(full_idx.is_complete);
}

#[test]
fn test_reset() {
    let session = IndexSession::new();
    let extractor = Av1IndexExtractor::new();
    let data = create_minimal_av1_stream();
    let mut cursor = Cursor::new(data);

    // Run quick index
    let _ = session.execute_quick_index(&extractor, &mut cursor, None::<fn(IndexingProgress)>);
    assert_eq!(session.state(), IndexingState::QuickComplete);

    // Reset
    session.reset();
    assert_eq!(session.state(), IndexingState::Idle);
    assert!(session.quick_index().is_none());
    assert!(session.full_index().is_none());
}

#[test]
fn test_estimated_progress() {
    let session = IndexSession::new();

    // Idle state
    assert_eq!(session.estimated_progress(), 0.0);
}

#[test]
fn test_evidence_integration() {
    let session = IndexSession::new();
    let extractor = Av1IndexExtractor::new();
    let data = create_minimal_av1_stream();
    let mut cursor = Cursor::new(data);

    // Run quick index
    let _ = session.execute_quick_index(&extractor, &mut cursor, None::<fn(IndexingProgress)>);

    // Check evidence manager has evidence
    let evidence_mgr = session.evidence_manager();
    let evidence_mgr = evidence_mgr.lock().unwrap();
    assert_eq!(evidence_mgr.frame_count(), 1);

    // Check we can trace evidence
    let bit_range = evidence_mgr.trace_to_bit_offset(0);
    assert!(bit_range.is_some());
}

#[test]
fn test_multiple_sessions_independent() {
    let session1 = IndexSession::new();
    let session2 = IndexSession::new();

    // Cancel session1
    session1.cancel();

    // Reset session1
    session1.reset();
}

// H.264-specific integration tests

#[test]
fn test_h264_quick_index_session() {
    // Create H.264 stream with keyframes
    let mut data = Vec::new();
    for _ in 0..3 {
        data.extend_from_slice(&[0x00, 0x00, 0x00, 0x01, 0x65]);
        data.extend_from_slice(&[0x00; 20]);
    }

    let session = IndexSession::new();
    let extractor = H264IndexExtractor::new();
    let mut cursor = Cursor::new(data);

    let result = session.execute_quick_index(&extractor, &mut cursor, None::<fn(IndexingProgress)>);

    assert!(result.is_ok());
    assert_eq!(session.state(), IndexingState::QuickComplete);
    assert!(session.quick_index().is_some());

    let quick_idx = session.quick_index().unwrap();
    assert_eq!(quick_idx.seek_points.len(), 3);
}

#[test]
fn test_h264_full_index_session() {
    // Create H.264 stream
    let mut data = Vec::new();
    // IDR frame
    data.extend_from_slice(&[0x00, 0x00, 0x00, 0x01, 0x65]);
    data.extend_from_slice(&[0xAA; 10]);
    // Non-IDR frames
    for _ in 0..4 {
        data.extend_from_slice(&[0x00, 0x00, 0x00, 0x01, 0x41]);
        data.extend_from_slice(&[0xBB; 10]);
    }

    let session = IndexSession::new();
    let extractor = H264IndexExtractor::new();

    // First execute quick index
    let mut cursor = Cursor::new(data.clone());
    session
        .execute_quick_index(&extractor, &mut cursor, None::<fn(IndexingProgress)>)
        .unwrap();
    assert_eq!(session.state(), IndexingState::QuickComplete);

    // Then execute full index (needs fresh cursor)
    let mut cursor = Cursor::new(data);
    let result = session.execute_full_index(&extractor, &mut cursor, None::<fn(IndexingProgress)>);

    assert!(result.is_ok());
    assert_eq!(session.state(), IndexingState::FullComplete);
    assert!(session.full_index().is_some());

    let full_idx = session.full_index().unwrap();
    assert_eq!(full_idx.frames.len(), 5);
    assert!(full_idx.frames[0].is_keyframe);
    assert!(!full_idx.frames[1].is_keyframe);
}

#[test]
fn test_h264_session_with_evidence() {
    let mut data = Vec::new();
    data.extend_from_slice(&[0x00, 0x00, 0x00, 0x01, 0x65]);
    data.extend_from_slice(&[0x00; 15]);

    let session = IndexSession::new();
    let extractor = H264IndexExtractor::new();
    let mut cursor = Cursor::new(data);

    // Execute quick index
    session
        .execute_quick_index(&extractor, &mut cursor, None::<fn(IndexingProgress)>)
        .unwrap();

    // Verify evidence integration
    let evidence_mgr = session.evidence_manager();
    let evidence_mgr = evidence_mgr.lock().unwrap();

    assert_eq!(evidence_mgr.frame_count(), 1);
    let bit_range = evidence_mgr.trace_to_bit_offset(0);
    assert!(bit_range.is_some());
}
