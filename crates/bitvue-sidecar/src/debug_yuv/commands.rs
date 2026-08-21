//! Wire-protocol command handlers for the debug-YUV subsystem -- load/unload the reference
//! session, mutate its offset/crop, and fetch the metrics/frame outputs `super::compare` computes.

use super::compare::{compute_frame_metrics, find_first_diff, get_frame, FindFirstDiffError};
use super::{load, Crop, LoadParams};
use bitvue_engine::Core;
use bitvue_protocol::{FrameKind, Request, Response, WireError, WireErrorCode};
use std::sync::atomic::AtomicBool;

/// Failure to load isn't a wire-level error -- same "domain outcome vs RPC outcome" split
/// `open_stream`'s module-doc note describes: the call succeeded, `result.success` carries whether
/// the load itself worked, matching the frontend's `YuvDiffContext.loadFile` expectations.
pub fn load_debug_yuv(state: &crate::DebugYuvSlot, request: &Request) -> Response {
    let params: LoadParams = match serde_json::from_value(request.params.clone()) {
        Ok(p) => p,
        Err(err) => {
            return Response::failure(
                request.id,
                WireError {
                    code: WireErrorCode::InvalidData,
                    message: err.to_string(),
                    offset: None,
                },
            )
        }
    };
    match load(params) {
        Ok(session) => {
            let frame_count = session.frame_count;
            let frame_size = session.frame_size;
            *state.lock().unwrap() = Some(session);
            Response::success(
                request.id,
                serde_json::json!({
                    "success": true,
                    "frame_count": frame_count,
                    "frame_size": frame_size,
                    "error": null,
                }),
            )
        }
        Err(message) => Response::success(
            request.id,
            serde_json::json!({
                "success": false,
                "frame_count": 0,
                "frame_size": 0,
                "error": message,
            }),
        ),
    }
}

pub fn unload_debug_yuv(state: &crate::DebugYuvSlot, request: &Request) -> Response {
    *state.lock().unwrap() = None;
    Response::success(request.id, serde_json::json!({}))
}

pub fn no_debug_yuv_loaded(request_id: u32) -> Response {
    Response::failure(
        request_id,
        WireError {
            code: WireErrorCode::NotFound,
            message: "no debug YUV file loaded".to_string(),
            offset: None,
        },
    )
}

#[derive(serde::Deserialize)]
struct SetDebugYuvOffsetParams {
    offset: i64,
}

pub fn set_debug_yuv_offset(state: &crate::DebugYuvSlot, request: &Request) -> Response {
    let params: SetDebugYuvOffsetParams = match serde_json::from_value(request.params.clone()) {
        Ok(p) => p,
        Err(err) => {
            return Response::failure(
                request.id,
                WireError {
                    code: WireErrorCode::InvalidData,
                    message: err.to_string(),
                    offset: None,
                },
            )
        }
    };
    let mut guard = state.lock().unwrap();
    match guard.as_mut() {
        Some(session) => {
            session.picture_offset = params.offset;
            Response::success(request.id, serde_json::json!({}))
        }
        None => no_debug_yuv_loaded(request.id),
    }
}

#[derive(serde::Deserialize)]
struct SetDebugYuvCropParams {
    crop: Crop,
}

pub fn set_debug_yuv_crop(state: &crate::DebugYuvSlot, request: &Request) -> Response {
    let params: SetDebugYuvCropParams = match serde_json::from_value(request.params.clone()) {
        Ok(p) => p,
        Err(err) => {
            return Response::failure(
                request.id,
                WireError {
                    code: WireErrorCode::InvalidData,
                    message: err.to_string(),
                    offset: None,
                },
            )
        }
    };
    let mut guard = state.lock().unwrap();
    match guard.as_mut() {
        Some(session) => {
            session.crop = params.crop;
            Response::success(request.id, serde_json::json!({}))
        }
        None => no_debug_yuv_loaded(request.id),
    }
}

#[derive(serde::Deserialize)]
struct DebugYuvFrameIndexParams {
    frame_index: usize,
}

pub fn get_yuv_diff_metrics(
    core: &Core,
    state: &crate::DebugYuvSlot,
    request: &Request,
) -> Response {
    let params: DebugYuvFrameIndexParams = match serde_json::from_value(request.params.clone()) {
        Ok(p) => p,
        Err(err) => {
            return Response::failure(
                request.id,
                WireError {
                    code: WireErrorCode::InvalidData,
                    message: err.to_string(),
                    offset: None,
                },
            )
        }
    };
    let guard = state.lock().unwrap();
    let session = match guard.as_ref() {
        Some(s) => s,
        None => return no_debug_yuv_loaded(request.id),
    };
    match compute_frame_metrics(core, session, params.frame_index) {
        Ok(m) => Response::success(
            request.id,
            serde_json::json!({
                "frame_index": m.frame_index,
                "psnr_y": m.psnr_y,
                "psnr_u": m.psnr_u,
                "psnr_v": m.psnr_v,
                "psnr_avg": m.psnr_avg,
                "ssim_y": m.ssim_y,
                "max_diff_y": m.max_diff_y,
                "has_mismatch": m.has_mismatch,
            }),
        ),
        Err(message) => Response::failure(
            request.id,
            WireError {
                code: WireErrorCode::FrameNotFound,
                message,
                offset: None,
            },
        ),
    }
}

pub fn find_first_diff_frame(
    core: &Core,
    state: &crate::DebugYuvSlot,
    request: &Request,
    cancel_flag: &AtomicBool,
) -> Response {
    let guard = state.lock().unwrap();
    let session = match guard.as_ref() {
        Some(s) => s,
        None => return no_debug_yuv_loaded(request.id),
    };
    match find_first_diff(core, session, cancel_flag) {
        Ok((frame_index, total_checked)) => Response::success(
            request.id,
            serde_json::json!({
                "frame_index": frame_index,
                "total_checked": total_checked,
            }),
        ),
        Err(FindFirstDiffError::Cancelled) => Response::failure(
            request.id,
            WireError {
                code: WireErrorCode::Cancelled,
                message: "cancelled".to_string(),
                offset: None,
            },
        ),
        Err(FindFirstDiffError::Other(message)) => Response::failure(
            request.id,
            WireError {
                code: WireErrorCode::Internal,
                message,
                offset: None,
            },
        ),
    }
}

#[derive(serde::Deserialize)]
struct GetDebugYuvFrameParams {
    frame_index: usize,
    mode: String,
    #[serde(default)]
    amplify: Option<u32>,
}

/// Data-plane command, same `Control` + `Data` two-frame pattern as `get_decoded_frame_yuv` --
/// see `get_frame` for how each of the four display modes is produced.
pub fn get_debug_yuv_frame(
    core: &Core,
    state: &crate::DebugYuvSlot,
    request: &Request,
) -> Vec<(FrameKind, Vec<u8>)> {
    let params: GetDebugYuvFrameParams = match serde_json::from_value(request.params.clone()) {
        Ok(p) => p,
        Err(err) => {
            return crate::command_support::single_control_frame(Response::failure(
                request.id,
                WireError {
                    code: WireErrorCode::InvalidData,
                    message: err.to_string(),
                    offset: None,
                },
            ))
        }
    };
    let guard = state.lock().unwrap();
    let session = match guard.as_ref() {
        Some(s) => s,
        None => {
            return crate::command_support::single_control_frame(no_debug_yuv_loaded(request.id))
        }
    };
    match get_frame(
        core,
        session,
        params.frame_index,
        &params.mode,
        params.amplify,
    ) {
        Ok(frame) => {
            let meta = Response::success(
                request.id,
                serde_json::json!({
                    "width": frame.descriptor.width,
                    "height": frame.descriptor.height,
                    "bit_depth": frame.descriptor.bit_depth,
                    "chroma_subsampling": frame.descriptor.chroma_subsampling,
                    "y_stride": frame.descriptor.y_stride,
                    "u_stride": frame.descriptor.u_stride,
                    "v_stride": frame.descriptor.v_stride,
                    "y_len": frame.descriptor.y_len,
                    "u_len": frame.descriptor.u_len,
                    "v_len": frame.descriptor.v_len,
                }),
            );
            vec![
                (
                    FrameKind::Control,
                    serde_json::to_vec(&meta).expect("Response always serializes"),
                ),
                (FrameKind::Data, frame.bytes),
            ]
        }
        Err(message) => crate::command_support::single_control_frame(Response::failure(
            request.id,
            WireError {
                code: WireErrorCode::FrameNotFound,
                message,
                offset: None,
            },
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{fresh_debug_yuv_state, open_real_fixture, write_i420_frame};
    use std::io::Write;

    #[test]
    fn load_debug_yuv_end_to_end_reports_real_frame_count() {
        let file = write_i420_frame(4, 4, &[9u8; 16], &[9u8; 4], &[9u8; 4]);
        let debug_yuv_state = fresh_debug_yuv_state();
        let response = load_debug_yuv(
            &debug_yuv_state,
            &Request {
                id: 300,
                method: "load_debug_yuv".to_string(),
                params: serde_json::json!({
                    "path": file.path().to_str().unwrap(),
                    "width": 4,
                    "height": 4,
                    "format": "i420",
                    "bitdepth": 8,
                }),
            },
        );
        assert!(response.ok, "expected ok response, got {response:?}");
        let result = response.result.unwrap();
        assert_eq!(result["success"], true);
        assert_eq!(result["frame_count"], 1);
        assert_eq!(result["frame_size"], 24);
        assert!(debug_yuv_state.lock().unwrap().is_some());
    }

    #[test]
    fn load_debug_yuv_bad_path_is_a_reported_failure_not_a_wire_error() {
        let debug_yuv_state = fresh_debug_yuv_state();
        let response = load_debug_yuv(
            &debug_yuv_state,
            &Request {
                id: 301,
                method: "load_debug_yuv".to_string(),
                params: serde_json::json!({
                    "path": "/nonexistent/does-not-exist.yuv",
                    "width": 4,
                    "height": 4,
                    "format": "i420",
                    "bitdepth": 8,
                }),
            },
        );
        // Same "domain outcome vs RPC outcome" split as open_stream -- the call itself succeeds,
        // result.success carries the real failure.
        assert!(response.ok, "expected ok response, got {response:?}");
        let result = response.result.unwrap();
        assert_eq!(result["success"], false);
        assert!(result["error"].as_str().unwrap().contains("cannot open"));
        assert!(debug_yuv_state.lock().unwrap().is_none());
    }

    #[test]
    fn unload_debug_yuv_clears_the_session() {
        let file = write_i420_frame(4, 4, &[0u8; 16], &[0u8; 4], &[0u8; 4]);
        let debug_yuv_state = fresh_debug_yuv_state();
        load_debug_yuv(
            &debug_yuv_state,
            &Request {
                id: 302,
                method: "load_debug_yuv".to_string(),
                params: serde_json::json!({
                    "path": file.path().to_str().unwrap(), "width": 4, "height": 4,
                    "format": "i420", "bitdepth": 8,
                }),
            },
        );
        assert!(debug_yuv_state.lock().unwrap().is_some());
        let response = unload_debug_yuv(
            &debug_yuv_state,
            &Request {
                id: 303,
                method: "unload_debug_yuv".to_string(),
                params: serde_json::json!({}),
            },
        );
        assert!(response.ok);
        assert!(debug_yuv_state.lock().unwrap().is_none());
    }

    #[test]
    fn set_debug_yuv_offset_and_crop_update_the_live_session() {
        let file = write_i420_frame(8, 8, &[0u8; 64], &[0u8; 16], &[0u8; 16]);
        let debug_yuv_state = fresh_debug_yuv_state();
        load_debug_yuv(
            &debug_yuv_state,
            &Request {
                id: 304,
                method: "load_debug_yuv".to_string(),
                params: serde_json::json!({
                    "path": file.path().to_str().unwrap(), "width": 8, "height": 8,
                    "format": "i420", "bitdepth": 8,
                }),
            },
        );

        let offset_response = set_debug_yuv_offset(
            &debug_yuv_state,
            &Request {
                id: 305,
                method: "set_debug_yuv_offset".to_string(),
                params: serde_json::json!({"offset": -3}),
            },
        );
        assert!(offset_response.ok);
        assert_eq!(
            debug_yuv_state
                .lock()
                .unwrap()
                .as_ref()
                .unwrap()
                .picture_offset,
            -3
        );

        let crop_response = set_debug_yuv_crop(
            &debug_yuv_state,
            &Request {
                id: 306,
                method: "set_debug_yuv_crop".to_string(),
                params: serde_json::json!({"crop": {"left": 2, "right": 0, "top": 0, "bottom": 0}}),
            },
        );
        assert!(crop_response.ok);
        assert_eq!(
            debug_yuv_state.lock().unwrap().as_ref().unwrap().crop.left,
            2
        );
    }

    #[test]
    fn set_debug_yuv_offset_without_a_loaded_session_is_not_found() {
        let debug_yuv_state = fresh_debug_yuv_state();
        let response = set_debug_yuv_offset(
            &debug_yuv_state,
            &Request {
                id: 307,
                method: "set_debug_yuv_offset".to_string(),
                params: serde_json::json!({"offset": 1}),
            },
        );
        assert!(!response.ok);
        assert_eq!(response.error.unwrap().code, WireErrorCode::NotFound);
    }

    #[test]
    fn get_debug_yuv_frame_reference_mode_end_to_end_returns_exact_bytes() {
        let y = (0u8..16).collect::<Vec<u8>>();
        let u = vec![50u8; 4];
        let v = vec![150u8; 4];
        let file = write_i420_frame(4, 4, &y, &u, &v);
        let debug_yuv_state = fresh_debug_yuv_state();
        load_debug_yuv(
            &debug_yuv_state,
            &Request {
                id: 310,
                method: "load_debug_yuv".to_string(),
                params: serde_json::json!({
                    "path": file.path().to_str().unwrap(), "width": 4, "height": 4,
                    "format": "i420", "bitdepth": 8,
                }),
            },
        );

        let core = Core::new(); // reference mode never touches stream A
        let frames = get_debug_yuv_frame(
            &core,
            &debug_yuv_state,
            &Request {
                id: 311,
                method: "get_debug_yuv_frame".to_string(),
                params: serde_json::json!({"frame_index": 0, "mode": "reference"}),
            },
        );
        assert_eq!(frames.len(), 2, "expected Control + Data frames");
        let ctrl: Response = serde_json::from_slice(&frames[0].1).unwrap();
        assert!(ctrl.ok, "expected ok response, got {ctrl:?}");
        let meta = ctrl.result.unwrap();
        assert_eq!(meta["width"], 4);
        assert_eq!(meta["height"], 4);
        assert_eq!(meta["y_len"], 16);
        assert_eq!(frames[1].1[..16], y[..]);
        assert_eq!(frames[1].1[16..20], u[..]);
        assert_eq!(frames[1].1[20..24], v[..]);
    }

    #[test]
    fn get_debug_yuv_frame_without_a_loaded_session_is_not_found() {
        let core = Core::new();
        let debug_yuv_state = fresh_debug_yuv_state();
        let frames = get_debug_yuv_frame(
            &core,
            &debug_yuv_state,
            &Request {
                id: 312,
                method: "get_debug_yuv_frame".to_string(),
                params: serde_json::json!({"frame_index": 0, "mode": "reference"}),
            },
        );
        assert_eq!(frames.len(), 1, "error path should not emit a Data frame");
        let response: Response = serde_json::from_slice(&frames[0].1).unwrap();
        assert!(!response.ok);
        assert_eq!(response.error.unwrap().code, WireErrorCode::NotFound);
    }

    /// End-to-end against the real AV1 fixture: builds a reference file from stream A's *actual*
    /// decoded frame 0 bytes, so "identical to what's already playing" is a real, independently
    /// reproducible fact rather than an assumption -- then confirms the diff/metrics path reports
    /// exactly that (no mismatch, nominally-infinite PSNR).
    fn load_reference_matching_real_fixture_frame_zero(
        core: &Core,
    ) -> (tempfile::NamedTempFile, u32, u32) {
        open_real_fixture(core, "A");
        let decode_request = Request {
            id: 320,
            method: "get_decoded_frame_yuv".to_string(),
            params: serde_json::json!({"stream": "A", "frame_index": 0}),
        };
        let decode_sessions = crate::decode_session::DecodeSessions::new();
        let frames = crate::commands::data_plane::get_decoded_frame_yuv(
            core,
            &decode_sessions,
            &decode_request,
            &std::sync::atomic::AtomicBool::new(false),
        );
        let ctrl: Response = serde_json::from_slice(&frames[0].1).unwrap();
        let meta = ctrl.result.unwrap();
        let width = meta["width"].as_u64().unwrap() as u32;
        let height = meta["height"].as_u64().unwrap() as u32;
        let mut file = tempfile::NamedTempFile::new().unwrap();
        file.write_all(&frames[1].1).unwrap();
        (file, width, height)
    }

    #[test]
    fn get_yuv_diff_metrics_identical_reference_reports_no_mismatch() {
        let core = Core::new();
        let (file, width, height) = load_reference_matching_real_fixture_frame_zero(&core);

        let debug_yuv_state = fresh_debug_yuv_state();
        load_debug_yuv(
            &debug_yuv_state,
            &Request {
                id: 321,
                method: "load_debug_yuv".to_string(),
                params: serde_json::json!({
                    "path": file.path().to_str().unwrap(), "width": width, "height": height,
                    "format": "i420", "bitdepth": 8,
                }),
            },
        );

        let response = get_yuv_diff_metrics(
            &core,
            &debug_yuv_state,
            &Request {
                id: 322,
                method: "get_yuv_diff_metrics".to_string(),
                params: serde_json::json!({"frame_index": 0}),
            },
        );
        assert!(response.ok, "expected ok response, got {response:?}");
        let result = response.result.unwrap();
        assert_eq!(result["has_mismatch"], false);
        assert_eq!(result["max_diff_y"], 0);
        assert!(
            result["psnr_avg"].as_f64().unwrap() > 90.0,
            "identical frames should report a very high (~infinite) PSNR, got {result:?}"
        );
    }

    #[test]
    fn get_yuv_diff_metrics_without_a_loaded_session_is_not_found() {
        let core = Core::new();
        let debug_yuv_state = fresh_debug_yuv_state();
        let response = get_yuv_diff_metrics(
            &core,
            &debug_yuv_state,
            &Request {
                id: 323,
                method: "get_yuv_diff_metrics".to_string(),
                params: serde_json::json!({"frame_index": 0}),
            },
        );
        assert!(!response.ok);
        assert_eq!(response.error.unwrap().code, WireErrorCode::NotFound);
    }

    #[test]
    fn get_debug_yuv_frame_diff_mode_flags_a_real_injected_mismatch() {
        let core = Core::new();
        let (file, width, height) = load_reference_matching_real_fixture_frame_zero(&core);

        // Corrupt one reference Y byte so decoded != reference at a known location.
        let mut bytes = std::fs::read(file.path()).unwrap();
        bytes[0] = bytes[0].wrapping_add(100);
        std::fs::write(file.path(), &bytes).unwrap();

        let debug_yuv_state = fresh_debug_yuv_state();
        load_debug_yuv(
            &debug_yuv_state,
            &Request {
                id: 330,
                method: "load_debug_yuv".to_string(),
                params: serde_json::json!({
                    "path": file.path().to_str().unwrap(), "width": width, "height": height,
                    "format": "i420", "bitdepth": 8,
                }),
            },
        );

        let metrics_response = get_yuv_diff_metrics(
            &core,
            &debug_yuv_state,
            &Request {
                id: 331,
                method: "get_yuv_diff_metrics".to_string(),
                params: serde_json::json!({"frame_index": 0}),
            },
        );
        let result = metrics_response.result.unwrap();
        assert_eq!(result["has_mismatch"], true);
        assert!(result["max_diff_y"].as_u64().unwrap() > 0);

        let frames = get_debug_yuv_frame(
            &core,
            &debug_yuv_state,
            &Request {
                id: 332,
                method: "get_debug_yuv_frame".to_string(),
                params: serde_json::json!({"frame_index": 0, "mode": "diff"}),
            },
        );
        assert_eq!(frames.len(), 2);
        let ctrl: Response = serde_json::from_slice(&frames[0].1).unwrap();
        assert!(ctrl.ok, "expected ok response, got {ctrl:?}");
        assert!(
            frames[1].1.iter().any(|&b| b != 0),
            "diff mode should show a non-zero delta somewhere given the injected mismatch"
        );
    }

    #[test]
    fn find_first_diff_frame_end_to_end_locates_the_injected_mismatch() {
        let core = Core::new();
        let (file, width, height) = load_reference_matching_real_fixture_frame_zero(&core);
        let mut bytes = std::fs::read(file.path()).unwrap();
        bytes[0] = bytes[0].wrapping_add(100);
        std::fs::write(file.path(), &bytes).unwrap();

        let debug_yuv_state = fresh_debug_yuv_state();
        load_debug_yuv(
            &debug_yuv_state,
            &Request {
                id: 340,
                method: "load_debug_yuv".to_string(),
                params: serde_json::json!({
                    "path": file.path().to_str().unwrap(), "width": width, "height": height,
                    "format": "i420", "bitdepth": 8,
                }),
            },
        );

        let response = find_first_diff_frame(
            &core,
            &debug_yuv_state,
            &Request {
                id: 341,
                method: "find_first_diff_frame".to_string(),
                params: serde_json::json!({}),
            },
            &AtomicBool::new(false),
        );
        assert!(response.ok, "expected ok response, got {response:?}");
        let result = response.result.unwrap();
        assert_eq!(
            result["frame_index"], 0,
            "the only reference frame -- and the only one with an injected mismatch -- is index 0"
        );
        assert_eq!(result["total_checked"], 1);
    }

    /// Real cancellation-checkpoint regression test for UIX-ASYNC-006: proves
    /// `find_first_diff_frame` actually stops (and reports `Cancelled`, not a silent success)
    /// instead of scanning to completion when its flag is already set. Before this fix, the
    /// worker thread only ever checked the flag once *before* calling the handler at all
    /// (`spawn_request`'s "cancelled before execution started" branch); this test exercises the
    /// handler's own mid-execution checkpoint directly, which didn't exist at all previously --
    /// the same fixture/mismatch setup as the "locates the injected mismatch" test above would
    /// have returned `frame_index: 0` regardless of cancellation with the old code.
    #[test]
    fn find_first_diff_frame_stops_early_when_already_cancelled() {
        let core = Core::new();
        let (file, width, height) = load_reference_matching_real_fixture_frame_zero(&core);
        let mut bytes = std::fs::read(file.path()).unwrap();
        bytes[0] = bytes[0].wrapping_add(100);
        std::fs::write(file.path(), &bytes).unwrap();

        let debug_yuv_state = fresh_debug_yuv_state();
        load_debug_yuv(
            &debug_yuv_state,
            &Request {
                id: 350,
                method: "load_debug_yuv".to_string(),
                params: serde_json::json!({
                    "path": file.path().to_str().unwrap(), "width": width, "height": height,
                    "format": "i420", "bitdepth": 8,
                }),
            },
        );

        let response = find_first_diff_frame(
            &core,
            &debug_yuv_state,
            &Request {
                id: 351,
                method: "find_first_diff_frame".to_string(),
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

    #[test]
    fn find_first_diff_frame_without_a_loaded_session_is_not_found() {
        let core = Core::new();
        let debug_yuv_state = fresh_debug_yuv_state();
        let response = find_first_diff_frame(
            &core,
            &debug_yuv_state,
            &Request {
                id: 342,
                method: "find_first_diff_frame".to_string(),
                params: serde_json::json!({}),
            },
            &AtomicBool::new(false),
        );
        assert!(!response.ok);
        assert_eq!(response.error.unwrap().code, WireErrorCode::NotFound);
    }
}
