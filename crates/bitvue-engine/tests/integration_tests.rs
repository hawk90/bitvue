//! Minimal integration tests for bitvue-engine
//!
//! The actual tests are in src/tests/ (compiled as part of lib.rs)
//! to avoid linker OOM from compiling 4500+ separate test binaries.

use bitvue_engine::*;

#[test]
fn lib_can_be_imported() {
    // Basic smoke test that the library compiles and imports work
    let _selection = SelectionState::default();
}

#[test]
fn basic_types_work() {
    let frame = FrameType::SI;
    assert!(matches!(frame, FrameType::SI));
}
