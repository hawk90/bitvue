//! bitvue-sidecar: standalone process hosting the bitvue engine, speaking `bitvue-protocol`
//! over stdio to `bitvue-desktop` (Electron main). See `docs/DEVELOPMENT_PHASES.md`
//! ("sidecar 결정" / "bitvue-protocol wire schema v0" / "동시성 모델") for the full design.
//!
//! Fourteen real commands are wired end to end: `open_stream`,
//! `select_frame`/`select_unit`/`select_syntax`/`select_bit_range`/`select_spatial_block`
//! (multi-sync — see `docs/DEVELOPMENT_PHASES.md`'s `SelectionState` note; these five map
//! straight onto `bitvue_engine::Command`'s existing "Tri-sync" selection variants, no new engine
//! work needed), `close_stream` (control-plane), `get_hex_range` (the first data-plane command —
//! a `Control` metadata frame followed by a `Data` frame of raw bytes, no JSON array, no base64),
//! `cancel_request` (see "Concurrency model" below), and five newer ones that go through
//! `bitvue-indexer` rather than `Core::handle_command`: `index_stream` (IVF/AV1 metadata
//! indexing — container + units, no pixel decode yet, see that crate's module doc for exactly
//! what's covered), two read-only pagination-style queries over the result,
//! `get_stream_info`/`get_frames_chunk`, `get_frame_syntax` (lazy, per-unit syntax tree — AV1
//! only, parsed on demand rather than eagerly for every unit), and `get_timeline` (display-order
//! timeline built from already-indexed units via `bitvue_engine::frame_identity::TimelineMapper`
//! — real, previously-unused engine code, not new logic; see `bitvue-indexer`'s module doc for why
//! this does NOT write to `StreamState.timeline`). These five are *not* `bitvue_engine::Command`
//! variants — `Core` is a leaf crate and can't call into codec/format crates itself (see
//! `docs/DEVELOPMENT_PHASES.md`'s decode-pipeline design note, 2026-08-08), so `bitvue-indexer`
//! sits on the other side of that dependency edge and mutates `StreamState` through `Core`'s
//! already-public `get_stream()`/`get_job_manager()` accessors. `Command::RunFullAnalysis` stays
//! defined-but-unused as a result — this sidecar deliberately doesn't route through it.
//!
//! **Every other `bitvue_engine::Command` variant is currently a no-op in `Core::handle_command`**
//! (falls through to its catch-all `_ => vec![]` arm — verified by reading `core.rs`, not
//! assumed) — `JumpToOffset`/`JumpToFrame`, `PlayPause`/`StepForward`/`StepBackward`,
//! `ToggleOverlay`/`SetOverlayOpacity`/`SetPlayerMode`, `SetWorkspaceMode`/`SetSyncMode`,
//! `ExportCsv`/`ExportBitstream`/`Export`, `RunFullAnalysis`. Wiring any of those to the sidecar
//! today would just proxy through to nothing — they need real `Core` implementation first (a
//! `bitvue-engine` engine task, not a sidecar wiring task). Everything else in this file's
//! method dispatch returns `WireErrorCode::Internal` "not implemented".
//!
//! # Concurrency model
//!
//! The reader loop (main thread) never blocks on request *handling* — it parses each incoming
//! `Control` frame and spawns a plain `std::thread` to compute the response, then immediately
//! goes back to reading the next frame. This means a slow request (once a genuinely slow command
//! exists — nothing today is slow enough to matter) can't block `hello`/`select_frame`/etc.
//! arriving concurrently. Chose OS threads over an async runtime (tokio) deliberately:
//! `bitvue_engine::Core`'s work is CPU-bound synchronous Rust, not I/O-bound waiting, so threads are
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
//! `Command`/`Event` (bitvue-engine) don't derive `Serialize`/`Deserialize` — they're the
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

mod av1_features;
mod codec_extended_info;
mod coding_flow;
mod context_menu;
mod deblocking;
mod debug_yuv;
mod decode_bridge;
mod evidence_export;
mod frame_analysis;
mod residual_analysis;

use bitvue_engine::{
    BitRange, BitvueError, Command, Core, Event, FrameKey, SpatialBlock, StreamId, UnitKey,
};
use bitvue_protocol::{
    CancelParams, FrameHeader, FrameKind, HelloParams, HelloResult, Request, Response, WireError,
    WireErrorCode, FRAME_HEADER_LEN, PROTOCOL_VERSION,
};

/// `correlation_id` → cancellation flag, for requests currently being computed on a worker
/// thread. Entries are removed once that thread finishes (success, failure, or cancellation).
type CancelRegistry = Arc<Mutex<HashMap<u32, Arc<AtomicBool>>>>;

/// The one currently-loaded debug YUV reference file (if any) -- see `debug_yuv`'s module doc for
/// why this lives here rather than in `Core`/`StreamState`. Not per-`correlation_id` like
/// `CancelRegistry`; genuinely one shared slot, same as the real VQ Analyzer "Load Debug YUV"
/// workflow only ever has one reference loaded at a time.
type DebugYuvSlot = Arc<Mutex<Option<debug_yuv::Session>>>;

fn main() {
    let core = Arc::new(Core::new());
    let debug_yuv_state: DebugYuvSlot = Arc::new(Mutex::new(None));
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
                    Arc::clone(&debug_yuv_state),
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
    debug_yuv_state: DebugYuvSlot,
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
            compute_frames(&core, &debug_yuv_state, &request)
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
fn compute_frames(
    core: &Core,
    debug_yuv_state: &DebugYuvSlot,
    request: &Request,
) -> Vec<(FrameKind, Vec<u8>)> {
    match request.method.as_str() {
        "get_hex_range" => return get_hex_range(core, request),
        "get_decoded_frame_yuv" => return get_decoded_frame_yuv(core, request),
        "get_debug_yuv_frame" => return get_debug_yuv_frame(core, debug_yuv_state, request),
        "load_debug_yuv" => return single_control_frame(load_debug_yuv(debug_yuv_state, request)),
        "unload_debug_yuv" => {
            return single_control_frame(unload_debug_yuv(debug_yuv_state, request))
        }
        "set_debug_yuv_offset" => {
            return single_control_frame(set_debug_yuv_offset(debug_yuv_state, request))
        }
        "set_debug_yuv_crop" => {
            return single_control_frame(set_debug_yuv_crop(debug_yuv_state, request))
        }
        "get_yuv_diff_metrics" => {
            return single_control_frame(get_yuv_diff_metrics(core, debug_yuv_state, request))
        }
        "find_first_diff_frame" => {
            return single_control_frame(find_first_diff_frame(core, debug_yuv_state, request))
        }
        _ => {}
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
        "select_unit" => select_unit(core, request),
        "select_syntax" => select_syntax(core, request),
        "select_bit_range" => select_bit_range(core, request),
        "select_spatial_block" => select_spatial_block(core, request),
        "close_stream" => close_stream(core, request),
        "index_stream" => index_stream(core, request),
        "get_stream_info" => get_stream_info(core, request),
        "get_frames_chunk" => get_frames_chunk(core, request),
        "get_frame_syntax" => get_frame_syntax(core, request),
        "get_timeline" => get_timeline(core, request),
        "get_thumbnails" => get_thumbnails(core, request),
        "get_frame_analysis" => get_frame_analysis(core, request),
        "get_av1_features" => get_av1_features(core, request),
        "get_coding_flow_analysis" => get_coding_flow_analysis(core, request),
        "get_deblocking_analysis" => get_deblocking_analysis(core, request),
        "get_codec_extended_info" => get_codec_extended_info(core, request),
        "get_residual_analysis" => get_residual_analysis(core, request),
        "get_context_menu_items" => context_menu::get_context_menu_items(request),
        "export_evidence_bundle" => evidence_export::export_evidence_bundle_command(core, request),
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
struct SelectUnitParams {
    stream: String,
    unit_type: String,
    offset: u64,
    size: usize,
}

/// Structural selection (multi-sync): a container-level unit (e.g. an OBU/NAL), independent of
/// `select_frame`'s temporal cursor. See `bitvue_engine::selection::UnitKey`.
fn select_unit(core: &Core, request: &Request) -> Response {
    let params: SelectUnitParams = match serde_json::from_value(request.params.clone()) {
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

    let events = core.handle_command(Command::SelectUnit {
        stream,
        unit_key: UnitKey {
            stream,
            unit_type: params.unit_type,
            offset: params.offset,
            size: params.size,
        },
    });
    let events_json: Vec<serde_json::Value> = events.iter().map(event_to_json).collect();
    Response::success(request.id, serde_json::json!({ "events": events_json }))
}

#[derive(serde::Deserialize)]
struct SelectSyntaxParams {
    stream: String,
    node_id: String,
    start_bit: u64,
    end_bit: u64,
}

/// Structural selection (multi-sync): a syntax tree node + its bit range — the syntax
/// tree ↔ hex direction of tri-sync. See `bitvue_engine::selection::SyntaxNodeId`/`BitRange`.
fn select_syntax(core: &Core, request: &Request) -> Response {
    let params: SelectSyntaxParams = match serde_json::from_value(request.params.clone()) {
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

    let events = core.handle_command(Command::SelectSyntax {
        stream,
        node_id: params.node_id,
        bit_range: BitRange {
            start_bit: params.start_bit,
            end_bit: params.end_bit,
        },
    });
    let events_json: Vec<serde_json::Value> = events.iter().map(event_to_json).collect();
    Response::success(request.id, serde_json::json!({ "events": events_json }))
}

#[derive(serde::Deserialize)]
struct SelectBitRangeParams {
    stream: String,
    start_bit: u64,
    end_bit: u64,
}

/// Structural selection (multi-sync): the hex ↔ syntax tree direction — Core finds the nearest
/// containing syntax node for this bit range itself (see `Command::SelectBitRange` handling in
/// `bitvue_engine::Core::handle_command`), so this handler doesn't need to do that mapping.
fn select_bit_range(core: &Core, request: &Request) -> Response {
    let params: SelectBitRangeParams = match serde_json::from_value(request.params.clone()) {
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

    let events = core.handle_command(Command::SelectBitRange {
        stream,
        bit_range: BitRange {
            start_bit: params.start_bit,
            end_bit: params.end_bit,
        },
    });
    let events_json: Vec<serde_json::Value> = events.iter().map(event_to_json).collect();
    Response::success(request.id, serde_json::json!({ "events": events_json }))
}

#[derive(serde::Deserialize)]
struct SelectSpatialBlockParams {
    stream: String,
    x: u32,
    y: u32,
    w: u32,
    h: u32,
}

/// Structural selection (multi-sync): a spatial block in the current frame — the QP/MV overlay
/// click-to-select direction. `Core` resolves the frame index from the current cursor itself
/// (defaulting to 0 if nothing is selected yet), so this handler doesn't pass one explicitly.
fn select_spatial_block(core: &Core, request: &Request) -> Response {
    let params: SelectSpatialBlockParams = match serde_json::from_value(request.params.clone()) {
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

    let events = core.handle_command(Command::SelectSpatialBlock {
        stream,
        block: SpatialBlock {
            x: params.x,
            y: params.y,
            w: params.w,
            h: params.h,
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
struct IndexStreamParams {
    stream: String,
}

/// Runs `bitvue_indexer::index_stream` -- IVF/AV1 metadata indexing (container + units, no pixel
/// decode). See that crate's module doc for exactly what is and isn't populated. Not a
/// `Core::handle_command` variant: `Command::RunFullAnalysis` stays unused, this bypasses it
/// entirely by calling the indexer directly against `Core::get_stream()`/`get_job_manager()`.
fn index_stream(core: &Core, request: &Request) -> Response {
    let params: IndexStreamParams = match serde_json::from_value(request.params.clone()) {
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

    let events = bitvue_indexer::index_stream(core, stream);
    let events_json: Vec<serde_json::Value> = events.iter().map(event_to_json).collect();
    Response::success(request.id, serde_json::json!({ "events": events_json }))
}

fn container_model_to_json(container: &bitvue_engine::ContainerModel) -> serde_json::Value {
    serde_json::json!({
        "format": format!("{:?}", container.format),
        "codec": container.codec,
        "track_count": container.track_count,
        "duration_ms": container.duration_ms,
        "bitrate_bps": container.bitrate_bps,
        "width": container.width,
        "height": container.height,
        "bit_depth": container.bit_depth,
    })
}

#[derive(serde::Deserialize)]
struct GetStreamInfoParams {
    stream: String,
}

/// Read-only query: returns the `ContainerModel` populated by `index_stream`, if any.
/// `{"indexed": false, "container": null}` (not a wire error) when nothing's been indexed yet --
/// "not indexed" is a normal, expected state for a freshly-opened stream, not a failure.
fn get_stream_info(core: &Core, request: &Request) -> Response {
    let params: GetStreamInfoParams = match serde_json::from_value(request.params.clone()) {
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

    let stream_state = core.get_stream(stream);
    let state = stream_state.read();
    match &state.container {
        Some(container) => Response::success(
            request.id,
            serde_json::json!({ "indexed": true, "container": container_model_to_json(container) }),
        ),
        None => Response::success(
            request.id,
            serde_json::json!({ "indexed": false, "container": null }),
        ),
    }
}

#[derive(serde::Deserialize)]
struct GetFramesChunkParams {
    stream: String,
    offset: usize,
    limit: usize,
}

/// Read-only, paginated query over `UnitModel.units` (populated by `index_stream`). `UnitNode`
/// already derives `Serialize` (unlike most `bitvue-engine` types -- see this file's module doc),
/// so units serialize directly, no hand-mapping needed.
fn get_frames_chunk(core: &Core, request: &Request) -> Response {
    let params: GetFramesChunkParams = match serde_json::from_value(request.params.clone()) {
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

    let stream_state = core.get_stream(stream);
    let state = stream_state.read();
    match &state.units {
        Some(unit_model) => {
            let end = (params.offset + params.limit).min(unit_model.units.len());
            let slice = if params.offset < unit_model.units.len() {
                &unit_model.units[params.offset..end]
            } else {
                &[]
            };
            Response::success(
                request.id,
                serde_json::json!({
                    "indexed": true,
                    "units": serde_json::to_value(slice).expect("UnitNode always serializes"),
                    "total_count": unit_model.unit_count,
                }),
            )
        }
        None => Response::success(
            request.id,
            serde_json::json!({ "indexed": false, "units": [], "total_count": 0 }),
        ),
    }
}

/// Converts `bitvue_engine::SyntaxModel`'s flat `HashMap<SyntaxNodeId, SyntaxNode>` + `root_id`
/// into a nested tree, recursing from the root -- `SyntaxNode` doesn't derive `Serialize` (see
/// this file's module doc on `bitvue-engine` types generally not being wire types), and the
/// frontend's `SyntaxNode` shape (`{type, name, children, ...}`) expects nesting, not a flat map.
fn syntax_node_to_json(model: &bitvue_engine::SyntaxModel, node_id: &str) -> serde_json::Value {
    let Some(node) = model.nodes.get(node_id) else {
        return serde_json::Value::Null;
    };
    let children: Vec<serde_json::Value> = node
        .children
        .iter()
        .map(|child_id| syntax_node_to_json(model, child_id))
        .collect();
    serde_json::json!({
        "type": node.field_name,
        "name": node.field_name,
        "value": node.value,
        "bit_range": { "start_bit": node.bit_range.start_bit, "end_bit": node.bit_range.end_bit },
        "children": children,
    })
}

#[derive(serde::Deserialize)]
struct GetFrameSyntaxParams {
    stream: String,
    frame_index: usize,
}

/// Lazy, per-unit syntax tree (AV1 only so far) via `bitvue_indexer::get_frame_syntax` -- not a
/// `Core::handle_command` variant, same reasoning as `index_stream`/`get_stream_info`/
/// `get_frames_chunk` (see this file's module doc). Unlike those, failure here IS a wire error
/// (`WireErrorCode::NotFound`/`InvalidData`) rather than an `{indexed: false}`-style payload --
/// there's no meaningful partial result for "this frame's syntax tree" the way there is for
/// "nothing indexed yet."
fn get_frame_syntax(core: &Core, request: &Request) -> Response {
    let params: GetFrameSyntaxParams = match serde_json::from_value(request.params.clone()) {
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

    match bitvue_indexer::get_frame_syntax(core, stream, params.frame_index) {
        Ok(model) => {
            let tree = syntax_node_to_json(&model, &model.root_id);
            Response::success(request.id, tree)
        }
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

#[derive(serde::Deserialize)]
struct GetTimelineParams {
    stream: String,
}

/// `bitvue_engine::timeline::TimelineBase` via `bitvue_indexer::get_timeline` -- not a
/// `Core::handle_command` variant, same reasoning as the other `bitvue-indexer`-backed commands.
/// `TimelineBase`/`TimelineFrame` already derive `Serialize` (unlike most `bitvue-engine` types),
/// so this serializes directly -- no hand-mapping function needed like `syntax_node_to_json`.
/// Failure is a real wire error, same as `get_frame_syntax` -- no meaningful partial timeline.
fn get_timeline(core: &Core, request: &Request) -> Response {
    let params: GetTimelineParams = match serde_json::from_value(request.params.clone()) {
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

    match bitvue_indexer::get_timeline(core, stream) {
        Ok(timeline) => Response::success(
            request.id,
            serde_json::to_value(&timeline).expect("TimelineBase always serializes"),
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

#[derive(serde::Deserialize)]
struct GetFrameAnalysisParams {
    frame_index: usize,
}

/// QP/MV/partition/prediction-mode/transform-size grids for one frame of stream A -- see
/// `frame_analysis`'s module doc. Control-only (unlike `get_decoded_frame_yuv`) -- these are
/// structured grids, not raw pixel bytes, so a single JSON response is the right shape, same as
/// `get_frame_syntax`/`get_timeline`.
fn get_frame_analysis(core: &Core, request: &Request) -> Response {
    let params: GetFrameAnalysisParams = match serde_json::from_value(request.params.clone()) {
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

    let stream_state = core.get_stream(StreamId::A);
    let state = stream_state.read();
    let byte_cache = match state.byte_cache.as_ref() {
        Some(cache) => std::sync::Arc::clone(cache),
        None => {
            return Response::failure(
                request.id,
                WireError {
                    code: WireErrorCode::NotFound,
                    message: "stream not open".to_string(),
                    offset: None,
                },
            )
        }
    };
    drop(state);

    let full_len = byte_cache.len() as usize;
    let data = match byte_cache.read_range(0, full_len) {
        Ok(bytes) => bytes,
        Err(err) => {
            return Response::failure(
                request.id,
                WireError {
                    code: wire_error_code_for(&err),
                    message: err.to_string(),
                    offset: None,
                },
            )
        }
    };

    match frame_analysis::get_frame_analysis(data, params.frame_index) {
        Ok(value) => Response::success(request.id, value),
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

/// CDEF/loop-restoration/film-grain/super-resolution data for one frame of stream A -- see
/// `av1_features`'s module doc. Control-only, same reasoning as `get_frame_analysis`.
fn get_av1_features(core: &Core, request: &Request) -> Response {
    let params: GetFrameAnalysisParams = match serde_json::from_value(request.params.clone()) {
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

    let stream_state = core.get_stream(StreamId::A);
    let state = stream_state.read();
    let byte_cache = match state.byte_cache.as_ref() {
        Some(cache) => std::sync::Arc::clone(cache),
        None => {
            return Response::failure(
                request.id,
                WireError {
                    code: WireErrorCode::NotFound,
                    message: "stream not open".to_string(),
                    offset: None,
                },
            )
        }
    };
    drop(state);

    let full_len = byte_cache.len() as usize;
    let data = match byte_cache.read_range(0, full_len) {
        Ok(bytes) => bytes,
        Err(err) => {
            return Response::failure(
                request.id,
                WireError {
                    code: wire_error_code_for(&err),
                    message: err.to_string(),
                    offset: None,
                },
            )
        }
    };

    match av1_features::get_av1_features(data, params.frame_index) {
        Ok(value) => Response::success(request.id, value),
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

/// Encoder/decoder pipeline stage completion + real codec features for one frame of stream A --
/// see `coding_flow`'s module doc. Control-only, same reasoning as `get_frame_analysis`.
fn get_coding_flow_analysis(core: &Core, request: &Request) -> Response {
    let params: GetFrameAnalysisParams = match serde_json::from_value(request.params.clone()) {
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

    let stream_state = core.get_stream(StreamId::A);
    let state = stream_state.read();
    let byte_cache = match state.byte_cache.as_ref() {
        Some(cache) => std::sync::Arc::clone(cache),
        None => {
            return Response::failure(
                request.id,
                WireError {
                    code: WireErrorCode::NotFound,
                    message: "stream not open".to_string(),
                    offset: None,
                },
            )
        }
    };
    drop(state);

    let full_len = byte_cache.len() as usize;
    let data = match byte_cache.read_range(0, full_len) {
        Ok(bytes) => bytes,
        Err(err) => {
            return Response::failure(
                request.id,
                WireError {
                    code: wire_error_code_for(&err),
                    message: err.to_string(),
                    offset: None,
                },
            )
        }
    };

    match coding_flow::get_coding_flow_analysis(data, params.frame_index) {
        Ok(value) => Response::success(request.id, value),
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

/// AV1 loop-filter boundary strength + real loop-filter parameters for one frame of stream A --
/// see `deblocking`'s module doc. Control-only, same reasoning as `get_frame_analysis`.
fn get_deblocking_analysis(core: &Core, request: &Request) -> Response {
    let params: GetFrameAnalysisParams = match serde_json::from_value(request.params.clone()) {
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

    let stream_state = core.get_stream(StreamId::A);
    let state = stream_state.read();
    let byte_cache = match state.byte_cache.as_ref() {
        Some(cache) => std::sync::Arc::clone(cache),
        None => {
            return Response::failure(
                request.id,
                WireError {
                    code: WireErrorCode::NotFound,
                    message: "stream not open".to_string(),
                    offset: None,
                },
            )
        }
    };
    drop(state);

    let full_len = byte_cache.len() as usize;
    let data = match byte_cache.read_range(0, full_len) {
        Ok(bytes) => bytes,
        Err(err) => {
            return Response::failure(
                request.id,
                WireError {
                    code: wire_error_code_for(&err),
                    message: err.to_string(),
                    offset: None,
                },
            )
        }
    };

    match deblocking::get_deblocking_analysis(data, params.frame_index) {
        Ok(value) => Response::success(request.id, value),
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

/// AV1 reference-frame lists (L0/L1) + QP histogram for one frame of stream A -- see
/// `codec_extended_info`'s module doc. Control-only, same reasoning as `get_frame_analysis`.
fn get_codec_extended_info(core: &Core, request: &Request) -> Response {
    let params: GetFrameAnalysisParams = match serde_json::from_value(request.params.clone()) {
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

    let stream_state = core.get_stream(StreamId::A);
    let state = stream_state.read();
    let byte_cache = match state.byte_cache.as_ref() {
        Some(cache) => std::sync::Arc::clone(cache),
        None => {
            return Response::failure(
                request.id,
                WireError {
                    code: WireErrorCode::NotFound,
                    message: "stream not open".to_string(),
                    offset: None,
                },
            )
        }
    };
    drop(state);

    let full_len = byte_cache.len() as usize;
    let data = match byte_cache.read_range(0, full_len) {
        Ok(bytes) => bytes,
        Err(err) => {
            return Response::failure(
                request.id,
                WireError {
                    code: wire_error_code_for(&err),
                    message: err.to_string(),
                    offset: None,
                },
            )
        }
    };

    match codec_extended_info::get_codec_extended_info(data, params.frame_index) {
        Ok(value) => Response::success(request.id, value),
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

/// Per-block residual coefficient magnitude statistics for one frame of stream A -- see
/// `residual_analysis`'s module doc. Control-only, same reasoning as `get_frame_analysis`.
fn get_residual_analysis(core: &Core, request: &Request) -> Response {
    let params: GetFrameAnalysisParams = match serde_json::from_value(request.params.clone()) {
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

    let stream_state = core.get_stream(StreamId::A);
    let state = stream_state.read();
    let byte_cache = match state.byte_cache.as_ref() {
        Some(cache) => std::sync::Arc::clone(cache),
        None => {
            return Response::failure(
                request.id,
                WireError {
                    code: WireErrorCode::NotFound,
                    message: "stream not open".to_string(),
                    offset: None,
                },
            )
        }
    };
    drop(state);

    let full_len = byte_cache.len() as usize;
    let data = match byte_cache.read_range(0, full_len) {
        Ok(bytes) => bytes,
        Err(err) => {
            return Response::failure(
                request.id,
                WireError {
                    code: wire_error_code_for(&err),
                    message: err.to_string(),
                    offset: None,
                },
            )
        }
    };

    match residual_analysis::get_residual_analysis(data, params.frame_index) {
        Ok(value) => Response::success(request.id, value),
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
fn get_decoded_frame_yuv(core: &Core, request: &Request) -> Vec<(FrameKind, Vec<u8>)> {
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

#[derive(serde::Deserialize)]
struct GetThumbnailsParams {
    stream: String,
    frame_indices: Vec<usize>,
    /// Defaults to `ThumbnailCache::default()`'s 120px (matches the frontend's
    /// `THUMBNAIL_SIZE.WIDTH`) when omitted.
    #[serde(default)]
    target_width: Option<u32>,
}

const DEFAULT_THUMBNAIL_WIDTH: u32 = 120;

/// Control-only (unlike `get_decoded_frame_yuv`/`get_hex_range`) -- thumbnails are small enough
/// that base64-in-JSON is a reasonable choice, and the frontend already expects a `data:` URL
/// string per thumbnail (feeds straight into an `<img src>`), not raw bytes. Business logic in
/// `decode_bridge::get_thumbnails` decodes once per batch, not once per requested index.
fn get_thumbnails(core: &Core, request: &Request) -> Response {
    let params: GetThumbnailsParams = match serde_json::from_value(request.params.clone()) {
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

    let stream_state = core.get_stream(stream);
    let state = stream_state.read();
    let byte_cache = match state.byte_cache.as_ref() {
        Some(cache) => std::sync::Arc::clone(cache),
        None => {
            return Response::failure(
                request.id,
                WireError {
                    code: WireErrorCode::NotFound,
                    message: "stream not open".to_string(),
                    offset: None,
                },
            )
        }
    };
    drop(state);

    let full_len = byte_cache.len() as usize;
    let data = match byte_cache.read_range(0, full_len) {
        Ok(bytes) => bytes,
        Err(err) => {
            return Response::failure(
                request.id,
                WireError {
                    code: wire_error_code_for(&err),
                    message: err.to_string(),
                    offset: None,
                },
            )
        }
    };

    let target_width = params.target_width.unwrap_or(DEFAULT_THUMBNAIL_WIDTH);
    match decode_bridge::get_thumbnails(data, &params.frame_indices, target_width) {
        Ok(results) => {
            let json_results: Vec<serde_json::Value> = results
                .into_iter()
                .map(|t| {
                    serde_json::json!({
                        "frame_index": t.frame_index,
                        "thumbnail_data": t.data_url,
                        "width": t.width,
                        "height": t.height,
                        "success": true,
                    })
                })
                .collect();
            Response::success(request.id, serde_json::json!(json_results))
        }
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

/// Failure to load isn't a wire-level error -- same "domain outcome vs RPC outcome" split
/// `open_stream`'s module-doc note describes: the call succeeded, `result.success` carries whether
/// the load itself worked, matching the frontend's `YuvDiffContext.loadFile` expectations.
fn load_debug_yuv(state: &DebugYuvSlot, request: &Request) -> Response {
    let params: debug_yuv::LoadParams = match serde_json::from_value(request.params.clone()) {
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
    match debug_yuv::load(params) {
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

fn unload_debug_yuv(state: &DebugYuvSlot, request: &Request) -> Response {
    *state.lock().unwrap() = None;
    Response::success(request.id, serde_json::json!({}))
}

fn no_debug_yuv_loaded(request_id: u32) -> Response {
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

fn set_debug_yuv_offset(state: &DebugYuvSlot, request: &Request) -> Response {
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
    crop: debug_yuv::Crop,
}

fn set_debug_yuv_crop(state: &DebugYuvSlot, request: &Request) -> Response {
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

fn get_yuv_diff_metrics(core: &Core, state: &DebugYuvSlot, request: &Request) -> Response {
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
    match debug_yuv::compute_frame_metrics(core, session, params.frame_index) {
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

fn find_first_diff_frame(core: &Core, state: &DebugYuvSlot, request: &Request) -> Response {
    let guard = state.lock().unwrap();
    let session = match guard.as_ref() {
        Some(s) => s,
        None => return no_debug_yuv_loaded(request.id),
    };
    match debug_yuv::find_first_diff(core, session) {
        Ok((frame_index, total_checked)) => Response::success(
            request.id,
            serde_json::json!({
                "frame_index": frame_index,
                "total_checked": total_checked,
            }),
        ),
        Err(message) => Response::failure(
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
/// see `debug_yuv::get_frame` for how each of the four display modes is produced.
fn get_debug_yuv_frame(
    core: &Core,
    state: &DebugYuvSlot,
    request: &Request,
) -> Vec<(FrameKind, Vec<u8>)> {
    let params: GetDebugYuvFrameParams = match serde_json::from_value(request.params.clone()) {
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
    let guard = state.lock().unwrap();
    let session = match guard.as_ref() {
        Some(s) => s,
        None => return single_control_frame(no_debug_yuv_loaded(request.id)),
    };
    match debug_yuv::get_frame(
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

fn single_control_frame(response: Response) -> Vec<(FrameKind, Vec<u8>)> {
    vec![(
        FrameKind::Control,
        serde_json::to_vec(&response).expect("Response always serializes"),
    )]
}

/// Mirror of `bitvue_engine::BitvueError` variants onto `WireErrorCode` — see the "not a direct
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
        let debug_yuv_state: DebugYuvSlot = Arc::new(Mutex::new(None));
        let frames = compute_frames(&core, &debug_yuv_state, &parsed);
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
            method: "create_compare_workspace".to_string(),
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
    fn select_unit_emits_selection_updated() {
        let core = Core::new();
        let request = Request {
            id: 15,
            method: "select_unit".to_string(),
            params: serde_json::json!({
                "stream": "A",
                "unit_type": "OBU_FRAME_HEADER",
                "offset": 128,
                "size": 16
            }),
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
    fn select_unit_unknown_stream_id_is_invalid_data() {
        let core = Core::new();
        let request = Request {
            id: 16,
            method: "select_unit".to_string(),
            params: serde_json::json!({"stream": "Z", "unit_type": "X", "offset": 0, "size": 0}),
        };
        let response = dispatch(&core, &request);
        assert!(!response.ok);
        assert_eq!(response.error.unwrap().code, WireErrorCode::InvalidData);
    }

    #[test]
    fn select_syntax_emits_selection_updated() {
        let core = Core::new();
        let request = Request {
            id: 17,
            method: "select_syntax".to_string(),
            params: serde_json::json!({
                "stream": "B",
                "node_id": "obu_header.obu_type",
                "start_bit": 0,
                "end_bit": 4
            }),
        };
        let response = dispatch(&core, &request);
        assert!(response.ok, "expected ok response, got {response:?}");
        let events = response.result.unwrap()["events"]
            .as_array()
            .unwrap()
            .clone();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0]["type"], "SelectionUpdated");
        assert_eq!(events[0]["stream"], "B");
    }

    #[test]
    fn select_syntax_unknown_stream_id_is_invalid_data() {
        let core = Core::new();
        let request = Request {
            id: 18,
            method: "select_syntax".to_string(),
            params: serde_json::json!({"stream": "Z", "node_id": "x", "start_bit": 0, "end_bit": 0}),
        };
        let response = dispatch(&core, &request);
        assert!(!response.ok);
        assert_eq!(response.error.unwrap().code, WireErrorCode::InvalidData);
    }

    #[test]
    fn select_bit_range_emits_selection_updated() {
        let core = Core::new();
        let request = Request {
            id: 19,
            method: "select_bit_range".to_string(),
            params: serde_json::json!({"stream": "A", "start_bit": 100, "end_bit": 200}),
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
    fn select_bit_range_unknown_stream_id_is_invalid_data() {
        let core = Core::new();
        let request = Request {
            id: 22,
            method: "select_bit_range".to_string(),
            params: serde_json::json!({"stream": "Z", "start_bit": 0, "end_bit": 0}),
        };
        let response = dispatch(&core, &request);
        assert!(!response.ok);
        assert_eq!(response.error.unwrap().code, WireErrorCode::InvalidData);
    }

    #[test]
    fn select_spatial_block_emits_selection_updated() {
        let core = Core::new();
        let request = Request {
            id: 23,
            method: "select_spatial_block".to_string(),
            params: serde_json::json!({"stream": "A", "x": 64, "y": 32, "w": 16, "h": 16}),
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
    fn select_spatial_block_unknown_stream_id_is_invalid_data() {
        let core = Core::new();
        let request = Request {
            id: 24,
            method: "select_spatial_block".to_string(),
            params: serde_json::json!({"stream": "Z", "x": 0, "y": 0, "w": 0, "h": 0}),
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
        let debug_yuv_state: DebugYuvSlot = Arc::new(Mutex::new(None));
        let mut handles = Vec::new();

        for i in 0..16u32 {
            let core = Arc::clone(&core);
            let debug_yuv_state = Arc::clone(&debug_yuv_state);
            handles.push(thread::spawn(move || {
                let stream = if i % 2 == 0 { "A" } else { "B" };
                let request = Request {
                    id: i,
                    method: "select_frame".to_string(),
                    params: serde_json::json!({"stream": stream, "frame_index": i as usize}),
                };
                let frames = compute_frames(&core, &debug_yuv_state, &request);
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

    // -- index_stream / get_stream_info / get_frames_chunk --------------------------------

    const AV1_IVF_FIXTURE: &[u8] = include_bytes!("../../../test_data/av1_test.ivf");

    /// Real fixture, not a fake file -- these commands need actual IVF/AV1 bytes to produce
    /// anything, unlike `open_stream`'s tests above which only need *a* file to exist.
    fn open_real_fixture(core: &Core, stream: &str) {
        let mut file = tempfile::NamedTempFile::new().unwrap();
        file.write_all(AV1_IVF_FIXTURE).unwrap();
        let request = Request {
            id: 100,
            method: "open_stream".to_string(),
            params: serde_json::json!({"stream": stream, "path": file.path().to_str().unwrap()}),
        };
        let response = dispatch(core, &request);
        assert!(response.ok, "expected real fixture to open: {response:?}");
        std::mem::forget(file); // keep the path alive for ByteCache, matches bitvue-indexer's tests
    }

    #[test]
    fn index_stream_end_to_end_populates_real_container_and_units() {
        let core = Core::new();
        open_real_fixture(&core, "A");

        let request = Request {
            id: 101,
            method: "index_stream".to_string(),
            params: serde_json::json!({"stream": "A"}),
        };
        let response = dispatch(&core, &request);
        assert!(response.ok, "expected ok response, got {response:?}");
        let events = response.result.unwrap()["events"]
            .as_array()
            .unwrap()
            .clone();
        assert_eq!(events.len(), 2);
        assert_eq!(events[0]["type"], "ModelUpdated");
        assert_eq!(events[0]["kind"], "Container");
        assert_eq!(events[1]["kind"], "Units");
    }

    #[test]
    fn get_stream_info_reflects_index_stream_result() {
        let core = Core::new();
        open_real_fixture(&core, "A");

        // Before indexing: honestly reports not-indexed, not a wire error.
        let before = dispatch(
            &core,
            &Request {
                id: 102,
                method: "get_stream_info".to_string(),
                params: serde_json::json!({"stream": "A"}),
            },
        );
        assert!(before.ok);
        assert_eq!(before.result.as_ref().unwrap()["indexed"], false);

        dispatch(
            &core,
            &Request {
                id: 103,
                method: "index_stream".to_string(),
                params: serde_json::json!({"stream": "A"}),
            },
        );

        let after = dispatch(
            &core,
            &Request {
                id: 104,
                method: "get_stream_info".to_string(),
                params: serde_json::json!({"stream": "A"}),
            },
        );
        assert!(after.ok);
        let result = after.result.unwrap();
        assert_eq!(result["indexed"], true);
        assert_eq!(result["container"]["codec"], "av1");
        assert_eq!(result["container"]["format"], "Ivf");
        assert!(result["container"]["width"].as_u64().unwrap() > 0);
    }

    #[test]
    fn get_frames_chunk_paginates_real_units() {
        let core = Core::new();
        open_real_fixture(&core, "A");
        dispatch(
            &core,
            &Request {
                id: 105,
                method: "index_stream".to_string(),
                params: serde_json::json!({"stream": "A"}),
            },
        );

        let chunk = dispatch(
            &core,
            &Request {
                id: 106,
                method: "get_frames_chunk".to_string(),
                params: serde_json::json!({"stream": "A", "offset": 0, "limit": 5}),
            },
        );
        assert!(chunk.ok, "expected ok response, got {chunk:?}");
        let result = chunk.result.unwrap();
        assert_eq!(result["indexed"], true);
        let units = result["units"].as_array().unwrap();
        assert_eq!(units.len(), 5, "limit=5 should return exactly 5 units");
        assert_eq!(units[0]["frame_index"], 0);
        assert_eq!(
            units[0]["frame_type"], "I",
            "first frame should be a keyframe"
        );
        let total_count = result["total_count"].as_u64().unwrap();
        assert!(total_count > 5, "fixture should have more than 5 frames");

        // Second page picks up where the first left off.
        let page_2 = dispatch(
            &core,
            &Request {
                id: 107,
                method: "get_frames_chunk".to_string(),
                params: serde_json::json!({"stream": "A", "offset": 5, "limit": 5}),
            },
        );
        let page_2_units = page_2.result.unwrap()["units"].as_array().unwrap().clone();
        assert_eq!(page_2_units[0]["frame_index"], 5);
    }

    #[test]
    fn get_frames_chunk_before_indexing_reports_not_indexed_not_an_error() {
        let core = Core::new();
        let response = dispatch(
            &core,
            &Request {
                id: 108,
                method: "get_frames_chunk".to_string(),
                params: serde_json::json!({"stream": "A", "offset": 0, "limit": 10}),
            },
        );
        assert!(response.ok);
        let result = response.result.unwrap();
        assert_eq!(result["indexed"], false);
        assert_eq!(result["total_count"], 0);
        assert_eq!(result["units"].as_array().unwrap().len(), 0);
    }

    #[test]
    fn get_frame_syntax_end_to_end_returns_a_real_nested_tree() {
        let core = Core::new();
        open_real_fixture(&core, "A");
        dispatch(
            &core,
            &Request {
                id: 109,
                method: "index_stream".to_string(),
                params: serde_json::json!({"stream": "A"}),
            },
        );

        let response = dispatch(
            &core,
            &Request {
                id: 110,
                method: "get_frame_syntax".to_string(),
                params: serde_json::json!({"stream": "A", "frame_index": 0}),
            },
        );
        assert!(response.ok, "expected ok response, got {response:?}");
        let tree = response.result.unwrap();
        let children = tree["children"]
            .as_array()
            .expect("root should have a children array");
        assert!(
            !children.is_empty(),
            "expected real syntax fields under the root, got {tree:?}"
        );
        assert!(tree["name"].as_str().is_some());
    }

    #[test]
    fn get_frame_syntax_before_indexing_is_a_wire_error_not_a_crash() {
        let core = Core::new();
        open_real_fixture(&core, "A");
        // Deliberately not calling index_stream first.

        let response = dispatch(
            &core,
            &Request {
                id: 111,
                method: "get_frame_syntax".to_string(),
                params: serde_json::json!({"stream": "A", "frame_index": 0}),
            },
        );
        assert!(!response.ok);
        assert_eq!(response.error.unwrap().code, WireErrorCode::FrameNotFound);
    }

    // -- get_decoded_frame_yuv --------------------------------------------------------------

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

    // -- get_frame_analysis ------------------------------------------------------------------

    #[test]
    fn get_frame_analysis_end_to_end_returns_real_grids() {
        let core = Core::new();
        open_real_fixture(&core, "A");

        let response = dispatch(
            &core,
            &Request {
                id: 220,
                method: "get_frame_analysis".to_string(),
                params: serde_json::json!({"frame_index": 0}),
            },
        );
        assert!(response.ok, "expected ok response, got {response:?}");
        let result = response.result.unwrap();
        assert_eq!(result["frame_index"], 0);
        assert_eq!(result["width"], 320);
        assert_eq!(result["height"], 240);
        assert!(!result["qp_grid"]["qp"].as_array().unwrap().is_empty());
        assert!(!result["partition_grid"]["blocks"]
            .as_array()
            .unwrap()
            .is_empty());
        let energy = result["energy_grid"]["energy_bpp"].as_array().unwrap();
        assert!(!energy.is_empty());
        assert_eq!(
            energy.len(),
            result["qp_grid"]["qp"].as_array().unwrap().len()
        );
    }

    #[test]
    fn get_frame_analysis_energy_grid_reflects_real_residual_data() {
        // Not the QP-only proxy this replaced: an all-zero energy grid would mean the fallback
        // (no-tile-data/parse-failure) path was hit instead of real per-CU residual magnitude.
        let core = Core::new();
        open_real_fixture(&core, "A");

        let response = dispatch(
            &core,
            &Request {
                id: 222,
                method: "get_frame_analysis".to_string(),
                params: serde_json::json!({"frame_index": 0}),
            },
        );
        assert!(response.ok, "expected ok response, got {response:?}");
        let result = response.result.unwrap();
        let energy = result["energy_grid"]["energy_bpp"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_f64().unwrap())
            .collect::<Vec<_>>();
        assert!(
            energy.iter().any(|&v| v > 0.0),
            "expected at least one block with real residual energy on a real I-frame, got all zeros: {energy:?}"
        );
    }

    #[test]
    fn get_frame_analysis_out_of_range_frame_index_is_a_wire_error() {
        let core = Core::new();
        open_real_fixture(&core, "A");

        let response = dispatch(
            &core,
            &Request {
                id: 221,
                method: "get_frame_analysis".to_string(),
                params: serde_json::json!({"frame_index": 999_999}),
            },
        );
        assert!(!response.ok);
        assert_eq!(response.error.unwrap().code, WireErrorCode::FrameNotFound);
    }

    #[test]
    fn get_frame_analysis_stream_not_open_returns_not_found() {
        let core = Core::new();
        let response = dispatch(
            &core,
            &Request {
                id: 222,
                method: "get_frame_analysis".to_string(),
                params: serde_json::json!({"frame_index": 0}),
            },
        );
        assert!(!response.ok);
        assert_eq!(response.error.unwrap().code, WireErrorCode::NotFound);
    }

    // -- get_av1_features ---------------------------------------------------------------------

    #[test]
    fn get_av1_features_end_to_end_returns_real_cdef_data() {
        let core = Core::new();
        open_real_fixture(&core, "A");

        let response = dispatch(
            &core,
            &Request {
                id: 223,
                method: "get_av1_features".to_string(),
                params: serde_json::json!({"frame_index": 0}),
            },
        );
        assert!(response.ok, "expected ok response, got {response:?}");
        let result = response.result.unwrap();
        assert_eq!(result["frame_index"], 0);
        assert!(!result["cdef"].is_null());
        assert_eq!(result["cdef"]["width"], 320);
        assert_eq!(result["cdef"]["height"], 240);
    }

    #[test]
    fn get_av1_features_stream_not_open_returns_not_found() {
        let core = Core::new();
        let response = dispatch(
            &core,
            &Request {
                id: 224,
                method: "get_av1_features".to_string(),
                params: serde_json::json!({"frame_index": 0}),
            },
        );
        assert!(!response.ok);
        assert_eq!(response.error.unwrap().code, WireErrorCode::NotFound);
    }

    // -- get_coding_flow_analysis --------------------------------------------------------------

    #[test]
    fn get_coding_flow_analysis_end_to_end_reaches_quantization() {
        let core = Core::new();
        open_real_fixture(&core, "A");

        let response = dispatch(
            &core,
            &Request {
                id: 225,
                method: "get_coding_flow_analysis".to_string(),
                params: serde_json::json!({"frame_index": 0}),
            },
        );
        assert!(response.ok, "expected ok response, got {response:?}");
        let result = response.result.unwrap();
        assert_eq!(result["frame_index"], 0);
        assert_eq!(result["current_stage"], "quantization");
        assert_eq!(result["stages"].as_array().unwrap().len(), 6);
        assert!(!result["codec_features"].as_array().unwrap().is_empty());
    }

    #[test]
    fn get_coding_flow_analysis_stream_not_open_returns_not_found() {
        let core = Core::new();
        let response = dispatch(
            &core,
            &Request {
                id: 226,
                method: "get_coding_flow_analysis".to_string(),
                params: serde_json::json!({"frame_index": 0}),
            },
        );
        assert!(!response.ok);
        assert_eq!(response.error.unwrap().code, WireErrorCode::NotFound);
    }

    // -- get_thumbnails ----------------------------------------------------------------------

    #[test]
    fn get_thumbnails_end_to_end_returns_real_png_data_urls() {
        let core = Core::new();
        open_real_fixture(&core, "A");

        let response = dispatch(
            &core,
            &Request {
                id: 210,
                method: "get_thumbnails".to_string(),
                params: serde_json::json!({"stream": "A", "frame_indices": [0, 3, 7]}),
            },
        );
        assert!(response.ok, "expected ok response, got {response:?}");
        let results = response.result.unwrap();
        let results = results.as_array().unwrap();
        assert_eq!(results.len(), 3);
        for thumb in results {
            assert_eq!(thumb["success"], true);
            assert_eq!(thumb["width"], 120, "default target_width should be 120");
            assert!(thumb["height"].as_u64().unwrap() > 0);
            let data_url = thumb["thumbnail_data"].as_str().unwrap();
            assert!(data_url.starts_with("data:image/png;base64,"));
        }
    }

    #[test]
    fn get_thumbnails_honors_a_custom_target_width() {
        let core = Core::new();
        open_real_fixture(&core, "A");

        let response = dispatch(
            &core,
            &Request {
                id: 211,
                method: "get_thumbnails".to_string(),
                params: serde_json::json!({"stream": "A", "frame_indices": [0], "target_width": 60}),
            },
        );
        assert!(response.ok);
        let results = response.result.unwrap();
        assert_eq!(results[0]["width"], 60);
    }

    #[test]
    fn get_thumbnails_out_of_range_index_is_a_wire_error() {
        let core = Core::new();
        open_real_fixture(&core, "A");

        let response = dispatch(
            &core,
            &Request {
                id: 212,
                method: "get_thumbnails".to_string(),
                params: serde_json::json!({"stream": "A", "frame_indices": [999_999]}),
            },
        );
        assert!(!response.ok);
        assert_eq!(response.error.unwrap().code, WireErrorCode::FrameNotFound);
    }

    #[test]
    fn get_thumbnails_stream_not_open_returns_not_found() {
        let core = Core::new();
        let response = dispatch(
            &core,
            &Request {
                id: 213,
                method: "get_thumbnails".to_string(),
                params: serde_json::json!({"stream": "A", "frame_indices": [0]}),
            },
        );
        assert!(!response.ok);
        assert_eq!(response.error.unwrap().code, WireErrorCode::NotFound);
    }

    #[test]
    fn get_frame_syntax_unknown_stream_id_is_invalid_data() {
        let core = Core::new();
        let response = dispatch(
            &core,
            &Request {
                id: 112,
                method: "get_frame_syntax".to_string(),
                params: serde_json::json!({"stream": "Z", "frame_index": 0}),
            },
        );
        assert!(!response.ok);
        assert_eq!(response.error.unwrap().code, WireErrorCode::InvalidData);
    }

    #[test]
    fn get_timeline_end_to_end_returns_a_real_display_order_timeline() {
        let core = Core::new();
        open_real_fixture(&core, "A");
        dispatch(
            &core,
            &Request {
                id: 113,
                method: "index_stream".to_string(),
                params: serde_json::json!({"stream": "A"}),
            },
        );

        let response = dispatch(
            &core,
            &Request {
                id: 114,
                method: "get_timeline".to_string(),
                params: serde_json::json!({"stream": "A"}),
            },
        );
        assert!(response.ok, "expected ok response, got {response:?}");
        let timeline = response.result.unwrap();
        assert_eq!(timeline["stream_id"], "A");
        let frames = timeline["frames"]
            .as_array()
            .expect("expected a frames array");
        assert!(
            !frames.is_empty(),
            "expected real frame entries, got {timeline:?}"
        );
        assert_eq!(
            frames[0]["marker"], "Key",
            "first frame should be marked as a keyframe"
        );
    }

    #[test]
    fn get_timeline_before_indexing_is_a_wire_error_not_a_crash() {
        let core = Core::new();
        open_real_fixture(&core, "A");
        // Deliberately not calling index_stream first.

        let response = dispatch(
            &core,
            &Request {
                id: 115,
                method: "get_timeline".to_string(),
                params: serde_json::json!({"stream": "A"}),
            },
        );
        assert!(!response.ok);
        assert_eq!(response.error.unwrap().code, WireErrorCode::FrameNotFound);
    }

    #[test]
    fn get_timeline_unknown_stream_id_is_invalid_data() {
        let core = Core::new();
        let response = dispatch(
            &core,
            &Request {
                id: 116,
                method: "get_timeline".to_string(),
                params: serde_json::json!({"stream": "Z"}),
            },
        );
        assert!(!response.ok);
        assert_eq!(response.error.unwrap().code, WireErrorCode::InvalidData);
    }

    // -- debug YUV commands (load/unload/offset/crop/get_debug_yuv_frame/metrics/find_first_diff)

    fn fresh_debug_yuv_state() -> DebugYuvSlot {
        Arc::new(Mutex::new(None))
    }

    fn write_i420_frame(
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
        let frames = get_decoded_frame_yuv(core, &decode_request);
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
        );
        assert!(response.ok, "expected ok response, got {response:?}");
        let result = response.result.unwrap();
        assert_eq!(
            result["frame_index"], 0,
            "the only reference frame -- and the only one with an injected mismatch -- is index 0"
        );
        assert_eq!(result["total_checked"], 1);
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
        );
        assert!(!response.ok);
        assert_eq!(response.error.unwrap().code, WireErrorCode::NotFound);
    }
}
