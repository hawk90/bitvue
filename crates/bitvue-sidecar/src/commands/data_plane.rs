//! Raw-byte data-plane commands -- `get_hex_range`/`get_decoded_frame_yuv` -- which return a
//! `Control` metadata frame followed by a `Data` frame of raw bytes, not a single JSON `Response`
//! (no base64-in-JSON for potentially multi-megabyte payloads). Split out of `main.rs` (2026-08-19)
//! as part of an SRP pass.

use crate::command_support::{parse_stream_id, single_control_frame, wire_error_code_for};
use crate::decode_bridge;
use bitvue_engine::Core;
use bitvue_protocol::{FrameKind, Request, Response, WireError, WireErrorCode};

#[derive(serde::Deserialize)]
struct GetHexRangeParams {
    stream: String,
    offset: u64,
    len: usize,
}

/// First data-plane command: produces a `Control` metadata frame followed by a `Data` frame
/// carrying the raw bytes — no JSON array, no base64. Both frames share `correlation_id` so the
/// client can pair them without a nested envelope. Pure (no I/O) — see `compute_frames`.
pub fn get_hex_range(core: &Core, request: &Request) -> Vec<(FrameKind, Vec<u8>)> {
    let params: GetHexRangeParams = match serde_json::from_value(request.params.clone()) {
        Ok(p) => p,
        Err(err) => {
            return single_control_frame(Response::failure(
                request.id,
                WireError {
                    code: WireErrorCode::InvalidData,
                    message: err.to_string(),
                    offset: None,
                },
            ))
        }
    };
    let stream = match parse_stream_id(request.id, &params.stream) {
        Ok(s) => s,
        Err(response) => return single_control_frame(response),
    };

    let stream_state = core.get_stream(stream);
    let state = stream_state.read();
    let byte_cache = match state.byte_cache.as_ref() {
        Some(cache) => std::sync::Arc::clone(cache),
        None => {
            return single_control_frame(Response::failure(
                request.id,
                WireError {
                    code: WireErrorCode::NotFound,
                    message: "stream not open".to_string(),
                    offset: None,
                },
            ))
        }
    };
    drop(state);

    match byte_cache.read_range(params.offset, params.len) {
        Ok(bytes) => {
            let meta = Response::success(
                request.id,
                serde_json::json!({ "offset": params.offset, "len": bytes.len() }),
            );
            vec![
                (
                    FrameKind::Control,
                    serde_json::to_vec(&meta).expect("Response always serializes"),
                ),
                (FrameKind::Data, bytes.to_vec()),
            ]
        }
        Err(err) => single_control_frame(Response::failure(
            request.id,
            WireError {
                code: wire_error_code_for(&err),
                message: err.to_string(),
                offset: None,
            },
        )),
    }
}

#[derive(serde::Deserialize)]
struct GetDecodedFrameYuvParams {
    stream: String,
    frame_index: usize,
}

/// Second data-plane command (after `get_hex_range`): a `Control` metadata frame (width/height/
/// strides/chroma format) followed by a `Data` frame of raw concatenated Y+U+V bytes -- same
/// no-base64 wire pattern as `get_hex_range`. Business logic lives in `decode_bridge`, not
/// `bitvue-indexer` (which is explicitly pixel-decode-free by design, see its crate doc) --
/// `bitvue-sidecar` is the orchestration layer allowed to depend on `bitvue-decode` directly.
pub fn get_decoded_frame_yuv(core: &Core, request: &Request) -> Vec<(FrameKind, Vec<u8>)> {
    let params: GetDecodedFrameYuvParams = match serde_json::from_value(request.params.clone()) {
        Ok(p) => p,
        Err(err) => {
            return single_control_frame(Response::failure(
                request.id,
                WireError {
                    code: WireErrorCode::InvalidData,
                    message: err.to_string(),
                    offset: None,
                },
            ))
        }
    };
    let stream = match parse_stream_id(request.id, &params.stream) {
        Ok(s) => s,
        Err(response) => return single_control_frame(response),
    };

    let stream_state = core.get_stream(stream);
    let state = stream_state.read();
    let byte_cache = match state.byte_cache.as_ref() {
        Some(cache) => std::sync::Arc::clone(cache),
        None => {
            return single_control_frame(Response::failure(
                request.id,
                WireError {
                    code: WireErrorCode::NotFound,
                    message: "stream not open".to_string(),
                    offset: None,
                },
            ))
        }
    };
    drop(state);

    let full_len = byte_cache.len() as usize;
    let data = match byte_cache.read_range(0, full_len) {
        Ok(bytes) => bytes,
        Err(err) => {
            return single_control_frame(Response::failure(
                request.id,
                WireError {
                    code: wire_error_code_for(&err),
                    message: err.to_string(),
                    offset: None,
                },
            ))
        }
    };

    match decode_bridge::get_decoded_frame_yuv(data, params.frame_index) {
        Ok(frame) => {
            let meta = Response::success(
                request.id,
                serde_json::json!({
                    "width": frame.width,
                    "height": frame.height,
                    "bit_depth": frame.bit_depth,
                    "chroma_subsampling": frame.chroma_subsampling,
                    "y_stride": frame.y_stride,
                    "u_stride": frame.u_stride,
                    "v_stride": frame.v_stride,
                    "y_len": frame.y_len,
                    "u_len": frame.u_len,
                    "v_len": frame.v_len,
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
        Err(message) => single_control_frame(Response::failure(
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
    use crate::test_support::{encode_frames, open_real_fixture};
    use bitvue_protocol::FRAME_HEADER_LEN;
    use bitvue_protocol::{FrameHeader, WireErrorCode};
    use std::io::Write;

    #[test]
    fn get_hex_range_stream_not_open_returns_not_found() {
        let core = Core::new();
        let request = Request {
            id: 14,
            method: "get_hex_range".to_string(),
            params: serde_json::json!({"stream": "A", "offset": 0, "len": 4}),
        };
        let frames = get_hex_range(&core, &request);
        let output = encode_frames(14, &frames);

        let out_header =
            FrameHeader::decode(output[..FRAME_HEADER_LEN].try_into().unwrap()).unwrap();
        assert_eq!(out_header.kind, FrameKind::Control);
        let response: Response = serde_json::from_slice(&output[FRAME_HEADER_LEN..]).unwrap();
        assert!(!response.ok);
        assert_eq!(response.error.unwrap().code, WireErrorCode::NotFound);
    }

    #[test]
    fn get_hex_range_success_returns_exact_bytes() {
        // Known content: bytes 0..=255, so `known_bytes[offset..offset+len]` is a precise,
        // independently-checkable expectation (not "looks plausible").
        let known_bytes: Vec<u8> = (0u8..=255).collect();
        let mut file = tempfile::NamedTempFile::new().unwrap();
        file.write_all(&known_bytes).unwrap();

        let core = Core::new();
        let open_response = crate::dispatch(
            &core,
            &Request {
                id: 20,
                method: "open_stream".to_string(),
                params: serde_json::json!({"stream": "A", "path": file.path().to_str().unwrap()}),
            },
        );
        assert!(open_response.ok, "open_stream failed: {open_response:?}");

        let offset: u64 = 10;
        let len: usize = 16;
        let request = Request {
            id: 21,
            method: "get_hex_range".to_string(),
            params: serde_json::json!({"stream": "A", "offset": offset, "len": len}),
        };
        let frames = get_hex_range(&core, &request);
        let output = encode_frames(21, &frames);

        // Frame 1: Control metadata.
        let ctrl_header =
            FrameHeader::decode(output[..FRAME_HEADER_LEN].try_into().unwrap()).unwrap();
        assert_eq!(ctrl_header.kind, FrameKind::Control);
        assert_eq!(ctrl_header.correlation_id, 21);
        let ctrl_body_end = FRAME_HEADER_LEN + ctrl_header.payload_len as usize;
        let ctrl_response: Response =
            serde_json::from_slice(&output[FRAME_HEADER_LEN..ctrl_body_end]).unwrap();
        assert!(ctrl_response.ok);
        assert_eq!(
            ctrl_response.result.unwrap(),
            serde_json::json!({"offset": offset, "len": len})
        );

        // Frame 2: Data, raw bytes, same correlation_id, no JSON/base64 wrapper.
        let data_header_start = ctrl_body_end;
        let data_header = FrameHeader::decode(
            output[data_header_start..data_header_start + FRAME_HEADER_LEN]
                .try_into()
                .unwrap(),
        )
        .unwrap();
        assert_eq!(data_header.kind, FrameKind::Data);
        assert_eq!(data_header.correlation_id, 21);
        assert_eq!(data_header.payload_len as usize, len);
        let data_body_start = data_header_start + FRAME_HEADER_LEN;
        let data_body = &output[data_body_start..data_body_start + len];
        assert_eq!(
            data_body,
            &known_bytes[offset as usize..offset as usize + len]
        );
        // Total output is exactly the two frames, nothing more.
        assert_eq!(output.len(), data_body_start + len);
    }

    #[test]
    fn get_hex_range_out_of_bounds_is_invalid_range() {
        let mut file = tempfile::NamedTempFile::new().unwrap();
        file.write_all(b"short file").unwrap(); // 10 bytes

        let core = Core::new();
        let open_response = crate::dispatch(
            &core,
            &Request {
                id: 30,
                method: "open_stream".to_string(),
                params: serde_json::json!({"stream": "A", "path": file.path().to_str().unwrap()}),
            },
        );
        assert!(open_response.ok);

        let request = Request {
            id: 31,
            method: "get_hex_range".to_string(),
            params: serde_json::json!({"stream": "A", "offset": 0, "len": 1000}),
        };
        let frames = get_hex_range(&core, &request);
        let output = encode_frames(31, &frames);

        let out_header =
            FrameHeader::decode(output[..FRAME_HEADER_LEN].try_into().unwrap()).unwrap();
        assert_eq!(out_header.kind, FrameKind::Control);
        let response: Response = serde_json::from_slice(&output[FRAME_HEADER_LEN..]).unwrap();
        assert!(!response.ok);
        assert_eq!(response.error.unwrap().code, WireErrorCode::InvalidRange);
    }

    #[test]
    fn get_decoded_frame_yuv_end_to_end_returns_real_yuv_planes() {
        let core = Core::new();
        open_real_fixture(&core, "A");
        // Deliberately not calling index_stream -- decode reads raw IVF bytes directly, it
        // doesn't depend on bitvue-indexer's metadata pass at all.

        let request = Request {
            id: 200,
            method: "get_decoded_frame_yuv".to_string(),
            params: serde_json::json!({"stream": "A", "frame_index": 0}),
        };
        let frames = get_decoded_frame_yuv(&core, &request);
        assert_eq!(
            frames.len(),
            2,
            "expected Control metadata + Data planes, got {frames:?}"
        );
        assert_eq!(frames[0].0, FrameKind::Control);
        let ctrl_response: Response = serde_json::from_slice(&frames[0].1).unwrap();
        assert!(ctrl_response.ok, "decode failed: {ctrl_response:?}");
        let meta = ctrl_response.result.unwrap();
        let width = meta["width"].as_u64().unwrap();
        let height = meta["height"].as_u64().unwrap();
        assert!(
            width > 0 && height > 0,
            "expected real dimensions: {meta:?}"
        );
        let y_len = meta["y_len"].as_u64().unwrap() as usize;
        let u_len = meta["u_len"].as_u64().unwrap() as usize;
        let v_len = meta["v_len"].as_u64().unwrap() as usize;
        assert!(y_len > 0, "expected non-empty Y plane");

        assert_eq!(frames[1].0, FrameKind::Data);
        assert_eq!(
            frames[1].1.len(),
            y_len + u_len + v_len,
            "Data frame should be exactly the concatenated Y+U+V planes"
        );
    }

    #[test]
    fn get_decoded_frame_yuv_out_of_range_frame_index_is_a_wire_error() {
        let core = Core::new();
        open_real_fixture(&core, "A");

        let request = Request {
            id: 201,
            method: "get_decoded_frame_yuv".to_string(),
            params: serde_json::json!({"stream": "A", "frame_index": 999_999}),
        };
        let frames = get_decoded_frame_yuv(&core, &request);
        assert_eq!(frames.len(), 1, "error path should not emit a Data frame");
        let response: Response = serde_json::from_slice(&frames[0].1).unwrap();
        assert!(!response.ok);
        assert_eq!(response.error.unwrap().code, WireErrorCode::FrameNotFound);
    }

    #[test]
    fn get_decoded_frame_yuv_stream_not_open_returns_not_found() {
        let core = Core::new();
        let request = Request {
            id: 202,
            method: "get_decoded_frame_yuv".to_string(),
            params: serde_json::json!({"stream": "A", "frame_index": 0}),
        };
        let frames = get_decoded_frame_yuv(&core, &request);
        assert_eq!(frames.len(), 1);
        let response: Response = serde_json::from_slice(&frames[0].1).unwrap();
        assert!(!response.ok);
        assert_eq!(response.error.unwrap().code, WireErrorCode::NotFound);
    }
}
