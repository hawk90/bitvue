//! `IndexSession::execute_full_index` with a scripted extractor: states, progress, cancellation,
//! the stored/returned index and the evidence it leaves behind.

use crate::indexing::{FrameMetadata, QuickIndex, SeekPoint};
use crate::{
    BitvueError, IndexExtractor, IndexSession, IndexingPhase, IndexingProgress, IndexingState,
    ReadSeek,
};
use std::cell::{Cell, RefCell};
use std::io::Cursor;

type Extract = dyn Fn(&IndexSession) -> Result<Vec<FrameMetadata>, BitvueError>;

/// Returns a scripted quick index and runs `full` for the full phase.
struct Scripted<'a> {
    session: &'a IndexSession,
    quick: Result<QuickIndex, &'static str>,
    full: Box<Extract>,
    cancel_seen_by_extractor: Cell<Option<bool>>,
    /// Progress reports the extractor makes before it returns its frames
    reports: Vec<(f64, &'static str)>,
}

impl IndexExtractor for Scripted<'_> {
    fn codec_name(&self) -> &'static str {
        "scripted"
    }

    fn extract_quick_index(&self, _: &mut dyn ReadSeek) -> Result<QuickIndex, BitvueError> {
        self.quick
            .clone()
            .map_err(|m| BitvueError::InvalidData(m.to_string()))
    }

    fn extract_full_index(
        &self,
        _: &mut dyn ReadSeek,
        progress: crate::ProgressCallback<'_>,
        should_cancel: crate::CancelCallback<'_>,
    ) -> Result<Vec<FrameMetadata>, BitvueError> {
        let progress = progress.expect("the session passes a progress callback");
        for &(fraction, message) in &self.reports {
            progress(fraction, message);
        }
        self.cancel_seen_by_extractor
            .set(should_cancel.map(|f| f()));
        (self.full)(self.session)
    }
}

fn frame(idx: usize, offset: u64, size: u64) -> FrameMetadata {
    FrameMetadata {
        display_idx: idx,
        decode_idx: idx,
        byte_offset: offset,
        size,
        is_keyframe: idx == 0,
        pts: Some(idx as u64),
        dts: Some(idx as u64),
        frame_type: Some(if idx == 0 { "KEY" } else { "INTER" }.to_string()),
    }
}

fn seek(idx: usize, offset: u64) -> SeekPoint {
    SeekPoint {
        display_idx: idx,
        byte_offset: offset,
        is_keyframe: true,
        pts: Some(idx as u64),
    }
}

fn quick(file_size: u64) -> QuickIndex {
    QuickIndex::new(vec![seek(0, 100), seek(2, 400)], file_size)
}

fn three_frames(_: &IndexSession) -> Result<Vec<FrameMetadata>, BitvueError> {
    Ok(vec![
        frame(0, 100, 300),
        frame(1, 400, 50),
        frame(2, 450, 70),
    ])
}

fn scripted<'a>(
    session: &'a IndexSession,
    full: impl Fn(&IndexSession) -> Result<Vec<FrameMetadata>, BitvueError> + 'static,
) -> Scripted<'a> {
    Scripted {
        session,
        quick: Ok(quick(9999)),
        full: Box::new(full),
        cancel_seen_by_extractor: Cell::new(None),
        reports: Vec::new(),
    }
}

fn reader() -> Cursor<Vec<u8>> {
    Cursor::new(vec![0u8; 8])
}

fn after_quick<'a>(ex: &Scripted<'a>) -> IndexSession {
    let session = IndexSession::new();
    session
        .execute_quick_index(ex, &mut reader(), None::<fn(IndexingProgress)>)
        .unwrap();
    assert_eq!(session.state(), IndexingState::QuickComplete);
    session
}

#[test]
fn full_index_stores_returns_and_completes() {
    let session = IndexSession::new();
    let ex = scripted(&session, three_frames);
    session
        .execute_quick_index(&ex, &mut reader(), None::<fn(IndexingProgress)>)
        .unwrap();
    let full = session
        .execute_full_index(&ex, &mut reader(), None::<fn(IndexingProgress)>)
        .unwrap();

    assert_eq!(session.state(), IndexingState::FullComplete);
    assert!(session.is_full_complete());
    assert_eq!(full.frames.len(), 3);
    assert!(full.is_complete);
    // The file size comes from the quick index, not from the frames.
    assert_eq!(full.file_size, 9999);
    let stored = session.full_index().unwrap();
    assert_eq!(format!("{:?}", stored.frames), format!("{:?}", full.frames));
    assert_eq!((stored.file_size, stored.is_complete), (9999, true));
    assert_eq!(ex.cancel_seen_by_extractor.get(), Some(false));
}

#[test]
fn full_index_reports_start_and_completion_only() {
    let session = IndexSession::new();
    let ex = scripted(&session, three_frames);
    session
        .execute_quick_index(&ex, &mut reader(), None::<fn(IndexingProgress)>)
        .unwrap();
    let seen = RefCell::new(Vec::new());
    session
        .execute_full_index(
            &ex,
            &mut reader(),
            Some(|p: IndexingProgress| seen.borrow_mut().push(p)),
        )
        .unwrap();
    let seen = seen.into_inner();
    assert_eq!(seen.len(), 2);
    assert_eq!(seen[0].phase, IndexingPhase::Full);
    assert_eq!((seen[0].progress, seen[0].frames_indexed), (0.0, 0));
    assert_eq!(seen[0].message, "Starting full index...");
    assert_eq!(seen[1].phase, IndexingPhase::Full);
    assert_eq!((seen[1].progress, seen[1].frames_indexed), (1.0, 3));
    assert_eq!(seen[1].message, "Full index complete: 3 frames");
}

#[test]
fn full_index_refused_unless_quick_is_complete() {
    for state_setup in ["idle", "full_complete", "error", "cancelled"] {
        let session = IndexSession::new();
        let ex = scripted(&session, three_frames);
        match state_setup {
            "idle" => {}
            "full_complete" => {
                session
                    .execute_full_workflow(&ex, &mut reader(), None::<fn(IndexingProgress)>)
                    .unwrap();
            }
            "error" => {
                let bad = Scripted {
                    quick: Err("no keyframes"),
                    ..scripted(&session, three_frames)
                };
                assert!(session
                    .execute_quick_index(&bad, &mut reader(), None::<fn(IndexingProgress)>)
                    .is_err());
            }
            _ => {
                let ex2 = scripted(&session, |s| {
                    s.cancel();
                    three_frames(s)
                });
                session
                    .execute_quick_index(&ex2, &mut reader(), None::<fn(IndexingProgress)>)
                    .unwrap();
                assert!(session
                    .execute_full_index(&ex2, &mut reader(), None::<fn(IndexingProgress)>)
                    .is_err());
            }
        }
        let before = session.state();
        let err = session
            .execute_full_index(&ex, &mut reader(), None::<fn(IndexingProgress)>)
            .unwrap_err();
        let text = err.to_string();
        assert!(
            text.contains("Cannot start full index from state")
                && text.contains("Run quick index first."),
            "{state_setup}: {text}"
        );
        assert!(text.contains(&format!("{before:?}")), "{text}");
        assert_eq!(
            session.state(),
            before,
            "{state_setup}: a refused call leaves the state"
        );
    }
}

#[test]
fn refused_call_runs_no_callback_and_no_extractor() {
    let session = IndexSession::new();
    let ex = scripted(&session, three_frames);
    let calls = Cell::new(0);
    let _ = session.execute_full_index(
        &ex,
        &mut reader(),
        Some(|_: IndexingProgress| calls.set(calls.get() + 1)),
    );
    assert_eq!(calls.get(), 0);
    assert_eq!(ex.cancel_seen_by_extractor.get(), None);
}

#[test]
fn extractor_error_without_cancel_sets_error_state_and_passes_the_error_through() {
    let session = IndexSession::new();
    let ex = scripted(&session, |_| Err(BitvueError::InvalidData("boom".into())));
    session
        .execute_quick_index(&ex, &mut reader(), None::<fn(IndexingProgress)>)
        .unwrap();
    let seen = RefCell::new(Vec::new());
    let err = session
        .execute_full_index(
            &ex,
            &mut reader(),
            Some(|p: IndexingProgress| seen.borrow_mut().push(p)),
        )
        .unwrap_err();
    assert_eq!(err.to_string(), "Invalid data: boom");
    assert_eq!(session.state(), IndexingState::Error);
    assert!(session.full_index().is_none());
    // Only the start report; no completion report on failure.
    assert_eq!(seen.borrow().len(), 1);
}

#[test]
fn extractor_error_after_cancel_sets_cancelled_state() {
    let session = IndexSession::new();
    let ex = scripted(&session, |s| {
        s.cancel();
        Err(BitvueError::InvalidData("stopped".into()))
    });
    session
        .execute_quick_index(&ex, &mut reader(), None::<fn(IndexingProgress)>)
        .unwrap();
    let err = session
        .execute_full_index(&ex, &mut reader(), None::<fn(IndexingProgress)>)
        .unwrap_err();
    assert_eq!(err.to_string(), "Invalid data: stopped");
    assert_eq!(session.state(), IndexingState::Cancelled);
}

#[test]
fn cancel_during_a_successful_extraction_discards_the_result() {
    let session = IndexSession::new();
    let ex = scripted(&session, |s| {
        s.cancel();
        three_frames(s)
    });
    session
        .execute_quick_index(&ex, &mut reader(), None::<fn(IndexingProgress)>)
        .unwrap();
    let seen = RefCell::new(Vec::new());
    let err = session
        .execute_full_index(
            &ex,
            &mut reader(),
            Some(|p: IndexingProgress| seen.borrow_mut().push(p)),
        )
        .unwrap_err();
    assert_eq!(
        err.to_string(),
        "Invalid data: Full index cancelled by user"
    );
    assert_eq!(session.state(), IndexingState::Cancelled);
    assert!(session.full_index().is_none(), "nothing stored");
    assert_eq!(seen.borrow().len(), 1, "no completion report");
}

#[test]
fn a_cancel_requested_before_the_call_is_cleared_when_it_starts() {
    let session = IndexSession::new();
    let ex = scripted(&session, three_frames);
    session
        .execute_quick_index(&ex, &mut reader(), None::<fn(IndexingProgress)>)
        .unwrap();
    session.cancel();
    let full = session
        .execute_full_index(&ex, &mut reader(), None::<fn(IndexingProgress)>)
        .unwrap();
    assert_eq!(full.frames.len(), 3);
    assert_eq!(ex.cancel_seen_by_extractor.get(), Some(false));
}

#[test]
fn an_empty_frame_list_is_a_complete_empty_index() {
    let session = IndexSession::new();
    let ex = scripted(&session, |_| Ok(vec![]));
    session
        .execute_quick_index(&ex, &mut reader(), None::<fn(IndexingProgress)>)
        .unwrap();
    let seen = RefCell::new(Vec::new());
    let full = session
        .execute_full_index(
            &ex,
            &mut reader(),
            Some(|p: IndexingProgress| seen.borrow_mut().push(p)),
        )
        .unwrap();
    assert!(full.frames.is_empty() && full.is_complete);
    assert_eq!(
        seen.borrow().last().unwrap().message,
        "Full index complete: 0 frames"
    );
}

#[test]
fn every_frame_gets_evidence_and_seek_points_get_their_real_size() {
    let session = IndexSession::new();
    let ex = scripted(&session, three_frames);
    session
        .execute_quick_index(&ex, &mut reader(), None::<fn(IndexingProgress)>)
        .unwrap();
    // After the quick phase a seek point carries the 1024-byte placeholder size.
    {
        let mgr = session.evidence_manager();
        let mgr = mgr.lock().unwrap();
        let r = mgr.trace_to_bit_offset(0).unwrap();
        assert_eq!((r.start_bit, r.end_bit), (800, 800 + 1024 * 8));
    }
    session
        .execute_full_index(&ex, &mut reader(), None::<fn(IndexingProgress)>)
        .unwrap();
    let mgr = session.evidence_manager();
    let mgr = mgr.lock().unwrap();
    for (idx, (offset, size)) in [(0usize, (100u64, 300u64)), (1, (400, 50)), (2, (450, 70))] {
        let r = mgr
            .trace_to_bit_offset(idx)
            .unwrap_or_else(|| panic!("frame {idx}"));
        assert_eq!(
            (r.start_bit, r.end_bit),
            (offset * 8, (offset + size) * 8),
            "frame {idx}"
        );
    }
}

#[test]
fn seek_points_without_a_matching_frame_keep_their_placeholder() {
    let session = IndexSession::new();
    // Quick index knows frames 0 and 2; the full scan only found frame 0 and 1.
    let ex = scripted(&session, |_| {
        Ok(vec![frame(0, 100, 300), frame(1, 400, 50)])
    });
    session
        .execute_quick_index(&ex, &mut reader(), None::<fn(IndexingProgress)>)
        .unwrap();
    session
        .execute_full_index(&ex, &mut reader(), None::<fn(IndexingProgress)>)
        .unwrap();
    let mgr = session.evidence_manager();
    let mgr = mgr.lock().unwrap();
    let r0 = mgr.trace_to_bit_offset(0).unwrap();
    assert_eq!(r0.end_bit - r0.start_bit, 300 * 8);
    let r2 = mgr.trace_to_bit_offset(2).unwrap();
    assert_eq!((r2.start_bit, r2.end_bit), (400 * 8, (400 + 1024) * 8));
}

#[test]
fn reset_after_full_index_clears_everything() {
    let session = IndexSession::new();
    let ex = scripted(&session, three_frames);
    session
        .execute_full_workflow(&ex, &mut reader(), None::<fn(IndexingProgress)>)
        .unwrap();
    session.reset();
    assert_eq!(session.state(), IndexingState::Idle);
    assert!(session.full_index().is_none() && session.quick_index().is_none());
    let _ = after_quick(&ex);
}

#[test]
fn estimated_progress_follows_the_state_through_a_full_workflow() {
    let session = IndexSession::new();
    assert_eq!(session.estimated_progress(), 0.0);
    let ex = scripted(&session, three_frames);
    session
        .execute_quick_index(&ex, &mut reader(), None::<fn(IndexingProgress)>)
        .unwrap();
    assert_eq!(session.estimated_progress(), 0.0, "QuickComplete");

    // While the full index runs, seen from the start report, an extractor that has not reported
    // yet gives 0.0.
    let during = Cell::new(f64::NAN);
    session
        .execute_full_index(
            &ex,
            &mut reader(),
            Some(|_: IndexingProgress| {
                if during.get().is_nan() {
                    assert_eq!(session.state(), IndexingState::FullIndexing);
                    during.set(session.estimated_progress());
                }
            }),
        )
        .unwrap();
    assert_eq!(during.get(), 0.0);
    assert_eq!(session.estimated_progress(), 1.0, "FullComplete");

    let session = IndexSession::new();
    let mut q = quick(10);
    q.estimated_frame_count = Some(100);
    let ex = Scripted {
        quick: Ok(q),
        ..scripted(&session, three_frames)
    };
    session
        .execute_quick_index(&ex, &mut reader(), None::<fn(IndexingProgress)>)
        .unwrap();
    session
        .execute_full_index(
            &ex,
            &mut reader(),
            Some(|_: IndexingProgress| {
                if during.get() != 0.0 {
                    during.set(session.estimated_progress());
                }
            }),
        )
        .unwrap();
    assert_eq!(during.get(), 0.0);
}

#[test]
fn full_index_progress_is_the_extractors_fraction_and_reaches_the_callback() {
    // #280: the session used to hand the extractor no progress callback, so the estimate sat at a
    // fixed 0.5 (or 0.0) for the whole full index.
    let session = IndexSession::new();
    let ex = Scripted {
        reports: vec![
            (0.25, "a"),
            (0.5, "b"),
            (2.0, "past the end"),
            (f64::NAN, "not a number"),
            (-1.0, "negative"),
        ],
        ..scripted(&session, three_frames)
    };
    session
        .execute_quick_index(&ex, &mut reader(), None::<fn(IndexingProgress)>)
        .unwrap();

    // (reported progress, estimate at that moment, frames indexed, message) per callback call
    let seen = std::cell::RefCell::new(Vec::new());
    session
        .execute_full_index(
            &ex,
            &mut reader(),
            Some(|p: IndexingProgress| {
                seen.borrow_mut().push((
                    p.progress,
                    session.estimated_progress(),
                    p.frames_indexed,
                    p.message,
                ));
            }),
        )
        .unwrap();
    let seen = seen.into_inner();
    let summary: Vec<(f64, f64, usize)> = seen.iter().map(|s| (s.0, s.1, s.2)).collect();
    assert_eq!(
        summary,
        vec![
            (0.0, 0.0, 0),   // the session's own start report
            (0.25, 0.25, 1), // the extractor's reports, in order
            (0.5, 0.5, 2),
            (1.0, 0.99, 3), // clamped to 1.0, but the estimate stays below done
            (0.0, 0.0, 4),  // NaN is treated as no progress
            (0.0, 0.0, 5),  // so is a negative fraction
            (1.0, 1.0, 3),  // completion: 3 frames stored, state FullComplete
        ]
    );
    assert_eq!(seen[1].3, "a");
    assert_eq!(session.estimated_progress(), 1.0);

    // reset puts the estimate back to zero
    session.reset();
    assert_eq!(session.estimated_progress(), 0.0);
}
