//! Dual-stream compare workspace (`docs/DEVELOPMENT_PHASES.md` Phase 7.5 MVP) -- sidecar IPC
//! wiring for `bitvue_engine::compare::CompareWorkspace` (PTS-based frame alignment via
//! `AlignmentEngine`) and `bitvue_engine::diff_heatmap` (real A/B luma diff heatmap), neither of
//! which had any sidecar exposure before this pass: `create_compare_workspace` previously only
//! existed inside `main.rs`'s `unknown_method_returns_internal_error` test, asserting it does
//! NOT exist. Both engine modules are fully implemented, well-tested, stream-agnostic logic --
//! this module is pure orchestration glue (build a `FrameIndexMap` per stream, construct the
//! workspace, fetch decoded luma planes, call into the engine), not new algorithm work.
//!
//! Session state (the constructed `CompareWorkspace`) lives in a `Mutex<Option<...>>` held by
//! `main.rs` (`CompareSlot`), same pattern as `DebugYuvSlot` -- it's cross-stream (A+B) derived
//! state, not per-`StreamId` state `Core` itself owns.
//!
//! **AV1-only for now**: `FrameIndexMap` construction goes through
//! `bitvue_indexer::build_frame_index_map`, which inherits `get_timeline`'s existing AV1-only
//! guard (`bitvue-indexer/src/lib.rs`'s module doc). Matches this crate's whole product path.

use bitvue_engine::{CompareWorkspace, Core, DiffHeatmapData, DiffMode, StreamId, SyncMode};
use bitvue_protocol::{Request, Response, WireError, WireErrorCode};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use crate::decode_bridge;

pub type CompareSlot = Arc<Mutex<Option<CompareWorkspace>>>;

fn failure(id: u32, code: WireErrorCode, message: impl Into<String>) -> Response {
    Response::failure(
        id,
        WireError {
            code,
            message: message.into(),
            offset: None,
        },
    )
}

/// Reads `(width, height)` from a stream's already-indexed `ContainerModel` -- same field
/// `get_stream_info`'s `container_model_to_json` reads (`main.rs`).
fn stream_dimensions(core: &Core, stream: StreamId) -> Result<(u32, u32), String> {
    let stream_state = core.get_stream(stream);
    let state = stream_state.read();
    let container = state
        .container
        .as_ref()
        .ok_or_else(|| "stream not indexed -- has index_stream run?".to_string())?;
    let width = container
        .width
        .ok_or_else(|| "stream has no known width".to_string())?;
    let height = container
        .height
        .ok_or_else(|| "stream has no known height".to_string())?;
    Ok((width, height))
}

/// Strips row-stride padding down to a tight `width * height` buffer -- `DiffHeatmapData::
/// from_luma_planes` asserts an exact `(width * height)` length, but `decode_bridge`'s decoded
/// luma plane is `y_stride * height` bytes wide with `y_stride` possibly `> width` (decoder
/// padding). Getting this wrong (e.g. using `width` as the row pitch when reading the source
/// buffer) reproduces the exact "stride metadata pollution" bug class already found and fixed
/// once in this codebase for `YuvViewerPanel` -- see this project's history for why this can't be
/// skipped even when it looks like `y_stride == width` in today's fixtures.
fn destride_luma(bytes: &[u8], y_stride: usize, width: u32, height: u32) -> Vec<u8> {
    let width = width as usize;
    let height = height as usize;
    let mut out = Vec::with_capacity(width * height);
    for row in 0..height {
        let start = row * y_stride;
        out.extend_from_slice(&bytes[start..start + width]);
    }
    out
}

/// Fetches one stream's decoded luma plane for one frame, de-strided to `width * height` bytes.
fn decoded_luma(core: &Core, stream: StreamId, frame_index: usize) -> Result<Vec<u8>, String> {
    let stream_state = core.get_stream(stream);
    let state = stream_state.read();
    let byte_cache = state
        .byte_cache
        .as_ref()
        .ok_or_else(|| "stream not open".to_string())?;
    let byte_cache = Arc::clone(byte_cache);
    drop(state);

    let full_len = byte_cache.len() as usize;
    let data = byte_cache
        .read_range(0, full_len)
        .map_err(|e| e.to_string())?;

    let frame = decode_bridge::get_decoded_frame_yuv(data, frame_index)?;
    let y_bytes = &frame.bytes[..frame.y_len];
    Ok(destride_luma(
        y_bytes,
        frame.y_stride,
        frame.width,
        frame.height,
    ))
}

/// No params -- always operates on whichever streams are currently open as A/B. Requires both to
/// be open AND indexed (`index_stream` already run for each) so `build_frame_index_map` can build
/// a real PTS-based `FrameIndexMap` for both.
pub fn create_compare_workspace(core: &Core, slot: &CompareSlot, request: &Request) -> Response {
    let index_a = match bitvue_indexer::build_frame_index_map(core, StreamId::A) {
        Ok(m) => m,
        Err(message) => return failure(request.id, WireErrorCode::NotFound, message),
    };
    let index_b = match bitvue_indexer::build_frame_index_map(core, StreamId::B) {
        Ok(m) => m,
        Err(message) => return failure(request.id, WireErrorCode::NotFound, message),
    };
    let resolution_a = match stream_dimensions(core, StreamId::A) {
        Ok(dims) => dims,
        Err(message) => return failure(request.id, WireErrorCode::NotFound, message),
    };
    let resolution_b = match stream_dimensions(core, StreamId::B) {
        Ok(dims) => dims,
        Err(message) => return failure(request.id, WireErrorCode::NotFound, message),
    };

    let workspace = CompareWorkspace::new(index_a, index_b, resolution_a, resolution_b);
    let res_info = workspace.resolution_info();
    let alignment = &workspace.alignment;
    // A purpose-built summary via the getters, not a raw `serde_json::to_value(&workspace)` --
    // `CompareWorkspace` derives `Serialize` for its own internal-state reasons, but a raw dump
    // would (a) omit `total_frames`/`is_diff_enabled()`/`ResolutionInfo`'s and `AlignmentEngine`'s
    // own derived getters (methods, not struct fields) and (b) leak the full per-frame
    // `stream_a`/`stream_b`/`alignment.frame_pairs` data into every response -- real per-frame
    // alignment goes through `get_aligned_frame` (one request per query), not a client-side scan
    // of a bulk-dumped pair list.
    let response = Response::success(
        request.id,
        serde_json::json!({
            "total_frames": workspace.total_frames(),
            "diff_enabled": workspace.is_diff_enabled(),
            "disable_reason": workspace.disable_reason(),
            "sync_mode": workspace.sync_mode(),
            "manual_offset": workspace.manual_offset(),
            "resolution_info": {
                "stream_a": res_info.stream_a,
                "stream_b": res_info.stream_b,
                "tolerance": res_info.tolerance,
                "is_compatible": res_info.is_compatible(),
                "mismatch_percentage": res_info.mismatch_percentage(),
                "is_exact_match": res_info.is_exact_match(),
                "scale_indicator": res_info.scale_indicator(),
            },
            "alignment": {
                // Raw enum variants (`PtsExact`/`PtsNearest`/`DisplayIdx`, `High`/`Medium`/`Low`),
                // not `display_text()`'s human strings ("PTS Exact" etc.) -- matches
                // `frontend/types/video.ts`'s `AlignmentMethod`/`AlignmentConfidence` enums, whose
                // frontend-side `display_text()`-equivalent formatting (if ever needed) belongs on
                // the frontend, not baked into the wire value.
                "method": alignment.method,
                "confidence": alignment.confidence,
                "gap_count": alignment.gap_count,
                "gap_percentage": alignment.gap_percentage(),
                "total_pairs": alignment.frame_pairs.len(),
            },
        }),
    );
    *slot.lock().unwrap() = Some(workspace);
    response
}

fn with_workspace<T>(
    slot: &CompareSlot,
    request_id: u32,
    f: impl FnOnce(&CompareWorkspace) -> T,
) -> Result<T, Response> {
    let guard = slot.lock().unwrap();
    match guard.as_ref() {
        Some(workspace) => Ok(f(workspace)),
        None => Err(failure(
            request_id,
            WireErrorCode::NotFound,
            "no compare workspace -- call create_compare_workspace first",
        )),
    }
}

fn with_workspace_mut(
    slot: &CompareSlot,
    request_id: u32,
    f: impl FnOnce(&mut CompareWorkspace),
) -> Result<(), Response> {
    let mut guard = slot.lock().unwrap();
    match guard.as_mut() {
        Some(workspace) => {
            f(workspace);
            Ok(())
        }
        None => Err(failure(
            request_id,
            WireErrorCode::NotFound,
            "no compare workspace -- call create_compare_workspace first",
        )),
    }
}

#[derive(serde::Deserialize)]
pub struct GetAlignedFrameParams {
    stream_a_frame_idx: usize,
}

pub fn get_aligned_frame(slot: &CompareSlot, request: &Request) -> Response {
    let params: GetAlignedFrameParams = match serde_json::from_value(request.params.clone()) {
        Ok(p) => p,
        Err(err) => return failure(request.id, WireErrorCode::InvalidData, err.to_string()),
    };

    match with_workspace(slot, request.id, |workspace| {
        workspace.get_aligned_frame(params.stream_a_frame_idx)
    }) {
        Ok(Some((stream_b_frame_idx, quality))) => Response::success(
            request.id,
            serde_json::json!({
                "stream_b_frame_idx": stream_b_frame_idx,
                "quality": quality,
            }),
        ),
        Ok(None) => Response::success(
            request.id,
            serde_json::json!({ "stream_b_frame_idx": null, "quality": null }),
        ),
        Err(response) => response,
    }
}

#[derive(serde::Deserialize)]
pub struct SetSyncModeParams {
    mode: SyncMode,
}

pub fn set_sync_mode(slot: &CompareSlot, request: &Request) -> Response {
    let params: SetSyncModeParams = match serde_json::from_value(request.params.clone()) {
        Ok(p) => p,
        Err(err) => return failure(request.id, WireErrorCode::InvalidData, err.to_string()),
    };
    match with_workspace_mut(slot, request.id, |workspace| {
        workspace.set_sync_mode(params.mode)
    }) {
        Ok(()) => Response::success(request.id, serde_json::json!({ "sync_mode": params.mode })),
        Err(response) => response,
    }
}

#[derive(serde::Deserialize)]
pub struct SetManualOffsetParams {
    offset: i32,
}

pub fn set_manual_offset(slot: &CompareSlot, request: &Request) -> Response {
    let params: SetManualOffsetParams = match serde_json::from_value(request.params.clone()) {
        Ok(p) => p,
        Err(err) => return failure(request.id, WireErrorCode::InvalidData, err.to_string()),
    };
    match with_workspace_mut(slot, request.id, |workspace| {
        workspace.set_manual_offset(params.offset)
    }) {
        Ok(()) => Response::success(
            request.id,
            serde_json::json!({ "manual_offset": params.offset }),
        ),
        Err(response) => response,
    }
}

pub fn reset_offset(slot: &CompareSlot, request: &Request) -> Response {
    match with_workspace_mut(slot, request.id, |workspace| workspace.reset_offset()) {
        Ok(()) => Response::success(request.id, serde_json::json!({ "manual_offset": 0 })),
        Err(response) => response,
    }
}

/// Why [`resolve_diff_heatmap`] couldn't produce a heatmap for one frame -- shared between
/// [`get_diff_frame`] (maps each variant to a wire error) and [`find_first_diff_frame`] (treats
/// [`NoAlignedFrame`](DiffFrameError::NoAlignedFrame) as a skippable per-frame gap, everything
/// else as a real scan-aborting failure -- see that function's doc).
enum DiffFrameError {
    DiffDisabled(String),
    NoAlignedFrame,
    ResolutionMismatch { a: (u32, u32), b: (u32, u32) },
    Other(String),
}

impl DiffFrameError {
    fn into_response(self, id: u32) -> Response {
        match self {
            DiffFrameError::DiffDisabled(reason) => failure(id, WireErrorCode::InvalidData, reason),
            DiffFrameError::NoAlignedFrame => failure(
                id,
                WireErrorCode::FrameNotFound,
                "no aligned stream B frame for this stream A frame",
            ),
            DiffFrameError::ResolutionMismatch { a, b } => failure(
                id,
                WireErrorCode::InvalidData,
                format!(
                    "diff frame requires an exact resolution match (A={}x{}, B={}x{}) -- small \
                     tolerance-based resolution differences aren't supported by this pass",
                    a.0, a.1, b.0, b.1
                ),
            ),
            DiffFrameError::Other(message) => failure(id, WireErrorCode::FrameNotFound, message),
        }
    }
}

/// Resolves the aligned B frame for `stream_a_frame_idx`, fetches both streams' decoded luma
/// planes (via the existing `decode_bridge::get_decoded_frame_yuv` path -- no new decode logic),
/// and returns a real `DiffHeatmapData` (half-res, per `diff_heatmap.rs`'s existing spec).
/// `is_diff_enabled()` only guards a *tolerance-based* "compatible" resolution check
/// (`ResolutionInfo::is_compatible`, this module's doc) -- `DiffHeatmapData::from_luma_planes`
/// asserts an EXACT `width * height` match on both planes, so a compatible-but-not-identical pair
/// would panic there instead of returning a clean error; this guards the stricter exact-match
/// requirement explicitly rather than letting that assert fire.
fn resolve_diff_heatmap(
    core: &Core,
    workspace: &CompareWorkspace,
    stream_a_frame_idx: usize,
    mode: DiffMode,
) -> Result<DiffHeatmapData, DiffFrameError> {
    if !workspace.is_diff_enabled() {
        return Err(DiffFrameError::DiffDisabled(
            workspace
                .disable_reason()
                .unwrap_or("diff overlays disabled")
                .to_string(),
        ));
    }
    let Some((stream_b_frame_idx, _quality)) = workspace.get_aligned_frame(stream_a_frame_idx)
    else {
        return Err(DiffFrameError::NoAlignedFrame);
    };

    let (width, height) = stream_dimensions(core, StreamId::A).map_err(DiffFrameError::Other)?;
    let resolution_b = stream_dimensions(core, StreamId::B).map_err(DiffFrameError::Other)?;
    if (width, height) != resolution_b {
        return Err(DiffFrameError::ResolutionMismatch {
            a: (width, height),
            b: resolution_b,
        });
    }

    let luma_a =
        decoded_luma(core, StreamId::A, stream_a_frame_idx).map_err(DiffFrameError::Other)?;
    let luma_b =
        decoded_luma(core, StreamId::B, stream_b_frame_idx).map_err(DiffFrameError::Other)?;
    debug_assert_eq!(
        luma_a.len(),
        luma_b.len(),
        "exact resolution match was just checked above"
    );

    Ok(DiffHeatmapData::from_luma_planes(
        &luma_a, &luma_b, width, height, mode,
    ))
}

#[derive(serde::Deserialize)]
pub struct GetDiffFrameParams {
    stream_a_frame_idx: usize,
    mode: DiffMode,
}

/// A structured JSON grid, same reasoning as `get_frame_analysis`'s sibling commands -- not raw
/// pixel bytes, so no data-plane binary framing needed.
pub fn get_diff_frame(core: &Core, slot: &CompareSlot, request: &Request) -> Response {
    let params: GetDiffFrameParams = match serde_json::from_value(request.params.clone()) {
        Ok(p) => p,
        Err(err) => return failure(request.id, WireErrorCode::InvalidData, err.to_string()),
    };

    match with_workspace(slot, request.id, |workspace| {
        resolve_diff_heatmap(core, workspace, params.stream_a_frame_idx, params.mode)
    }) {
        Ok(Ok(heatmap)) => Response::success(
            request.id,
            serde_json::to_value(&heatmap).expect("DiffHeatmapData always serializes"),
        ),
        Ok(Err(err)) => err.into_response(request.id),
        Err(response) => response,
    }
}

/// PARITY_CHECKLIST.md CMP-04 -- scans stream A frame by frame (0..`total_frames`) for the first
/// one whose real diff heatmap (`resolve_diff_heatmap`, `DiffMode::Abs`) has ANY non-zero value.
/// `Abs` mode's per-heatmap-cell average is `sum(|lumaA - lumaB|) / 4` over each 2x2 source pixel
/// block (`DiffHeatmapData::from_luma_planes`'s doc) -- since every term is non-negative, that
/// average is exactly zero if and only if all 4 source pixels are identical, so "heatmap max > 0"
/// is an exact (not approximate) test for "at least one pixel differs somewhere in this frame",
/// reusing the existing half-res heatmap rather than a separate full-res byte-equality pass.
///
/// A per-frame `NoAlignedFrame` gap (`DiffFrameError`'s doc) is skipped, not fatal -- expected
/// near stream boundaries when PTS alignment has real gaps. Every other error (diff disabled,
/// resolution mismatch, decode failure) is a workspace-wide or otherwise real condition that
/// won't get better by continuing the scan, so it aborts immediately as a failure response
/// instead of silently skipping up to `total_frames` frames' worth of the same error.
///
/// This is a real per-frame-loopy scan (up to `total_frames` full decodes on each stream), same
/// cancellation-cost class as `index_stream`/`get_thumbnails`/debug-YUV's `find_first_diff_frame`
/// -- cooperatively checks `cancel_flag` once per frame (see `main.rs`'s "Concurrency model" doc).
pub fn find_first_diff_frame(
    core: &Core,
    slot: &CompareSlot,
    request: &Request,
    cancel_flag: &AtomicBool,
) -> Response {
    let total_frames = match with_workspace(slot, request.id, |workspace| workspace.total_frames())
    {
        Ok(n) => n,
        Err(response) => return response,
    };

    let mut checked = 0usize;
    for stream_a_frame_idx in 0..total_frames {
        if cancel_flag.load(Ordering::SeqCst) {
            return failure(request.id, WireErrorCode::Cancelled, "cancelled");
        }

        let result = with_workspace(slot, request.id, |workspace| {
            resolve_diff_heatmap(core, workspace, stream_a_frame_idx, DiffMode::Abs)
        });
        let heatmap = match result {
            Ok(Ok(heatmap)) => heatmap,
            Ok(Err(DiffFrameError::NoAlignedFrame)) => continue,
            Ok(Err(err)) => return err.into_response(request.id),
            Err(response) => return response,
        };
        checked += 1;

        if heatmap.max_value > 0.0 {
            return Response::success(
                request.id,
                serde_json::json!({
                    "frame_index": stream_a_frame_idx,
                    "total_checked": checked,
                }),
            );
        }
    }

    Response::success(
        request.id,
        serde_json::json!({ "frame_index": null, "total_checked": checked }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{
        fresh_compare_state, open_and_index_real_fixture_as_both_streams, open_real_fixture,
    };

    #[test]
    fn create_compare_workspace_end_to_end_builds_a_real_workspace() {
        let core = Core::new();
        open_and_index_real_fixture_as_both_streams(&core);
        let compare_state = fresh_compare_state();

        let response = create_compare_workspace(
            &core,
            &compare_state,
            &Request {
                id: 501,
                method: "create_compare_workspace".to_string(),
                params: serde_json::json!({}),
            },
        );
        assert!(response.ok, "expected ok response, got {response:?}");
        let result = response.result.unwrap();
        assert!(result["total_frames"].as_u64().unwrap() > 0);
        assert_eq!(
            result["diff_enabled"], true,
            "identical-resolution streams must have diff overlays enabled: {result:?}"
        );
        assert!(result["disable_reason"].is_null());
        assert_eq!(
            result["resolution_info"]["is_compatible"], true,
            "identical streams must report resolution-compatible: {result:?}"
        );
        assert_eq!(result["resolution_info"]["is_exact_match"], true);
        assert_eq!(result["resolution_info"]["mismatch_percentage"], 0.0);
        assert_eq!(
            result["alignment"]["method"], "PtsExact",
            "identical streams' PTS values must align exactly: {result:?}"
        );
        assert_eq!(result["alignment"]["gap_count"], 0);
        assert!(result["alignment"]["total_pairs"].as_u64().unwrap() > 0);
        assert!(compare_state.lock().unwrap().is_some());
    }

    #[test]
    fn create_compare_workspace_without_indexing_is_not_found() {
        let core = Core::new();
        open_real_fixture(&core, "A");
        open_real_fixture(&core, "B");
        // Deliberately no index_stream calls -- build_frame_index_map requires indexed units.
        let compare_state = fresh_compare_state();

        let response = create_compare_workspace(
            &core,
            &compare_state,
            &Request {
                id: 502,
                method: "create_compare_workspace".to_string(),
                params: serde_json::json!({}),
            },
        );
        assert!(!response.ok);
        assert_eq!(response.error.unwrap().code, WireErrorCode::NotFound);
        assert!(compare_state.lock().unwrap().is_none());
    }

    #[test]
    fn get_aligned_frame_end_to_end_returns_a_real_exact_pair() {
        let core = Core::new();
        open_and_index_real_fixture_as_both_streams(&core);
        let compare_state = fresh_compare_state();
        create_compare_workspace(
            &core,
            &compare_state,
            &Request {
                id: 510,
                method: "create_compare_workspace".to_string(),
                params: serde_json::json!({}),
            },
        );

        let response = get_aligned_frame(
            &compare_state,
            &Request {
                id: 511,
                method: "get_aligned_frame".to_string(),
                params: serde_json::json!({"stream_a_frame_idx": 0}),
            },
        );
        assert!(response.ok, "expected ok response, got {response:?}");
        let result = response.result.unwrap();
        assert_eq!(
            result["stream_b_frame_idx"], 0,
            "identical streams' frame 0 must align to frame 0: {result:?}"
        );
        assert_eq!(result["quality"], "Exact");
    }

    #[test]
    fn get_aligned_frame_without_a_workspace_is_not_found() {
        let compare_state = fresh_compare_state();
        let response = get_aligned_frame(
            &compare_state,
            &Request {
                id: 512,
                method: "get_aligned_frame".to_string(),
                params: serde_json::json!({"stream_a_frame_idx": 0}),
            },
        );
        assert!(!response.ok);
        assert_eq!(response.error.unwrap().code, WireErrorCode::NotFound);
    }

    #[test]
    fn set_sync_mode_and_manual_offset_and_reset_offset_round_trip() {
        let core = Core::new();
        open_and_index_real_fixture_as_both_streams(&core);
        let compare_state = fresh_compare_state();
        create_compare_workspace(
            &core,
            &compare_state,
            &Request {
                id: 520,
                method: "create_compare_workspace".to_string(),
                params: serde_json::json!({}),
            },
        );

        let response = set_sync_mode(
            &compare_state,
            &Request {
                id: 521,
                method: "set_sync_mode".to_string(),
                params: serde_json::json!({"mode": "Playhead"}),
            },
        );
        assert!(response.ok, "expected ok response, got {response:?}");
        assert_eq!(response.result.unwrap()["sync_mode"], "Playhead");
        assert_eq!(
            compare_state.lock().unwrap().as_ref().unwrap().sync_mode(),
            bitvue_engine::SyncMode::Playhead
        );

        let response = set_manual_offset(
            &compare_state,
            &Request {
                id: 522,
                method: "set_manual_offset".to_string(),
                params: serde_json::json!({"offset": 5}),
            },
        );
        assert!(response.ok, "expected ok response, got {response:?}");
        assert_eq!(response.result.unwrap()["manual_offset"], 5);
        assert_eq!(
            compare_state
                .lock()
                .unwrap()
                .as_ref()
                .unwrap()
                .manual_offset(),
            5
        );

        let response = reset_offset(
            &compare_state,
            &Request {
                id: 523,
                method: "reset_offset".to_string(),
                params: serde_json::json!({}),
            },
        );
        assert!(response.ok, "expected ok response, got {response:?}");
        assert_eq!(response.result.unwrap()["manual_offset"], 0);
        assert_eq!(
            compare_state
                .lock()
                .unwrap()
                .as_ref()
                .unwrap()
                .manual_offset(),
            0
        );
    }

    #[test]
    fn get_diff_frame_identical_streams_is_an_all_zero_heatmap() {
        // A real, non-degenerate check of the actual de-stride + diff math -- if
        // `destride_luma`'s row-pitch handling were wrong (the exact "stride metadata pollution"
        // bug class this module's doc warns about), diffing a stream against ITSELF would NOT
        // come back all-zero (misaligned rows would show spurious diffs), so this genuinely
        // exercises the risky part, not just "did it return 200 OK".
        let core = Core::new();
        open_and_index_real_fixture_as_both_streams(&core);
        let compare_state = fresh_compare_state();
        create_compare_workspace(
            &core,
            &compare_state,
            &Request {
                id: 530,
                method: "create_compare_workspace".to_string(),
                params: serde_json::json!({}),
            },
        );

        for mode in ["abs", "signed"] {
            let response = get_diff_frame(
                &core,
                &compare_state,
                &Request {
                    id: 531,
                    method: "get_diff_frame".to_string(),
                    params: serde_json::json!({"stream_a_frame_idx": 0, "mode": mode}),
                },
            );
            assert!(
                response.ok,
                "expected ok response for mode {mode}, got {response:?}"
            );
            let result = response.result.unwrap();
            assert!(result["heatmap_width"].as_u64().unwrap() > 0);
            assert!(result["heatmap_height"].as_u64().unwrap() > 0);
            assert_eq!(
                result["min_value"], 0.0,
                "identical streams (mode {mode}) must have zero min diff: {result:?}"
            );
            assert_eq!(
                result["max_value"], 0.0,
                "identical streams (mode {mode}) must have zero max diff: {result:?}"
            );
            let values = result["values"].as_array().unwrap();
            assert!(values.iter().all(|v| v.as_f64().unwrap() == 0.0));
        }
    }

    #[test]
    fn get_diff_frame_without_a_workspace_is_not_found() {
        let core = Core::new();
        let compare_state = fresh_compare_state();
        let response = get_diff_frame(
            &core,
            &compare_state,
            &Request {
                id: 532,
                method: "get_diff_frame".to_string(),
                params: serde_json::json!({"stream_a_frame_idx": 0, "mode": "abs"}),
            },
        );
        assert!(!response.ok);
        assert_eq!(response.error.unwrap().code, WireErrorCode::NotFound);
    }

    #[test]
    fn find_first_diff_frame_ab_identical_streams_finds_no_diff() {
        let core = Core::new();
        open_and_index_real_fixture_as_both_streams(&core);
        let compare_state = fresh_compare_state();
        create_compare_workspace(
            &core,
            &compare_state,
            &Request {
                id: 540,
                method: "create_compare_workspace".to_string(),
                params: serde_json::json!({}),
            },
        );

        let response = find_first_diff_frame(
            &core,
            &compare_state,
            &Request {
                id: 541,
                method: "find_first_diff_frame_ab".to_string(),
                params: serde_json::json!({}),
            },
            &AtomicBool::new(false),
        );
        assert!(response.ok, "expected ok response, got {response:?}");
        let result = response.result.unwrap();
        assert!(
            result["frame_index"].is_null(),
            "identical A/B streams must report no diff frame: {result:?}"
        );
        assert_eq!(result["total_checked"], 250);
    }

    #[test]
    fn find_first_diff_frame_ab_genuinely_misaligned_content_finds_a_real_diff() {
        // Same fixture opened as both A and B (no second same-resolution-but-different-content
        // AV1 IVF fixture exists in this repo, and this project's own convention is to not add
        // third-party test data just to manufacture one -- see project memory on that). Instead,
        // `set_manual_offset` deliberately misaligns A against B: since both streams share
        // identical PTS values 1:1, offsetting by +5 pairs stream A frame N against stream B
        // frame N+5 -- genuinely different content (this is a real, changing test-pattern video,
        // not a static image), so this exercises the real "found a diff" path without needing a
        // second fixture.
        let core = Core::new();
        open_and_index_real_fixture_as_both_streams(&core);
        let compare_state = fresh_compare_state();
        create_compare_workspace(
            &core,
            &compare_state,
            &Request {
                id: 543,
                method: "create_compare_workspace".to_string(),
                params: serde_json::json!({}),
            },
        );
        let offset_response = set_manual_offset(
            &compare_state,
            &Request {
                id: 544,
                method: "set_manual_offset".to_string(),
                params: serde_json::json!({"offset": 5}),
            },
        );
        assert!(
            offset_response.ok,
            "expected ok response, got {offset_response:?}"
        );

        let response = find_first_diff_frame(
            &core,
            &compare_state,
            &Request {
                id: 545,
                method: "find_first_diff_frame_ab".to_string(),
                params: serde_json::json!({}),
            },
            &AtomicBool::new(false),
        );
        assert!(response.ok, "expected ok response, got {response:?}");
        let result = response.result.unwrap();
        assert_eq!(
            result["frame_index"], 0,
            "a +5 manual offset against a real changing video must be caught immediately, not \
             silently reported as no diff: {result:?}"
        );
        assert_eq!(result["total_checked"], 1);
    }

    #[test]
    fn find_first_diff_frame_ab_without_a_workspace_is_not_found() {
        let core = Core::new();
        let compare_state = fresh_compare_state();
        let response = find_first_diff_frame(
            &core,
            &compare_state,
            &Request {
                id: 545,
                method: "find_first_diff_frame_ab".to_string(),
                params: serde_json::json!({}),
            },
            &AtomicBool::new(false),
        );
        assert!(!response.ok);
        assert_eq!(response.error.unwrap().code, WireErrorCode::NotFound);
    }

    #[test]
    fn find_first_diff_frame_ab_stops_early_when_already_cancelled() {
        let core = Core::new();
        open_and_index_real_fixture_as_both_streams(&core);
        let compare_state = fresh_compare_state();
        create_compare_workspace(
            &core,
            &compare_state,
            &Request {
                id: 546,
                method: "create_compare_workspace".to_string(),
                params: serde_json::json!({}),
            },
        );

        let response = find_first_diff_frame(
            &core,
            &compare_state,
            &Request {
                id: 547,
                method: "find_first_diff_frame_ab".to_string(),
                params: serde_json::json!({}),
            },
            &AtomicBool::new(true),
        );
        assert!(
            !response.ok,
            "expected a failure response, got {response:?}"
        );
        assert_eq!(response.error.unwrap().code, WireErrorCode::Cancelled);
    }
}
