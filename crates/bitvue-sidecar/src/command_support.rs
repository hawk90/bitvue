//! Small helpers shared across every command module (`commands/*.rs`, `compare.rs`,
//! `debug_yuv.rs`, `decode_bridge.rs`, etc.) -- stream-id parsing, the `BitvueError` ->
//! `WireErrorCode` mapping, panic-payload extraction, and the two response-framing helpers every
//! handler uses. Split out of `main.rs` (2026-08-19) as part of an SRP pass -- `main.rs` had grown
//! to ~4200 lines mixing bootstrap/orchestration with every individual command's implementation
//! and tests; this crate already had the convention (`compare.rs`, `context_menu.rs`,
//! `frame_analysis.rs`, etc. each own their full request-to-response handling) for everything
//! except a handful of commands that had simply never been pulled out yet.

use std::sync::Arc;

use bitvue_engine::{BitvueError, ByteCache, Core, Event, StreamId};
use bitvue_protocol::{FrameKind, Request, Response, WireError, WireErrorCode};
use serde::de::DeserializeOwned;

/// Failure reply with an explicit wire code and message, no byte offset.
pub fn wire_failure(request_id: u32, code: WireErrorCode, message: impl Into<String>) -> Response {
    Response::failure(
        request_id,
        WireError {
            code,
            message: message.into(),
            offset: None,
        },
    )
}

/// Deserializes `request.params` into a handler's params struct; a mismatch is answered with
/// `InvalidData` carrying serde's message verbatim.
pub fn parse_params<T: DeserializeOwned>(request: &Request) -> Result<T, Response> {
    serde_json::from_value(request.params.clone())
        .map_err(|err| wire_failure(request.id, WireErrorCode::InvalidData, err.to_string()))
}

/// The byte cache of an open stream, or the `NotFound "stream not open"` reply. The stream's
/// read lock is released before this returns, so callers never hold it while reading bytes.
pub fn stream_byte_cache(
    core: &Core,
    stream: StreamId,
    request_id: u32,
) -> Result<Arc<ByteCache>, Response> {
    let stream_state = core.get_stream(stream);
    let state = stream_state.read();
    let cache = state.byte_cache.as_ref().map(Arc::clone);
    drop(state);
    cache.ok_or_else(|| wire_failure(request_id, WireErrorCode::NotFound, "stream not open"))
}

/// The whole stream's bytes, or the engine-error reply (`read_range` failure).
pub fn read_all(cache: &ByteCache, request_id: u32) -> Result<&[u8], Response> {
    cache
        .read_range(0, cache.len() as usize)
        .map_err(|err| engine_error_response(request_id, &err))
}

pub fn parse_stream_id(request_id: u32, s: &str) -> Result<StreamId, Response> {
    match s {
        "A" => Ok(StreamId::A),
        "B" => Ok(StreamId::B),
        other => Err(wire_failure(
            request_id,
            WireErrorCode::InvalidData,
            format!("unknown stream id: {other} (expected \"A\" or \"B\")"),
        )),
    }
}

/// Extracts a human-readable message from a `catch_unwind` payload. `panic!("literal")` yields
/// `&'static str`, `panic!("{}", x)`/`.expect(...)`/`.unwrap()` yield `String` -- those two cover
/// the overwhelming majority of real panics; anything else (a custom payload type from
/// `panic_any`) falls back to a generic message rather than failing to report at all.
pub fn panic_payload_message(payload: &(dyn std::any::Any + Send)) -> String {
    if let Some(s) = payload.downcast_ref::<&str>() {
        (*s).to_string()
    } else if let Some(s) = payload.downcast_ref::<String>() {
        s.clone()
    } else {
        "non-string panic payload".to_string()
    }
}

pub fn single_control_frame(response: Response) -> Vec<(FrameKind, Vec<u8>)> {
    vec![(
        FrameKind::Control,
        serde_json::to_vec(&response).expect("Response always serializes"),
    )]
}

/// Failure reply for an engine error: wire code from [`wire_error_code_for`], message from
/// `Display`, no byte offset.
pub fn engine_error_response(request_id: u32, err: &BitvueError) -> Response {
    wire_failure(request_id, wire_error_code_for(err), err.to_string())
}

/// Mirror of `bitvue_engine::BitvueError` variants onto `WireErrorCode` — see the "not a direct
/// `Serialize` derive" note on `WireErrorCode` in `bitvue-protocol` for why this mapping lives
/// here instead of on the engine's error type.
pub fn wire_error_code_for(err: &BitvueError) -> WireErrorCode {
    match err {
        BitvueError::Io(_) | BitvueError::IoError { .. } => WireErrorCode::Io,
        BitvueError::Parse { .. } => WireErrorCode::Parse,
        BitvueError::InvalidObuType(_) => WireErrorCode::InvalidObuType,
        BitvueError::UnexpectedEof(_) => WireErrorCode::UnexpectedEof,
        BitvueError::UnsupportedCodec(_) => WireErrorCode::UnsupportedCodec,
        BitvueError::Decode(_) => WireErrorCode::Decode,
        BitvueError::InsufficientData { .. } => WireErrorCode::InsufficientData,
        BitvueError::InvalidData(_) => WireErrorCode::InvalidData,
        BitvueError::InvalidFile(_) => WireErrorCode::InvalidFile,
        BitvueError::InvalidRange { .. } => WireErrorCode::InvalidRange,
        BitvueError::FileModified { .. } => WireErrorCode::FileModified,
        BitvueError::FrameNotFound(_) => WireErrorCode::FrameNotFound,
        BitvueError::NotFound(_) => WireErrorCode::NotFound,
        BitvueError::Serialization(_) => WireErrorCode::Serialization,
    }
}

/// Manual `Event` → JSON mapping (`Event` isn't `Serialize` -- it's the internal UI<->Core bus,
/// not a wire contract; see `main.rs`'s module doc).
pub fn event_to_json(event: &Event) -> serde_json::Value {
    match event {
        Event::ModelUpdated { kind, stream } => {
            serde_json::json!({"type": "ModelUpdated", "kind": format!("{kind:?}"), "stream": format!("{stream:?}")})
        }
        Event::SelectionUpdated {
            stream,
            syntax_node,
            bit_range,
        } => {
            serde_json::json!({
                "type": "SelectionUpdated",
                "stream": format!("{stream:?}"),
                "syntax_node": syntax_node,
                "bit_range": bit_range.map(|r| serde_json::json!({
                    "start_bit": r.start_bit,
                    "end_bit": r.end_bit,
                })),
            })
        }
        Event::FrameDecoded {
            stream,
            frame_index,
        } => {
            serde_json::json!({"type": "FrameDecoded", "stream": format!("{stream:?}"), "frame_index": frame_index})
        }
        Event::WorkerProgress { job_id, progress } => {
            serde_json::json!({"type": "WorkerProgress", "job_id": job_id, "progress": progress})
        }
        Event::WorkerFinished { job_id } => {
            serde_json::json!({"type": "WorkerFinished", "job_id": job_id})
        }
        Event::WorkerError { job_id, error } => {
            serde_json::json!({"type": "WorkerError", "job_id": job_id, "error": error})
        }
        Event::DiagnosticAdded { diagnostic } => {
            serde_json::json!({
                "type": "DiagnosticAdded",
                "diagnostic": {
                    "id": diagnostic.id,
                    "severity": format!("{:?}", diagnostic.severity),
                    "message": diagnostic.message,
                    "category": format!("{:?}", diagnostic.category),
                    "offset_bytes": diagnostic.offset_bytes,
                    "timestamp_ms": diagnostic.timestamp_ms,
                    "frame_index": diagnostic.frame_index,
                    "count": diagnostic.count,
                    "impact_score": diagnostic.impact_score,
                },
            })
        }
        Event::DiagnosticsCleared { stream } => {
            serde_json::json!({"type": "DiagnosticsCleared", "stream": format!("{stream:?}")})
        }
        Event::ExportFinished { path } => {
            serde_json::json!({"type": "ExportFinished", "path": path.display().to_string()})
        }
        Event::ExportFailed { error } => {
            serde_json::json!({"type": "ExportFailed", "error": error})
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn engine_error_response_carries_id_code_message_and_no_offset() {
        let err = BitvueError::FrameNotFound(7);
        let resp = engine_error_response(42, &err);
        assert_eq!(resp.id, 42);
        assert!(!resp.ok);
        assert!(resp.result.is_none());
        let wire = resp.error.expect("failure carries an error");
        assert_eq!(wire.code, wire_error_code_for(&err));
        assert!(matches!(wire.code, WireErrorCode::FrameNotFound));
        assert_eq!(wire.message, err.to_string());
        assert_eq!(wire.offset, None);
    }
}
