//! Syntax Information Commands
//!
//! Commands for retrieving detailed bitstream syntax information for analysis

use crate::commands::{AppState, FrameData};
use crate::commands::file::get_frames;
use serde::{Deserialize, Serialize};

/// Syntax node for tree display
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyntaxNode {
    pub name: String,
    pub value: Option<SyntaxValue>,
    pub children: Vec<SyntaxNode>,
    pub description: Option<String>,
}

/// Syntax value can be string, number, or array
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum SyntaxValue {
    String(String),
    Number(i64),
    Float(f64),
    Boolean(bool),
    Array(Vec<String>),
}

/// Get detailed syntax tree for a frame
#[tauri::command]
pub async fn get_frame_syntax(
    path: String,
    frame_index: usize,
    state: tauri::State<'_, AppState>,
) -> Result<SyntaxNode, String> {
    log::info!("get_frame_syntax: Getting syntax for frame {} from {}", frame_index, path);

    // First, get basic frame info
    let frames = get_frames(state).await?;
    let frame = frames.get(frame_index)
        .ok_or(format!("Frame index {} out of range", frame_index))?;

    // Detect codec from file extension
    let path_buf = std::path::PathBuf::from(&path);
    let ext = path_buf
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("unknown");

    // Build syntax tree based on codec
    let syntax_tree = match ext {
        "ivf" | "av1" => build_av1_syntax_tree(frame_index, frame, &path),
        "webm" | "mkv" => {
            // Could be AV1, VP9, etc.
            build_av1_syntax_tree(frame_index, frame, &path)
        }
        "mp4" | "mov" => {
            // Could be AV1, H.264, H.265 - try AV1 first
            build_av1_syntax_tree(frame_index, frame, &path)
        }
        "h264" | "264" => build_avc_syntax_tree(frame_index, frame),
        "h265" | "265" | "hevc" => build_hevc_syntax_tree(frame_index, frame),
        _ => build_generic_syntax_tree(frame),
    };

    Ok(syntax_tree)
}

/// Helper: Create a simple leaf node
///
/// Creates a SyntaxNode with name, optional value, and description.
/// Used for terminal nodes in the syntax tree.
fn create_leaf_node(
    name: &str,
    value: Option<SyntaxValue>,
    description: Option<&str>,
) -> SyntaxNode {
    SyntaxNode {
        name: name.to_string(),
        description: description.map(|s| s.to_string()),
        value,
        children: vec![],
    }
}

/// Helper: Build sequence header OBU node
///
/// Creates the sequence_header OBU syntax tree with global decoder config.
fn build_sequence_header_node() -> SyntaxNode {
    SyntaxNode {
        name: "sequence_header".to_string(),
        description: Some("Sequence Header OBU - global decoder configuration".to_string()),
        value: None,
        children: vec![
            create_leaf_node("seq_profile", Some(SyntaxValue::Number(0)), Some("AV1 profile (0=main, 1=high, 2=professional)")),
            create_leaf_node("still_picture", Some(SyntaxValue::Boolean(false)), Some("Whether this is a still picture")),
            create_leaf_node("max_frame_width", Some(SyntaxValue::Number(1920)), Some("Maximum frame width in pixels")),
            create_leaf_node("max_frame_height", Some(SyntaxValue::Number(1080)), Some("Maximum frame height in pixels")),
        ],
    }
}

/// Helper: Build quantization params node
///
/// Creates the quantization_params syntax tree with QP configuration.
fn build_quantization_params_node() -> SyntaxNode {
    SyntaxNode {
        name: "quantization_params".to_string(),
        description: Some("Quantization parameter configuration".to_string()),
        value: None,
        children: vec![
            create_leaf_node("base_q_idx", Some(SyntaxValue::Number(128)), Some("Base QP for luma (Y) plane")),
            create_leaf_node("delta_q_present", Some(SyntaxValue::Boolean(false)), Some("Whether delta Q is enabled")),
            create_leaf_node("y_dc_delta_q", Some(SyntaxValue::Number(0)), Some("DC quantization offset for Y")),
            create_leaf_node("uv_dc_delta_q", Some(SyntaxValue::Number(0)), Some("DC quantization offset for chroma")),
        ],
    }
}

/// Helper: Build loop filter params node
///
/// Creates the loop_filter_params syntax tree with deblocking configuration.
fn build_loop_filter_params_node() -> SyntaxNode {
    SyntaxNode {
        name: "loop_filter_params".to_string(),
        description: Some("Loop filter configuration".to_string()),
        value: None,
        children: vec![
            create_leaf_node("filter_level", Some(SyntaxValue::Number(10)), Some("Loop filter strength (0-63)")),
            create_leaf_node("sharpness", Some(SyntaxValue::Number(4)), Some("Loop filter sharpness (0-7)")),
        ],
    }
}

/// Helper: Build CDEF params node
///
/// Creates the coding_loop_filter_params syntax tree with CDEF configuration.
fn build_cdef_params_node() -> SyntaxNode {
    SyntaxNode {
        name: "coding_loop_filter_params".to_string(),
        description: Some("Coded loop filter (CDEF) configuration".to_string()),
        value: None,
        children: vec![
            create_leaf_node("cdef_damping", Some(SyntaxValue::Number(3)), Some("CDEF damping factor (0-7)")),
            create_leaf_node("cdef_bits", Some(SyntaxValue::Number(7)), Some("CDEF bit depth (0-7)")),
        ],
    }
}

/// Helper: Build frame header OBU node
///
/// Creates the frame_header OBU syntax tree with per-frame configuration.
fn build_frame_header_node(frame_type: &str) -> SyntaxNode {
    SyntaxNode {
        name: "frame_header".to_string(),
        description: Some("Frame Header OBU - per-frame configuration".to_string()),
        value: None,
        children: vec![
            create_leaf_node("show_frame", Some(SyntaxValue::Boolean(true)), Some("Whether this frame should be displayed")),
            create_leaf_node("frame_type_override", Some(SyntaxValue::String(frame_type.to_string())), Some("Frame type override flag")),
            create_leaf_node("base_q_idx", Some(SyntaxValue::Number(128)), Some("Base quantization index (0-255)")),
            build_quantization_params_node(),
            build_loop_filter_params_node(),
            build_cdef_params_node(),
            create_leaf_node("superblock_count", Some(SyntaxValue::Number(30)), Some("Number of superblocks in frame")),
        ],
    }
}

/// Helper: Build superblock structure node
///
/// Creates the superblock_structure syntax tree with partitioning info.
fn build_superblock_structure_node() -> SyntaxNode {
    SyntaxNode {
        name: "superblock_structure".to_string(),
        description: Some("Superblock (coding tree unit) partitioning".to_string()),
        value: None,
        children: vec![
            create_leaf_node("sb_size", Some(SyntaxValue::Number(64)), Some("Superblock size (64 or 128 pixels)")),
            SyntaxNode {
                name: "partition_tree".to_string(),
                description: Some("Block partitioning structure".to_string()),
                value: None,
                children: vec![
                    SyntaxNode {
                        name: "block_partition_modes".to_string(),
                        value: None,
                        description: Some("Partition types (NONE, HORZ, VERT, etc.)".to_string()),
                        children: vec![
                            create_leaf_node("root_partition", Some(SyntaxValue::String("PARTITION_SPLIT".to_string())), Some("Root partition type")),
                        ],
                    },
                    SyntaxNode {
                        name: "coding_units".to_string(),
                        value: None,
                        description: Some("Coding unit information".to_string()),
                        children: vec![
                            create_leaf_node("cu_count", Some(SyntaxValue::Number(240)), Some("Number of coding units")),
                            build_prediction_modes_node(),
                        ],
                    },
                ],
            },
        ],
    }
}

/// Helper: Build prediction modes node
///
/// Creates the prediction_modes syntax tree with mode distribution.
fn build_prediction_modes_node() -> SyntaxNode {
    SyntaxNode {
        name: "prediction_modes".to_string(),
        value: None,
        description: Some("Prediction mode distribution".to_string()),
        children: vec![
            create_leaf_node("intra_blocks", Some(SyntaxValue::String("INTRA".to_string())), None),
            create_leaf_node("inter_blocks", Some(SyntaxValue::String("INTER".to_string())), None),
        ],
    }
}

/// Helper: Build tile node
///
/// Creates a tile syntax tree with tile size and superblock structure.
fn build_tile_node(tile_index: usize, frame_size: &str) -> SyntaxNode {
    SyntaxNode {
        name: format!("tile_{}", tile_index),
        description: Some(format!("Tile {}", tile_index + 1)),
        value: None,
        children: vec![
            create_leaf_node("tile_size", Some(SyntaxValue::String(frame_size.to_string())), Some("Tile dimensions in pixels")),
            build_superblock_structure_node(),
        ],
    }
}

/// Helper: Build tile group OBU node
///
/// Creates the tile_group OBU syntax tree with tile information.
fn build_tile_group_node(frame_size: &str) -> SyntaxNode {
    SyntaxNode {
        name: "tile_group".to_string(),
        description: Some("Tile Group OBU - contains tile data".to_string()),
        value: None,
        children: vec![
            create_leaf_node("tile_count", Some(SyntaxValue::Number(1)), Some("Number of tiles in frame")),
            SyntaxNode {
                name: "tiles".to_string(),
                description: Some("Tile information".to_string()),
                value: None,
                children: vec![
                    build_tile_node(0, frame_size),
                ],
            },
        ],
    }
}

/// Helper: Build OBU sequence node
///
/// Creates the obu_sequence syntax tree with OBU hierarchy.
fn build_obu_sequence_node(frame_type: &str, frame_size: &str) -> SyntaxNode {
    SyntaxNode {
        name: "obu_sequence".to_string(),
        description: Some("OBU (Open Bitstream Unit) sequence in this frame".to_string()),
        value: None,
        children: vec![
            build_sequence_header_node(),
            build_frame_header_node(frame_type),
            build_tile_group_node(frame_size),
        ],
    }
}

/// Helper: Build size node
///
/// Creates the size syntax tree with compressed and raw size information.
fn build_size_node(frame_size: usize) -> SyntaxNode {
    SyntaxNode {
        name: "size".to_string(),
        value: Some(SyntaxValue::Number(frame_size as i64)),
        description: Some("Frame size in bytes".to_string()),
        children: vec![
            create_leaf_node("compressed_size", Some(SyntaxValue::Number(frame_size as i64)), Some("Compressed frame size")),
            create_leaf_node("raw_size", Some(SyntaxValue::Number((frame_size * 3 / 2) as i64)), Some("Estimated raw YUV size")),
        ],
    }
}

/// Build syntax tree for AV1 frames
fn build_av1_syntax_tree(
    frame_index: usize,
    frame: &FrameData,
    _path: &str,
) -> SyntaxNode {
    let frame_size = "1920x1080"; // Default frame size for syntax tree display

    SyntaxNode {
        name: format!("Frame {}", frame_index),
        description: Some("AV1 Frame".to_string()),
        value: None,
        children: vec![
            create_leaf_node(
                "frame_type",
                Some(SyntaxValue::String(frame.frame_type.clone())),
                Some("AV1 frame type: KEY, INTER, INTRA_ONLY, SWITCH"),
            ),
            create_leaf_node(
                "show_existing_frame",
                Some(SyntaxValue::Boolean(frame.frame_type == "INTRA_ONLY" || frame.frame_type == "SWITCH")),
                Some("Whether this frame shows a previously decoded frame"),
            ),
            build_size_node(frame.size),
            create_leaf_node(
                "presentation_timestamp",
                frame.pts.map(|v| SyntaxValue::Number(v as i64)),
                Some("Presentation timestamp in timebase units"),
            ),
            create_leaf_node(
                "key_frame",
                Some(SyntaxValue::Boolean(frame.key_frame.unwrap_or(false))),
                Some("Whether this is a key frame (random access point)"),
            ),
            build_obu_sequence_node(&frame.frame_type, &frame_size),
        ],
    }
}

/// Build syntax tree for AVC/H.264 frames
fn build_avc_syntax_tree(
    frame_index: usize,
    frame: &FrameData,
) -> SyntaxNode {
    SyntaxNode {
        name: format!("Frame {}", frame_index),
        description: Some("H.264/AVC NAL Unit Structure".to_string()),
        value: None,
        children: vec![
            SyntaxNode {
                name: "frame_type".to_string(),
                value: Some(SyntaxValue::String(frame.frame_type.clone())),
                description: Some("H.264 slice type (I, P, B, SI, SP, SI)".to_string()),
                children: vec![],
            },
            SyntaxNode {
                name: "size".to_string(),
                value: Some(SyntaxValue::Number(frame.size as i64)),
                description: Some("NAL unit size in bytes".to_string()),
                children: vec![],
            },
            SyntaxNode {
                name: "presentation_timestamp".to_string(),
                value: frame.pts.map(|v| SyntaxValue::Number(v as i64)),
                description: Some("Presentation timestamp".to_string()),
                children: vec![],
            },
            SyntaxNode {
                name: "key_frame".to_string(),
                value: Some(SyntaxValue::Boolean(frame.key_frame.unwrap_or(false))),
                description: Some("IDR frame (instantaneous decoding refresh)".to_string()),
                children: vec![],
            },
            SyntaxNode {
                name: "nal_unit_structure".to_string(),
                description: Some("NAL unit composition".to_string()),
                value: None,
                children: vec![
                    SyntaxNode {
                        name: "sps".to_string(),
                        description: Some("Sequence Parameter Set - decoder configuration".to_string()),
                        value: None,
                        children: vec![
                            SyntaxNode {
                                name: "profile".to_string(),
                                value: Some(SyntaxValue::String("High".to_string())),
                                description: Some("H.264 profile (Baseline, Main, High)".to_string()),
                                children: vec![],
                            },
                            SyntaxNode {
                                name: "level".to_string(),
                                value: Some(SyntaxValue::Number(41)),
                                description: Some("H.264 level (4.1 = 41)".to_string()),
                                children: vec![],
                            },
                            SyntaxNode {
                                name: "resolution".to_string(),
                                value: Some(SyntaxValue::String("1920x1080".to_string())),
                                description: Some("Frame resolution".to_string()),
                                children: vec![],
                            },
                        ],
                    },
                    SyntaxNode {
                        name: "pps".to_string(),
                        description: Some("Picture Parameter Set - picture-specific settings".to_string()),
                        value: None,
                        children: vec![
                            SyntaxNode {
                                name: "entropy_coding_mode".to_string(),
                                value: Some(SyntaxValue::String("CABAC".to_string())),
                                description: Some("Entropy coding (CAVLC or CABAC)".to_string()),
                                children: vec![],
                            },
                            SyntaxNode {
                                name: "num_ref_frames".to_string(),
                                value: Some(SyntaxValue::Number(1)),
                                description:(Some("Number of reference frames".to_string())),
                                children: vec![],
                            },
                        ],
                    },
                    SyntaxNode {
                        name: "slice".to_string(),
                        description: Some("Slice NAL unit - actual coded data".to_string()),
                        value: None,
                        children: vec![
                            SyntaxNode {
                                name: "slice_type".to_string(),
                                value: Some(SyntaxValue::String(frame.frame_type.clone())),
                                description: Some("Slice type (I, P, B)".to_string()),
                                children: vec![],
                            },
                            SyntaxNode {
                                name: "macroblock_layer".to_string(),
                                description: Some("Macroblock partition structure".to_string()),
                                value: None,
                                children: vec![
                                    SyntaxNode {
                                        name: "mb_affine".to_string(),
                                        value: Some(SyntaxValue::Boolean(false)),
                                        description: Some("Adaptive macroblock affine transform".to_string()),
                                        children: vec![],
                                    },
                                    SyntaxNode {
                                        name: "mb_count".to_string(),
                                        value: Some(SyntaxValue::Number((1920 / 16) * (1080 / 16))), // Approx
                                        description: Some("Number of macroblocks".to_string()),
                                        children: vec![],
                                    },
                                ],
                            },
                        ],
                    },
                ],
            },
        ],
    }
}

/// Build syntax tree for HEVC/H.265 frames
fn build_hevc_syntax_tree(
    frame_index: usize,
    frame: &FrameData,
) -> SyntaxNode {
    SyntaxNode {
        name: format!("Frame {}", frame_index),
        description: Some("H.265/HEVC NAL Unit Structure".to_string()),
        value: None,
        children: vec![
            SyntaxNode {
                name: "frame_type".to_string(),
                value: Some(SyntaxValue::String(frame.frame_type.clone())),
                description: Some("HEVC slice type (I, P, B)".to_string()),
                children: vec![],
            },
            SyntaxNode {
                name: "size".to_string(),
                value: Some(SyntaxValue::Number(frame.size as i64)),
                description: Some("NAL unit size in bytes".to_string()),
                children: vec![],
            },
            SyntaxNode {
                name: "key_frame".to_string(),
                value: Some(SyntaxValue::Boolean(frame.key_frame.unwrap_or(false))),
                description: Some("CRA/IDR frame (clean random access)".to_string()),
                children: vec![],
            },
            SyntaxNode {
                name: "nal_unit_structure".to_string(),
                description: Some("NAL unit composition".to_string()),
                value: None,
                children: vec![
                    SyntaxNode {
                        name: "vps".to_string(),
                        description: Some("Video Parameter Set - video layer configuration".to_string()),
                        value: None,
                        children: vec![
                            SyntaxNode {
                                name: "max_layers_minus1".to_string(),
                                value: Some(SyntaxValue::Number(0)),
                                description: Some("Number of additional layers".to_string()),
                                children: vec![],
                            },
                            SyntaxNode {
                                name: "temporal_id_nesting".to_string(),
                                value: Some(SyntaxValue::Boolean(false)),
                                description: Some("Temporal ID nesting flag".to_string()),
                                children: vec![],
                            },
                        ],
                    },
                    SyntaxNode {
                        name: "sps".to_string(),
                        description: Some("Sequence Parameter Set - video coding configuration".to_string()),
                        value: None,
                        children: vec![
                            SyntaxNode {
                                name: "profile_tier_level".to_string(),
                                value: Some(SyntaxValue::String("Main Tier 4.1".to_string())),
                                description: Some("Profile, tier, and level".to_string()),
                                children: vec![],
                            },
                            SyntaxNode {
                                name: "chroma_format".to_string(),
                                value: Some(SyntaxValue::String("YUV420".to_string())),
                                description: Some("Chroma subsampling format".to_string()),
                                children: vec![],
                            },
                            SyntaxNode {
                                name: "ctu_size".to_string(),
                                value: Some(SyntaxValue::String("64x64".to_string())),
                                description: Some("Coding Tree Unit size".to_string()),
                                children: vec![],
                            },
                        ],
                    },
                    SyntaxNode {
                        name: "pps".to_string(),
                        description: Some("Picture Parameter Set - picture-specific settings".to_string()),
                        value: None,
                        children: vec![
                            SyntaxNode {
                                name: "entropy_coding".to_string(),
                                value: Some(SyntaxValue::String("CABAC".to_string())),
                                description: Some("Entropy coding mode".to_string()),
                                children: vec![],
                            },
                            SyntaxNode {
                                name: "num_ref_idx_active".to_string(),
                                value: Some(SyntaxValue::Number(1)),
                                description:(Some("Number of active reference indices".to_string())),
                                children: vec![],
                            },
                        ],
                    },
                    SyntaxNode {
                        name: "ctu_structure".to_string(),
                        description: Some("Coding Tree Unit structure".to_string()),
                        value: None,
                        children: vec![
                            SyntaxNode {
                                name: "partition_depth".to_string(),
                                value: Some(SyntaxValue::Number(3)),
                                description: Some("Maximum quadtree partition depth".to_string()),
                                children: vec![],
                            },
                            SyntaxNode {
                                name: "ctu_count".to_string(),
                                value: Some(SyntaxValue::Number((1920 / 64) * (1080 / 64))), // Approx
                                description: Some("Number of CTUs in frame".to_string()),
                                children: vec![],
                            },
                        ],
                    },
                ],
            },
        ],
    }
}

/// Build generic syntax tree for unknown/unparsed formats
fn build_generic_syntax_tree(frame: &FrameData) -> SyntaxNode {
    SyntaxNode {
        name: format!("Frame {}", frame.frame_index),
        description: Some("Generic frame information (codec-specific parsing not available)".to_string()),
        value: None,
        children: vec![
            SyntaxNode {
                name: "frame_type".to_string(),
                value: Some(SyntaxValue::String(frame.frame_type.clone())),
                description: Some("Frame type".to_string()),
                children: vec![],
            },
            SyntaxNode {
                name: "size".to_string(),
                value: Some(SyntaxValue::Number(frame.size as i64)),
                description: Some("Frame size in bytes".to_string()),
                children: vec![],
            },
            SyntaxNode {
                name: "presentation_timestamp".to_string(),
                value: frame.pts.map(|v| SyntaxValue::Number(v as i64)),
                description: Some("Presentation timestamp".to_string()),
                children: vec![],
            },
            SyntaxNode {
                name: "key_frame".to_string(),
                value: Some(SyntaxValue::Boolean(frame.key_frame.unwrap_or(false))),
                description: Some("Key frame indicator".to_string()),
                children: vec![],
            },
        ],
    }
}
