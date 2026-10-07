//! MCP method routing: `initialize`, `tools/list`, `tools/call`, `ping` and unknown methods.

use crate::catalog::get_tools;
use crate::protocol::{McpError, McpRequest, McpResponse};
use crate::state::AppState;
use crate::tools::{
    analyze_frame, compare_streams, find_decoding_issues, get_gop_structure, get_motion_vectors,
    get_qp_map, get_stream_info, list_files, load_file, search_syntax,
};
use serde_json::{json, Value};

/// Handle MCP request
pub(crate) fn handle_request(request: McpRequest, state: &AppState) -> McpResponse {
    match request.method.as_str() {
        "initialize" => handle_initialize(request.id),
        "tools/list" => handle_tools_list(request.id),
        "tools/call" => handle_tool_call(request.id, request.params, state),
        "ping" => handle_ping(request.id),
        _ => handle_unknown(request.id, request.method),
    }
}

/// Handle initialize request
pub(crate) fn handle_initialize(id: Value) -> McpResponse {
    McpResponse {
        jsonrpc: String::from("2.0"),
        id,
        result: Some(json!({
            "protocolVersion": "2024-11-05",
            "serverInfo": {
                "name": "bitvue-mcp",
                "version": "0.1.0"
            },
            "capabilities": {
                "tools": {}
            }
        })),
        error: None,
    }
}

/// Handle tools/list request
pub(crate) fn handle_tools_list(id: Value) -> McpResponse {
    let tools = get_tools();
    McpResponse {
        jsonrpc: String::from("2.0"),
        id,
        result: Some(json!({
            "tools": tools
        })),
        error: None,
    }
}

/// Handle tools/call request
pub(crate) fn handle_tool_call(id: Value, params: Option<Value>, state: &AppState) -> McpResponse {
    let params = match params {
        Some(p) => p,
        None => {
            return McpResponse {
                jsonrpc: String::from("2.0"),
                id,
                result: None,
                error: Some(McpError {
                    code: -32602,
                    message: "Invalid params".to_string(),
                    data: None,
                }),
            }
        }
    };

    let tool_name = params["name"].as_str().unwrap_or("");
    let arguments = params.get("arguments").cloned().unwrap_or(json!({}));

    let result = match tool_name {
        "load_file" => load_file(arguments, state),
        "analyze_frame" => analyze_frame(arguments, state),
        "get_qp_map" => get_qp_map(arguments, state),
        "get_motion_vectors" => get_motion_vectors(arguments, state),
        "compare_streams" => compare_streams(arguments, state),
        "get_gop_structure" => get_gop_structure(arguments, state),
        "find_decoding_issues" => find_decoding_issues(arguments, state),
        "get_stream_info" => get_stream_info(arguments, state),
        "search_syntax" => search_syntax(arguments, state),
        "list_files" => list_files(state),
        _ => Err(anyhow::anyhow!("Unknown tool: {}", tool_name)),
    };

    match result {
        Ok(data) => McpResponse {
            jsonrpc: String::from("2.0"),
            id,
            result: Some(json!({
                "content": [{
                    "type": "text",
                    "text": data
                }]
            })),
            error: None,
        },
        Err(e) => McpResponse {
            jsonrpc: String::from("2.0"),
            id,
            result: None,
            error: Some(McpError {
                code: -1,
                message: e.to_string(),
                data: None,
            }),
        },
    }
}

/// Handle ping request
pub(crate) fn handle_ping(id: Value) -> McpResponse {
    McpResponse {
        jsonrpc: String::from("2.0"),
        id,
        result: Some(json!({})),
        error: None,
    }
}

/// Handle unknown request
pub(crate) fn handle_unknown(id: Value, method: String) -> McpResponse {
    McpResponse {
        jsonrpc: String::from("2.0"),
        id,
        result: None,
        error: Some(McpError {
            code: -32601,
            message: format!("Method not found: {}", method),
            data: None,
        }),
    }
}
