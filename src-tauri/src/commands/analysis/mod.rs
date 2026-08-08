//! Frame analysis commands
//!
//! Commands for getting frame analysis data (QP, MV, partition, etc.).
//! Supports multiple codecs: AV1, H.264/AVC, HEVC, VP9, VVC/H.266, AV3.

mod extractors;
pub mod views;

pub use views::{
    Av1FeaturesData, BlockResidualData, BoundaryEdgeData, CodingFlowData, CodingStage,
    CoefficientStats, DeblockingAnalysisData, DeblockingParams, DeblockingStats,
    ResidualAnalysisData,
};

use bitvue_av1_codec::overlay_extraction::{
    extract_mv_grid, extract_partition_grid, extract_prediction_mode_grid, extract_qp_grid,
    extract_transform_grid,
};
use bitvue_engine::StreamId;
use serde::{Deserialize, Serialize};

use crate::commands::{
    AppState, FrameAnalysisData, MVGridData, MotionVectorData, PartitionGridData,
    PredictionModeGridData, QPGridData, TransformGridData,
};

/// Validate frame_index against the loaded stream's frame count
/// Returns an error if frame_index is out of bounds
pub(crate) fn validate_frame_index(
    state: &tauri::State<'_, AppState>,
    frame_index: usize,
) -> Result<(), String> {
    let core = state.core.lock().map_err(|e| e.to_string())?;
    let stream_a_lock = core.get_stream(StreamId::A);
    let stream_a = stream_a_lock.read();

    if let Some(units) = &stream_a.units {
        let frame_count = units.frame_count;
        if frame_index >= frame_count {
            return Err(format!(
                "Frame index {} out of bounds (total frames: {})",
                frame_index, frame_count
            ));
        }
    }
    Ok(())
}

/// Supported video codecs for analysis
#[allow(clippy::upper_case_acronyms)]
pub(crate) enum VideoCodec {
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
    pub(crate) fn from_str(s: &str) -> Self {
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

/// Extract frame dimensions (width, height) from the bitstream by parsing SPS/SequenceHeader.
/// Returns (1920, 1080) as a safe fallback if parsing fails.
pub(crate) fn extract_stream_dimensions(file_data: &[u8], codec: &str) -> (u32, u32) {
    match VideoCodec::from_str(codec) {
        VideoCodec::AVC => {
            let Ok(nal_units) = bitvue_avc::parse_nal_units(file_data) else {
                return (1920, 1080);
            };
            nal_units
                .iter()
                .find_map(|nal| {
                    if nal.header.nal_unit_type == bitvue_avc::NalUnitType::Sps {
                        bitvue_avc::sps::parse_sps(&nal.payload)
                            .ok()
                            .map(|sps| (sps.display_width(), sps.display_height()))
                    } else {
                        None
                    }
                })
                .unwrap_or((1920, 1080))
        }
        VideoCodec::HEVC => {
            let Ok(nal_units) = bitvue_hevc::parse_nal_units(file_data) else {
                return (1920, 1080);
            };
            nal_units
                .iter()
                .find_map(|nal| {
                    if nal.header.nal_unit_type == bitvue_hevc::NalUnitType::SpsNut {
                        bitvue_hevc::sps::parse_sps(&nal.payload)
                            .ok()
                            .map(|sps| (sps.display_width(), sps.display_height()))
                    } else {
                        None
                    }
                })
                .unwrap_or((1920, 1080))
        }
        VideoCodec::AV1 => {
            // IVF header stores width/height at bytes 12-15 and 16-19 (little-endian u16 each)
            if file_data.len() >= 20 {
                let w = u16::from_le_bytes([file_data[12], file_data[13]]) as u32;
                let h = u16::from_le_bytes([file_data[14], file_data[15]]) as u32;
                if w > 0 && h > 0 {
                    return (w, h);
                }
            }
            (1920, 1080)
        }
        _ => (1920, 1080),
    }
}

/// Helper: Load file data and detect codec
///
/// Extracts file path, loads cached file data, and detects codec type.
/// Reduces nesting in get_frame_analysis.
pub(crate) async fn load_file_data_and_codec(
    state: &tauri::State<'_, AppState>,
) -> Result<(Vec<u8>, String), String> {
    let (file_path, file_data) = {
        let core = state.core.lock().map_err(|e| e.to_string())?;
        let stream_a_lock = core.get_stream(StreamId::A);
        let stream_a = stream_a_lock.read();
        let file_path = stream_a.file_path.as_ref().ok_or("No file loaded")?.clone();

        // SECURITY: Don't log unit count to prevent information disclosure
        log::info!("get_frame_analysis: Stream A loaded");

        // Use cached file data from decode_service to avoid repeated disk reads
        let file_data = state
            .decode_service
            .lock()
            .map_err(|e| e.to_string())?
            .get_file_data()?;

        // SECURITY: Don't log file data size to prevent information leakage
        (file_path, file_data)
    };

    // Detect codec from file extension or stream metadata
    let codec = detect_codec_from_path(&file_path.to_string_lossy());
    log::info!("get_frame_analysis: Detected codec: {}", codec);

    Ok((file_data, codec))
}

/// Helper: Extract frame analysis based on codec type
///
/// Matches codec type and calls appropriate extraction function.
/// Falls back to trying multiple codecs for unknown types.
pub(crate) fn extract_analysis_by_codec(
    file_data: &[u8],
    frame_index: usize,
    core: &bitvue_engine::Core,
    codec: &str,
) -> Result<FrameAnalysisData, String> {
    let video_codec = VideoCodec::from_str(codec);

    match video_codec {
        VideoCodec::AV1 => extractors::extract_av1_analysis(file_data, frame_index, core),
        VideoCodec::AVC => extractors::extract_avc_analysis(file_data, frame_index, core),
        VideoCodec::HEVC => extractors::extract_hevc_analysis(file_data, frame_index, core),
        VideoCodec::VP9 => extractors::extract_vp9_analysis(file_data, frame_index, core),
        VideoCodec::VVC => extractors::extract_vvc_analysis(file_data, frame_index, core),
        VideoCodec::AV3 => extractors::extract_av3_analysis(file_data, frame_index, core),
        VideoCodec::Unknown => {
            // Try codecs in order of likelihood
            log::warn!("get_frame_analysis: Unknown codec, trying AV1");
            extractors::extract_av1_analysis(file_data, frame_index, core)
                .or_else(|_| {
                    log::warn!("get_frame_analysis: AV1 failed, trying AVC");
                    extractors::extract_avc_analysis(file_data, frame_index, core)
                })
                .or_else(|_| {
                    log::warn!("get_frame_analysis: AVC failed, trying HEVC");
                    extractors::extract_hevc_analysis(file_data, frame_index, core)
                })
                .or_else(|_| {
                    log::warn!("get_frame_analysis: HEVC failed, trying VP9");
                    extractors::extract_vp9_analysis(file_data, frame_index, core)
                })
                .or_else(|_| {
                    log::warn!("get_frame_analysis: VP9 failed, trying VVC");
                    extractors::extract_vvc_analysis(file_data, frame_index, core)
                })
        }
    }
}

/// Helper: Log analysis result
fn log_analysis_result(result: &Result<FrameAnalysisData, String>) {
    match result {
        Ok(analysis) => {
            log::info!("get_frame_analysis: === Analysis successful ===");
            log::info!(
                "get_frame_analysis: QP grid: {}",
                if analysis.qp_grid.is_some() {
                    "present"
                } else {
                    "none"
                }
            );
            log::info!(
                "get_frame_analysis: MV grid: {}",
                if analysis.mv_grid.is_some() {
                    "present"
                } else {
                    "none"
                }
            );
            log::info!(
                "get_frame_analysis: Partition grid: {}",
                if analysis.partition_grid.is_some() {
                    "present"
                } else {
                    "none"
                }
            );
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

    // SECURITY: Apply rate limiting to prevent CPU exhaustion attacks
    state.rate_limiter.check_rate_limit().map_err(|wait_time| {
        format!(
            "Rate limited: too many analysis requests. Please try again in {:.1}s",
            wait_time.as_secs_f64()
        )
    })?;

    log::info!("get_frame_analysis: Frame index: {}", frame_index);

    // SECURITY: Validate frame_index bounds early to prevent out-of-bounds access
    validate_frame_index(&state, frame_index)?;

    // Load file data and detect codec
    let (file_data, codec) = load_file_data_and_codec(&state).await?;

    // Extract analysis based on codec type
    log::info!(
        "get_frame_analysis: Selecting extraction function for codec: {}",
        codec
    );
    let core = state.core.lock().map_err(|e| e.to_string())?;
    let result = extract_analysis_by_codec(&file_data, frame_index, &core, &codec);

    // Log analysis result
    log_analysis_result(&result);

    result
}

/// Detect codec from file path
pub(crate) fn detect_codec_from_path(path: &str) -> String {
    let path_buf = std::path::PathBuf::from(path);
    path_buf
        .extension()
        .and_then(|e| e.to_str())
        .and_then(|ext| match ext.to_lowercase().as_str() {
            "ivf" => Some("av1".to_string()),
            "webm" => Some("vp9".to_string()),
            "mkv" => None,
            "mp4" | "mov" => None,
            "h264" | "264" | "avc" => Some("avc".to_string()),
            "h265" | "265" | "hevc" => Some("hevc".to_string()),
            "h266" | "266" | "vvc" => Some("vvc".to_string()),
            "av1" => Some("av1".to_string()),
            "av3" => Some("av3".to_string()),
            "vp9" => Some("vp9".to_string()),
            _ => None,
        })
        .unwrap_or_else(|| detect_codec_from_content(path))
}

/// Detect codec from file content (magic bytes)
/// SECURITY: Validates path before reading to prevent path traversal
fn detect_codec_from_content(path: &str) -> String {
    if let Ok(validated_path) = super::file::validate_file_path(path) {
        if let Ok(data) = std::fs::read(&validated_path) {
            return detect_codec_from_data(&data);
        }
    }
    "unknown".to_string()
}

/// Detect codec from byte data (magic bytes)
pub(crate) fn detect_codec_from_data(data: &[u8]) -> String {
    if data.len() >= 4 {
        let magic = &data[0..4];
        if magic == b"DKIF" {
            if data.len() >= 32 {
                let codec_tag = &data[4..8];
                if codec_tag == b"AV01" {
                    return "av1".to_string();
                } else if codec_tag == b"VP90" {
                    return "vp9".to_string();
                } else if codec_tag == b"AV03" {
                    return "av3".to_string();
                }
            }
            return "av1".to_string();
        } else if magic == [0x1A, 0x45, 0xDF, 0xA3] {
            return "vp9".to_string();
        } else if data.len() >= 8 {
            let box_type = &data[4..8];
            if box_type == b"ftyp" {
                let search_range = &data[..data.len().min(8192)];
                if search_range
                    .windows(4)
                    .any(|w| w == b"vvc1" || w == b"vvi1")
                {
                    return "vvc".to_string();
                } else if search_range
                    .windows(4)
                    .any(|w| w == b"hvc1" || w == b"hev1")
                {
                    return "hevc".to_string();
                } else if search_range
                    .windows(4)
                    .any(|w| w == b"avc1" || w == b"avc3")
                {
                    return "avc".to_string();
                } else if search_range.windows(4).any(|w| w == b"av01") {
                    return "av1".to_string();
                } else if search_range.windows(4).any(|w| w == b"av03") {
                    return "av3".to_string();
                }
                return "vvc".to_string();
            }
        }
    }
    "unknown".to_string()
}
