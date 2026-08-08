//! bitvue-sidecar: standalone process hosting the bitvue engine, speaking `bitvue-protocol`
//! over stdio to `bitvue-desktop` (Electron main). See `docs/DEVELOPMENT_PHASES.md`
//! ("sidecar 결정" / "bitvue-protocol wire schema v0") for the full design.
//!
//! One real command (`open_stream`) is wired end to end to `bitvue_core::Core` to prove the
//! bridge — everything else still returns `WireErrorCode::Internal` "not implemented".
//!
//! `Command`/`Event` (bitvue-core) don't derive `Serialize`/`Deserialize` — they're the
//! internal UI↔Core bus, not a wire contract. `open_stream`'s params/result are hand-mapped
//! JSON shapes here rather than a derive, matching the same decoupling reasoning already
//! applied to `WireErrorCode` vs `BitvueError` in `bitvue-protocol`.
//!
//! `stdin`/`stdout` carry only protocol frames; all diagnostics go to `stderr` so a crash here
//! stays readable from the Electron main process's captured logs.

use std::io::{self, Read, Write};
use std::path::PathBuf;

use bitvue_core::{Command, Core, Event, StreamId};
use bitvue_protocol::{
    FrameHeader, FrameKind, HelloParams, HelloResult, Request, Response, WireError, WireErrorCode,
    FRAME_HEADER_LEN, PROTOCOL_VERSION,
};

fn main() {
    let core = Core::new();
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut reader = stdin.lock();
    let mut writer = stdout.lock();

    loop {
        match read_frame(&mut reader) {
            Ok(Some((header, payload))) => {
                if let Err(err) = handle_frame(&core, header, &payload, &mut writer) {
                    eprintln!("bitvue-sidecar: error writing response: {err}");
                }
            }
            // stdin closed — Electron main exited or is shutting the sidecar down.
            Ok(None) => break,
            Err(err) => {
                eprintln!("bitvue-sidecar: frame read error: {err}");
                break;
            }
        }
    }
}

fn read_frame<R: Read>(reader: &mut R) -> io::Result<Option<(FrameHeader, Vec<u8>)>> {
    let mut header_buf = [0u8; FRAME_HEADER_LEN];
    match reader.read_exact(&mut header_buf) {
        Ok(()) => {}
        Err(err) if err.kind() == io::ErrorKind::UnexpectedEof => return Ok(None),
        Err(err) => return Err(err),
    }
    let header = FrameHeader::decode(&header_buf)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))?;
    let mut payload = vec![0u8; header.payload_len as usize];
    reader.read_exact(&mut payload)?;
    Ok(Some((header, payload)))
}

fn write_frame<W: Write>(
    writer: &mut W,
    kind: FrameKind,
    correlation_id: u32,
    payload: &[u8],
) -> io::Result<()> {
    let header = FrameHeader {
        kind,
        correlation_id,
        payload_len: payload.len() as u32,
    };
    writer.write_all(&header.encode())?;
    writer.write_all(payload)?;
    writer.flush()
}

fn handle_frame<W: Write>(
    core: &Core,
    header: FrameHeader,
    payload: &[u8],
    writer: &mut W,
) -> io::Result<()> {
    match header.kind {
        FrameKind::Control => {
            let request: Request = match serde_json::from_slice(payload) {
                Ok(r) => r,
                Err(err) => {
                    eprintln!("bitvue-sidecar: malformed control frame (dropped): {err}");
                    return Ok(());
                }
            };
            let response = dispatch(core, &request);
            let body = serde_json::to_vec(&response).expect("Response always serializes");
            write_frame(writer, FrameKind::Control, header.correlation_id, &body)
        }
        FrameKind::Data | FrameKind::Event => {
            eprintln!(
                "bitvue-sidecar: unexpected {:?} frame from main, ignoring",
                header.kind
            );
            Ok(())
        }
    }
}

fn dispatch(core: &Core, request: &Request) -> Response {
    match request.method.as_str() {
        "hello" => match serde_json::from_value::<HelloParams>(request.params.clone()) {
            Ok(_params) => {
                let result = HelloResult {
                    protocol_version: PROTOCOL_VERSION.to_string(),
                    capabilities: vec![],
                };
                Response::success(
                    request.id,
                    serde_json::to_value(result).expect("HelloResult always serializes"),
                )
            }
            Err(err) => Response::failure(
                request.id,
                WireError {
                    code: WireErrorCode::InvalidData,
                    message: err.to_string(),
                    offset: None,
                },
            ),
        },
        "open_stream" => open_stream(core, request),
        other => Response::failure(
            request.id,
            WireError {
                code: WireErrorCode::Internal,
                message: format!("method not implemented yet: {other}"),
                offset: None,
            },
        ),
    }
}

#[derive(serde::Deserialize)]
struct OpenStreamParams {
    stream: String,
    path: String,
}

fn open_stream(core: &Core, request: &Request) -> Response {
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
    let stream = match params.stream.as_str() {
        "A" => StreamId::A,
        "B" => StreamId::B,
        other => {
            return Response::failure(
                request.id,
                WireError {
                    code: WireErrorCode::InvalidData,
                    message: format!("unknown stream id: {other} (expected \"A\" or \"B\")"),
                    offset: None,
                },
            )
        }
    };

    let events = core.handle_command(Command::OpenFile {
        stream,
        path: PathBuf::from(params.path),
    });
    let events_json: Vec<serde_json::Value> = events.iter().map(event_to_json).collect();
    Response::success(request.id, serde_json::json!({ "events": events_json }))
}

/// Manual `Event` → JSON mapping (see module doc — `Event` isn't `Serialize`).
fn event_to_json(event: &Event) -> serde_json::Value {
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn hello_handshake_roundtrips() {
        let request = Request {
            id: 1,
            method: "hello".to_string(),
            params: serde_json::to_value(HelloParams {
                client_version: "0.1.0-test".to_string(),
            })
            .unwrap(),
        };
        let body = serde_json::to_vec(&request).unwrap();
        let mut input = Vec::new();
        input.extend_from_slice(
            &FrameHeader {
                kind: FrameKind::Control,
                correlation_id: 1,
                payload_len: body.len() as u32,
            }
            .encode(),
        );
        input.extend_from_slice(&body);

        let mut reader = Cursor::new(input);
        let (header, payload) = read_frame(&mut reader).unwrap().unwrap();

        let core = Core::new();
        let mut output = Vec::new();
        handle_frame(&core, header, &payload, &mut output).unwrap();

        let out_header =
            FrameHeader::decode(output[..FRAME_HEADER_LEN].try_into().unwrap()).unwrap();
        assert_eq!(out_header.correlation_id, 1);
        let response: Response = serde_json::from_slice(&output[FRAME_HEADER_LEN..]).unwrap();
        assert!(response.ok);
        let result: HelloResult = serde_json::from_value(response.result.unwrap()).unwrap();
        assert_eq!(result.protocol_version, PROTOCOL_VERSION);
    }

    #[test]
    fn unknown_method_returns_internal_error() {
        let core = Core::new();
        let request = Request {
            id: 7,
            method: "get_frame_analysis".to_string(),
            params: serde_json::json!({}),
        };
        let response = dispatch(&core, &request);
        assert!(!response.ok);
        assert_eq!(response.error.unwrap().code, WireErrorCode::Internal);
    }

    #[test]
    fn stdin_close_yields_none() {
        let mut reader = Cursor::new(Vec::<u8>::new());
        assert!(read_frame(&mut reader).unwrap().is_none());
    }

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
        let response = dispatch(&core, &request);
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
        let response = dispatch(&core, &request);
        assert!(response.ok);
        let events = response.result.unwrap()["events"]
            .as_array()
            .unwrap()
            .clone();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0]["type"], "DiagnosticAdded");
        assert!(events[0]["diagnostic"].as_str().unwrap().contains("Error"));
    }

    #[test]
    fn open_stream_unknown_stream_id_is_invalid_data() {
        let core = Core::new();
        let request = Request {
            id: 5,
            method: "open_stream".to_string(),
            params: serde_json::json!({"stream": "C", "path": "irrelevant"}),
        };
        let response = dispatch(&core, &request);
        assert!(!response.ok);
        assert_eq!(response.error.unwrap().code, WireErrorCode::InvalidData);
    }
}
