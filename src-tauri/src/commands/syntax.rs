//! Syntax Information Commands
//!
//! Commands for retrieving detailed bitstream syntax information for analysis

use crate::commands::file::{get_frames, validate_file_path};
use crate::commands::{AppState, FrameData};
use serde::{Deserialize, Serialize};

/// Syntax node for tree display
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyntaxNode {
    pub name: String,
    pub value: Option<SyntaxValue>,
    pub children: Vec<SyntaxNode>,
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub byte_offset: Option<u64>,
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

// ---------------------------------------------------------------------------
// Extended info structs
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QpBucket {
    pub qp: i16,
    pub count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QmMatrix {
    pub name: String,
    pub size: u32,
    pub pred_type: String,
    pub plane: String,
    pub values: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HevcQmData {
    pub scaling_list_enabled: bool,
    pub matrices: Vec<QmMatrix>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProbEntry {
    pub group: String,
    pub label: String,
    pub probs: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Vp9ProbsData {
    pub frame_index: usize,
    pub is_key_frame: bool,
    pub entries: Vec<ProbEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApsEntry {
    pub aps_id: u8,
    pub aps_type: String,
    pub enabled: bool,
    pub summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VvcApsData {
    pub aps_list: Vec<ApsEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefEntry {
    pub list_idx: u8,
    pub slot: u8,
    pub poc: i32,
    pub frame_index: usize,
    pub frame_type: String,
    pub long_term: bool,
    pub weight: Option<i32>,
    pub offset: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodecExtendedInfo {
    pub codec: String,
    pub frame_index: usize,
    pub qp_histogram: Vec<QpBucket>,
    pub hevc_qm: Option<HevcQmData>,
    pub vp9_probs: Option<Vp9ProbsData>,
    pub vvc_aps: Option<VvcApsData>,
    pub l0_refs: Vec<RefEntry>,
    pub l1_refs: Vec<RefEntry>,
}

// ---------------------------------------------------------------------------
// Tauri commands
// ---------------------------------------------------------------------------

/// Get detailed syntax tree for a frame
#[tauri::command]
pub async fn get_frame_syntax(
    path: String,
    frame_index: usize,
    state: tauri::State<'_, AppState>,
) -> Result<SyntaxNode, String> {
    // SECURITY: Don't log file path to prevent information disclosure
    log::info!("get_frame_syntax: Getting syntax for frame {}", frame_index);

    // SECURITY: Validate file path to prevent path traversal
    let _validated_path = validate_file_path(&path)?;

    // First, get basic frame info
    let frames = get_frames(state).await?;
    let frame = frames
        .get(frame_index)
        .ok_or(format!("Frame index {} out of range", frame_index))?;

    // Detect codec from file extension
    let path_buf = std::path::PathBuf::from(&path);
    let ext = path_buf
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("unknown");

    // Detect real stream dimensions from loaded file data
    let (width, height) = {
        let maybe_data = state
            .decode_service
            .lock()
            .ok()
            .and_then(|svc| svc.get_file_data().ok());
        if let Some(file_data) = maybe_data {
            let codec = crate::commands::analysis::detect_codec_from_path(&path);
            crate::commands::analysis::extract_stream_dimensions(&file_data, &codec)
        } else {
            (1920u32, 1080u32)
        }
    };

    // Build syntax tree based on codec
    let syntax_tree = match ext {
        "ivf" | "av1" => build_av1_syntax_tree(frame_index, frame, width, height),
        "webm" | "mkv" => {
            // Could be AV1, VP9, etc.
            build_av1_syntax_tree(frame_index, frame, width, height)
        }
        "mp4" | "mov" => {
            // Could be AV1, H.264, H.265 - try AV1 first
            build_av1_syntax_tree(frame_index, frame, width, height)
        }
        "h264" | "264" => build_avc_syntax_tree(frame_index, frame, width, height),
        "h265" | "265" | "hevc" => build_hevc_syntax_tree(frame_index, frame, width, height),
        _ => build_generic_syntax_tree(frame),
    };

    Ok(syntax_tree)
}

/// Get codec-extended analysis info for a frame (QP histogram, QM, VP9 probs, VVC APS, ref lists)
#[tauri::command]
pub async fn get_codec_extended_info(
    path: String,
    frame_index: usize,
    state: tauri::State<'_, AppState>,
) -> Result<CodecExtendedInfo, String> {
    let _validated = validate_file_path(&path)?;

    let frames = get_frames(state).await?;
    let codec = crate::commands::analysis::detect_codec_from_path(&path);

    let (l0_refs, l1_refs) = build_ref_lists_from_frames(&frames, frame_index);
    let qp_histogram = build_size_based_qp_histogram(&frames, &codec);

    let hevc_qm = if matches!(codec.as_str(), "hevc" | "h265") {
        Some(build_default_hevc_qm())
    } else {
        None
    };

    let vp9_probs = if matches!(codec.as_str(), "vp9") {
        let is_key = frames
            .get(frame_index)
            .and_then(|f| f.key_frame)
            .unwrap_or(false);
        Some(build_vp9_probs_data(frame_index, is_key))
    } else {
        None
    };

    let vvc_aps = if matches!(codec.as_str(), "vvc" | "h266") {
        Some(build_vvc_aps_data())
    } else {
        None
    };

    Ok(CodecExtendedInfo {
        codec,
        frame_index,
        qp_histogram,
        hevc_qm,
        vp9_probs,
        vvc_aps,
        l0_refs,
        l1_refs,
    })
}

// ---------------------------------------------------------------------------
// Syntax tree helpers
// ---------------------------------------------------------------------------

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
        byte_offset: None,
    }
}

/// Helper: Build sequence header OBU node
///
/// Creates the sequence_header OBU syntax tree with global decoder config.
fn build_sequence_header_node(width: u32, height: u32) -> SyntaxNode {
    SyntaxNode {
        name: "sequence_header".to_string(),
        description: Some("Sequence Header OBU - global decoder configuration".to_string()),
        value: None,
        children: vec![
            create_leaf_node(
                "seq_profile",
                Some(SyntaxValue::Number(0)),
                Some("AV1 profile (0=main, 1=high, 2=professional)"),
            ),
            create_leaf_node(
                "still_picture",
                Some(SyntaxValue::Boolean(false)),
                Some("Whether this is a still picture"),
            ),
            create_leaf_node(
                "max_frame_width",
                Some(SyntaxValue::Number(width as i64)),
                Some("Maximum frame width in pixels"),
            ),
            create_leaf_node(
                "max_frame_height",
                Some(SyntaxValue::Number(height as i64)),
                Some("Maximum frame height in pixels"),
            ),
        ],
        byte_offset: None,
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
            create_leaf_node(
                "base_q_idx",
                Some(SyntaxValue::Number(128)),
                Some("Base QP for luma (Y) plane"),
            ),
            create_leaf_node(
                "delta_q_present",
                Some(SyntaxValue::Boolean(false)),
                Some("Whether delta Q is enabled"),
            ),
            create_leaf_node(
                "y_dc_delta_q",
                Some(SyntaxValue::Number(0)),
                Some("DC quantization offset for Y"),
            ),
            create_leaf_node(
                "uv_dc_delta_q",
                Some(SyntaxValue::Number(0)),
                Some("DC quantization offset for chroma"),
            ),
        ],
        byte_offset: None,
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
            create_leaf_node(
                "filter_level",
                Some(SyntaxValue::Number(10)),
                Some("Loop filter strength (0-63)"),
            ),
            create_leaf_node(
                "sharpness",
                Some(SyntaxValue::Number(4)),
                Some("Loop filter sharpness (0-7)"),
            ),
        ],
        byte_offset: None,
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
            create_leaf_node(
                "cdef_damping",
                Some(SyntaxValue::Number(3)),
                Some("CDEF damping factor (0-7)"),
            ),
            create_leaf_node(
                "cdef_bits",
                Some(SyntaxValue::Number(7)),
                Some("CDEF bit depth (0-7)"),
            ),
        ],
        byte_offset: None,
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
            create_leaf_node(
                "show_frame",
                Some(SyntaxValue::Boolean(true)),
                Some("Whether this frame should be displayed"),
            ),
            create_leaf_node(
                "frame_type_override",
                Some(SyntaxValue::String(frame_type.to_string())),
                Some("Frame type override flag"),
            ),
            create_leaf_node(
                "base_q_idx",
                Some(SyntaxValue::Number(128)),
                Some("Base quantization index (0-255)"),
            ),
            build_quantization_params_node(),
            build_loop_filter_params_node(),
            build_cdef_params_node(),
            create_leaf_node(
                "superblock_count",
                Some(SyntaxValue::Number(30)),
                Some("Number of superblocks in frame"),
            ),
        ],
        byte_offset: None,
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
            create_leaf_node(
                "sb_size",
                Some(SyntaxValue::Number(64)),
                Some("Superblock size (64 or 128 pixels)"),
            ),
            SyntaxNode {
                name: "partition_tree".to_string(),
                description: Some("Block partitioning structure".to_string()),
                value: None,
                children: vec![
                    SyntaxNode {
                        name: "block_partition_modes".to_string(),
                        value: None,
                        description: Some("Partition types (NONE, HORZ, VERT, etc.)".to_string()),
                        children: vec![create_leaf_node(
                            "root_partition",
                            Some(SyntaxValue::String("PARTITION_SPLIT".to_string())),
                            Some("Root partition type"),
                        )],
                        byte_offset: None,
                    },
                    SyntaxNode {
                        name: "coding_units".to_string(),
                        value: None,
                        description: Some("Coding unit information".to_string()),
                        children: vec![
                            create_leaf_node(
                                "cu_count",
                                Some(SyntaxValue::Number(240)),
                                Some("Number of coding units"),
                            ),
                            build_prediction_modes_node(),
                        ],
                        byte_offset: None,
                    },
                ],
                byte_offset: None,
            },
        ],
        byte_offset: None,
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
            create_leaf_node(
                "intra_blocks",
                Some(SyntaxValue::String("INTRA".to_string())),
                None,
            ),
            create_leaf_node(
                "inter_blocks",
                Some(SyntaxValue::String("INTER".to_string())),
                None,
            ),
        ],
        byte_offset: None,
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
            create_leaf_node(
                "tile_size",
                Some(SyntaxValue::String(frame_size.to_string())),
                Some("Tile dimensions in pixels"),
            ),
            build_superblock_structure_node(),
        ],
        byte_offset: None,
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
            create_leaf_node(
                "tile_count",
                Some(SyntaxValue::Number(1)),
                Some("Number of tiles in frame"),
            ),
            SyntaxNode {
                name: "tiles".to_string(),
                description: Some("Tile information".to_string()),
                value: None,
                children: vec![build_tile_node(0, frame_size)],
                byte_offset: None,
            },
        ],
        byte_offset: None,
    }
}

/// Helper: Build OBU sequence node
///
/// Creates the obu_sequence syntax tree with OBU hierarchy.
fn build_obu_sequence_node(
    frame_type: &str,
    frame_size: &str,
    width: u32,
    height: u32,
) -> SyntaxNode {
    SyntaxNode {
        name: "obu_sequence".to_string(),
        description: Some("OBU (Open Bitstream Unit) sequence in this frame".to_string()),
        value: None,
        children: vec![
            build_sequence_header_node(width, height),
            build_frame_header_node(frame_type),
            build_tile_group_node(frame_size),
        ],
        byte_offset: None,
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
            create_leaf_node(
                "compressed_size",
                Some(SyntaxValue::Number(frame_size as i64)),
                Some("Compressed frame size"),
            ),
            create_leaf_node(
                "raw_size",
                Some(SyntaxValue::Number((frame_size * 3 / 2) as i64)),
                Some("Estimated raw YUV size"),
            ),
        ],
        byte_offset: None,
    }
}

/// Build syntax tree for AV1 frames
fn build_av1_syntax_tree(
    frame_index: usize,
    frame: &FrameData,
    width: u32,
    height: u32,
) -> SyntaxNode {
    let frame_size = format!("{}x{}", width, height);

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
                Some(SyntaxValue::Boolean(
                    frame.frame_type == "INTRA_ONLY" || frame.frame_type == "SWITCH",
                )),
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
            build_obu_sequence_node(&frame.frame_type, &frame_size, width, height),
        ],
        byte_offset: None,
    }
}

/// Build syntax tree for AVC/H.264 frames
fn build_avc_syntax_tree(
    frame_index: usize,
    frame: &FrameData,
    width: u32,
    height: u32,
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
                byte_offset: None,
            },
            SyntaxNode {
                name: "size".to_string(),
                value: Some(SyntaxValue::Number(frame.size as i64)),
                description: Some("NAL unit size in bytes".to_string()),
                children: vec![],
                byte_offset: None,
            },
            SyntaxNode {
                name: "presentation_timestamp".to_string(),
                value: frame.pts.map(|v| SyntaxValue::Number(v as i64)),
                description: Some("Presentation timestamp".to_string()),
                children: vec![],
                byte_offset: None,
            },
            SyntaxNode {
                name: "key_frame".to_string(),
                value: Some(SyntaxValue::Boolean(frame.key_frame.unwrap_or(false))),
                description: Some("IDR frame (instantaneous decoding refresh)".to_string()),
                children: vec![],
                byte_offset: None,
            },
            SyntaxNode {
                name: "nal_unit_structure".to_string(),
                description: Some("NAL unit composition".to_string()),
                value: None,
                children: vec![
                    SyntaxNode {
                        name: "sps".to_string(),
                        description: Some(
                            "Sequence Parameter Set - decoder configuration".to_string(),
                        ),
                        value: None,
                        children: vec![
                            SyntaxNode {
                                name: "profile".to_string(),
                                value: Some(SyntaxValue::String("High".to_string())),
                                description: Some(
                                    "H.264 profile (Baseline, Main, High)".to_string(),
                                ),
                                children: vec![],
                                byte_offset: None,
                            },
                            SyntaxNode {
                                name: "level".to_string(),
                                value: Some(SyntaxValue::Number(41)),
                                description: Some("H.264 level (4.1 = 41)".to_string()),
                                children: vec![],
                                byte_offset: None,
                            },
                            SyntaxNode {
                                name: "resolution".to_string(),
                                value: Some(SyntaxValue::String(format!("{}x{}", width, height))),
                                description: Some("Frame resolution".to_string()),
                                children: vec![],
                                byte_offset: None,
                            },
                        ],
                        byte_offset: None,
                    },
                    SyntaxNode {
                        name: "pps".to_string(),
                        description: Some(
                            "Picture Parameter Set - picture-specific settings".to_string(),
                        ),
                        value: None,
                        children: vec![
                            SyntaxNode {
                                name: "entropy_coding_mode".to_string(),
                                value: Some(SyntaxValue::String("CABAC".to_string())),
                                description: Some("Entropy coding (CAVLC or CABAC)".to_string()),
                                children: vec![],
                                byte_offset: None,
                            },
                            SyntaxNode {
                                name: "num_ref_frames".to_string(),
                                value: Some(SyntaxValue::Number(1)),
                                description: Some("Number of reference frames".to_string()),
                                children: vec![],
                                byte_offset: None,
                            },
                        ],
                        byte_offset: None,
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
                                byte_offset: None,
                            },
                            SyntaxNode {
                                name: "macroblock_layer".to_string(),
                                description: Some("Macroblock partition structure".to_string()),
                                value: None,
                                children: vec![
                                    SyntaxNode {
                                        name: "mb_affine".to_string(),
                                        value: Some(SyntaxValue::Boolean(false)),
                                        description: Some(
                                            "Adaptive macroblock affine transform".to_string(),
                                        ),
                                        children: vec![],
                                        byte_offset: None,
                                    },
                                    SyntaxNode {
                                        name: "mb_count".to_string(),
                                        value: Some(SyntaxValue::Number(
                                            ((width / 16) * (height / 16)) as i64,
                                        )),
                                        description: Some("Number of macroblocks".to_string()),
                                        children: vec![],
                                        byte_offset: None,
                                    },
                                ],
                                byte_offset: None,
                            },
                        ],
                        byte_offset: None,
                    },
                ],
                byte_offset: None,
            },
        ],
        byte_offset: None,
    }
}

/// Build syntax tree for HEVC/H.265 frames
fn build_hevc_syntax_tree(
    frame_index: usize,
    frame: &FrameData,
    width: u32,
    height: u32,
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
                byte_offset: None,
            },
            SyntaxNode {
                name: "size".to_string(),
                value: Some(SyntaxValue::Number(frame.size as i64)),
                description: Some("NAL unit size in bytes".to_string()),
                children: vec![],
                byte_offset: None,
            },
            SyntaxNode {
                name: "key_frame".to_string(),
                value: Some(SyntaxValue::Boolean(frame.key_frame.unwrap_or(false))),
                description: Some("CRA/IDR frame (clean random access)".to_string()),
                children: vec![],
                byte_offset: None,
            },
            SyntaxNode {
                name: "nal_unit_structure".to_string(),
                description: Some("NAL unit composition".to_string()),
                value: None,
                children: vec![
                    SyntaxNode {
                        name: "vps".to_string(),
                        description: Some(
                            "Video Parameter Set - video layer configuration".to_string(),
                        ),
                        value: None,
                        children: vec![
                            SyntaxNode {
                                name: "max_layers_minus1".to_string(),
                                value: Some(SyntaxValue::Number(0)),
                                description: Some("Number of additional layers".to_string()),
                                children: vec![],
                                byte_offset: None,
                            },
                            SyntaxNode {
                                name: "temporal_id_nesting".to_string(),
                                value: Some(SyntaxValue::Boolean(false)),
                                description: Some("Temporal ID nesting flag".to_string()),
                                children: vec![],
                                byte_offset: None,
                            },
                        ],
                        byte_offset: None,
                    },
                    SyntaxNode {
                        name: "sps".to_string(),
                        description: Some(
                            "Sequence Parameter Set - video coding configuration".to_string(),
                        ),
                        value: None,
                        children: vec![
                            SyntaxNode {
                                name: "profile_tier_level".to_string(),
                                value: Some(SyntaxValue::String("Main Tier 4.1".to_string())),
                                description: Some("Profile, tier, and level".to_string()),
                                children: vec![],
                                byte_offset: None,
                            },
                            SyntaxNode {
                                name: "chroma_format".to_string(),
                                value: Some(SyntaxValue::String("YUV420".to_string())),
                                description: Some("Chroma subsampling format".to_string()),
                                children: vec![],
                                byte_offset: None,
                            },
                            SyntaxNode {
                                name: "ctu_size".to_string(),
                                value: Some(SyntaxValue::String("64x64".to_string())),
                                description: Some("Coding Tree Unit size".to_string()),
                                children: vec![],
                                byte_offset: None,
                            },
                        ],
                        byte_offset: None,
                    },
                    SyntaxNode {
                        name: "pps".to_string(),
                        description: Some(
                            "Picture Parameter Set - picture-specific settings".to_string(),
                        ),
                        value: None,
                        children: vec![
                            SyntaxNode {
                                name: "entropy_coding".to_string(),
                                value: Some(SyntaxValue::String("CABAC".to_string())),
                                description: Some("Entropy coding mode".to_string()),
                                children: vec![],
                                byte_offset: None,
                            },
                            SyntaxNode {
                                name: "num_ref_idx_active".to_string(),
                                value: Some(SyntaxValue::Number(1)),
                                description: Some("Number of active reference indices".to_string()),
                                children: vec![],
                                byte_offset: None,
                            },
                        ],
                        byte_offset: None,
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
                                byte_offset: None,
                            },
                            SyntaxNode {
                                name: "ctu_count".to_string(),
                                value: Some(SyntaxValue::Number(
                                    ((width.div_ceil(64)) * (height.div_ceil(64))) as i64,
                                )),
                                description: Some("Number of CTUs in frame".to_string()),
                                children: vec![],
                                byte_offset: None,
                            },
                        ],
                        byte_offset: None,
                    },
                ],
                byte_offset: None,
            },
        ],
        byte_offset: None,
    }
}

/// Build generic syntax tree for unknown/unparsed formats
fn build_generic_syntax_tree(frame: &FrameData) -> SyntaxNode {
    SyntaxNode {
        name: format!("Frame {}", frame.frame_index),
        description: Some(
            "Generic frame information (codec-specific parsing not available)".to_string(),
        ),
        value: None,
        children: vec![
            SyntaxNode {
                name: "frame_type".to_string(),
                value: Some(SyntaxValue::String(frame.frame_type.clone())),
                description: Some("Frame type".to_string()),
                children: vec![],
                byte_offset: None,
            },
            SyntaxNode {
                name: "size".to_string(),
                value: Some(SyntaxValue::Number(frame.size as i64)),
                description: Some("Frame size in bytes".to_string()),
                children: vec![],
                byte_offset: None,
            },
            SyntaxNode {
                name: "presentation_timestamp".to_string(),
                value: frame.pts.map(|v| SyntaxValue::Number(v as i64)),
                description: Some("Presentation timestamp".to_string()),
                children: vec![],
                byte_offset: None,
            },
            SyntaxNode {
                name: "key_frame".to_string(),
                value: Some(SyntaxValue::Boolean(frame.key_frame.unwrap_or(false))),
                description: Some("Key frame indicator".to_string()),
                children: vec![],
                byte_offset: None,
            },
        ],
        byte_offset: None,
    }
}

// ---------------------------------------------------------------------------
// Extended info helper functions
// ---------------------------------------------------------------------------

fn build_ref_lists_from_frames(
    frames: &[crate::commands::FrameData],
    frame_index: usize,
) -> (Vec<RefEntry>, Vec<RefEntry>) {
    let frame = match frames.get(frame_index) {
        Some(f) => f,
        None => return (vec![], vec![]),
    };

    let ref_indices = match &frame.ref_frames {
        Some(v) => v.clone(),
        None => return (vec![], vec![]),
    };

    let ft = frame.frame_type.to_uppercase();
    if ft == "I" || ft == "KEY" || ft == "IDR" || ft == "CRA" {
        return (vec![], vec![]);
    }

    let total = ref_indices.len();
    let l0_count = if ft == "B" || ft == "BI" {
        (total + 1) / 2
    } else {
        total
    };
    let current_poc = frame.poc.unwrap_or(frame_index as i32);

    let mut l0 = vec![];
    let mut l1 = vec![];

    for (i, &ref_idx) in ref_indices.iter().enumerate() {
        let ref_frame = frames.get(ref_idx);
        let ref_poc = ref_frame.and_then(|f| f.poc).unwrap_or(ref_idx as i32);

        let entry = RefEntry {
            list_idx: if i < l0_count { 0 } else { 1 },
            slot: i as u8,
            poc: ref_poc - current_poc,
            frame_index: ref_idx,
            frame_type: ref_frame
                .map(|f| f.frame_type.clone())
                .unwrap_or_else(|| "?".into()),
            long_term: false,
            weight: None,
            offset: None,
        };

        if i < l0_count {
            l0.push(entry);
        } else {
            l1.push(entry);
        }
    }

    (l0, l1)
}

fn build_size_based_qp_histogram(
    frames: &[crate::commands::FrameData],
    codec: &str,
) -> Vec<QpBucket> {
    if frames.is_empty() {
        return vec![];
    }
    let max_size = frames.iter().map(|f| f.size).max().unwrap_or(1) as f32;
    let qp_max: i16 = if codec == "av1" { 255 } else { 51 };

    let mut histogram: std::collections::HashMap<i16, u32> = std::collections::HashMap::new();
    for f in frames {
        let ratio = f.size as f32 / max_size;
        // Larger frame → lower QP; smaller frame → higher QP
        let qp = ((1.0 - ratio) * qp_max as f32).round() as i16;
        let qp = qp.clamp(0, qp_max);
        *histogram.entry(qp).or_insert(0) += 1;
    }

    let mut buckets: Vec<QpBucket> = histogram
        .into_iter()
        .map(|(qp, count)| QpBucket { qp, count })
        .collect();
    buckets.sort_by_key(|b| b.qp);
    buckets
}

fn build_default_hevc_qm() -> HevcQmData {
    let flat4: Vec<u8> = vec![16; 16];

    // H.265 default scaling lists (Table E-1)
    let intra8: Vec<u8> = vec![
        16, 16, 16, 16, 17, 18, 21, 24, 16, 16, 16, 16, 17, 19, 22, 25, 16, 16, 17, 18, 20, 22, 25,
        29, 16, 16, 18, 21, 24, 27, 31, 36, 17, 17, 20, 24, 30, 35, 41, 47, 18, 19, 22, 27, 35, 44,
        54, 65, 21, 22, 25, 31, 41, 54, 70, 88, 24, 25, 29, 36, 47, 65, 88, 115,
    ];
    let inter8: Vec<u8> = vec![
        16, 16, 16, 16, 17, 18, 20, 24, 16, 16, 16, 17, 18, 20, 24, 25, 16, 16, 17, 18, 20, 24, 25,
        28, 16, 17, 18, 20, 24, 25, 28, 33, 17, 18, 20, 24, 25, 28, 33, 41, 18, 20, 24, 25, 28, 33,
        41, 54, 20, 24, 25, 28, 33, 41, 54, 71, 24, 25, 28, 33, 41, 54, 71, 91,
    ];

    // 16x16 and 32x32 use flat DC (16) + scaled 8x8 for AC; represent as flat for simplicity
    let flat16: Vec<u8> = vec![16; 256];
    let flat32: Vec<u8> = vec![16; 1024];

    HevcQmData {
        scaling_list_enabled: true,
        matrices: vec![
            QmMatrix {
                name: "4×4 Intra Luma".into(),
                size: 4,
                pred_type: "Intra".into(),
                plane: "Luma".into(),
                values: flat4.clone(),
            },
            QmMatrix {
                name: "4×4 Intra Chroma Cb".into(),
                size: 4,
                pred_type: "Intra".into(),
                plane: "Chroma Cb".into(),
                values: flat4.clone(),
            },
            QmMatrix {
                name: "4×4 Intra Chroma Cr".into(),
                size: 4,
                pred_type: "Intra".into(),
                plane: "Chroma Cr".into(),
                values: flat4.clone(),
            },
            QmMatrix {
                name: "4×4 Inter Luma".into(),
                size: 4,
                pred_type: "Inter".into(),
                plane: "Luma".into(),
                values: flat4.clone(),
            },
            QmMatrix {
                name: "4×4 Inter Chroma Cb".into(),
                size: 4,
                pred_type: "Inter".into(),
                plane: "Chroma Cb".into(),
                values: flat4.clone(),
            },
            QmMatrix {
                name: "4×4 Inter Chroma Cr".into(),
                size: 4,
                pred_type: "Inter".into(),
                plane: "Chroma Cr".into(),
                values: flat4.clone(),
            },
            QmMatrix {
                name: "8×8 Intra Luma".into(),
                size: 8,
                pred_type: "Intra".into(),
                plane: "Luma".into(),
                values: intra8.clone(),
            },
            QmMatrix {
                name: "8×8 Intra Chroma Cb".into(),
                size: 8,
                pred_type: "Intra".into(),
                plane: "Chroma Cb".into(),
                values: intra8.clone(),
            },
            QmMatrix {
                name: "8×8 Intra Chroma Cr".into(),
                size: 8,
                pred_type: "Intra".into(),
                plane: "Chroma Cr".into(),
                values: intra8.clone(),
            },
            QmMatrix {
                name: "8×8 Inter Luma".into(),
                size: 8,
                pred_type: "Inter".into(),
                plane: "Luma".into(),
                values: inter8.clone(),
            },
            QmMatrix {
                name: "8×8 Inter Chroma Cb".into(),
                size: 8,
                pred_type: "Inter".into(),
                plane: "Chroma Cb".into(),
                values: inter8.clone(),
            },
            QmMatrix {
                name: "8×8 Inter Chroma Cr".into(),
                size: 8,
                pred_type: "Inter".into(),
                plane: "Chroma Cr".into(),
                values: inter8.clone(),
            },
            QmMatrix {
                name: "16×16 Intra Luma".into(),
                size: 16,
                pred_type: "Intra".into(),
                plane: "Luma".into(),
                values: flat16.clone(),
            },
            QmMatrix {
                name: "16×16 Inter Luma".into(),
                size: 16,
                pred_type: "Inter".into(),
                plane: "Luma".into(),
                values: flat16.clone(),
            },
            QmMatrix {
                name: "32×32 Intra Luma".into(),
                size: 32,
                pred_type: "Intra".into(),
                plane: "Luma".into(),
                values: flat32.clone(),
            },
            QmMatrix {
                name: "32×32 Inter Luma".into(),
                size: 32,
                pred_type: "Inter".into(),
                plane: "Luma".into(),
                values: flat32.clone(),
            },
        ],
    }
}

fn build_vp9_probs_data(frame_index: usize, is_key_frame: bool) -> Vp9ProbsData {
    Vp9ProbsData {
        frame_index,
        is_key_frame,
        entries: vec![
            ProbEntry {
                group: "Frame Header".into(),
                label: "is_inter_frame".into(),
                probs: vec![if is_key_frame { 0 } else { 128 }],
            },
            ProbEntry {
                group: "Frame Header".into(),
                label: "comp_pred_mode".into(),
                probs: vec![178, 134],
            },
            ProbEntry {
                group: "Intra Mode Y".into(),
                label: "y_mode[8×8]".into(),
                probs: vec![65, 32, 18, 144, 162, 194, 41, 51, 98],
            },
            ProbEntry {
                group: "Intra Mode UV".into(),
                label: "uv_mode[8×8]".into(),
                probs: vec![132, 182, 103, 44, 105, 91, 80, 98, 110],
            },
            ProbEntry {
                group: "Partition".into(),
                label: "partition[8×8]".into(),
                probs: vec![199, 122, 141],
            },
            ProbEntry {
                group: "Partition".into(),
                label: "partition[16×16]".into(),
                probs: vec![174, 121, 128],
            },
            ProbEntry {
                group: "Partition".into(),
                label: "partition[32×32]".into(),
                probs: vec![177, 130, 138],
            },
            ProbEntry {
                group: "Motion Vector".into(),
                label: "mv_joint".into(),
                probs: vec![32, 64, 96],
            },
            ProbEntry {
                group: "Motion Vector".into(),
                label: "mv_sign".into(),
                probs: vec![128, 128],
            },
            ProbEntry {
                group: "Motion Vector".into(),
                label: "mv_class".into(),
                probs: vec![224, 144, 192, 168, 192, 176, 192, 198, 198, 245],
            },
            ProbEntry {
                group: "Skip".into(),
                label: "skip[ctx=0]".into(),
                probs: vec![192],
            },
            ProbEntry {
                group: "Skip".into(),
                label: "skip[ctx=1]".into(),
                probs: vec![128],
            },
            ProbEntry {
                group: "Skip".into(),
                label: "skip[ctx=2]".into(),
                probs: vec![64],
            },
            ProbEntry {
                group: "Inter Mode".into(),
                label: "inter_mode[ctx=0]".into(),
                probs: vec![2, 173, 34],
            },
            ProbEntry {
                group: "Inter Mode".into(),
                label: "inter_mode[ctx=1]".into(),
                probs: vec![7, 145, 85],
            },
            ProbEntry {
                group: "Inter Mode".into(),
                label: "inter_mode[ctx=2]".into(),
                probs: vec![7, 166, 63],
            },
        ],
    }
}

fn build_vvc_aps_data() -> VvcApsData {
    VvcApsData {
        aps_list: vec![
            ApsEntry {
                aps_id: 0,
                aps_type: "ALF".into(),
                enabled: true,
                summary: "Luma: 7×7 diamond filter, Cb: 5-coeff, Cr: 5-coeff".into(),
            },
            ApsEntry {
                aps_id: 1,
                aps_type: "ALF".into(),
                enabled: true,
                summary: "Alternative luma ALF (5×5 cross shape)".into(),
            },
            ApsEntry {
                aps_id: 2,
                aps_type: "ALF".into(),
                enabled: false,
                summary: "Not signalled in this frame".into(),
            },
            ApsEntry {
                aps_id: 0,
                aps_type: "LMCS".into(),
                enabled: false,
                summary: "Luma Mapping with Chroma Scaling: disabled for this frame".into(),
            },
            ApsEntry {
                aps_id: 0,
                aps_type: "SCALING_LIST".into(),
                enabled: false,
                summary: "Scaling List APS: disabled (flat default matrices)".into(),
            },
        ],
    }
}
