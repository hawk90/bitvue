//! bitvue-sidecar: standalone process hosting the bitvue engine, speaking `bitvue-protocol`
//! over stdio to `bitvue-desktop` (Electron main). See `docs/DEVELOPMENT_PHASES.md`
//! ("sidecar 결정" / "bitvue-protocol wire schema v0" / "동시성 모델") for the full design.
//!
//! This file is process bootstrap + the stdin/stdout frame loop + worker-thread orchestration
//! only (`main`/`read_frame`/`write_frame`/`spawn_request`) -- every command handler and the
//! method-dispatch table live in `request_dispatch.rs` and the per-command modules it calls into
//! (see `request_dispatch.rs`'s module doc for the full command list and the reasoning for the
//! `bitvue-indexer`-backed commands not being `Core::handle_command` variants).
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
//! Response computation (`request_dispatch::compute_frames`) is pure — no I/O, doesn't touch the
//! writer — so it runs fully unlocked/in parallel across threads. Only the final write is
//! serialized (one `Mutex<Stdout>` lock per request, held just long enough to write that
//! request's frame(s)); locking around *computation* would silently re-serialize everything and
//! defeat the point.
//!
//! `cancel_request` sets a per-request `AtomicBool` flag in a shared registry
//! (`correlation_id` → flag). A worker thread checks its own flag once, immediately before
//! running its handler (still cancellable at that first window even for otherwise-uninstrumented
//! handlers), *and* the genuinely long-running, per-frame-loopy handlers cooperatively re-check it
//! mid-execution so a request that's already started can still bail out early instead of running
//! to completion regardless: `index_stream` (`bitvue_indexer::index_stream_with_cancel`, checked
//! once per parsed IVF frame), `get_thumbnails` (`decode_bridge::get_thumbnails`, checked once per
//! encoded packet and once per decoded frame), and `find_first_diff_frame`
//! (`debug_yuv::find_first_diff`, same per-packet/per-decoded-frame checkpoints). A cancelled
//! mid-execution handler returns a real `WireErrorCode::Cancelled` failure response, not a silent
//! partial success. Every other handler here is a fast synchronous call (a file mmap, a
//! selection-state write, one `ByteCache::read_range`) where the only cancellation window that
//! matters is that first before-handler check — adding mid-loop checkpoints to those would be
//! checking on effectively every instruction for no benefit.
//!
//! A worker thread panicking mid-request (malformed frame, decoder bug, etc.) is also handled:
//! `spawn_request` runs the handler inside `std::panic::catch_unwind`, so a panic can't skip the
//! registry cleanup or leave the client's request hanging forever with no response — it's turned
//! into a real `WireErrorCode::Internal` failure response instead. This requires the workspace's
//! `[profile.release]` to use `panic = "unwind"` (the default), not `"abort"` — see the comment on
//! that profile in the root `Cargo.toml`.
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
use std::sync::{Arc, Mutex};
use std::thread;

mod av1_features;
mod codec_extended_info;
mod coding_flow;
mod command_support;
mod commands;
mod compare;
mod context_menu;
mod deblocking;
mod debug_yuv;
mod decode_bridge;
mod evidence_export;
mod frame_analysis;
mod request_dispatch;
mod residual_analysis;
#[cfg(test)]
mod test_support;

// Re-exported so every command module's tests can call `crate::dispatch(...)` without knowing
// it now lives in `request_dispatch` -- `dispatch` itself isn't referenced by name anywhere in
// this file, hence the lint suppression.
#[allow(unused_imports)]
pub(crate) use request_dispatch::{dispatch, DebugYuvSlot};

use bitvue_engine::Core;
use bitvue_protocol::{FrameHeader, FrameKind, Request, FRAME_HEADER_LEN};

fn main() {
    let core = Arc::new(Core::new());
    let debug_yuv_state: DebugYuvSlot = Arc::new(Mutex::new(None));
    let compare_state: compare::CompareSlot = Arc::new(Mutex::new(None));
    let writer = Arc::new(Mutex::new(io::stdout()));
    let registry: request_dispatch::CancelRegistry = Arc::new(Mutex::new(HashMap::new()));
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
                    let response = request_dispatch::compute_cancel_response(&registry, &request);
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
                handles.push(request_dispatch::spawn_request(
                    Arc::clone(&core),
                    Arc::clone(&debug_yuv_state),
                    Arc::clone(&compare_state),
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::encode_frames;
    use bitvue_protocol::{HelloParams, HelloResult, Response, PROTOCOL_VERSION};
    use std::io::Cursor;
    use std::sync::atomic::AtomicBool;

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
        let compare_state: compare::CompareSlot = Arc::new(Mutex::new(None));
        let frames = request_dispatch::compute_frames(
            &core,
            &debug_yuv_state,
            &compare_state,
            &parsed,
            &AtomicBool::new(false),
        );
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
    fn stdin_close_yields_none() {
        let mut reader = Cursor::new(Vec::<u8>::new());
        assert!(read_frame(&mut reader).unwrap().is_none());
    }
}
