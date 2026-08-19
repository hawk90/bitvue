//! Read-only, `bitvue-indexer`-backed queries over an already-`index_stream`'d stream --
//! `get_stream_info`/`get_frames_chunk`/`get_frame_syntax`/`get_timeline`. Not `Core::
//! handle_command` variants (`Core` is a leaf crate, can't call into `bitvue-indexer`'s
//! codec-dependent parsing -- see `main.rs`'s module doc). Split out of `main.rs` (2026-08-19)
//! as part of an SRP pass.

use crate::command_support::parse_stream_id;
use bitvue_engine::Core;
use bitvue_protocol::{Request, Response, WireError, WireErrorCode};

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
pub fn get_stream_info(core: &Core, request: &Request) -> Response {
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
/// already derives `Serialize` (unlike most `bitvue-engine` types -- see `main.rs`'s module doc),
/// so units serialize directly, no hand-mapping needed.
pub fn get_frames_chunk(core: &Core, request: &Request) -> Response {
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
/// `main.rs`'s module doc on `bitvue-engine` types generally not being wire types), and the
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
/// `get_frames_chunk`. Unlike those, failure here IS a wire error (`WireErrorCode::NotFound`/
/// `InvalidData`) rather than an `{indexed: false}`-style payload -- there's no meaningful
/// partial result for "this frame's syntax tree" the way there is for "nothing indexed yet."
pub fn get_frame_syntax(core: &Core, request: &Request) -> Response {
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
pub fn get_timeline(core: &Core, request: &Request) -> Response {
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

#[cfg(test)]
mod tests {
    use crate::test_support::open_real_fixture;
    use bitvue_engine::Core;
    use bitvue_protocol::{Request, WireErrorCode};

    #[test]
    fn get_stream_info_reflects_index_stream_result() {
        let core = Core::new();
        open_real_fixture(&core, "A");

        // Before indexing: honestly reports not-indexed, not a wire error.
        let before = crate::dispatch(
            &core,
            &Request {
                id: 102,
                method: "get_stream_info".to_string(),
                params: serde_json::json!({"stream": "A"}),
            },
        );
        assert!(before.ok);
        assert_eq!(before.result.as_ref().unwrap()["indexed"], false);

        crate::dispatch(
            &core,
            &Request {
                id: 103,
                method: "index_stream".to_string(),
                params: serde_json::json!({"stream": "A"}),
            },
        );

        let after = crate::dispatch(
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
        crate::dispatch(
            &core,
            &Request {
                id: 105,
                method: "index_stream".to_string(),
                params: serde_json::json!({"stream": "A"}),
            },
        );

        let chunk = crate::dispatch(
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
        let page_2 = crate::dispatch(
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
        let response = crate::dispatch(
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
        crate::dispatch(
            &core,
            &Request {
                id: 109,
                method: "index_stream".to_string(),
                params: serde_json::json!({"stream": "A"}),
            },
        );

        let response = crate::dispatch(
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

        let response = crate::dispatch(
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

    #[test]
    fn get_frame_syntax_unknown_stream_id_is_invalid_data() {
        let core = Core::new();
        let response = crate::dispatch(
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
        crate::dispatch(
            &core,
            &Request {
                id: 113,
                method: "index_stream".to_string(),
                params: serde_json::json!({"stream": "A"}),
            },
        );

        let response = crate::dispatch(
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

        let response = crate::dispatch(
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
        let response = crate::dispatch(
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
}
