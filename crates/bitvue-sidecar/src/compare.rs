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

#[derive(serde::Deserialize)]
pub struct GetDiffFrameParams {
    stream_a_frame_idx: usize,
    mode: DiffMode,
}

/// Resolves the aligned B frame for `stream_a_frame_idx`, fetches both streams' decoded luma
/// planes (via the existing `decode_bridge::get_decoded_frame_yuv` path -- no new decode logic),
/// and returns a real `DiffHeatmapData` (half-res, per `diff_heatmap.rs`'s existing spec). A
/// structured JSON grid, same reasoning as `get_frame_analysis`/`get_diff_frame`'s sibling
/// commands -- not raw pixel bytes, so no data-plane binary framing needed.
pub fn get_diff_frame(core: &Core, slot: &CompareSlot, request: &Request) -> Response {
    let params: GetDiffFrameParams = match serde_json::from_value(request.params.clone()) {
        Ok(p) => p,
        Err(err) => return failure(request.id, WireErrorCode::InvalidData, err.to_string()),
    };

    let workspace_check = with_workspace(slot, request.id, |workspace| {
        if !workspace.is_diff_enabled() {
            return Err(workspace
                .disable_reason()
                .unwrap_or("diff overlays disabled")
                .to_string());
        }
        Ok(workspace.get_aligned_frame(params.stream_a_frame_idx))
    });
    let aligned = match workspace_check {
        Ok(Ok(aligned)) => aligned,
        Ok(Err(reason)) => return failure(request.id, WireErrorCode::InvalidData, reason),
        Err(response) => return response,
    };
    let Some((stream_b_frame_idx, _quality)) = aligned else {
        return failure(
            request.id,
            WireErrorCode::FrameNotFound,
            format!(
                "no aligned stream B frame for stream A frame {}",
                params.stream_a_frame_idx
            ),
        );
    };

    let (width, height) = match stream_dimensions(core, StreamId::A) {
        Ok(dims) => dims,
        Err(message) => return failure(request.id, WireErrorCode::NotFound, message),
    };
    let resolution_b = match stream_dimensions(core, StreamId::B) {
        Ok(dims) => dims,
        Err(message) => return failure(request.id, WireErrorCode::NotFound, message),
    };
    // `is_diff_enabled()` above only guards a *tolerance-based* "compatible" resolution check
    // (`ResolutionInfo::is_compatible`, `compare.rs`'s doc) -- `DiffHeatmapData::from_luma_planes`
    // asserts an EXACT `width * height` match on both planes, so a compatible-but-not-identical
    // pair would panic there instead of returning a clean error. Guard the stricter exact-match
    // requirement explicitly rather than letting that assert fire.
    if (width, height) != resolution_b {
        return failure(
            request.id,
            WireErrorCode::InvalidData,
            format!(
                "diff frame requires an exact resolution match (A={width}x{height}, \
                 B={}x{}) -- small tolerance-based resolution differences aren't supported by \
                 this pass",
                resolution_b.0, resolution_b.1
            ),
        );
    }

    let luma_a = match decoded_luma(core, StreamId::A, params.stream_a_frame_idx) {
        Ok(bytes) => bytes,
        Err(message) => return failure(request.id, WireErrorCode::FrameNotFound, message),
    };
    let luma_b = match decoded_luma(core, StreamId::B, stream_b_frame_idx) {
        Ok(bytes) => bytes,
        Err(message) => return failure(request.id, WireErrorCode::FrameNotFound, message),
    };
    debug_assert_eq!(
        luma_a.len(),
        luma_b.len(),
        "exact resolution match was just checked above"
    );

    let heatmap = DiffHeatmapData::from_luma_planes(&luma_a, &luma_b, width, height, params.mode);
    Response::success(
        request.id,
        serde_json::to_value(&heatmap).expect("DiffHeatmapData always serializes"),
    )
}
