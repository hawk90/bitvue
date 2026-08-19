//! Structural (multi-sync) selection commands -- `select_frame`/`select_unit`/`select_syntax`/
//! `select_bit_range`/`select_spatial_block` all map straight onto `bitvue_engine::Command`'s
//! existing "Tri-sync" selection variants (see `main.rs`'s module doc / `docs/
//! DEVELOPMENT_PHASES.md`'s `SelectionState` note), no engine work needed -- these are thin
//! request-decode + `Core::handle_command` + event-encode wrappers. Split out of `main.rs`
//! (2026-08-19) as part of an SRP pass.

use crate::command_support::{event_to_json, parse_stream_id};
use bitvue_engine::{BitRange, Command, Core, FrameKey, SpatialBlock, UnitKey};
use bitvue_protocol::{Request, Response, WireError, WireErrorCode};

#[derive(serde::Deserialize)]
struct SelectFrameParams {
    stream: String,
    frame_index: usize,
}

pub fn select_frame(core: &Core, request: &Request) -> Response {
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
pub fn select_unit(core: &Core, request: &Request) -> Response {
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
pub fn select_syntax(core: &Core, request: &Request) -> Response {
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
pub fn select_bit_range(core: &Core, request: &Request) -> Response {
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
pub fn select_spatial_block(core: &Core, request: &Request) -> Response {
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

#[cfg(test)]
mod tests {
    use bitvue_engine::Core;
    use bitvue_protocol::{Request, WireErrorCode};

    #[test]
    fn select_frame_emits_selection_updated() {
        let core = Core::new();
        let request = Request {
            id: 10,
            method: "select_frame".to_string(),
            params: serde_json::json!({"stream": "A", "frame_index": 42}),
        };
        let response = crate::dispatch(&core, &request);
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
        let response = crate::dispatch(&core, &request);
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
        let response = crate::dispatch(&core, &request);
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
        let response = crate::dispatch(&core, &request);
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
        let response = crate::dispatch(&core, &request);
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
        let response = crate::dispatch(&core, &request);
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
        let response = crate::dispatch(&core, &request);
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
        let response = crate::dispatch(&core, &request);
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
        let response = crate::dispatch(&core, &request);
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
        let response = crate::dispatch(&core, &request);
        assert!(!response.ok);
        assert_eq!(response.error.unwrap().code, WireErrorCode::InvalidData);
    }
}
