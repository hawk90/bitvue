//! Shared test fixtures used across multiple command modules' `#[cfg(test)] mod tests` blocks
//! (`compare.rs`, and every `commands/*.rs` module split out of `main.rs`'s former 4200-line
//! test module, 2026-08-19). Declared `#[cfg(test)] mod test_support;` in `main.rs`, so nothing
//! here is compiled into a real build.

use bitvue_engine::Core;
use bitvue_protocol::{FrameKind, Request};
use std::io::Write;

/// Encode a list of (kind, payload) frames the same way the real writer does, for tests
/// that assert on the exact byte stream `compute_frames`'s output would produce.
pub(crate) fn encode_frames(correlation_id: u32, frames: &[(FrameKind, Vec<u8>)]) -> Vec<u8> {
    let mut out = Vec::new();
    for (kind, payload) in frames {
        crate::write_frame(&mut out, *kind, correlation_id, payload).unwrap();
    }
    out
}

pub(crate) const AV1_IVF_FIXTURE: &[u8] = include_bytes!("../../../test_data/av1_test.ivf");

/// Real fixture, not a fake file -- these commands need actual IVF/AV1 bytes to produce
/// anything, unlike a bare "does a file exist" open test.
pub(crate) fn open_real_fixture(core: &Core, stream: &str) {
    open_fixture_bytes(core, stream, AV1_IVF_FIXTURE);
}

pub(crate) fn open_fixture_bytes(core: &Core, stream: &str, bytes: &[u8]) {
    let mut file = tempfile::NamedTempFile::new().unwrap();
    file.write_all(bytes).unwrap();
    let request = Request {
        id: 100,
        method: "open_stream".to_string(),
        params: serde_json::json!({"stream": stream, "path": file.path().to_str().unwrap()}),
    };
    let response = crate::dispatch(core, &request);
    assert!(response.ok, "expected real fixture to open: {response:?}");
    std::mem::forget(file); // keep the path alive for ByteCache, matches bitvue-indexer's tests
}

pub(crate) fn fresh_debug_yuv_state() -> crate::DebugYuvSlot {
    std::sync::Arc::new(std::sync::Mutex::new(None))
}

pub(crate) fn fresh_compare_state() -> crate::compare::CompareSlot {
    std::sync::Arc::new(std::sync::Mutex::new(None))
}

/// Writes `frames.len()` consecutive I420 frames at `width`x`height`, each filled with the
/// corresponding byte in `frames` -- a cheap way to build a multi-frame debug-YUV reference file
/// without caring about real pixel content, just distinguishable per-frame fill values.
pub(crate) fn write_i420_fixture(
    width: u32,
    height: u32,
    frames: &[u8],
) -> tempfile::NamedTempFile {
    let mut file = tempfile::NamedTempFile::new().unwrap();
    let frame_size =
        crate::debug_yuv::frame_byte_size(width, height, crate::debug_yuv::YuvFormat::I420, 8);
    for &fill in frames {
        file.write_all(&vec![fill; frame_size]).unwrap();
    }
    file
}

/// Writes a single I420 frame from explicit Y/U/V plane bytes -- for debug-YUV tests that need
/// real, distinguishable per-plane content rather than a uniform fill.
pub(crate) fn write_i420_frame(
    width: u32,
    height: u32,
    y: &[u8],
    u: &[u8],
    v: &[u8],
) -> tempfile::NamedTempFile {
    let mut file = tempfile::NamedTempFile::new().unwrap();
    assert_eq!(y.len(), (width * height) as usize);
    file.write_all(y).unwrap();
    file.write_all(u).unwrap();
    file.write_all(v).unwrap();
    file
}

/// Opens the same real fixture as BOTH stream A and stream B, and indexes both -- gives a
/// deterministic "identical streams" baseline (frame N of A aligns exactly to frame N of B,
/// same PTS, same decoded content) that's genuinely useful for alignment/diff tests, not just a
/// smoke test: an all-zero diff or a non-1:1 alignment on identical content would be a real bug,
/// not a fixture artifact.
pub(crate) fn open_and_index_real_fixture_as_both_streams(core: &Core) {
    open_real_fixture(core, "A");
    open_real_fixture(core, "B");
    for stream in ["A", "B"] {
        let response = crate::dispatch(
            core,
            &Request {
                id: 500,
                method: "index_stream".to_string(),
                params: serde_json::json!({"stream": stream}),
            },
        );
        assert!(
            response.ok,
            "expected index_stream({stream}) to succeed: {response:?}"
        );
    }
}
