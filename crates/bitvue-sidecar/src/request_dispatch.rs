//! Worker-thread response computation and method dispatch -- `spawn_request`/`compute_frames`/
//! `dispatch`/cancel-response handling, plus the `CancelRegistry`/`DebugYuvSlot` shared-state
//! type aliases. Split out of `main.rs` (2026-08-19) as part of an SRP pass; see `main.rs`'s
//! module doc ("Concurrency model") for the full design this implements.

use crate::command_support::{
    event_to_json, panic_payload_message, parse_stream_id, single_control_frame,
};
use crate::{av1_features, codec_extended_info, coding_flow, commands, compare, context_menu};
use crate::{deblocking, debug_yuv, decode_bridge, evidence_export, frame_analysis};
use crate::{residual_analysis, write_frame};
use bitvue_engine::Core;
use bitvue_protocol::{
    CancelParams, FrameKind, HelloParams, HelloResult, Request, Response, WireError, WireErrorCode,
    PROTOCOL_VERSION,
};
use std::collections::HashMap;
use std::io;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;

/// `correlation_id` → cancellation flag, for requests currently being computed on a worker
/// thread. Entries are removed once that thread finishes (success, failure, or cancellation).
pub type CancelRegistry = Arc<Mutex<HashMap<u32, Arc<AtomicBool>>>>;

/// The one currently-loaded debug YUV reference file (if any) -- see `debug_yuv`'s module doc for
/// why this lives here rather than in `Core`/`StreamState`. Not per-`correlation_id` like
/// `CancelRegistry`; genuinely one shared slot, same as the real VQ Analyzer "Load Debug YUV"
/// workflow only ever has one reference loaded at a time.
pub type DebugYuvSlot = Arc<Mutex<Option<debug_yuv::Session>>>;

/// Registers a cancel flag for `correlation_id`, then spawns a worker thread that computes the
/// response (unlocked — no I/O, no shared-writer contention while it runs) and writes whatever
/// frames the computation produces under a brief writer-lock. See module doc for the concurrency
/// model, the mid-execution cancellation checkpoints, and the panic guard below.
pub fn spawn_request(
    core: Arc<Core>,
    debug_yuv_state: DebugYuvSlot,
    compare_state: compare::CompareSlot,
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
            compute_frames_with_panic_guard(
                &core,
                &debug_yuv_state,
                &compare_state,
                &request,
                &cancel_flag,
            )
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

/// Runs `compute_frames` behind a panic guard (CONC-022): without this, a handler that panics
/// (malformed frame, decoder bug, etc.) would kill the worker thread before `spawn_request`'s
/// `registry.remove` runs and before any response is written — the client's Promise for that
/// `correlation_id` would then hang forever with no error. `catch_unwind` lets execution continue
/// past the panic with a real failure response instead, so cleanup and a reply both still happen.
///
/// Requires `panic = "unwind"` in `[profile.release]` (see the root `Cargo.toml`) — with
/// `"abort"` the process dies before unwinding ever reaches this catch, so the guard would be a
/// no-op in a release build specifically, defeating the point.
///
/// `AssertUnwindSafe` is safe here: `Core`'s internal state uses `parking_lot::RwLock`/`Mutex`,
/// neither of which poisons on panic (unlike `std::sync`'s), so a handler panicking mid-lock
/// doesn't leave shared state in a form other threads would observe as "poisoned" — the lock is
/// simply released, and the (possibly partially-written) data is the same risk any panic leaves
/// behind, not a new hazard `catch_unwind` introduces.
///
/// Split out from `spawn_request` so it's testable directly without a real thread/writer --
/// see `compute_frames`'s `#[cfg(test)]`-only `__test_trigger_panic__` method.
fn compute_frames_with_panic_guard(
    core: &Core,
    debug_yuv_state: &DebugYuvSlot,
    compare_state: &compare::CompareSlot,
    request: &Request,
    cancel_flag: &AtomicBool,
) -> Vec<(FrameKind, Vec<u8>)> {
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        compute_frames(core, debug_yuv_state, compare_state, request, cancel_flag)
    })) {
        Ok(frames) => frames,
        Err(panic_payload) => {
            // `&*panic_payload`, not `&panic_payload` -- `panic_payload` is a `Box<dyn Any +
            // Send>`, and `Box<dyn Any + Send>` itself also satisfies `Any` (blanket impl for
            // any `T: 'static`), so an un-derefed `&panic_payload` coerces to `&dyn Any` *over
            // the Box*, not its contents -- every downcast in `panic_payload_message` would then
            // silently miss and always fall through to "non-string panic payload", even for an
            // ordinary `panic!("literal")`. Caught by this fix's own regression test.
            let message = panic_payload_message(&*panic_payload);
            eprintln!(
                "bitvue-sidecar: handler for '{}' panicked: {message}",
                request.method
            );
            let response = Response::failure(
                request.id,
                WireError {
                    code: WireErrorCode::Internal,
                    message: format!("internal error: handler panicked: {message}"),
                    offset: None,
                },
            );
            single_control_frame(response)
        }
    }
}

/// Pure compute for `cancel_request` — sets the target's flag (if it's still registered) and
/// reports whether it was found. Kept separate from I/O for the same reason `compute_frames` is:
/// testable without touching a real writer, and consistent with how every other command works.
pub fn compute_cancel_response(registry: &CancelRegistry, request: &Request) -> Response {
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

/// Pure compute: turns a request into the frame(s) that should be written for it. No I/O, no
/// locking — safe (and intended) to run concurrently across worker threads, which is the whole
/// point of the concurrency model above.
///
/// `cancel_flag` is this request's own cancellation flag (see module doc) — threaded down to the
/// handful of handlers below whose work is genuinely long-running/loopy (`index_stream`,
/// `get_thumbnails`, `find_first_diff_frame`) so they can bail out mid-execution instead of
/// running to completion regardless of a `cancel_request`. Everything else ignores it; a fast
/// single-lookup handler has nothing meaningful to check mid-execution.
pub fn compute_frames(
    core: &Core,
    debug_yuv_state: &DebugYuvSlot,
    compare_state: &compare::CompareSlot,
    request: &Request,
    cancel_flag: &AtomicBool,
) -> Vec<(FrameKind, Vec<u8>)> {
    match request.method.as_str() {
        "get_hex_range" => return commands::data_plane::get_hex_range(core, request),
        "get_decoded_frame_yuv" => {
            return commands::data_plane::get_decoded_frame_yuv(core, request)
        }
        "get_frame_analysis" => return frame_analysis::get_frame_analysis_command(core, request),
        "get_debug_yuv_frame" => {
            return debug_yuv::get_debug_yuv_frame(core, debug_yuv_state, request)
        }
        "load_debug_yuv" => {
            return single_control_frame(debug_yuv::load_debug_yuv(debug_yuv_state, request))
        }
        "unload_debug_yuv" => {
            return single_control_frame(debug_yuv::unload_debug_yuv(debug_yuv_state, request))
        }
        "set_debug_yuv_offset" => {
            return single_control_frame(debug_yuv::set_debug_yuv_offset(debug_yuv_state, request))
        }
        "set_debug_yuv_crop" => {
            return single_control_frame(debug_yuv::set_debug_yuv_crop(debug_yuv_state, request))
        }
        "get_yuv_diff_metrics" => {
            return single_control_frame(debug_yuv::get_yuv_diff_metrics(
                core,
                debug_yuv_state,
                request,
            ))
        }
        "find_first_diff_frame" => {
            return single_control_frame(debug_yuv::find_first_diff_frame(
                core,
                debug_yuv_state,
                request,
                cancel_flag,
            ))
        }
        "index_stream" => return single_control_frame(index_stream(core, request, cancel_flag)),
        "get_thumbnails" => {
            return single_control_frame(decode_bridge::get_thumbnails_command(
                core,
                request,
                cancel_flag,
            ))
        }
        "create_compare_workspace" => {
            return single_control_frame(compare::create_compare_workspace(
                core,
                compare_state,
                request,
            ))
        }
        "get_aligned_frame" => {
            return single_control_frame(compare::get_aligned_frame(compare_state, request))
        }
        "set_sync_mode" => {
            return single_control_frame(compare::set_sync_mode(compare_state, request))
        }
        "set_manual_offset" => {
            return single_control_frame(compare::set_manual_offset(compare_state, request))
        }
        "reset_offset" => {
            return single_control_frame(compare::reset_offset(compare_state, request))
        }
        "get_diff_frame" => {
            return single_control_frame(compare::get_diff_frame(core, compare_state, request))
        }
        "find_first_diff_frame_ab" => {
            return single_control_frame(compare::find_first_diff_frame(
                core,
                compare_state,
                request,
                cancel_flag,
            ))
        }
        // Test-only escape hatch (not reachable outside `cfg(test)`, so zero cost/risk in a real
        // build): lets a test drive a real handler panic through the *actual* dispatch path
        // rather than calling `panic!()` inline, so `compute_frames_with_panic_guard`'s
        // catch_unwind is exercised the same way a real malformed-frame/decoder-bug panic would
        // be. See CONC-022's regression test.
        #[cfg(test)]
        "__test_trigger_panic__" => panic!("intentional test panic"),
        _ => {}
    }
    let response = dispatch(core, request);
    vec![(
        FrameKind::Control,
        serde_json::to_vec(&response).expect("Response always serializes"),
    )]
}

pub fn dispatch(core: &Core, request: &Request) -> Response {
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
        "open_stream" => commands::stream::open_stream(core, request),
        "select_frame" => commands::selection::select_frame(core, request),
        "select_unit" => commands::selection::select_unit(core, request),
        "select_syntax" => commands::selection::select_syntax(core, request),
        "select_bit_range" => commands::selection::select_bit_range(core, request),
        "select_spatial_block" => commands::selection::select_spatial_block(core, request),
        "close_stream" => commands::stream::close_stream(core, request),
        // These two are also reachable through `compute_frames`'s early-return match with the
        // real per-request `cancel_flag` (see there) -- that's the path `spawn_request` actually
        // uses. This arm only serves direct `dispatch()` callers (tests), which have no
        // cancellation context, so it passes a flag that's never set.
        "index_stream" => index_stream(core, request, &AtomicBool::new(false)),
        "get_stream_info" => commands::stream_query::get_stream_info(core, request),
        "get_frames_chunk" => commands::stream_query::get_frames_chunk(core, request),
        "get_frame_syntax" => commands::stream_query::get_frame_syntax(core, request),
        "get_timeline" => commands::stream_query::get_timeline(core, request),
        "get_thumbnails" => {
            decode_bridge::get_thumbnails_command(core, request, &AtomicBool::new(false))
        }
        "get_av1_features" => av1_features::get_av1_features_command(core, request),
        "get_coding_flow_analysis" => coding_flow::get_coding_flow_analysis_command(core, request),
        "get_deblocking_analysis" => deblocking::get_deblocking_analysis_command(core, request),
        "get_codec_extended_info" => {
            codec_extended_info::get_codec_extended_info_command(core, request)
        }
        "get_residual_analysis" => residual_analysis::get_residual_analysis_command(core, request),
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

#[derive(serde::Deserialize)]
struct IndexStreamParams {
    stream: String,
}

/// Runs `bitvue_indexer::index_stream` -- IVF/AV1 metadata indexing (container + units, no pixel
/// decode). See that crate's module doc for exactly what is and isn't populated. Not a
/// `Core::handle_command` variant: `Command::RunFullAnalysis` stays unused, this bypasses it
/// entirely by calling the indexer directly against `Core::get_stream()`/`get_job_manager()`.
fn index_stream(core: &Core, request: &Request, cancel_flag: &AtomicBool) -> Response {
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

    // Whole-stream, per-frame loop -- can take real time on a long stream, so it's one of the
    // handlers cooperatively cancellable mid-execution (see module doc / `compute_frames`).
    let (events, cancelled) = bitvue_indexer::index_stream_with_cancel(core, stream, cancel_flag);
    if cancelled {
        return Response::failure(
            request.id,
            WireError {
                code: WireErrorCode::Cancelled,
                message: "cancelled".to_string(),
                offset: None,
            },
        );
    }
    let events_json: Vec<serde_json::Value> = events.iter().map(event_to_json).collect();
    Response::success(request.id, serde_json::json!({ "events": events_json }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::open_real_fixture;
    use bitvue_protocol::CancelParams;

    #[test]
    fn unknown_method_returns_internal_error() {
        let core = Core::new();
        let request = Request {
            id: 7,
            // `dispatch` alone (not `compute_frames`) doesn't know about anything routed through
            // `compute_frames`'s own match arms (e.g. `create_compare_workspace`) -- a genuinely
            // nonexistent method name, so this test still means what it says.
            method: "totally_not_a_real_method".to_string(),
            params: serde_json::json!({}),
        };
        let response = dispatch(&core, &request);
        assert!(!response.ok);
        assert_eq!(response.error.unwrap().code, WireErrorCode::Internal);
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
        let compare_state: compare::CompareSlot = Arc::new(Mutex::new(None));
        let mut handles = Vec::new();

        for i in 0..16u32 {
            let core = Arc::clone(&core);
            let debug_yuv_state = Arc::clone(&debug_yuv_state);
            let compare_state = Arc::clone(&compare_state);
            handles.push(thread::spawn(move || {
                let stream = if i % 2 == 0 { "A" } else { "B" };
                let request = Request {
                    id: i,
                    method: "select_frame".to_string(),
                    params: serde_json::json!({"stream": stream, "frame_index": i as usize}),
                };
                let frames = compute_frames(
                    &core,
                    &debug_yuv_state,
                    &compare_state,
                    &request,
                    &AtomicBool::new(false),
                );
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

    /// Real regression test for CONC-022: a handler that panics mid-request must not skip
    /// cancel-registry cleanup or leave the client hanging with no response. Drives an actual
    /// panic through `compute_frames`'s real dispatch (`__test_trigger_panic__`, a `cfg(test)`
    /// only method -- see its match arm) rather than calling `panic!()` inline, so this exercises
    /// the same `catch_unwind` boundary `spawn_request` uses in production, not a hand-rolled
    /// substitute. Mirrors `spawn_request`'s exact registry-insert / guarded-compute /
    /// registry-remove sequence (minus the real thread + stdout writer, which aren't needed to
    /// prove this contract and would need a non-trivial writer-abstraction refactor to fake --
    /// see this test's sibling for why that trade-off was made).
    #[test]
    fn panicking_handler_still_cleans_up_the_registry_and_returns_a_real_error_response() {
        let core = Core::new();
        let debug_yuv_state: DebugYuvSlot = Arc::new(Mutex::new(None));
        let compare_state: compare::CompareSlot = Arc::new(Mutex::new(None));
        let registry: CancelRegistry = Arc::new(Mutex::new(HashMap::new()));
        let correlation_id = 999;
        let cancel_flag = Arc::new(AtomicBool::new(false));
        registry
            .lock()
            .unwrap()
            .insert(correlation_id, Arc::clone(&cancel_flag));

        let request = Request {
            id: 42,
            method: "__test_trigger_panic__".to_string(),
            params: serde_json::json!({}),
        };

        // Same shape as spawn_request's worker body: guarded compute, then registry cleanup.
        let frames = compute_frames_with_panic_guard(
            &core,
            &debug_yuv_state,
            &compare_state,
            &request,
            &cancel_flag,
        );
        registry.lock().unwrap().remove(&correlation_id);

        assert!(
            !registry.lock().unwrap().contains_key(&correlation_id),
            "the registry entry must be cleaned up even though the handler panicked -- a leaked \
             entry here is exactly the CONC-022 bug (a future cancel_request for a reused/replayed \
             id would silently target stale state)"
        );

        assert_eq!(
            frames.len(),
            1,
            "a panicking handler must still produce exactly one response frame, not zero (which \
             would hang the client's Promise forever)"
        );
        let response: Response = serde_json::from_slice(&frames[0].1).unwrap();
        assert!(
            !response.ok,
            "a panicking handler must produce a failure response, not hang: {response:?}"
        );
        let error = response.error.unwrap();
        assert_eq!(error.code, WireErrorCode::Internal);
        assert!(
            error.message.contains("intentional test panic"),
            "the real panic message should be surfaced, not swallowed: {}",
            error.message
        );
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

    /// Real cancellation-checkpoint regression test for UIX-ASYNC-006, mirroring
    /// `find_first_diff_frame_stops_early_when_already_cancelled`: an already-set flag must make
    /// `index_stream` bail out of `index_ivf_av1`'s per-frame loop instead of indexing the whole
    /// fixture, must report `Cancelled` rather than a silent success, and -- since a cancelled
    /// index run is documented to write no partial state -- must leave `StreamState.container`
    /// unset so a later real `index_stream` call isn't shadowed by a half-finished one.
    #[test]
    fn index_stream_stops_early_when_already_cancelled() {
        let core = Core::new();
        open_real_fixture(&core, "A");

        let request = Request {
            id: 111,
            method: "index_stream".to_string(),
            params: serde_json::json!({"stream": "A"}),
        };
        let response = index_stream(&core, &request, &AtomicBool::new(true));
        assert!(
            !response.ok,
            "expected a failure response, got {response:?}"
        );
        assert_eq!(response.error.unwrap().code, WireErrorCode::Cancelled);

        let info_response = dispatch(
            &core,
            &Request {
                id: 112,
                method: "get_stream_info".to_string(),
                params: serde_json::json!({"stream": "A"}),
            },
        );
        assert!(info_response.ok);
        assert_eq!(
            info_response.result.unwrap()["indexed"],
            false,
            "a cancelled index run must not write partial container/units state"
        );
    }
}
