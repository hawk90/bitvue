//! Small helpers shared across every command module (`commands/*.rs`, `compare.rs`,
//! `debug_yuv.rs`, `decode_bridge.rs`, etc.) -- stream-id parsing, the `BitvueError` ->
//! `WireErrorCode` mapping, panic-payload extraction, and the two response-framing helpers every
//! handler uses. Split out of `main.rs` (2026-08-19) as part of an SRP pass -- `main.rs` had grown
//! to ~4200 lines mixing bootstrap/orchestration with every individual command's implementation
//! and tests; this crate already had the convention (`compare.rs`, `context_menu.rs`,
//! `frame_analysis.rs`, etc. each own their full request-to-response handling) for everything
//! except a handful of commands that had simply never been pulled out yet.

use bitvue_engine::{BitvueError, Event, StreamId};
use bitvue_protocol::{FrameKind, Response, WireError, WireErrorCode};

pub fn parse_stream_id(request_id: u32, s: &str) -> Result<StreamId, Response> {
    match s {
        "A" => Ok(StreamId::A),
        "B" => Ok(StreamId::B),
        other => Err(Response::failure(
            request_id,
            WireError {
                code: WireErrorCode::InvalidData,
                message: format!("unknown stream id: {other} (expected \"A\" or \"B\")"),
                offset: None,
            },
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
        Event::SelectionUpdated { stream } => {
            serde_json::json!({"type": "SelectionUpdated", "stream": format!("{stream:?}")})
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
            serde_json::json!({"type": "DiagnosticAdded", "diagnostic": format!("{diagnostic:?}")})
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
