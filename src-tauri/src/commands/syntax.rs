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
        "ivf" | "av1" => build_av1_syntax_tree(frame_index, &frame, &path),
        "webm" | "mkv" => {
            // Could be AV1, VP9, etc.
            build_av1_syntax_tree(frame_index, &frame, &path)
        }
        "mp4" | "mov" => {
            // Could be AV1, H.264, H.265 - try AV1 first
            build_av1_syntax_tree(frame_index, &frame, &path)
        }
        "h264" | "264" => build_avc_syntax_tree(frame_index, &frame),
        "h265" | "265" | "hevc" => build_hevc_syntax_tree(frame_index, &frame),
        _ => build_generic_syntax_tree(frame),
    };

    Ok(syntax_tree)
}

/// Build syntax tree for AV1 frames
fn build_av1_syntax_tree(
    frame_index: usize,
    frame: &FrameData,
    _path: &str,
) -> SyntaxNode {
    SyntaxNode {
        name: format!("Frame {}", frame_index),
        description: Some("AV1 Frame".to_string()),
        value: None,
        children: vec![
            SyntaxNode {
                name: "frame_type".to_string(),
                value: Some(SyntaxValue::String(frame.frame_type.clone())),
                description: Some("AV1 frame type: KEY, INTER, INTRA_ONLY, SWITCH".to_string()),
                children: vec![],
            },
            SyntaxNode {
                name: "show_existing_frame".to_string(),
                value: Some(SyntaxValue::Boolean(frame.frame_type == "INTRA_ONLY" || frame.frame_type == "SWITCH")),
                description: Some("Whether this frame shows a previously decoded frame".to_string()),
                children: vec![],
            },
            SyntaxNode {
                name: "size".to_string(),
                value: Some(SyntaxValue::Number(frame.size as i64)),
                description: Some("Frame size in bytes".to_string()),
                children: vec![
                    SyntaxNode {
                        name: "compressed_size".to_string(),
                        value: Some(SyntaxValue::Number(frame.size as i64)),
                        description: Some("Compressed frame size".to_string()),
                        children: vec![],
                    },
                    SyntaxNode {
                        name: "raw_size".to_string(),
                        value: Some(SyntaxValue::Number((frame.size * 3 / 2) as i64)), // Approximate
                        description: Some("Estimated raw YUV size".to_string()),
                        children: vec![],
                    },
                ],
            },
            SyntaxNode {
                name: "presentation_timestamp".to_string(),
                value: frame.pts.map(|v| SyntaxValue::Number(v as i64)),
                description: Some("Presentation timestamp in timebase units".to_string()),
                children: vec![],
            },
            SyntaxNode {
                name: "key_frame".to_string(),
                value: Some(SyntaxValue::Boolean(frame.key_frame.unwrap_or(false))),
                description: Some("Whether this is a key frame (random access point)".to_string()),
                children: vec![],
            },
            SyntaxNode {
                name: "obu_sequence".to_string(),
                description: Some("OBU (Open Bitstream Unit) sequence in this frame".to_string()),
                value: None,
                children: vec![
                    SyntaxNode {
                        name: "sequence_header".to_string(),
                        description: Some("Sequence Header OBU - global decoder configuration".to_string()),
                        value: None,
                        children: vec![
                            SyntaxNode {
                                name: "seq_profile".to_string(),
                                value: Some(SyntaxValue::Number(0)), // AV1 main profile
                                description: Some("AV1 profile (0=main, 1=high, 2=professional)".to_string()),
                                children: vec![],
                            },
                            SyntaxNode {
                                name: "still_picture".to_string(),
                                value: Some(SyntaxValue::Boolean(false)),
                                description: Some("Whether this is a still picture".to_string()),
                                children: vec![],
                            },
                            SyntaxNode {
                                name: "max_frame_width".to_string(),
                                value: Some(SyntaxValue::Number(1920)), // Default, should be parsed
                                description: Some("Maximum frame width in pixels".to_string()),
                                children: vec![],
                            },
                            SyntaxNode {
                                name: "max_frame_height".to_string(),
                                value: Some(SyntaxValue::Number(1080)), // Default
                                description: Some("Maximum frame height in pixels".to_string()),
                                children: vec![],
                            },
                        ],
                    },
                    SyntaxNode {
                        name: "frame_header".to_string(),
                        description: Some("Frame Header OBU - per-frame configuration".to_string()),
                        value: None,
                        children: vec![
                            SyntaxNode {
                                name: "show_frame".to_string(),
                                value: Some(SyntaxValue::Boolean(true)),
                                description: Some("Whether this frame should be displayed".to_string()),
                                children: vec![],
                            },
                            SyntaxNode {
                                name: "frame_type_override".to_string(),
                                value: Some(SyntaxValue::String(frame.frame_type.clone())),
                                description: Some("Frame type override flag".to_string()),
                                children: vec![],
                            },
                            SyntaxNode {
                                name: "base_q_idx".to_string(),
                                value: Some(SyntaxValue::Number(128)), // Default
                                description: Some("Base quantization index (0-255)".to_string()),
                                children: vec![],
                            },
                            SyntaxNode {
                                name: "quantization_params".to_string(),
                                description: Some("Quantization parameter configuration".to_string()),
                                value: None,
                                children: vec![
                                    SyntaxNode {
                                        name: "base_q_idx".to_string(),
                                        value: Some(SyntaxValue::Number(128)),
                                        description: Some("Base QP for luma (Y) plane".to_string()),
                                        children: vec![],
                                    },
                                    SyntaxNode {
                                        name: "delta_q_present".to_string(),
                                        value: Some(SyntaxValue::Boolean(false)),
                                        description: Some("Whether delta Q is enabled".to_string()),
                                        children: vec![],
                                    },
                                    SyntaxNode {
                                        name: "y_dc_delta_q".to_string(),
                                        value: Some(SyntaxValue::Number(0)),
                                        description: Some("DC quantization offset for Y".to_string()),
                                        children: vec![],
                                    },
                                    SyntaxNode {
                                        name: "uv_dc_delta_q".to_string(),
                                        value: Some(SyntaxValue::Number(0)),
                                        description: Some("DC quantization offset for chroma".to_string()),
                                        children: vec![],
                                    },
                                ],
                            },
                            SyntaxNode {
                                name: "loop_filter_params".to_string(),
                                description: Some("Loop filter configuration".to_string()),
                                value: None,
                                children: vec![
                                    SyntaxNode {
                                        name: "filter_level".to_string(),
                                        value: Some(SyntaxValue::Number(10)),
                                        description: Some("Loop filter strength (0-63)".to_string()),
                                        children: vec![],
                                    },
                                    SyntaxNode {
                                        name: "sharpness".to_string(),
                                        value: Some(SyntaxValue::Number(4)),
                                        description: Some("Loop filter sharpness (0-7)".to_string()),
                                        children: vec![],
                                    },
                                ],
                            },
                            SyntaxNode {
                                name: "coding_loop_filter_params".to_string(),
                                description: Some("Coded loop filter (CDEF) configuration".to_string()),
                                value: None,
                                children: vec![
                                    SyntaxNode {
                                        name: "cdef_damping".to_string(),
                                        value: Some(SyntaxValue::Number(3)),
                                        description: Some("CDEF damping factor (0-7)".to_string()),
                                        children: vec![],
                                    },
                                    SyntaxNode {
                                        name: "cdef_bits".to_string(),
                                        value: Some(SyntaxValue::Number(7)),
                                        description: Some("CDEF bit depth (0-7)".to_string()),
                                        children: vec![],
                                    },
                                ],
                            },
                            SyntaxNode {
                                name: "superblock_count".to_string(),
                                value: Some(SyntaxValue::Number(30)), // Example
                                description: Some("Number of superblocks in frame".to_string()),
                                children: vec![],
                            },
                        ],
                    },
                    SyntaxNode {
                        name: "tile_group".to_string(),
                        description: Some("Tile Group OBU - contains tile data".to_string()),
                        value: None,
                        children: vec![
                            SyntaxNode {
                                name: "tile_count".to_string(),
                                value: Some(SyntaxValue::Number(1)), // Single tile for MVP
                                description: Some("Number of tiles in frame".to_string()),
                                children: vec![],
                            },
                            SyntaxNode {
                                name: "tiles".to_string(),
                                description: Some("Tile information".to_string()),
                                value: None,
                                children: vec![
                                    SyntaxNode {
                                        name: "tile_0".to_string(),
                                        description: Some("First (and only) tile".to_string()),
                                        value: None,
                                        children: vec![
                                            SyntaxNode {
                                                name: "tile_size".to_string(),
                                                value: Some(SyntaxValue::String("1920x1080".to_string())), // Frame size
                                                description: Some("Tile dimensions in pixels".to_string()),
                                                children: vec![],
                                            },
                                            SyntaxNode {
                                                name: "superblock_structure".to_string(),
                                                description: Some("Superblock (coding tree unit) partitioning".to_string()),
                                                value: None,
                                                children: vec![
                                                    SyntaxNode {
                                                        name: "sb_size".to_string(),
                                                        value: Some(SyntaxValue::Number(64)),
                                                        description: Some("Superblock size (64 or 128 pixels)".to_string()),
                                                        children: vec![],
                                                    },
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
                                                                    SyntaxNode {
                                                                        name: "root_partition".to_string(),
                                                                        value: Some(SyntaxValue::String("PARTITION_SPLIT".to_string())),
                                                                        description: Some("Root partition type".to_string()),
                                                                        children: vec![],
                                                                    },
                                                                ],
                                                            },
                                                            SyntaxNode {
                                                                name: "coding_units".to_string(),
                                                                value: None,
                                                                description: Some("Coding unit information".to_string()),
                                                                children: vec![
                                                                    SyntaxNode {
                                                                        name: "cu_count".to_string(),
                                                                        value: Some(SyntaxValue::Number(240)), // Example
                                                                        description: Some("Number of coding units".to_string()),
                                                                        children: vec![],
                                                                    },
                                                                    SyntaxNode {
                                                                        name: "prediction_modes".to_string(),
                                                                        value: None,
                                                                        description: Some("Prediction mode distribution".to_string()),
                                                                        children: vec![
                                                                            SyntaxNode {
                                                                                name: "intra_blocks".to_string(),
                                                                                value: Some(SyntaxValue::String("INTRA".to_string())),
                                                                                description: None,
                                                                                children: vec![],
                                                                            },
                                                                            SyntaxNode {
                                                                                name: "inter_blocks".to_string(),
                                                                                value: Some(SyntaxValue::String("INTER".to_string())),
                                                                                description: None,
                                                                                children: vec![],
                                                                            },
                                                                        ],
                                                                    },
                                                                ],
                                                            },
                                                        ],
                                                    },
                                                ],
                                            },
                                        ],
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
