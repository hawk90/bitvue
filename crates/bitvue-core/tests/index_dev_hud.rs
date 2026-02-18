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
//! Tests for index_dev_hud module

use bitvue_core::{
    Av1IndexExtractor, IndexDevHUD, IndexExtractorEvidenceManager, IndexSession,
    IndexSessionEvidenceManager, IndexSessionWindow, IndexWindowPolicy, IndexingState, SeekPoint,
};
use std::io::Cursor;
use std::sync::{Arc, Mutex};

fn create_minimal_av1_stream() -> Vec<u8> {
    vec![0x0A, 0x05, 0x00, 0x00, 0x00, 0x00, 0x00]
}

#[test]
fn test_dev_hud_creation() {
    let hud = IndexDevHUD::new("test_session".to_string());

    assert_eq!(hud.session_id(), "test_session");
    assert_eq!(hud.update_count(), 0);
    assert_eq!(hud.session_snapshot().state, IndexingState::Idle);
}

#[test]
fn test_update_from_session() {
    let mut hud = IndexDevHUD::new("test_session".to_string());
    let session = IndexSession::new();

    hud.update_from_session(&session);

    assert_eq!(hud.update_count(), 1);
    assert_eq!(hud.session_snapshot().state, IndexingState::Idle);
    assert!(!hud.session_snapshot().quick_complete);
    assert!(!hud.session_snapshot().full_complete);
}

#[test]
fn test_update_from_session_with_index() {
    let mut hud = IndexDevHUD::new("test_session".to_string());
    let session = IndexSession::new();
    let extractor = Av1IndexExtractor::new();
    let data = create_minimal_av1_stream();
    let mut cursor = Cursor::new(data);

    // Run quick index
    let _ = session.execute_quick_index(&extractor, &mut cursor, None::<fn(_)>);

    hud.update_from_session(&session);

    assert_eq!(hud.session_snapshot().state, IndexingState::QuickComplete);
    assert!(hud.session_snapshot().quick_complete);
}

#[test]
fn test_update_from_window() {
    let mut hud = IndexDevHUD::new("test_session".to_string());

    let sparse_kf = vec![SeekPoint {
        display_idx: 0,
        byte_offset: 0,
        is_keyframe: true,
        pts: None,
    }];

    let window = IndexSessionWindow::new(
        "test_session".to_string(),
        100,
        IndexWindowPolicy::Fixed(20),
        sparse_kf,
    );

    hud.update_from_window(&window);

    assert_eq!(hud.update_count(), 1);
    let snapshot = hud.window_snapshot().unwrap();
    assert_eq!(snapshot.total_frames, 100);
    assert_eq!(snapshot.window_size, 20);
}

#[test]
fn test_update_from_evidence() {
    let mut hud = IndexDevHUD::new("test_session".to_string());

    let frame_evidence = Arc::new(Mutex::new(IndexExtractorEvidenceManager::new_empty()));
    let session_evidence =
        IndexSessionEvidenceManager::new("test_session".to_string(), Arc::clone(&frame_evidence));

    hud.update_from_evidence(frame_evidence, &session_evidence);

    assert_eq!(hud.update_count(), 1);
    let snapshot = hud.evidence_snapshot();
    assert_eq!(snapshot.frame_evidence_count, 0);
    assert_eq!(snapshot.session_operation_count, 0);
}

#[test]
fn test_update_performance() {
    let mut hud = IndexDevHUD::new("test_session".to_string());

    // Set frame count first
    hud.update_from_session(&IndexSession::new());
    // Manually set the actual_frames_indexed for this test
    // (In real usage this would come from a completed full index)

    hud.update_performance(Some(50), Some(500));

    assert_eq!(hud.update_count(), 2); // One from update_from_session, one from update_performance
    let metrics = hud.performance_metrics();
    assert_eq!(metrics.quick_index_duration_ms, Some(50));
    assert_eq!(metrics.full_index_duration_ms, Some(500));
}

#[test]
fn test_format_text() {
    let mut hud = IndexDevHUD::new("test_session".to_string());

    let session = IndexSession::new();
    let extractor = Av1IndexExtractor::new();
    let data = create_minimal_av1_stream();
    let mut cursor = Cursor::new(data);
    let _ = session.execute_quick_index(&extractor, &mut cursor, None::<fn(_)>);

    hud.update_from_session(&session);

    let text = hud.format_text();

    assert!(text.contains("test_session"));
    assert!(text.contains("QuickComplete"));
}

#[test]
fn test_multiple_updates() {
    let mut hud = IndexDevHUD::new("test_session".to_string());

    let session = IndexSession::new();
    hud.update_from_session(&session);
    assert_eq!(hud.update_count(), 1);

    hud.update_performance(Some(100), None);
    assert_eq!(hud.update_count(), 2);

    hud.update_from_session(&session);
    assert_eq!(hud.update_count(), 3);
}

#[test]
fn test_snapshots_independent() {
    let mut hud = IndexDevHUD::new("test_session".to_string());

    // Initial state
    assert_eq!(hud.session_snapshot().state, IndexingState::Idle);
    assert!(hud.window_snapshot().is_none());

    // Update session (use actual session that went through quick index)
    let session = IndexSession::new();
    let extractor = Av1IndexExtractor::new();
    let data = create_minimal_av1_stream();
    let mut cursor = Cursor::new(data);
    let _ = session.execute_quick_index(&extractor, &mut cursor, None::<fn(_)>);

    hud.update_from_session(&session);

    // Session snapshot updated, window still None
    assert_eq!(hud.session_snapshot().state, IndexingState::QuickComplete);
    assert!(hud.window_snapshot().is_none());
}
