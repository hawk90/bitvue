//! The tool catalogue advertised by `tools/list` (names, descriptions, input schemas).

use crate::protocol::Tool;
use serde_json::json;

/// Available MCP tools
pub(crate) fn get_tools() -> Vec<Tool> {
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
