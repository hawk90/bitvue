#![allow(dead_code)]
//! Foundations AVS3 Compare Cache Provenance #004: Frame Identity Contract
//! Subtask: S.T0-2.AVS3.Foundations.Compare.impl.cache_provenance.004

use bitvue_core::frame_identity::{FrameIndexMap, FrameMetadata, PtsQuality};
use bitvue_core::selection::{SelectionState, StreamId, TemporalSelection};

#[derive(Debug, Clone)]
struct CompareFrame {
    display_idx: u64,
    decode_idx: u64,
}
struct CompareState {
    frames: Vec<CompareFrame>,
}
impl CompareState {
    fn new() -> Self {
        Self { frames: Vec::new() }
    }
    fn add_frame(&mut self, f: CompareFrame) {
        self.frames.push(f);
    }
    fn get_frame(&self, display_idx: u64) -> Option<&CompareFrame> {
        self.frames.iter().find(|f| f.display_idx == display_idx)
    }
}

#[test]
fn test_display_idx_primary() {
    let frames = vec![
        FrameMetadata {
            pts: Some(0),
            dts: Some(0),
        },
        FrameMetadata {
            pts: Some(2000),
            dts: Some(1000),
        },
        FrameMetadata {
            pts: Some(1000),
            dts: Some(2000),
        },
    ];
    assert_eq!(FrameIndexMap::new(&frames).pts_quality(), PtsQuality::Ok);
}
#[test]
fn test_frame_by_display_idx() {
    let mut state = CompareState::new();
    state.add_frame(CompareFrame {
        display_idx: 0,
        decode_idx: 0,
    });
    state.add_frame(CompareFrame {
        display_idx: 1,
        decode_idx: 2,
    });
    state.add_frame(CompareFrame {
        display_idx: 2,
        decode_idx: 1,
    });
    assert_eq!(state.get_frame(1).unwrap().decode_idx, 2);
}
#[test]
fn test_selection_uses_display_idx() {
    let mut sel = SelectionState::new(StreamId::A);
    sel.select_point(5);
    match &sel.temporal {
        Some(TemporalSelection::Point { frame_index }) => assert_eq!(*frame_index, 5),
        _ => panic!("Expected Point selection"),
    }
}
#[test]
fn test_empty_state() {
    assert!(CompareState::new().frames.is_empty());
}
#[test]
fn test_display_decode_mismatch() {
    let mut state = CompareState::new();
    state.add_frame(CompareFrame {
        display_idx: 0,
        decode_idx: 0,
    });
    state.add_frame(CompareFrame {
        display_idx: 2,
        decode_idx: 1,
    });
    state.add_frame(CompareFrame {
        display_idx: 1,
        decode_idx: 2,
    });
    let mut order: Vec<_> = state.frames.iter().map(|f| f.display_idx).collect();
    order.sort();
    assert_eq!(order, vec![0, 1, 2]);
}
