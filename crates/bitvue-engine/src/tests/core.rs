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
//! Tests for core module

use crate::{Command, Core, Event, FrameKey, StreamId};
use std::io::Write;
use tempfile::NamedTempFile;

#[test]
fn test_core_open_file() {
    let core = Core::new();

    // Create temp file
    let mut file = NamedTempFile::new().unwrap();
    file.write_all(b"DKIF").unwrap(); // IVF signature
    file.flush().unwrap();

    // Open file
    let events = core.handle_command(Command::OpenFile {
        stream: StreamId::A,
        path: file.path().to_path_buf(),
    });

    // Should emit at least one event
    assert!(!events.is_empty());

    // Stream should have byte cache
    let state = core.get_stream(StreamId::A);
    let state = state.read();
    assert!(state.byte_cache.is_some());
    assert_eq!(state.stream_id, StreamId::A);
}

#[test]
fn test_core_close_file() {
    let core = Core::new();

    // Create temp file and open it
    let mut file = NamedTempFile::new().unwrap();
    file.write_all(b"test").unwrap();
    file.flush().unwrap();

    core.handle_command(Command::OpenFile {
        stream: StreamId::A,
        path: file.path().to_path_buf(),
    });

    // Close file
    let events = core.handle_command(Command::CloseFile {
        stream: StreamId::A,
    });

    // Should emit event
    assert!(!events.is_empty());

    // Stream should be cleared
    let state = core.get_stream(StreamId::A);
    let state = state.read();
    assert!(state.byte_cache.is_none());
}

#[test]
fn test_core_selection() {
    let core = Core::new();

    let frame_key = FrameKey {
        stream: StreamId::A,
        frame_index: 0,
        pts: None,
    };

    let events = core.handle_command(Command::SelectFrame {
        stream: StreamId::A,
        frame_key: frame_key.clone(),
    });

    // Should emit selection updated event
    assert_eq!(events.len(), 1);
    matches!(events[0], Event::SelectionUpdated { .. });

    // Selection should be updated - check temporal selection
    let selection = core.get_selection();
    let selection = selection.read();
    assert_eq!(selection.stream_id, StreamId::A);
    // The temporal selection should contain the frame index
    assert!(selection.temporal.is_some());
    if let Some(ref temporal) = selection.temporal {
        assert_eq!(temporal.frame_index(), 0);
    }
}
