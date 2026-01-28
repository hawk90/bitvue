//! bitvue MCP Server
//!
//! Model Context Protocol server for bitvue video analyzer.
//! Exposes video analysis capabilities to AI assistants like Claude.

use anyhow::Result;
use bitvue_av1::frame_header::{parse_frame_header_basic, FrameType};
use bitvue_core::{Core, StreamId, UnitModel, UnitNode};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::fs::File;
use std::io::{self, BufRead, BufReader, Read, Seek, SeekFrom, Write};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

/// Shared application state
struct AppState {
    core: Arc<Mutex<Core>>,
    loaded_file: Arc<Mutex<Option<PathBuf>>>,
}

impl AppState {
    fn new() -> Self {
        Self {
            core: Arc::new(Mutex::new(Core::new())),
            loaded_file: Arc::new(Mutex::new(None)),
        }
    }
}

/// MCP Request
#[derive(Debug, Deserialize)]
struct McpRequest {
    jsonrpc: String,
    id: Value,
    method: String,
    params: Option<Value>,
}

/// MCP Response
struct McpResponse {
    jsonrpc: String,
    id: Value,
    result: Option<Value>,
    error: Option<McpError>,
}

impl McpResponse {
    fn to_json(&self) -> Result<String> {
        let mut obj = json!({
            "jsonrpc": self.jsonrpc,
            "id": self.id
        });

        if let Some(result) = &self.result {
            obj["result"] = result.clone();
        }
        if let Some(error) = &self.error {
            obj["error"] = json!({
                "code": error.code,
                "message": error.message,
                "data": error.data
            });
        }

        Ok(serde_json::to_string(&obj)?)
    }
}

/// MCP Error
#[derive(Debug)]
struct McpError {
    code: i32,
    message: String,
    data: Option<Value>,
}

/// MCP Tool definition
#[derive(Debug, Serialize)]
struct Tool {
    name: String,
    description: String,
    input_schema: Value,
}

/// Available MCP tools
fn get_tools() -> Vec<Tool> {
    vec![
        Tool {
            name: "load_file".to_string(),
            description: "Load a video file for analysis. Supports IVF, MP4, MKV, TS formats.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Path to the video file"
                    },
                    "stream": {
                        "type": "string",
                        "enum": ["A", "B"],
                        "description": "Stream to load into (default: A)"
                    }
                },
                "required": ["path"]
            })
        },
        Tool {
            name: "analyze_frame".to_string(),
            description: "Analyze a specific video frame. Returns frame type, size, offset, PTS, and QP if available.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "frame_index": {
                        "type": "integer",
                        "description": "Frame index to analyze (0-based)"
                    },
                    "stream": {
                        "type": "string",
                        "enum": ["A", "B"],
                        "description": "Stream to analyze (default: A)"
                    }
                },
                "required": ["frame_index"]
            })
        },
        Tool {
            name: "get_qp_map".to_string(),
            description: "Get QP (Quantization Parameter) data for a frame. Lower QP = higher quality.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "frame_index": {
                        "type": "integer",
                        "description": "Frame index"
                    },
                    "stream": {
                        "type": "string",
                        "enum": ["A", "B"],
                        "description": "Stream (default: A)"
                    }
                },
                "required": ["frame_index"]
            })
        },
        Tool {
            name: "get_motion_vectors".to_string(),
            description: "Get motion vector information for a frame if available.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "frame_index": {
                        "type": "integer",
                        "description": "Frame index"
                    },
                    "stream": {
                        "type": "string",
                        "enum": ["A", "B"],
                        "description": "Stream (default: A)"
                    }
                },
                "required": ["frame_index"]
            })
        },
        Tool {
            name: "compare_streams".to_string(),
            description: "Compare two video streams (Stream A and Stream B) at a specific frame.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "frame_index": {
                        "type": "integer",
                        "description": "Frame index to compare"
                    }
                },
                "required": ["frame_index"]
            })
        },
        Tool {
            name: "get_gop_structure".to_string(),
            description: "Get GOP (Group of Pictures) structure. Shows frame types, sizes, and dependencies.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "stream": {
                        "type": "string",
                        "enum": ["A", "B"],
                        "description": "Stream to analyze (default: A)"
                    },
                    "max_frames": {
                        "type": "integer",
                        "description": "Maximum frames to return (default: 100)"
                    }
                }
            })
        },
        Tool {
            name: "find_decoding_issues".to_string(),
            description: "Analyze the video stream for potential issues like corrupted frames or size anomalies.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "stream": {
                        "type": "string",
                        "enum": ["A", "B"],
                        "description": "Stream to analyze (default: A)"
                    }
                }
            })
        },
        Tool {
            name: "get_stream_info".to_string(),
            description: "Get overall stream information including codec, frame count, and file path.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "stream": {
                        "type": "string",
                        "enum": ["A", "B"],
                        "description": "Stream to query (default: A)"
                    }
                }
            })
        },
        Tool {
            name: "search_syntax".to_string(),
            description: "Search for frames matching specific criteria (e.g., frame_type, size range, QP range).".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "frame_type": {
                        "type": "string",
                        "enum": ["I", "P", "B", "all"],
                        "description": "Filter by frame type"
                    },
                    "min_qp": {
                        "type": "integer",
                        "description": "Minimum QP value"
                    },
                    "max_qp": {
                        "type": "integer",
                        "description": "Maximum QP value"
                    },
                    "stream": {
                        "type": "string",
                        "enum": ["A", "B"],
                        "description": "Stream to search (default: A)"
                    },
                    "limit": {
                        "type": "integer",
                        "description": "Maximum results to return (default: 50)"
                    }
                }
            })
        },
        Tool {
            name: "list_files".to_string(),
            description: "List all currently loaded video files.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {}
            })
        }
    ]
}

/// Parse stream ID from string
fn parse_stream_id(stream: Option<&str>) -> StreamId {
    match stream.unwrap_or("A") {
        "B" => StreamId::B,
        _ => StreamId::A,
    }
}

/// Handle MCP request
fn handle_request(request: McpRequest, state: &AppState) -> McpResponse {
    match request.method.as_str() {
        "initialize" => handle_initialize(request.id),
        "tools/list" => handle_tools_list(request.id),
        "tools/call" => handle_tool_call(request.id, request.params, state),
        "ping" => handle_ping(request.id),
        _ => handle_unknown(request.id, request.method),
    }
}

/// Handle initialize request
fn handle_initialize(id: Value) -> McpResponse {
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
fn handle_tools_list(id: Value) -> McpResponse {
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
fn handle_tool_call(id: Value, params: Option<Value>, state: &AppState) -> McpResponse {
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
fn handle_ping(id: Value) -> McpResponse {
    McpResponse {
        jsonrpc: String::from("2.0"),
        id,
        result: Some(json!({})),
        error: None,
    }
}

/// Handle unknown request
fn handle_unknown(id: Value, method: String) -> McpResponse {
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

// ============================================================================
// Tool Implementations
// ============================================================================

/// Parse IVF file and return units
fn parse_ivf_file(path: &PathBuf, stream_id: StreamId) -> Result<Vec<UnitNode>, String> {
    let file = File::open(path).map_err(|e| format!("Failed to open file: {}", e))?;
    let mut reader = BufReader::new(file);

    // Read IVF header (32 bytes)
    let mut header = [0u8; 32];
    reader
        .read_exact(&mut header)
        .map_err(|e| format!("Failed to read header: {}", e))?;

    // Verify IVF signature
    if &header[0..4] != b"DKIF" {
        return Err(format!(
            "Not a valid IVF file: signature = {:?}, expected DKIF",
            &header[0..4]
        ));
    }

    // Parse IVF header
    let timebase_den = u32::from_le_bytes([header[16], header[17], header[18], header[19]]);
    let _frame_count =
        u32::from_le_bytes([header[24], header[25], header[26], header[27]]) as usize;

    tracing::info!("IVF header: timebase_den={}", timebase_den);

    let mut units = Vec::new();
    let mut frame_index = 0;
    let mut current_offset = 32u64;

    // Read frames
    loop {
        let frame_start = current_offset;

        // Read frame header (12 bytes)
        let mut frame_header = [0u8; 12];
        match reader.read_exact(&mut frame_header) {
            Ok(_) => {}
            Err(_) if frame_index == 0 => {
                return Err("Failed to read first frame header".to_string())
            }
            Err(_) => break,
        }

        let frame_size = u32::from_le_bytes([
            frame_header[0],
            frame_header[1],
            frame_header[2],
            frame_header[3],
        ]) as usize;
        let pts = u64::from_le_bytes([
            frame_header[4],
            frame_header[5],
            frame_header[6],
            frame_header[7],
            frame_header[8],
            frame_header[9],
            frame_header[10],
            frame_header[11],
        ]);

        // Calculate timestamp in nanoseconds
        let timestamp_ns = if timebase_den > 0 {
            pts as u64 * 1_000_000_000 / timebase_den as u64
        } else {
            0
        };

        // Read frame data to determine frame type
        let header_read_size = frame_size.min(100);
        let mut frame_data = vec![0u8; header_read_size];
        reader
            .read_exact(&mut frame_data)
            .map_err(|e| format!("Failed to read frame data: {}", e))?;

        // Skip remaining frame data
        if frame_size > header_read_size {
            reader
                .seek(SeekFrom::Current((frame_size - header_read_size) as i64))
                .ok();
        }

        // Parse OBU header to determine frame type
        let obu_header = frame_data[0];
        let obu_type = (obu_header >> 3) & 0x0F;

        // Skip OBU header to get frame header payload
        let obu_header_size = 1 + ((obu_header & 0x04) != 0) as usize;
        let frame_header_payload = if obu_header_size < frame_data.len() {
            &frame_data[obu_header_size..]
        } else {
            &[]
        };

        // Parse frame header using bitvue-av1
        let frame_header = parse_frame_header_basic(frame_header_payload);

        // Determine frame type string
        let frame_type_str = match &frame_header {
            Ok(fh) => match fh.frame_type {
                FrameType::Key => "I".to_string(),
                FrameType::Inter => "P".to_string(),
                FrameType::IntraOnly => "I".to_string(),
                FrameType::Switch => "I".to_string(),
            },
            Err(_) => match obu_type {
                6 => "I".to_string(),
                _ => "P".to_string(),
            },
        };

        // Create unit for this frame
        let mut unit = UnitNode::new(stream_id, "FRAME".to_string(), frame_start, frame_size + 12);
        unit.frame_index = Some(frame_index);
        unit.frame_type = Some(frame_type_str.clone());
        unit.pts = Some(timestamp_ns);
        unit.display_name = format!(
            "Frame {} @ 0x{:08X} ({} bytes)",
            frame_index, frame_start, frame_size
        );

        // Store reference frame indices and QP if available
        if let Ok(fh) = &frame_header {
            // Convert ref_frame_idx from [u8; 3] to Vec<usize>
            if let Some(ref_idx) = fh.ref_frame_idx {
                unit.ref_frames = Some(ref_idx.iter().map(|&x| x as usize).collect());
            }
            if let Some(qp) = fh.base_q_idx {
                unit.qp_avg = Some(qp);
            }
        }

        units.push(unit);

        current_offset += 12 + frame_size as u64;
        frame_index += 1;

        if frame_index >= 10000 {
            break;
        }
    }

    tracing::info!("Parsed {} frames from IVF file", units.len());
    Ok(units)
}

fn load_file(args: Value, state: &AppState) -> Result<String> {
    let path = args["path"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("Missing path parameter"))?;

    let stream_str = args["stream"].as_str();
    let stream_id = parse_stream_id(stream_str);

    let path_buf = PathBuf::from(path);

    // Check if file exists
    if !path_buf.exists() {
        return Ok(json!({
            "success": false,
            "error": format!("File not found: {}", path)
        })
        .to_string());
    }

    // Get file size
    let file_size = path_buf.metadata().and_then(|m| Ok(m.len())).unwrap_or(0);

    // Get file extension
    let ext = path_buf
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("unknown");

    // Parse the file based on extension
    let units_result: Result<Vec<UnitNode>, String> = match ext {
        "ivf" | "av1" => {
            tracing::info!("Parsing IVF file: {}", path);
            parse_ivf_file(&path_buf, stream_id)
        }
        _ => {
            // For now, only IVF is supported
            return Ok(json!({
                "success": false,
                "error": format!("Unsupported format: {}. Only IVF is currently supported.", ext)
            })
            .to_string());
        }
    };

    match units_result {
        Ok(units) => {
            // Populate the stream state
            let core = state
                .core
                .lock()
                .map_err(|e| anyhow::anyhow!("Lock error: {}", e))?;

            let stream = core.get_stream(stream_id);
            let mut stream = stream.write();

            let unit_count = units.len();
            let frame_count = units.iter().filter(|u| u.frame_index.is_some()).count();

            stream.units = Some(UnitModel {
                units,
                unit_count,
                frame_count,
            });

            // Update loaded file state
            let mut loaded = state
                .loaded_file
                .lock()
                .map_err(|e| anyhow::anyhow!("Lock error: {}", e))?;
            *loaded = Some(path_buf.clone());

            Ok(json!({
                "success": true,
                "path": path,
                "stream": if stream_id == StreamId::A { "A" } else { "B" },
                "file_size": file_size,
                "format": ext,
                "frame_count": frame_count,
                "message": format!("Successfully loaded {} frames from {}", frame_count, path)
            })
            .to_string())
        }
        Err(e) => Ok(json!({
            "success": false,
            "error": e
        })
        .to_string()),
    }
}

fn get_stream_model(state: &AppState, stream_id: StreamId) -> Result<(UnitModel, PathBuf)> {
    let core = state
        .core
        .lock()
        .map_err(|e| anyhow::anyhow!("Lock error: {}", e))?;

    let stream = core.get_stream(stream_id);
    let stream = stream.read();

    let units = stream
        .units
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("No data loaded for stream. Use load_file first."))?;

    let loaded = state
        .loaded_file
        .lock()
        .map_err(|e| anyhow::anyhow!("Lock error: {}", e))?;

    let path = loaded
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("No file loaded"))?
        .clone();

    Ok((units.clone(), path))
}

fn analyze_frame(args: Value, state: &AppState) -> Result<String> {
    let frame_index = args["frame_index"]
        .as_u64()
        .ok_or_else(|| anyhow::anyhow!("Invalid frame_index"))? as usize;

    let stream_str = args["stream"].as_str();
    let stream_id = parse_stream_id(stream_str);

    let (units, path) = get_stream_model(state, stream_id)?;

    // Find the frame
    let frame = units
        .units
        .iter()
        .find(|u| u.frame_index == Some(frame_index))
        .ok_or_else(|| anyhow::anyhow!("Frame {} not found", frame_index))?;

    Ok(json!({
        "frame_index": frame_index,
        "stream": if stream_id == StreamId::A { "A" } else { "B" },
        "file": path.to_string_lossy(),
        "unit_type": frame.unit_type.clone(),
        "frame_type": frame.frame_type.clone().unwrap_or_else(|| "Unknown".to_string()),
        "offset": frame.offset,
        "size": frame.size,
        "pts": frame.pts,
        "dts": frame.dts,
        "qp_avg": frame.qp_avg,
        "ref_frames": frame.ref_frames.clone(),
        "display_name": frame.display_name.clone()
    })
    .to_string())
}

fn get_qp_map(args: Value, state: &AppState) -> Result<String> {
    let frame_index = args["frame_index"]
        .as_u64()
        .ok_or_else(|| anyhow::anyhow!("Invalid frame_index"))? as usize;

    let stream_str = args["stream"].as_str();
    let stream_id = parse_stream_id(stream_str);

    let (units, _path) = get_stream_model(state, stream_id)?;

    // Find the frame
    let frame = units
        .units
        .iter()
        .find(|u| u.frame_index == Some(frame_index))
        .ok_or_else(|| anyhow::anyhow!("Frame {} not found", frame_index))?;

    let qp = frame.qp_avg.unwrap_or(0);

    // Collect QP statistics across all frames
    let qp_values: Vec<u8> = units.units.iter().filter_map(|u| u.qp_avg).collect();

    let qp_min = qp_values.iter().copied().min().unwrap_or(0);
    let qp_max = qp_values.iter().copied().max().unwrap_or(0);
    let qp_avg = if qp_values.is_empty() {
        0
    } else {
        qp_values.iter().map(|&x| x as u32).sum::<u32>() / qp_values.len() as u32
    } as u8;

    Ok(json!({
        "frame_index": frame_index,
        "frame_qp": qp,
        "stream_qp_stats": {
            "min": qp_min,
            "max": qp_max,
            "avg": qp_avg
        },
        "note": "QP data is per-frame average. Block-level QP requires bitstream parsing."
    })
    .to_string())
}

fn get_motion_vectors(args: Value, state: &AppState) -> Result<String> {
    let frame_index = args["frame_index"]
        .as_u64()
        .ok_or_else(|| anyhow::anyhow!("Invalid frame_index"))? as usize;

    let stream_str = args["stream"].as_str();
    let stream_id = parse_stream_id(stream_str);

    let (units, _path) = get_stream_model(state, stream_id)?;

    // Find the frame
    let frame = units
        .units
        .iter()
        .find(|u| u.frame_index == Some(frame_index))
        .ok_or_else(|| anyhow::anyhow!("Frame {} not found", frame_index))?;

    let ref_idx = frame.ref_frames.clone().unwrap_or_else(|| vec![]);

    Ok(json!({
        "frame_index": frame_index,
        "frame_type": frame.frame_type.clone().unwrap_or_else(|| "Unknown".to_string()),
        "ref_frames": ref_idx,
        "note": "Motion vector extraction requires bitstream-level parsing. Currently showing reference frame indices."
    }).to_string())
}

fn compare_streams(args: Value, state: &AppState) -> Result<String> {
    let frame_index = args["frame_index"]
        .as_u64()
        .ok_or_else(|| anyhow::anyhow!("Invalid frame_index"))? as usize;

    // Get both streams
    let (units_a, path_a) = match get_stream_model(state, StreamId::A) {
        Ok(u) => u,
        Err(e) => {
            return Ok(json!({
                "message": format!("Stream A error: {}", e),
                "note": "Load a file first using load_file"
            })
            .to_string())
        }
    };
    let (units_b, path_b) = match get_stream_model(state, StreamId::B) {
        Ok(u) => u,
        Err(_) => {
            return Ok(json!({
                "message": "Stream B not loaded. Load a second file with load_file to compare.",
                "stream_a": {
                    "file": path_a.to_string_lossy(),
                    "frame_count": units_a.frame_count
                }
            })
            .to_string())
        }
    };

    let frame_a = units_a
        .units
        .iter()
        .find(|u| u.frame_index == Some(frame_index));

    let frame_b = units_b
        .units
        .iter()
        .find(|u| u.frame_index == Some(frame_index));

    match (frame_a, frame_b) {
        (Some(fa), Some(fb)) => {
            let size_diff = if fa.size > 0 && fb.size > 0 {
                (fa.size as f64 - fb.size as f64) / fb.size as f64 * 100.0
            } else {
                0.0
            };

            let qp_a = fa.qp_avg.unwrap_or(0);
            let qp_b = fb.qp_avg.unwrap_or(0);

            Ok(json!({
                "frame_index": frame_index,
                "stream_a": {
                    "file": path_a.to_string_lossy(),
                    "type": fa.frame_type.clone().unwrap_or_else(|| "Unknown".to_string()),
                    "size": fa.size,
                    "qp": qp_a,
                    "offset": fa.offset
                },
                "stream_b": {
                    "file": path_b.to_string_lossy(),
                    "type": fb.frame_type.clone().unwrap_or_else(|| "Unknown".to_string()),
                    "size": fb.size,
                    "qp": qp_b,
                    "offset": fb.offset
                },
                "comparison": {
                    "size_diff_percent": size_diff,
                    "qp_diff": qp_a as i16 - qp_b as i16,
                    "size_larger": if size_diff > 0.0 { "A" } else if size_diff < 0.0 { "B" } else { "Equal" },
                    "quality_note": if qp_a < qp_b {
                        "Stream A has lower QP (higher quality)"
                    } else if qp_a > qp_b {
                        "Stream B has lower QP (higher quality)"
                    } else {
                        "Both streams have similar QP"
                    }
                }
            }).to_string())
        }
        (Some(fa), None) => Ok(json!({
            "message": format!("Frame {} exists in Stream A but not in Stream B", frame_index),
            "stream_a": {
                "type": fa.frame_type.clone().unwrap_or_else(|| "Unknown".to_string()),
                "size": fa.size
            }
        })
        .to_string()),
        (None, Some(fb)) => Ok(json!({
            "message": format!("Frame {} exists in Stream B but not in Stream A", frame_index),
            "stream_b": {
                "type": fb.frame_type.clone().unwrap_or_else(|| "Unknown".to_string()),
                "size": fb.size
            }
        })
        .to_string()),
        (None, None) => Ok(json!({
            "message": format!("Frame {} not found in either stream", frame_index)
        })
        .to_string()),
    }
}

fn get_gop_structure(args: Value, state: &AppState) -> Result<String> {
    let stream_str = args["stream"].as_str();
    let stream_id = parse_stream_id(stream_str);

    let max_frames = args["max_frames"].as_u64().unwrap_or(100) as usize;

    let (units, path) = get_stream_model(state, stream_id)?;

    let frames: Vec<Value> = units
        .units
        .iter()
        .filter(|u| u.frame_index.is_some())
        .take(max_frames)
        .map(|u| {
            json!({
                "frame_index": u.frame_index,
                "type": u.frame_type.clone().unwrap_or_else(|| "Unknown".to_string()),
                "size": u.size,
                "qp": u.qp_avg,
                "pts": u.pts,
                "ref_frames": u.ref_frames.clone()
            })
        })
        .collect();

    let i_count = frames.iter().filter(|f| f["type"] == "I").count();
    let p_count = frames.iter().filter(|f| f["type"] == "P").count();
    let b_count = frames.iter().filter(|f| f["type"] == "B").count();

    Ok(json!({
        "file": path.to_string_lossy(),
        "stream": if stream_id == StreamId::A { "A" } else { "B" },
        "total_frames": units.frame_count,
        "shown_frames": frames.len(),
        "frame_type_counts": {
            "I": i_count,
            "P": p_count,
            "B": b_count
        },
        "frames": frames
    })
    .to_string())
}

fn find_decoding_issues(args: Value, state: &AppState) -> Result<String> {
    let stream_str = args["stream"].as_str();
    let stream_id = parse_stream_id(stream_str);

    let (units, path) = get_stream_model(state, stream_id)?;

    let mut issues = Vec::new();
    let mut warnings = Vec::new();

    // Check for unusually large or small frames
    let sizes: Vec<usize> = units.units.iter().map(|u| u.size).collect();
    if !sizes.is_empty() {
        let avg_size = sizes.iter().map(|&s| s).sum::<usize>() / sizes.len();
        let max_size = *sizes.iter().max().unwrap();
        let min_size = *sizes.iter().min().unwrap();

        if max_size > avg_size * 10 {
            warnings.push(format!(
                "Found unusually large frame ({} bytes vs avg {} bytes)",
                max_size, avg_size
            ));
        }

        if min_size < avg_size / 10 && min_size > 0 {
            warnings.push(format!(
                "Found unusually small frame ({} bytes vs avg {} bytes)",
                min_size, avg_size
            ));
        }
    }

    // Check for missing QP data
    let qp_missing = units
        .units
        .iter()
        .filter(|u| u.frame_index.is_some() && u.qp_avg.is_none())
        .count();

    if qp_missing > 0 {
        warnings.push(format!("QP data not available for {} frames", qp_missing));
    }

    // Check file path
    if path.to_string_lossy().contains(".html") {
        issues.push("File appears to be an HTML file, not a valid video file".to_string());
    }

    Ok(json!({
        "file": path.to_string_lossy(),
        "stream": if stream_id == StreamId::A { "A" } else { "B" },
        "frames_checked": units.frame_count,
        "issues": issues,
        "warnings": warnings,
        "status": if issues.is_empty() { "No critical issues found" } else { "Issues detected" }
    })
    .to_string())
}

fn get_stream_info(args: Value, state: &AppState) -> Result<String> {
    let stream_str = args["stream"].as_str();
    let stream_id = parse_stream_id(stream_str);

    let (units, path) = get_stream_model(state, stream_id)?;

    // Calculate total size
    let total_size: u64 = units.units.iter().map(|u| u.size as u64).sum();

    // Get file extension
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("unknown");

    Ok(json!({
        "file": path.to_string_lossy(),
        "stream": if stream_id == StreamId::A { "A" } else { "B" },
        "format": ext,
        "frame_count": units.frame_count,
        "unit_count": units.unit_count,
        "total_size": total_size,
        "avg_frame_size": if units.frame_count > 0 {
            total_size / units.frame_count as u64
        } else {
            0
        }
    })
    .to_string())
}

fn search_syntax(args: Value, state: &AppState) -> Result<String> {
    let frame_type_filter = args["frame_type"].as_str();
    let min_qp = args["min_qp"].as_u64().map(|x| x as u8);
    let max_qp = args["max_qp"].as_u64().map(|x| x as u8);
    let stream_str = args["stream"].as_str();
    let stream_id = parse_stream_id(stream_str);
    let limit = args["limit"].as_u64().unwrap_or(50) as usize;

    let (units, _path) = get_stream_model(state, stream_id)?;

    let mut results: Vec<Value> = Vec::new();

    for unit in units.units.iter() {
        if results.len() >= limit {
            break;
        }

        let frame_idx = unit.frame_index;
        let ftype = unit.frame_type.as_ref().map(|s| s.as_str());

        // Apply filters
        if let Some(filter) = frame_type_filter {
            if filter != "all" {
                if let Some(ft) = ftype {
                    if ft != filter {
                        continue;
                    }
                } else {
                    continue;
                }
            }
        }

        if let Some(qp) = unit.qp_avg {
            if let Some(min) = min_qp {
                if qp < min {
                    continue;
                }
            }
            if let Some(max) = max_qp {
                if qp > max {
                    continue;
                }
            }
        }

        results.push(json!({
            "frame_index": frame_idx,
            "type": ftype,
            "size": unit.size,
            "qp": unit.qp_avg,
            "offset": unit.offset
        }));
    }

    Ok(json!({
        "query": {
            "frame_type": frame_type_filter,
            "min_qp": min_qp,
            "max_qp": max_qp
        },
        "results_count": results.len(),
        "results": results
    })
    .to_string())
}

fn list_files(state: &AppState) -> Result<String> {
    let loaded = state
        .loaded_file
        .lock()
        .map_err(|e| anyhow::anyhow!("Lock error: {}", e))?;

    let core = state
        .core
        .lock()
        .map_err(|e| anyhow::anyhow!("Lock error: {}", e))?;

    let mut files = Vec::new();

    // Check Stream A
    let stream_a = core.get_stream(StreamId::A);
    let stream_a = stream_a.read();

    if let Some(units) = &stream_a.units {
        files.push(json!({
            "stream": "A",
            "frame_count": units.frame_count,
            "loaded": true
        }));
    }

    // Check Stream B
    let stream_b = core.get_stream(StreamId::B);
    let stream_b = stream_b.read();

    if let Some(units) = &stream_b.units {
        files.push(json!({
            "stream": "B",
            "frame_count": units.frame_count,
            "loaded": true
        }));
    }

    Ok(json!({
        "loaded_file": loaded.as_ref().map(|p| p.to_string_lossy().to_string()),
        "streams": files
    })
    .to_string())
}

// ============================================================================
// Main
// ============================================================================

fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter("bitvue_mcp=debug,info")
        .init();

    tracing::info!("bitvue MCP Server starting...");

    let state = AppState::new();
    let stdin = io::stdin();
    let stdout = io::stdout();
    let reader = BufReader::new(stdin.lock());
    let mut writer = stdout.lock();

    // Read JSON-RPC requests line by line from stdin
    for line in reader.lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }

        // Parse request
        let request: McpRequest = match serde_json::from_str(&line) {
            Ok(r) => r,
            Err(e) => {
                tracing::error!("Failed to parse request: {}", e);
                continue;
            }
        };

        tracing::debug!("Received request: {}", request.method);

        // Handle request
        let response = handle_request(request, &state);

        // Write response
        let response_json = response.to_json()?;
        writeln!(writer, "{}", response_json)?;
        writer.flush()?;
    }

    Ok(())
}
