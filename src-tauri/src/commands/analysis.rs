//! Frame analysis commands
//!
//! Commands for getting frame analysis data (QP, MV, partition, etc.).
//! Supports multiple codecs: AV1, H.264/AVC, HEVC, VP9, VVC/H.266, AV3.

use bitvue_av1::overlay_extraction::{extract_qp_grid, extract_mv_grid, extract_partition_grid, extract_prediction_mode_grid, extract_transform_grid};
use bitvue_core::StreamId;
use serde::{Deserialize, Serialize};

use crate::commands::{FrameAnalysisData, QPGridData, MVGridData, PartitionGridData, PredictionModeGridData, TransformGridData, AppState, MotionVectorData};

/// Supported video codecs for analysis
#[allow(clippy::upper_case_acronyms)]
enum VideoCodec {
    AV1,
    AVC,
    HEVC,
    VP9,
    VVC,
    AV3,
    Unknown,
}

impl VideoCodec {
    /// Parse codec from string
    fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "av1" => VideoCodec::AV1,
            "avc" | "h264" | "264" => VideoCodec::AVC,
            "hevc" | "h265" | "265" => VideoCodec::HEVC,
            "vp9" => VideoCodec::VP9,
            "vvc" | "h266" | "266" => VideoCodec::VVC,
            "av3" => VideoCodec::AV3,
            _ => VideoCodec::Unknown,
        }
    }

    /// Get codec name as string
    #[allow(dead_code)]
    fn as_str(&self) -> &'static str {
        match self {
            VideoCodec::AV1 => "av1",
            VideoCodec::AVC => "avc",
            VideoCodec::HEVC => "hevc",
            VideoCodec::VP9 => "vp9",
            VideoCodec::VVC => "vvc",
            VideoCodec::AV3 => "av3",
            VideoCodec::Unknown => "unknown",
        }
    }
}

/// Get frame analysis data (QP heatmap, MV field, Partition grid, etc.)
///
/// Detects the codec type and uses appropriate extraction functions:
/// - AV1: Uses bitvue-av1 overlay extraction
/// - H.264/AVC: Uses bitvue-avc overlay extraction
/// - HEVC: Uses bitvue-hevc overlay extraction
/// - VP9: Uses bitvue-vp9 overlay extraction
/// - VVC: Uses bitvue-vvc overlay extraction
/// - AV3: Uses bitvue-av3 overlay extraction

/// Helper: Load file data and detect codec
///
/// Extracts file path, loads cached file data, and detects codec type.
/// Reduces nesting in get_frame_analysis.
async fn load_file_data_and_codec(
    state: &tauri::State<'_, AppState>,
) -> Result<(Vec<u8>, String), String> {
    let core = state.core.lock().map_err(|e| {
        log::error!("get_frame_analysis: Failed to lock core: {}", e);
        e.to_string()
    })?;

    let stream_a_lock = core.get_stream(StreamId::A);
    let stream_a = stream_a_lock.read();
    let file_path = stream_a.file_path.as_ref().ok_or("No file loaded")?.clone();
    let file_path_str = file_path.to_str().ok_or("Invalid file path")?;

    log::info!("get_frame_analysis: File path: {}", file_path_str);
    log::info!("get_frame_analysis: Stream A loaded: {} units",
        stream_a.units.as_ref().map_or(0, |u| u.units.len()));

    // Use cached file data from decode_service to avoid repeated disk reads
    let file_data = state.decode_service.lock()
        .map_err(|e| e.to_string())?
        .get_file_data()?;

    log::info!("get_frame_analysis: File data size: {} bytes (from cache)", file_data.len());

    // Detect codec from file extension or stream metadata
    let codec = detect_codec_from_path(file_path_str);
    log::info!("get_frame_analysis: Detected codec: {}", codec);

    Ok((file_data, codec))
}

/// Helper: Extract frame analysis based on codec type
///
/// Matches codec type and calls appropriate extraction function.
/// Falls back to trying multiple codecs for unknown types.
fn extract_analysis_by_codec(
    file_data: &[u8],
    frame_index: usize,
    core: &bitvue_core::Core,
    codec: &str,
) -> Result<FrameAnalysisData, String> {
    let video_codec = VideoCodec::from_str(codec);

    match video_codec {
        VideoCodec::AV1 => extract_av1_analysis(file_data, frame_index, core),
        VideoCodec::AVC => extract_avc_analysis(file_data, frame_index, core),
        VideoCodec::HEVC => extract_hevc_analysis(file_data, frame_index, core),
        VideoCodec::VP9 => extract_vp9_analysis(file_data, frame_index, core),
        VideoCodec::VVC => extract_vvc_analysis(file_data, frame_index, core),
        VideoCodec::AV3 => extract_av3_analysis(file_data, frame_index, core),
        VideoCodec::Unknown => {
            // Try codecs in order of likelihood
            log::warn!("get_frame_analysis: Unknown codec, trying AV1");
            extract_av1_analysis(file_data, frame_index, core)
                .or_else(|_| {
                    log::warn!("get_frame_analysis: AV1 failed, trying AVC");
                    extract_avc_analysis(file_data, frame_index, core)
                })
                .or_else(|_| {
                    log::warn!("get_frame_analysis: AVC failed, trying HEVC");
                    extract_hevc_analysis(file_data, frame_index, core)
                })
                .or_else(|_| {
                    log::warn!("get_frame_analysis: HEVC failed, trying VP9");
                    extract_vp9_analysis(file_data, frame_index, core)
                })
                .or_else(|_| {
                    log::warn!("get_frame_analysis: VP9 failed, trying VVC");
                    extract_vvc_analysis(file_data, frame_index, core)
                })
        }
    }
}

/// Helper: Log analysis result
///
/// Logs analysis success/failure and extracted grid data.
/// Separates logging concerns from main function logic.
fn log_analysis_result(result: &Result<FrameAnalysisData, String>) {
    match result {
        Ok(analysis) => {
            log::info!("get_frame_analysis: === Analysis successful ===");
            log::info!("get_frame_analysis: Frame size: {}x{}", analysis.width, analysis.height);
            log::info!("get_frame_analysis: QP grid: {}",
                if analysis.qp_grid.is_some() { "present" } else { "none" });
            log::info!("get_frame_analysis: MV grid: {}",
                if analysis.mv_grid.is_some() { "present" } else { "none" });
            log::info!("get_frame_analysis: Partition grid: {}",
                if analysis.partition_grid.is_some() { "present" } else { "none" });
        }
        Err(e) => {
            log::error!("get_frame_analysis: === Analysis failed: {} ===", e);
        }
    }
}

#[tauri::command]
pub async fn get_frame_analysis(
    state: tauri::State<'_, AppState>,
    frame_index: usize,
) -> Result<FrameAnalysisData, String> {
    log::info!("get_frame_analysis: === Starting frame analysis request ===");
    log::info!("get_frame_analysis: Frame index: {}", frame_index);

    // Load file data and detect codec
    let (file_data, codec) = load_file_data_and_codec(&state).await?;

    // Extract analysis based on codec type
    log::info!("get_frame_analysis: Selecting extraction function for codec: {}", codec);
    let core = state.core.lock().map_err(|e| e.to_string())?;
    let result = extract_analysis_by_codec(&file_data, frame_index, &core, &codec);

    // Log analysis result
    log_analysis_result(&result);

    result
}

/// Detect codec from file path
fn detect_codec_from_path(path: &str) -> String {
    let path_buf = std::path::PathBuf::from(path);
    path_buf.extension()
        .and_then(|e| e.to_str())
        .and_then(|ext| match ext.to_lowercase().as_str() {
            "ivf" => Some("av1".to_string()),
            "webm" => Some("vp9".to_string()),
            "mkv" => None, // Could be AV1, VP9, HEVC, VVC - detect from content
            "mp4" | "mov" => None, // Could be AVC, HEVC, VVC, AV1, AV3 - detect from content
            "h264" | "264" | "avc" => Some("avc".to_string()),
            "h265" | "265" | "hevc" => Some("hevc".to_string()),
            "h266" | "266" | "vvc" => Some("vvc".to_string()),
            "av1" => Some("av1".to_string()),
            "av3" => Some("av3".to_string()),
            "vp9" => Some("vp9".to_string()),
            _ => None,
        })
        .unwrap_or_else(|| {
            // Try to detect from content by checking file signature
            detect_codec_from_content(path)
        })
}

/// Detect codec from file content (magic bytes)
/// SECURITY: Validates path before reading to prevent path traversal
fn detect_codec_from_content(path: &str) -> String {
    // Validate path before reading to prevent security issues
    if let Ok(validated_path) = super::file::validate_file_path(path) {
        if let Ok(data) = std::fs::read(&validated_path) {
            return detect_codec_from_data(&data);
        }
    }
    "unknown".to_string()
}

/// Detect codec from byte data (magic bytes)
fn detect_codec_from_data(data: &[u8]) -> String {
    // Check magic bytes
    if data.len() >= 4 {
        let magic = &data[0..4];
        if magic == b"DKIF" {
            // IVF container - check codec tag
            if data.len() >= 32 {
                let codec_tag = &data[4..8];
                if codec_tag == b"AV01" {
                    return "av1".to_string();
                } else if codec_tag == b"VP90" {
                    return "vp9".to_string();
                } else if codec_tag == b"AV03" {
                    return "av3".to_string(); // AV3 in IVF
                }
            }
            return "av1".to_string(); // Default IVF to AV1
        } else if magic == [0x1A, 0x45, 0xDF, 0xA3] {
            // EBML header - MKV/WebM
            // Could be AV1, VP9, HEVC, VVC, or AVC
            // For WebM, default to VP9
            // For MKV, try to detect by scanning for codec signatures
            return "vp9".to_string(); // Default to VP9 for WebM
        } else if data.len() >= 8 {
            let box_type = &data[4..8];
            if box_type == b"ftyp" {
                // MP4/MOV - could be AVC, HEVC, VVC, AV1, or AV3
                // Use byte matching instead of string conversion for efficiency
                let search_range = &data[..data.len().min(8192)];
                if search_range.windows(4).any(|w| w == b"vvc1" || w == b"vvi1") {
                    return "vvc".to_string();
                } else if search_range.windows(4).any(|w| w == b"hvc1" || w == b"hev1") {
                    return "hevc".to_string();
                } else if search_range.windows(4).any(|w| w == b"avc1" || w == b"avc3") {
                    return "avc".to_string();
                } else if search_range.windows(4).any(|w| w == b"av01") {
                    return "av1".to_string();
                } else if search_range.windows(4).any(|w| w == b"av03") {
                    return "av3".to_string();
                }
                return "vvc".to_string(); // Default to VVC for modern MP4
            }
        }
    }
    "unknown".to_string()
}

/// Extract frame analysis for AV1 codec
fn extract_av1_analysis(
    file_data: &[u8],
    frame_index: usize,
    core: &bitvue_core::Core,
) -> Result<FrameAnalysisData, String> {
    log::info!("extract_av1_analysis: === Starting AV1 analysis extraction ===");
    log::info!("extract_av1_analysis: Frame index: {}", frame_index);
    log::info!("extract_av1_analysis: File data size: {} bytes", file_data.len());

    // Get frame OBU data from unit model
    log::info!("extract_av1_analysis: Reading stream A units...");

    let obu_data = {
        let stream_a_arc = core.get_stream(StreamId::A);
        let stream_a_guard = stream_a_arc.read();

        if let Some(unit_model) = &stream_a_guard.units {
            log::info!("extract_av1_analysis: Unit model found with {} units", unit_model.units.len());
            unit_model.units.get(frame_index)
                .and_then(|_unit| {
                    // Extract frame data from IVF using provided file data
                    log::info!("extract_av1_analysis: Parsing IVF frames...");
                    if let Ok((_, ivf_frames)) = bitvue_av1::parse_ivf_frames(file_data) {
                        log::info!("extract_av1_analysis: IVF parsing successful, {} frames", ivf_frames.len());
                        ivf_frames.get(frame_index).map(|f| {
                            log::info!("extract_av1_analysis: Frame {} data size: {} bytes", frame_index, f.data.len());
                            f.data.clone()
                        })
                    } else {
                        log::warn!("extract_av1_analysis: IVF parsing failed");
                        None
                    }
                })
        } else {
            log::warn!("extract_av1_analysis: No unit model found");
            None
        }
    };

    let obu_data = obu_data.ok_or("Frame data not available")?;
    log::info!("extract_av1_analysis: OBU data size: {} bytes", obu_data.len());

    // Extract grids using AV1 functions
    log::info!("extract_av1_analysis: Extracting QP grid...");
    let qp_grid = extract_qp_grid(&obu_data, frame_index, 20)
        .ok()
        .map(|grid| QPGridData {
            grid_w: grid.grid_w,
            grid_h: grid.grid_h,
            block_w: grid.block_w,
            block_h: grid.block_h,
            qp: grid.qp,
            qp_min: grid.qp_min,
            qp_max: grid.qp_max,
        });

    let mv_grid = extract_mv_grid(&obu_data, frame_index)
        .ok()
        .map(|grid| MVGridData {
            coded_width: grid.coded_width,
            coded_height: grid.coded_height,
            block_w: grid.block_w,
            block_h: grid.block_h,
            grid_w: grid.grid_w,
            grid_h: grid.grid_h,
            mv_l0: grid.mv_l0.into_iter().map(|mv| MotionVectorData {
                dx_qpel: mv.dx_qpel,
                dy_qpel: mv.dy_qpel,
            }).collect(),
            mv_l1: grid.mv_l1.into_iter().map(|mv| MotionVectorData {
                dx_qpel: mv.dx_qpel,
                dy_qpel: mv.dy_qpel,
            }).collect(),
            mode: grid.mode.map(|modes: Vec<bitvue_core::mv_overlay::BlockMode>| modes.into_iter().map(|m| m as u8).collect()),
        });

    let partition_grid = extract_partition_grid(&obu_data, frame_index)
        .ok()
        .map(|grid| PartitionGridData {
            coded_width: grid.coded_width,
            coded_height: grid.coded_height,
            sb_size: grid.sb_size,
            blocks: grid.blocks.into_iter().map(|b| crate::commands::PartitionBlockData {
                x: b.x,
                y: b.y,
                width: b.width,
                height: b.height,
                partition: b.partition as u8,
                depth: b.depth,
            }).collect(),
        });

    let prediction_mode_grid = extract_prediction_mode_grid(&obu_data, frame_index)
        .ok()
        .map(|grid| PredictionModeGridData {
            coded_width: grid.coded_width,
            coded_height: grid.coded_height,
            block_w: grid.block_w,
            block_h: grid.block_h,
            grid_w: grid.grid_w,
            grid_h: grid.grid_h,
            modes: grid.modes.into_iter().map(|m| m.map(|pm| pm as u8)).collect(),
        });

    let transform_grid = extract_transform_grid(&obu_data, frame_index)
        .ok()
        .map(|grid| TransformGridData {
            coded_width: grid.coded_width,
            coded_height: grid.coded_height,
            block_w: grid.block_w,
            block_h: grid.block_h,
            grid_w: grid.grid_w,
            grid_h: grid.grid_h,
            tx_sizes: grid.tx_sizes.into_iter().map(|t| t.map(|tx| tx as u8)).collect(),
        });

    // Get frame dimensions
    let width = qp_grid.as_ref()
        .map(|g| g.grid_w * g.block_w)
        .or_else(|| mv_grid.as_ref().map(|g| g.coded_width))
        .or_else(|| partition_grid.as_ref().map(|g| g.coded_width))
        .unwrap_or(1920);

    let height = qp_grid.as_ref()
        .map(|g| g.grid_h * g.block_h)
        .or_else(|| mv_grid.as_ref().map(|g| g.coded_height))
        .or_else(|| partition_grid.as_ref().map(|g| g.coded_height))
        .unwrap_or(1080);

    log::info!("extract_av1_analysis: Frame dimensions: {}x{}", width, height);
    log::info!("extract_av1_analysis: QP grid extracted: {}", qp_grid.is_some());
    log::info!("extract_av1_analysis: MV grid extracted: {}", mv_grid.is_some());
    log::info!("extract_av1_analysis: Partition grid extracted: {}", partition_grid.is_some());
    log::info!("extract_av1_analysis: Prediction mode grid extracted: {}", prediction_mode_grid.is_some());
    log::info!("extract_av1_analysis: Transform grid extracted: {}", transform_grid.is_some());
    log::info!("extract_av1_analysis: === AV1 analysis extraction complete ===");

    Ok(FrameAnalysisData {
        frame_index,
        width,
        height,
        qp_grid,
        mv_grid,
        partition_grid,
        prediction_mode_grid,
        transform_grid,
    })
}

/// Extract frame analysis for H.264/AVC codec
fn extract_avc_analysis(
    file_data: &[u8],
    frame_index: usize,
    _core: &bitvue_core::Core,
) -> Result<FrameAnalysisData, String> {
    log::info!("extract_avc_analysis: Extracting AVC analysis for frame {}", frame_index);
    log::info!("extract_avc_analysis: File data size: {} bytes", file_data.len());

    // Parse H.264 NAL units
    let nal_units = bitvue_avc::parse_nal_units(&file_data)
        .map_err(|e| format!("Failed to parse NAL units: {}", e))?;

    // Parse SPS for dimensions
    let sps = nal_units.iter()
        .find_map(|nal| {
            if nal.header.nal_unit_type == bitvue_avc::NalUnitType::Sps {
                bitvue_avc::sps::parse_sps(&nal.payload).ok()
            } else {
                None
            }
        })
        .ok_or("No SPS found in stream")?;

    // Extract grids using H.264 functions
    let qp_grid = bitvue_avc::extract_qp_grid(&nal_units, &sps, 26)
        .ok()
        .map(|grid| QPGridData {
            grid_w: grid.grid_w,
            grid_h: grid.grid_h,
            block_w: grid.block_w,
            block_h: grid.block_h,
            qp: grid.qp,
            qp_min: grid.qp_min,
            qp_max: grid.qp_max,
        });

    let mv_grid = bitvue_avc::extract_mv_grid(&nal_units, &sps)
        .ok()
        .map(|grid| MVGridData {
            coded_width: grid.coded_width,
            coded_height: grid.coded_height,
            block_w: grid.block_w,
            block_h: grid.block_h,
            grid_w: grid.grid_w,
            grid_h: grid.grid_h,
            mv_l0: grid.mv_l0.into_iter().map(|mv| MotionVectorData {
                dx_qpel: mv.dx_qpel,
                dy_qpel: mv.dy_qpel,
            }).collect(),
            mv_l1: grid.mv_l1.into_iter().map(|mv| MotionVectorData {
                dx_qpel: mv.dx_qpel,
                dy_qpel: mv.dy_qpel,
            }).collect(),
            mode: grid.mode.map(|modes: Vec<bitvue_core::mv_overlay::BlockMode>| modes.into_iter().map(|m| m as u8).collect()),
        });

    let partition_grid = bitvue_avc::extract_partition_grid(&nal_units, &sps)
        .ok()
        .map(|grid| PartitionGridData {
            coded_width: grid.coded_width,
            coded_height: grid.coded_height,
            sb_size: grid.sb_size,
            blocks: grid.blocks.into_iter().map(|b| crate::commands::PartitionBlockData {
                x: b.x,
                y: b.y,
                width: b.width,
                height: b.height,
                partition: b.partition as u8,
                depth: b.depth,
            }).collect(),
        });

    // Get frame dimensions
    let width = sps.display_width();
    let height = sps.display_height();

    log::info!("extract_avc_analysis: Returning analysis for frame {} ({}x{})",
        frame_index, width, height);

    Ok(FrameAnalysisData {
        frame_index,
        width,
        height,
        qp_grid,
        mv_grid,
        partition_grid,
        prediction_mode_grid: None, // Not implemented for H.264 yet
        transform_grid: None,       // Not implemented for H.264 yet
    })
}

/// Extract frame analysis for HEVC/H.265 codec
fn extract_hevc_analysis(
    file_data: &[u8],
    frame_index: usize,
    _core: &bitvue_core::Core,
) -> Result<FrameAnalysisData, String> {
    log::info!("extract_hevc_analysis: Extracting HEVC analysis for frame {}", frame_index);
    log::info!("extract_hevc_analysis: File data size: {} bytes", file_data.len());

    // Parse HEVC NAL units
    let nal_units = bitvue_hevc::parse_nal_units(&file_data)
        .map_err(|e| format!("Failed to parse NAL units: {}", e))?;

    // Parse SPS for dimensions
    let sps = nal_units.iter()
        .find_map(|nal| {
            if nal.header.nal_unit_type == bitvue_hevc::NalUnitType::SpsNut {
                bitvue_hevc::sps::parse_sps(&nal.payload).ok()
            } else {
                None
            }
        })
        .ok_or("No SPS found in stream")?;

    // Extract grids using HEVC functions
    let qp_grid = bitvue_hevc::extract_qp_grid(&nal_units, &sps, 26)
        .ok()
        .map(|grid| QPGridData {
            grid_w: grid.grid_w,
            grid_h: grid.grid_h,
            block_w: grid.block_w,
            block_h: grid.block_h,
            qp: grid.qp,
            qp_min: grid.qp_min,
            qp_max: grid.qp_max,
        });

    let mv_grid = bitvue_hevc::extract_mv_grid(&nal_units, &sps)
        .ok()
        .map(|grid| MVGridData {
            coded_width: grid.coded_width,
            coded_height: grid.coded_height,
            block_w: grid.block_w,
            block_h: grid.block_h,
            grid_w: grid.grid_w,
            grid_h: grid.grid_h,
            mv_l0: grid.mv_l0.into_iter().map(|mv| MotionVectorData {
                dx_qpel: mv.dx_qpel,
                dy_qpel: mv.dy_qpel,
            }).collect(),
            mv_l1: grid.mv_l1.into_iter().map(|mv| MotionVectorData {
                dx_qpel: mv.dx_qpel,
                dy_qpel: mv.dy_qpel,
            }).collect(),
            mode: grid.mode.map(|modes: Vec<bitvue_core::mv_overlay::BlockMode>| modes.into_iter().map(|m| m as u8).collect()),
        });

    let partition_grid = bitvue_hevc::extract_partition_grid(&nal_units, &sps)
        .ok()
        .map(|grid| PartitionGridData {
            coded_width: grid.coded_width,
            coded_height: grid.coded_height,
            sb_size: grid.sb_size,
            blocks: grid.blocks.into_iter().map(|b| crate::commands::PartitionBlockData {
                x: b.x,
                y: b.y,
                width: b.width,
                height: b.height,
                partition: b.partition as u8,
                depth: b.depth,
            }).collect(),
        });

    // Get frame dimensions
    let width = sps.pic_width_in_luma_samples;
    let height = sps.pic_height_in_luma_samples;

    log::info!("extract_hevc_analysis: Returning analysis for frame {} ({}x{})",
        frame_index, width, height);

    Ok(FrameAnalysisData {
        frame_index,
        width,
        height,
        qp_grid,
        mv_grid,
        partition_grid,
        prediction_mode_grid: None, // Not implemented for HEVC yet
        transform_grid: None,       // Not implemented for HEVC yet
    })
}

/// Extract frame analysis for VP9 codec
fn extract_vp9_analysis(
    file_data: &[u8],
    frame_index: usize,
    _core: &bitvue_core::Core,
) -> Result<FrameAnalysisData, String> {
    log::info!("extract_vp9_analysis: Extracting VP9 analysis for frame {}", frame_index);
    log::info!("extract_vp9_analysis: File data size: {} bytes", file_data.len());

    // Parse VP9 stream
    let stream = bitvue_vp9::parse_vp9(&file_data)
        .map_err(|e| format!("Failed to parse VP9 stream: {}", e))?;

    // Get frame header for this frame
    let frame_header = stream.frames.get(frame_index)
        .ok_or("Frame index out of bounds")?;

    // Extract grids using VP9 functions
    let qp_grid = bitvue_vp9::extract_qp_grid(frame_header)
        .ok()
        .map(|grid| QPGridData {
            grid_w: grid.grid_w,
            grid_h: grid.grid_h,
            block_w: grid.block_w,
            block_h: grid.block_h,
            qp: grid.qp,
            qp_min: grid.qp_min,
            qp_max: grid.qp_max,
        });

    let mv_grid = bitvue_vp9::extract_mv_grid(frame_header)
        .ok()
        .map(|grid| MVGridData {
            coded_width: grid.coded_width,
            coded_height: grid.coded_height,
            block_w: grid.block_w,
            block_h: grid.block_h,
            grid_w: grid.grid_w,
            grid_h: grid.grid_h,
            mv_l0: grid.mv_l0.into_iter().map(|mv| MotionVectorData {
                dx_qpel: mv.dx_qpel,
                dy_qpel: mv.dy_qpel,
            }).collect(),
            mv_l1: grid.mv_l1.into_iter().map(|mv| MotionVectorData {
                dx_qpel: mv.dx_qpel,
                dy_qpel: mv.dy_qpel,
            }).collect(),
            mode: grid.mode.map(|modes: Vec<bitvue_core::mv_overlay::BlockMode>| modes.into_iter().map(|m| m as u8).collect()),
        });

    let partition_grid = bitvue_vp9::extract_partition_grid(frame_header)
        .ok()
        .map(|grid| PartitionGridData {
            coded_width: grid.coded_width,
            coded_height: grid.coded_height,
            sb_size: grid.sb_size,
            blocks: grid.blocks.into_iter().map(|b| crate::commands::PartitionBlockData {
                x: b.x,
                y: b.y,
                width: b.width,
                height: b.height,
                partition: b.partition as u8,
                depth: b.depth,
            }).collect(),
        });

    // Get frame dimensions
    let width = frame_header.width;
    let height = frame_header.height;

    log::info!("extract_vp9_analysis: Returning analysis for frame {} ({}x{})",
        frame_index, width, height);

    Ok(FrameAnalysisData {
        frame_index,
        width,
        height,
        qp_grid,
        mv_grid,
        partition_grid,
        prediction_mode_grid: None, // Not implemented for VP9 yet
        transform_grid: None,       // Not implemented for VP9 yet
    })
}

/// Extract frame analysis for VVC/H.266 codec
fn extract_vvc_analysis(
    file_data: &[u8],
    frame_index: usize,
    _core: &bitvue_core::Core,
) -> Result<FrameAnalysisData, String> {
    log::info!("extract_vvc_analysis: Extracting VVC analysis for frame {}", frame_index);
    log::info!("extract_vvc_analysis: File data size: {} bytes", file_data.len());

    // Parse VVC NAL units
    let nal_units = bitvue_vvc::parse_nal_units(&file_data)
        .map_err(|e| format!("Failed to parse NAL units: {}", e))?;

    // Parse SPS for dimensions
    let sps = nal_units.iter()
        .find_map(|nal| {
            if nal.header.nal_unit_type == bitvue_vvc::NalUnitType::SpsNut {
                bitvue_vvc::sps::parse_sps(&nal.payload).ok()
            } else {
                None
            }
        })
        .ok_or("No SPS found in stream")?;

    // Extract grids using VVC functions
    let qp_grid = bitvue_vvc::extract_qp_grid(&nal_units, &sps, 26)
        .ok()
        .map(|grid| QPGridData {
            grid_w: grid.grid_w,
            grid_h: grid.grid_h,
            block_w: grid.block_w,
            block_h: grid.block_h,
            qp: grid.qp,
            qp_min: grid.qp_min,
            qp_max: grid.qp_max,
        });

    let mv_grid = bitvue_vvc::extract_mv_grid(&nal_units, &sps)
        .ok()
        .map(|grid| MVGridData {
            coded_width: grid.coded_width,
            coded_height: grid.coded_height,
            block_w: grid.block_w,
            block_h: grid.block_h,
            grid_w: grid.grid_w,
            grid_h: grid.grid_h,
            mv_l0: grid.mv_l0.into_iter().map(|mv| MotionVectorData {
                dx_qpel: mv.dx_qpel,
                dy_qpel: mv.dy_qpel,
            }).collect(),
            mv_l1: grid.mv_l1.into_iter().map(|mv| MotionVectorData {
                dx_qpel: mv.dx_qpel,
                dy_qpel: mv.dy_qpel,
            }).collect(),
            mode: grid.mode.map(|modes: Vec<bitvue_core::mv_overlay::BlockMode>| modes.into_iter().map(|m| m as u8).collect()),
        });

    let partition_grid = bitvue_vvc::extract_partition_grid(&nal_units, &sps)
        .ok()
        .map(|grid| PartitionGridData {
            coded_width: grid.coded_width,
            coded_height: grid.coded_height,
            sb_size: grid.sb_size,
            blocks: grid.blocks.into_iter().map(|b| crate::commands::PartitionBlockData {
                x: b.x,
                y: b.y,
                width: b.width,
                height: b.height,
                partition: b.partition as u8,
                depth: b.depth,
            }).collect(),
        });

    // Get frame dimensions
    let width = sps.sps_pic_width_max_in_luma_samples;
    let height = sps.sps_pic_height_max_in_luma_samples;

    log::info!("extract_vvc_analysis: Returning analysis for frame {} ({}x{})",
        frame_index, width, height);

    Ok(FrameAnalysisData {
        frame_index,
        width,
        height,
        qp_grid,
        mv_grid,
        partition_grid,
        prediction_mode_grid: None, // Not implemented for VVC yet
        transform_grid: None,       // Not implemented for VVC yet
    })
}

/// Extract frame analysis for AV3 codec
fn extract_av3_analysis(
    file_data: &[u8],
    frame_index: usize,
    _core: &bitvue_core::Core,
) -> Result<FrameAnalysisData, String> {
    log::info!("extract_av3_analysis: Extracting AV3 analysis for frame {}", frame_index);
    log::info!("extract_av3_analysis: File data size: {} bytes", file_data.len());

    // Parse AV3 stream
    let stream = bitvue_av3::parse_av3(&file_data)
        .map_err(|e| format!("Failed to parse AV3 stream: {}", e))?;

    // Get frame header for this frame
    let frame_header = stream.frame_headers.get(frame_index)
        .ok_or("Frame index out of bounds")?;

    // Extract grids using AV3 functions
    let qp_grid = bitvue_av3::extract_qp_grid(frame_header)
        .ok()
        .map(|grid| QPGridData {
            grid_w: grid.grid_w,
            grid_h: grid.grid_h,
            block_w: grid.block_w,
            block_h: grid.block_h,
            qp: grid.qp,
            qp_min: grid.qp_min,
            qp_max: grid.qp_max,
        });

    let mv_grid = bitvue_av3::extract_mv_grid(frame_header)
        .ok()
        .map(|grid| MVGridData {
            coded_width: grid.coded_width,
            coded_height: grid.coded_height,
            block_w: grid.block_w,
            block_h: grid.block_h,
            grid_w: grid.grid_w,
            grid_h: grid.grid_h,
            mv_l0: grid.mv_l0.into_iter().map(|mv| MotionVectorData {
                dx_qpel: mv.dx_qpel,
                dy_qpel: mv.dy_qpel,
            }).collect(),
            mv_l1: grid.mv_l1.into_iter().map(|mv| MotionVectorData {
                dx_qpel: mv.dx_qpel,
                dy_qpel: mv.dy_qpel,
            }).collect(),
            mode: grid.mode.map(|modes: Vec<bitvue_core::mv_overlay::BlockMode>| modes.into_iter().map(|m| m as u8).collect()),
        });

    let partition_grid = bitvue_av3::extract_partition_grid(frame_header)
        .ok()
        .map(|grid| PartitionGridData {
            coded_width: grid.coded_width,
            coded_height: grid.coded_height,
            sb_size: grid.sb_size,
            blocks: grid.blocks.into_iter().map(|b| crate::commands::PartitionBlockData {
                x: b.x,
                y: b.y,
                width: b.width,
                height: b.height,
                partition: b.partition as u8,
                depth: b.depth,
            }).collect(),
        });

    // Get frame dimensions
    let width = frame_header.width;
    let height = frame_header.height;

    log::info!("extract_av3_analysis: Returning analysis for frame {} ({}x{})",
        frame_index, width, height);

    Ok(FrameAnalysisData {
        frame_index,
        width,
        height,
        qp_grid,
        mv_grid,
        partition_grid,
        prediction_mode_grid: None, // Not implemented for AV3 yet
        transform_grid: None,       // Not implemented for AV3 yet
    })
}

// =============================================================================
// Advanced Analysis Commands (Phase 1.2)
// =============================================================================

/// Coding flow analysis data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodingFlowData {
    pub frame_index: usize,
    pub stages: Vec<CodingStage>,
    pub current_stage: String,
    pub codec_features: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodingStage {
    pub id: String,
    pub label: String,
    pub completed: bool,
    pub data_size: Option<usize>,
}

/// Get coding flow analysis for a frame
#[tauri::command]
pub async fn get_coding_flow_analysis(
    state: tauri::State<'_, AppState>,
    frame_index: usize,
) -> Result<CodingFlowData, String> {
    log::info!("get_coding_flow_analysis: Frame {}", frame_index);

    let core = state.core.lock().map_err(|e| e.to_string())?;
    let stream_a = core.get_stream(StreamId::A);
    let stream_a = stream_a.read();
    let file_path = stream_a.file_path.as_ref().ok_or("No file loaded")?.clone();

    let codec = detect_codec_from_path(file_path.to_str().ok_or("Invalid path")?);

    let stages = vec![
        CodingStage {
            id: "input".to_string(),
            label: "Input".to_string(),
            completed: true,
            data_size: None,
        },
        CodingStage {
            id: "prediction".to_string(),
            label: "Prediction".to_string(),
            completed: true,
            data_size: Some(1024),
        },
        CodingStage {
            id: "transform".to_string(),
            label: "Transform".to_string(),
            completed: true,
            data_size: Some(2048),
        },
        CodingStage {
            id: "quantization".to_string(),
            label: "Quantization".to_string(),
            completed: true,
            data_size: Some(512),
        },
        CodingStage {
            id: "entropy".to_string(),
            label: "Entropy Coding".to_string(),
            completed: true,
            data_size: Some(256),
        },
    ];

    let codec_features = match codec.as_str() {
        "av1" => vec!["Directional Intra Pred".to_string(), "Compound Prediction".to_string()],
        "hevc" => vec!["35 Intra Modes".to_string(), "Advanced Motion Vector Pred".to_string()],
        "vvc" => vec!["67 Intra Modes".to_string(), "GPM/Combine Pred".to_string()],
        _ => vec!["Standard Features".to_string()],
    };

    Ok(CodingFlowData {
        frame_index,
        stages,
        current_stage: "prediction".to_string(),
        codec_features,
    })
}

/// Residual analysis data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResidualAnalysisData {
    pub frame_index: usize,
    pub width: u32,
    pub height: u32,
    pub coefficient_stats: CoefficientStats,
    pub block_residuals: Vec<BlockResidualData>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoefficientStats {
    pub min: f32,
    pub max: f32,
    pub mean: f32,
    pub variance: f32,
    pub energy: f64,
    pub zero_count: usize,
    pub non_zero_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockResidualData {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
    pub energy: f32,
    pub max_coeff: f32,
    pub non_zeros: usize,
}

/// Get residual analysis for a frame
#[tauri::command]
pub async fn get_residual_analysis(
    state: tauri::State<'_, AppState>,
    frame_index: usize,
) -> Result<ResidualAnalysisData, String> {
    log::info!("get_residual_analysis: Frame {}", frame_index);

    let core = state.core.lock().map_err(|e| e.to_string())?;
    let stream_a = core.get_stream(StreamId::A);
    let stream_a = stream_a.read();

    // Get frame data
    let units = stream_a.units.as_ref().ok_or("No units loaded")?;
    let _frame_data = units.units.get(frame_index).ok_or("Frame not found")?;

    // Use default dimensions for now - this is mock data
    let width = 1920u32;
    let height = 1080u32;

    // Generate simple mock residual data
    let block_size = 16u32;
    let grid_w = width.div_ceil(block_size);
    let grid_h = height.div_ceil(block_size);

    let block_residuals: Vec<BlockResidualData> = (0..grid_h)
        .flat_map(|y| {
            (0..grid_w).map(move |x| {
                let base_idx = (y * grid_w + x) as f32;
                BlockResidualData {
                    x: x * block_size,
                    y: y * block_size,
                    width: block_size,
                    height: block_size,
                    energy: (base_idx % 100.0) + 10.0,
                    max_coeff: (base_idx % 255.0) + 1.0,
                    non_zeros: ((base_idx % (block_size * block_size) as f32) as usize) + 1,
                }
            })
        })
        .take(100) // Limit to 100 blocks for performance
        .collect();

    let _total_blocks = block_residuals.len();
    let all_coeffs: Vec<f32> = block_residuals.iter()
        .flat_map(|b| {
            let mut coeffs = Vec::with_capacity(b.non_zeros.min(10));
            for i in 0..b.non_zeros.min(10) {
                coeffs.push(b.max_coeff * (i as f32 + 1.0) / (b.non_zeros as f32 + 1.0));
            }
            coeffs
        })
        .collect();

    let coefficient_stats = if all_coeffs.is_empty() {
        CoefficientStats {
            min: 0.0,
            max: 0.0,
            mean: 0.0,
            variance: 0.0,
            energy: 0.0,
            zero_count: (width * height) as usize,
            non_zero_count: 0,
        }
    } else {
        let mean = all_coeffs.iter().sum::<f32>() / all_coeffs.len() as f32;
        let variance = all_coeffs.iter()
            .map(|v| (v - mean).powi(2))
            .sum::<f32>() / all_coeffs.len() as f32;
        let energy = all_coeffs.iter().map(|v| (v * v) as f64).sum::<f64>();

        CoefficientStats {
            min: all_coeffs.iter().cloned().reduce(f32::min).unwrap_or(0.0),
            max: all_coeffs.iter().cloned().reduce(f32::max).unwrap_or(0.0),
            mean,
            variance: variance.sqrt(),
            energy,
            zero_count: (width * height) as usize - all_coeffs.len(),
            non_zero_count: all_coeffs.len(),
        }
    };

    Ok(ResidualAnalysisData {
        frame_index,
        width,
        height,
        coefficient_stats,
        block_residuals,
    })
}

/// Deblocking analysis data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeblockingAnalysisData {
    pub frame_index: usize,
    pub width: u32,
    pub height: u32,
    pub boundaries: Vec<BoundaryEdgeData>,
    pub params: DeblockingParams,
    pub stats: DeblockingStats,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoundaryEdgeData {
    pub x: u32,
    pub y: u32,
    pub length: u32,
    pub orientation: String, // "vertical" or "horizontal"
    pub strength: f32,
    pub filtered: bool,
    pub bs: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeblockingParams {
    pub beta_offset: i8,
    pub tc_offset: i8,
    pub filter_strength: u8,
    pub chroma_edge: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeblockingStats {
    pub total_boundaries: usize,
    pub filtered_boundaries: usize,
    pub strong_boundaries: usize,
    pub weak_boundaries: usize,
}

/// Get deblocking analysis for a frame
#[tauri::command]
pub async fn get_deblocking_analysis(
    state: tauri::State<'_, AppState>,
    frame_index: usize,
) -> Result<DeblockingAnalysisData, String> {
    log::info!("get_deblocking_analysis: Frame {}", frame_index);

    let core = state.core.lock().map_err(|e| e.to_string())?;
    let stream_a = core.get_stream(StreamId::A);
    let stream_a = stream_a.read();

    let units = stream_a.units.as_ref().ok_or("No units loaded")?;
    let _frame_data = units.units.get(frame_index).ok_or("Frame not found")?;

    // Use default dimensions for now - this is mock data
    let width = 1920u32;
    let height = 1080u32;

    // Generate simple mock boundary data
    let block_size = 8u32;
    let mut boundaries = Vec::new();
    let mut bs_idx = 0u8;

    // Vertical edges (limited for performance)
    for y in (0..height).step_by(block_size as usize).take(20) {
        for x in (block_size..width).step_by(block_size as usize).take(30) {
            let bs = bs_idx % 5;
            bs_idx = bs_idx.wrapping_add(1);
            boundaries.push(BoundaryEdgeData {
                x,
                y,
                length: block_size,
                orientation: "vertical".to_string(),
                strength: if bs > 0 { (bs as f32) / 4.0 * 4.0 } else { 0.0 },
                filtered: bs > 0 && (bs_idx % 10) > 3,
                bs,
            });
        }
    }

    // Horizontal edges (limited for performance)
    for y in (block_size..height).step_by(block_size as usize).take(20) {
        for x in (0..width).step_by(block_size as usize).take(30) {
            let bs = bs_idx % 5;
            bs_idx = bs_idx.wrapping_add(1);
            boundaries.push(BoundaryEdgeData {
                x,
                y,
                length: block_size,
                orientation: "horizontal".to_string(),
                strength: if bs > 0 { (bs as f32) / 4.0 * 4.0 } else { 0.0 },
                filtered: bs > 0 && (bs_idx % 10) > 3,
                bs,
            });
        }
    }

    let total_count = boundaries.len();
    let filtered_count = boundaries.iter().filter(|b| b.filtered).count();
    let strong_count = boundaries.iter().filter(|b| b.bs >= 3).count();
    let weak_count = boundaries.iter().filter(|b| b.bs > 0 && b.bs < 3).count();

    Ok(DeblockingAnalysisData {
        frame_index,
        width,
        height,
        boundaries,
        params: DeblockingParams {
            beta_offset: 0,
            tc_offset: 0,
            filter_strength: 1,
            chroma_edge: true,
        },
        stats: DeblockingStats {
            total_boundaries: total_count,
            filtered_boundaries: filtered_count,
            strong_boundaries: strong_count,
            weak_boundaries: weak_count,
        },
    })
}

