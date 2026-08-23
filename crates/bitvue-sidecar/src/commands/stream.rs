//! `open_stream`/`close_stream` -- thin wrappers over `Core::handle_command`'s
//! `Command::OpenFile`/`Command::CloseFile`, returning the resulting events as-is. Split out of
//! `main.rs` (2026-08-19) as part of an SRP pass.

use crate::command_support::{event_to_json, parse_stream_id};
use bitvue_engine::{Command, Core};
use bitvue_protocol::{Request, Response, WireError, WireErrorCode};
use std::path::PathBuf;

#[derive(serde::Deserialize)]
struct OpenStreamParams {
    stream: String,
    path: String,
}

pub fn open_stream(core: &Core, request: &Request) -> Response {
    let params: OpenStreamParams = match serde_json::from_value(request.params.clone()) {
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
    let stream = match parse_stream_id(request.id, &params.stream) {
        Ok(s) => s,
        Err(response) => return response,
    };

    let events = core.handle_command(Command::OpenFile {
        stream,
        path: PathBuf::from(params.path),
    });
    let events_json: Vec<serde_json::Value> = events.iter().map(event_to_json).collect();
    Response::success(request.id, serde_json::json!({ "events": events_json }))
}

#[derive(serde::Deserialize)]
struct CloseStreamParams {
    stream: String,
}

pub fn close_stream(core: &Core, request: &Request) -> Response {
    let params: CloseStreamParams = match serde_json::from_value(request.params.clone()) {
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
    let stream = match parse_stream_id(request.id, &params.stream) {
        Ok(s) => s,
        Err(response) => return response,
    };

    let events = core.handle_command(Command::CloseFile { stream });
    let events_json: Vec<serde_json::Value> = events.iter().map(event_to_json).collect();
    Response::success(request.id, serde_json::json!({ "events": events_json }))
}

#[cfg(test)]
mod tests {
    use bitvue_engine::Core;
    use bitvue_protocol::{Request, WireErrorCode};
    use std::io::Write;

    #[test]
    fn open_stream_success_emits_model_updated() {
        let mut file = tempfile::NamedTempFile::new().unwrap();
        file.write_all(b"not a real bitstream, just needs to exist")
            .unwrap();

        let core = Core::new();
        let request = Request {
            id: 3,
            method: "open_stream".to_string(),
            params: serde_json::json!({"stream": "A", "path": file.path().to_str().unwrap()}),
        };
        let response = crate::dispatch(&core, &request);
        assert!(response.ok, "expected ok response, got {response:?}");
        let events = response.result.unwrap()["events"]
            .as_array()
            .unwrap()
            .clone();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0]["type"], "ModelUpdated");
        assert_eq!(events[0]["stream"], "A");
    }

    #[test]
    fn open_stream_missing_file_emits_diagnostic_not_protocol_error() {
        // Core::handle_command never fails the RPC itself — file-open errors surface as a
        // Event::DiagnosticAdded (severity Error), same as the UI would see it. `ok: true` here
        // is correct, not a bug: the wire call succeeded, the *domain outcome* is an error event.
        let core = Core::new();
        let request = Request {
            id: 4,
            method: "open_stream".to_string(),
            params: serde_json::json!({"stream": "A", "path": "/nonexistent/path/does-not-exist.ivf"}),
        };
        let response = crate::dispatch(&core, &request);
        assert!(response.ok);
        let events = response.result.unwrap()["events"]
            .as_array()
            .unwrap()
            .clone();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0]["type"], "DiagnosticAdded");
        assert_eq!(events[0]["diagnostic"]["severity"], "Error");
    }

    #[test]
    fn open_stream_unknown_stream_id_is_invalid_data() {
        let core = Core::new();
        let request = Request {
            id: 5,
            method: "open_stream".to_string(),
            params: serde_json::json!({"stream": "C", "path": "irrelevant"}),
        };
        let response = crate::dispatch(&core, &request);
        assert!(!response.ok);
        assert_eq!(response.error.unwrap().code, WireErrorCode::InvalidData);
    }

    #[test]
    fn close_stream_emits_model_updated() {
        let core = Core::new();
        let request = Request {
            id: 12,
            method: "close_stream".to_string(),
            params: serde_json::json!({"stream": "B"}),
        };
        let response = crate::dispatch(&core, &request);
        assert!(response.ok, "expected ok response, got {response:?}");
        let events = response.result.unwrap()["events"]
            .as_array()
            .unwrap()
            .clone();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0]["type"], "ModelUpdated");
        assert_eq!(events[0]["stream"], "B");
    }

    #[test]
    fn close_stream_unknown_stream_id_is_invalid_data() {
        let core = Core::new();
        let request = Request {
            id: 13,
            method: "close_stream".to_string(),
            params: serde_json::json!({"stream": "Z"}),
        };
        let response = crate::dispatch(&core, &request);
        assert!(!response.ok);
        assert_eq!(response.error.unwrap().code, WireErrorCode::InvalidData);
    }
}
