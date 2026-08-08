//! bitvue-sidecar: standalone process hosting the bitvue engine, speaking `bitvue-protocol`
//! over stdio to `bitvue-desktop` (Electron main). See `docs/DEVELOPMENT_PHASES.md`
//! ("sidecar 결정" / "bitvue-protocol wire schema v0" / "동시성 모델") for the full design.
//!
//! Five real commands are wired end to end to `bitvue_core::Core`: `open_stream`,
//! `select_frame`, `close_stream` (control-plane), `get_hex_range` (the first data-plane
//! command — a `Control` metadata frame followed by a `Data` frame of raw bytes, no JSON array,
//! no base64), and `cancel_request` (see "Concurrency model" below). Everything else still
//! returns `WireErrorCode::Internal` "not implemented".
//!
//! # Concurrency model
//!
//! The reader loop (main thread) never blocks on request *handling* — it parses each incoming
//! `Control` frame and spawns a plain `std::thread` to compute the response, then immediately
//! goes back to reading the next frame. This means a slow request (once a genuinely slow command
//! exists — nothing today is slow enough to matter) can't block `hello`/`select_frame`/etc.
//! arriving concurrently. Chose OS threads over an async runtime (tokio) deliberately:
//! `bitvue_core::Core`'s work is CPU-bound synchronous Rust, not I/O-bound waiting, so threads are
//! the simpler fit — pulling in an async runtime would just mean wrapping every `Core` call in
//! `spawn_blocking` anyway.
//!
//! Response computation (`compute_frames`) is pure — no I/O, doesn't touch the writer — so it
//! runs fully unlocked/in parallel across threads. Only the final write is serialized (one
//! `Mutex<Stdout>` lock per request, held just long enough to write that request's frame(s));
//! locking around *computation* would silently re-serialize everything and defeat the point.
//!
//! `cancel_request` sets a per-request `AtomicBool` flag in a shared registry
//! (`correlation_id` → flag). A worker thread checks its own flag exactly once, immediately
//! before running its handler. **This is best-effort, not preemption**: none of today's handlers
//! have a cooperative checkpoint mid-execution (they're all fast synchronous calls — a file mmap,
//! a selection-state write, one `ByteCache::read_range`), so cancelling a request that has already
//! started executing has no effect; it only works for the (currently narrow, timing-dependent)
//! window before the worker thread's check runs. Once a genuinely slow command exists, it will
//! need to add its own checkpoints against the flag — this mechanism doesn't do that for free.
//!
//! `Command`/`Event` (bitvue-core) don't derive `Serialize`/`Deserialize` — they're the
//! internal UI↔Core bus, not a wire contract. Params/results for the commands above are
//! hand-mapped JSON shapes here rather than a derive, matching the same decoupling reasoning
//! already applied to `WireErrorCode` vs `BitvueError` in `bitvue-protocol`.
//!
//! `stdin`/`stdout` carry only protocol frames; all diagnostics go to `stderr` so a crash here
//! stays readable from the Electron main process's captured logs.

use std::collections::HashMap;
use std::io::{self, Read, Write};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;

use bitvue_core::{BitvueError, Command, Core, Event, FrameKey, StreamId};
use bitvue_protocol::{
    CancelParams, FrameHeader, FrameKind, HelloParams, HelloResult, Request, Response, WireError,
    WireErrorCode, FRAME_HEADER_LEN, PROTOCOL_VERSION,
};

/// `correlation_id` → cancellation flag, for requests currently being computed on a worker
/// thread. Entries are removed once that thread finishes (success, failure, or cancellation).
type CancelRegistry = Arc<Mutex<HashMap<u32, Arc<AtomicBool>>>>;

fn main() {
    let core = Arc::new(Core::new());
    let writer = Arc::new(Mutex::new(io::stdout()));
    let registry: CancelRegistry = Arc::new(Mutex::new(HashMap::new()));
    // In-flight worker handles. Rust does NOT wait for detached `thread::spawn`ed threads when
    // `main()` returns — a request whose worker hasn't finished writing yet when stdin closes
    // would silently lose its response otherwise. Joined below, right before exit. Pruned
    // opportunistically (not just at shutdown) so this doesn't grow unbounded over a long-lived
    // process handling many requests.
    let mut handles: Vec<thread::JoinHandle<()>> = Vec::new();

    let stdin = io::stdin();
    let mut reader = stdin.lock();

    loop {
        match read_frame(&mut reader) {
            Ok(Some((header, payload))) => {
                if header.kind != FrameKind::Control {
                    eprintln!(
                        "bitvue-sidecar: unexpected {:?} frame from main, ignoring",
                        header.kind
                    );
                    continue;
                }
                let request: Request = match serde_json::from_slice(&payload) {
                    Ok(r) => r,
                    Err(err) => {
                        eprintln!("bitvue-sidecar: malformed control frame (dropped): {err}");
                        continue;
                    }
                };

                if request.method == "cancel_request" {
                    let response = compute_cancel_response(&registry, &request);
                    let body = serde_json::to_vec(&response).expect("Response always serializes");
                    let mut guard = writer.lock().unwrap();
                    if let Err(err) = write_frame(
                        &mut *guard,
                        FrameKind::Control,
                        header.correlation_id,
                        &body,
                    ) {
                        eprintln!("bitvue-sidecar: error writing cancel_request response: {err}");
                    }
                    continue;
                }

                handles.retain(|h| !h.is_finished());
                handles.push(spawn_request(
                    Arc::clone(&core),
                    Arc::clone(&writer),
                    Arc::clone(&registry),
                    header.correlation_id,
                    request,
                ));
            }
            // stdin closed — Electron main exited or is shutting the sidecar down.
            Ok(None) => break,
            Err(err) => {
                eprintln!("bitvue-sidecar: frame read error: {err}");
                break;
            }
        }
    }

    // Let every in-flight request finish and write its response before the process exits —
    // otherwise a request accepted right before shutdown would silently lose its response.
    for handle in handles {
        let _ = handle.join();
    }
}

/// Registers a cancel flag for `correlation_id`, then spawns a worker thread that computes the
/// response (unlocked — no I/O, no shared-writer contention while it runs) and writes whatever
/// frames the computation produces under a brief writer-lock. See module doc for the concurrency
/// model and its cancellation limitations.
fn spawn_request(
    core: Arc<Core>,
    writer: Arc<Mutex<io::Stdout>>,
    registry: CancelRegistry,
    correlation_id: u32,
    request: Request,
) -> thread::JoinHandle<()> {
    let cancel_flag = Arc::new(AtomicBool::new(false));
    registry
        .lock()
        .unwrap()
        .insert(correlation_id, Arc::clone(&cancel_flag));

    thread::spawn(move || {
        let frames = if cancel_flag.load(Ordering::SeqCst) {
            let response = Response::failure(
                request.id,
                WireError {
                    code: WireErrorCode::Cancelled,
                    message: "cancelled before execution started".to_string(),
                    offset: None,
                },
            );
            vec![(
                FrameKind::Control,
                serde_json::to_vec(&response).expect("Response always serializes"),
            )]
        } else {
            compute_frames(&core, &request)
        };

        registry.lock().unwrap().remove(&correlation_id);

        let mut guard = writer.lock().unwrap();
        for (kind, payload) in frames {
            if let Err(err) = write_frame(&mut *guard, kind, correlation_id, &payload) {
                eprintln!(
                    "bitvue-sidecar: error writing response for {}: {err}",
                    request.method
                );
                break;
            }
        }
    })
}

/// Pure compute for `cancel_request` — sets the target's flag (if it's still registered) and
/// reports whether it was found. Kept separate from I/O for the same reason `compute_frames` is:
/// testable without touching a real writer, and consistent with how every other command works.
fn compute_cancel_response(registry: &CancelRegistry, request: &Request) -> Response {
    match serde_json::from_value::<CancelParams>(request.params.clone()) {
        Ok(params) => {
            let found = match registry.lock().unwrap().get(&params.target_id) {
                Some(flag) => {
                    flag.store(true, Ordering::SeqCst);
                    true
                }
                None => false,
            };
            Response::success(request.id, serde_json::json!({ "found": found }))
        }
        Err(err) => Response::failure(
            request.id,
            WireError {
                code: WireErrorCode::InvalidData,
                message: err.to_string(),
                offset: None,
            },
        ),
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

/// Pure compute: turns a request into the frame(s) that should be written for it. No I/O, no
/// locking — safe (and intended) to run concurrently across worker threads, which is the whole
/// point of the concurrency model above.
fn compute_frames(core: &Core, request: &Request) -> Vec<(FrameKind, Vec<u8>)> {
    if request.method == "get_hex_range" {
        return get_hex_range(core, request);
    }
    let response = dispatch(core, request);
    vec![(
        FrameKind::Control,
        serde_json::to_vec(&response).expect("Response always serializes"),
    )]
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
        "select_frame" => select_frame(core, request),
        "close_stream" => close_stream(core, request),
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

/// Parse `"A"`/`"B"` into `StreamId`, producing the same `InvalidData` wire error shape
/// every command that takes a `stream` param should use.
fn parse_stream_id(request_id: u32, s: &str) -> Result<StreamId, Response> {
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
struct SelectFrameParams {
    stream: String,
    frame_index: usize,
}

fn select_frame(core: &Core, request: &Request) -> Response {
    let params: SelectFrameParams = match serde_json::from_value(request.params.clone()) {
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

    let events = core.handle_command(Command::SelectFrame {
        stream,
        frame_key: FrameKey {
            stream,
            frame_index: params.frame_index,
            pts: None,
        },
    });
    let events_json: Vec<serde_json::Value> = events.iter().map(event_to_json).collect();
    Response::success(request.id, serde_json::json!({ "events": events_json }))
}

#[derive(serde::Deserialize)]
struct CloseStreamParams {
    stream: String,
}

fn close_stream(core: &Core, request: &Request) -> Response {
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

#[derive(serde::Deserialize)]
struct GetHexRangeParams {
    stream: String,
    offset: u64,
    len: usize,
}

/// First data-plane command: produces a `Control` metadata frame followed by a `Data` frame
/// carrying the raw bytes — no JSON array, no base64. Both frames share `correlation_id` so the
/// client can pair them without a nested envelope. Pure (no I/O) — see `compute_frames`.
fn get_hex_range(core: &Core, request: &Request) -> Vec<(FrameKind, Vec<u8>)> {
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

fn single_control_frame(response: Response) -> Vec<(FrameKind, Vec<u8>)> {
    vec![(
        FrameKind::Control,
        serde_json::to_vec(&response).expect("Response always serializes"),
    )]
}

/// Mirror of `bitvue_core::BitvueError` variants onto `WireErrorCode` — see the "not a direct
/// `Serialize` derive" note on `WireErrorCode` in `bitvue-protocol` for why this mapping lives
/// here instead of on the engine's error type.
fn wire_error_code_for(err: &BitvueError) -> WireErrorCode {
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

    /// Encode a list of (kind, payload) frames the same way the real writer does, for tests
    /// that assert on the exact byte stream `compute_frames`'s output would produce.
    fn encode_frames(correlation_id: u32, frames: &[(FrameKind, Vec<u8>)]) -> Vec<u8> {
        let mut out = Vec::new();
        for (kind, payload) in frames {
            write_frame(&mut out, *kind, correlation_id, payload).unwrap();
        }
        out
    }

    #[test]
    fn frame_read_write_roundtrips() {
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
        let parsed: Request = serde_json::from_slice(&payload).unwrap();
        assert_eq!(header.correlation_id, 1);
        assert_eq!(parsed.method, "hello");

        let core = Core::new();
        let frames = compute_frames(&core, &parsed);
        let output = encode_frames(header.correlation_id, &frames);

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

    #[test]
    fn select_frame_emits_selection_updated() {
        let core = Core::new();
        let request = Request {
            id: 10,
            method: "select_frame".to_string(),
            params: serde_json::json!({"stream": "A", "frame_index": 42}),
        };
        let response = dispatch(&core, &request);
        assert!(response.ok, "expected ok response, got {response:?}");
        let events = response.result.unwrap()["events"]
            .as_array()
            .unwrap()
            .clone();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0]["type"], "SelectionUpdated");
        assert_eq!(events[0]["stream"], "A");
    }

    #[test]
    fn select_frame_unknown_stream_id_is_invalid_data() {
        let core = Core::new();
        let request = Request {
            id: 11,
            method: "select_frame".to_string(),
            params: serde_json::json!({"stream": "Z", "frame_index": 0}),
        };
        let response = dispatch(&core, &request);
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
        let response = dispatch(&core, &request);
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
        let response = dispatch(&core, &request);
        assert!(!response.ok);
        assert_eq!(response.error.unwrap().code, WireErrorCode::InvalidData);
    }

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
        let open_response = dispatch(
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
        let open_response = dispatch(
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
    fn cancel_request_reports_not_found_for_unknown_target() {
        let registry: CancelRegistry = Arc::new(Mutex::new(HashMap::new()));
        let request = Request {
            id: 99,
            method: "cancel_request".to_string(),
            params: serde_json::to_value(CancelParams { target_id: 12345 }).unwrap(),
        };
        let response = compute_cancel_response(&registry, &request);
        assert!(response.ok);
        assert_eq!(
            response.result.unwrap(),
            serde_json::json!({"found": false})
        );
    }

    #[test]
    fn cancel_request_reports_found_and_sets_flag_for_registered_target() {
        let registry: CancelRegistry = Arc::new(Mutex::new(HashMap::new()));
        let flag = Arc::new(AtomicBool::new(false));
        registry.lock().unwrap().insert(42, Arc::clone(&flag));

        let request = Request {
            id: 100,
            method: "cancel_request".to_string(),
            params: serde_json::to_value(CancelParams { target_id: 42 }).unwrap(),
        };
        let response = compute_cancel_response(&registry, &request);
        assert!(response.ok);
        assert_eq!(response.result.unwrap(), serde_json::json!({"found": true}));
        assert!(flag.load(Ordering::SeqCst));
    }

    #[test]
    fn cancel_request_malformed_params_is_invalid_data() {
        let registry: CancelRegistry = Arc::new(Mutex::new(HashMap::new()));
        let request = Request {
            id: 101,
            method: "cancel_request".to_string(),
            params: serde_json::json!({"wrong_field": true}),
        };
        let response = compute_cancel_response(&registry, &request);
        assert!(!response.ok);
        assert_eq!(response.error.unwrap().code, WireErrorCode::InvalidData);
    }

    /// Real multi-threaded stress test: spawns worker threads the same way `spawn_request` does
    /// (via `compute_frames` against a shared `Arc<Core>`, not by calling functions sequentially
    /// on one thread) and asserts no panics/deadlocks and every response correlates correctly.
    /// This is what actually proves the concurrency model is thread-safe, not just structurally
    /// plausible — timing-based proof that a slow request can't block a fast one isn't included
    /// because no command today is slow enough to need it (see module doc).
    #[test]
    fn concurrent_requests_on_shared_core_do_not_panic_or_corrupt_state() {
        let core = Arc::new(Core::new());
        let mut handles = Vec::new();

        for i in 0..16u32 {
            let core = Arc::clone(&core);
            handles.push(thread::spawn(move || {
                let stream = if i % 2 == 0 { "A" } else { "B" };
                let request = Request {
                    id: i,
                    method: "select_frame".to_string(),
                    params: serde_json::json!({"stream": stream, "frame_index": i as usize}),
                };
                let frames = compute_frames(&core, &request);
                assert_eq!(frames.len(), 1);
                let (kind, payload) = &frames[0];
                assert_eq!(*kind, FrameKind::Control);
                let response: Response = serde_json::from_slice(payload).unwrap();
                assert!(response.ok, "request {i} failed: {response:?}");
                response.id
            }));
        }

        let mut seen_ids: Vec<u32> = handles.into_iter().map(|h| h.join().unwrap()).collect();
        seen_ids.sort_unstable();
        assert_eq!(seen_ids, (0..16u32).collect::<Vec<_>>());
    }
}
