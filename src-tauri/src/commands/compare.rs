//! Compare Commands - T6-2
//!
//! Commands for A/B compare workspace functionality.
//!
//! Per COMPARE_ALIGNMENT_POLICY.md and LAYOUT_CONTRACT.md:
//! - Side-by-side comparison with automatic alignment
//! - Sync controls (Off/Playhead/Full)
//! - Manual offset adjustment
//! - Resolution mismatch detection

use crate::commands::AppState;
use bitvue_core::{
    AlignmentEngine, AlignmentMethod, AlignmentConfidence, CompareWorkspace, ResolutionInfo,
    SyncMode, FrameIndexMap, FramePair,
};
use bitvue_core::frame_identity::FrameMetadata;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Frame index map for alignment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrameIndexMapData {
    pub frame_count: usize,
    pub pts_values: Vec<Option<u64>>,
    pub display_indices: Vec<usize>,
    pub coding_indices: Vec<usize>,
}

impl From<FrameIndexMap> for FrameIndexMapData {
    fn from(map: FrameIndexMap) -> Self {
        let frame_count = map.frame_count();
        Self {
            frame_count,
            pts_values: (0..frame_count)
                .map(|i| map.get_pts(i))
                .collect(),
            display_indices: (0..frame_count).collect(),
            coding_indices: (0..frame_count).collect(),
        }
    }
}

/// Frame pair in alignment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FramePairData {
    pub stream_a_idx: Option<usize>,
    pub stream_b_idx: Option<usize>,
    pub pts_delta: Option<i64>,
    pub has_gap: bool,
}

impl From<FramePair> for FramePairData {
    fn from(pair: FramePair) -> Self {
        Self {
            stream_a_idx: pair.stream_a_idx,
            stream_b_idx: pair.stream_b_idx,
            pts_delta: pair.pts_delta,
            has_gap: pair.has_gap,
        }
    }
}

/// Alignment engine result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlignmentEngineData {
    pub stream_a_count: usize,
    pub stream_b_count: usize,
    pub method: String,
    pub confidence: String,
    pub frame_pairs: Vec<FramePairData>,
    pub gap_count: usize,
}

impl From<AlignmentEngine> for AlignmentEngineData {
    fn from(engine: AlignmentEngine) -> Self {
        Self {
            stream_a_count: engine.stream_a_count,
            stream_b_count: engine.stream_b_count,
            method: match engine.method {
                AlignmentMethod::PtsExact => "PtsExact".to_string(),
                AlignmentMethod::PtsNearest => "PtsNearest".to_string(),
                AlignmentMethod::DisplayIdx => "DisplayIdx".to_string(),
            },
            confidence: match engine.confidence {
                AlignmentConfidence::High => "High".to_string(),
                AlignmentConfidence::Medium => "Medium".to_string(),
                AlignmentConfidence::Low => "Low".to_string(),
            },
            frame_pairs: engine.frame_pairs.into_iter().map(Into::into).collect(),
            gap_count: engine.gap_count,
        }
    }
}

/// Resolution information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolutionInfoData {
    pub stream_a: (u32, u32),
    pub stream_b: (u32, u32),
    pub tolerance: f64,
    pub is_compatible: bool,
    pub mismatch_percentage: f64,
    pub is_exact_match: bool,
    pub scale_indicator: String,
}

impl From<ResolutionInfo> for ResolutionInfoData {
    fn from(info: ResolutionInfo) -> Self {
        Self {
            stream_a: info.stream_a,
            stream_b: info.stream_b,
            tolerance: info.tolerance,
            is_compatible: info.is_compatible(),
            mismatch_percentage: info.mismatch_percentage(),
            is_exact_match: info.is_exact_match(),
            scale_indicator: info.scale_indicator(),
        }
    }
}

/// Compare workspace data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompareWorkspaceData {
    pub stream_a: FrameIndexMapData,
    pub stream_b: FrameIndexMapData,
    pub alignment: AlignmentEngineData,
    pub manual_offset: i32,
    pub sync_mode: String,
    pub resolution_info: ResolutionInfoData,
    pub diff_enabled: bool,
    pub disable_reason: Option<String>,
}

impl From<CompareWorkspace> for CompareWorkspaceData {
    fn from(workspace: CompareWorkspace) -> Self {
        let diff_enabled = workspace.is_diff_enabled();
        let disable_reason = workspace.disable_reason().map(ToString::to_string);

        Self {
            stream_a: workspace.stream_a.into(),
            stream_b: workspace.stream_b.into(),
            alignment: workspace.alignment.into(),
            manual_offset: workspace.manual_offset,
            sync_mode: match workspace.sync_mode {
                SyncMode::Off => "Off".to_string(),
                SyncMode::Playhead => "Playhead".to_string(),
                SyncMode::Full => "Full".to_string(),
            },
            resolution_info: workspace.resolution_info.into(),
            diff_enabled,
            disable_reason,
        }
    }
}

/// Parse video file and extract frame metadata
///
/// Reads a video file and extracts frame metadata for alignment.
/// Supports IVF, MP4, and MKV containers.
fn parse_video_metadata(path: &Path) -> Result<Vec<FrameMetadata>, String> {
    use bitvue_formats::detect_container_format;

    let file_data = std::fs::read(path)
        .map_err(|e| format!("Failed to read file: {}", e))?;

    let container_format = detect_container_format(path)
        .unwrap_or(bitvue_formats::ContainerFormat::Unknown);

    match container_format {
        bitvue_formats::ContainerFormat::IVF => {
            // Parse IVF header and frames
            use bitvue_av1::parse_ivf_frames;
            let (_header, frames) = parse_ivf_frames(&file_data)
                .map_err(|e| format!("Failed to parse IVF: {}", e))?;

            Ok(frames.iter().enumerate().map(|(_idx, frame)| {
                FrameMetadata {
                    pts: Some(frame.timestamp),
                    dts: Some(frame.timestamp),
                }
            }).collect())
        }
        bitvue_formats::ContainerFormat::MP4 => {
            // Extract AV1 samples from MP4
            match bitvue_formats::mp4::extract_av1_samples(&file_data) {
                Ok(samples) => {
                    Ok(samples.iter().enumerate().map(|(idx, _sample)| {
                        FrameMetadata {
                            pts: Some(idx as u64 * 1000), // Approximate PTS
                            dts: Some(idx as u64 * 1000),
                        }
                    }).collect())
                }
                Err(_) => {
                    // Try other codecs
                    Ok(Vec::new())
                }
            }
        }
        bitvue_formats::ContainerFormat::Matroska => {
            // Extract AV1 samples from MKV
            match bitvue_formats::mkv::extract_av1_samples(&file_data) {
                Ok(samples) => {
                    Ok(samples.iter().enumerate().map(|(idx, _sample)| {
                        FrameMetadata {
                            pts: Some(idx as u64 * 1000),
                            dts: Some(idx as u64 * 1000),
                        }
                    }).collect())
                }
                Err(_) => Ok(Vec::new())
            }
        }
        _ => Err(format!("Unsupported container format: {:?}", container_format))
    }
}

/// Get resolution from video file
///
/// Attempts to extract resolution from video file.
/// Returns default (1920x1080) if unable to determine.
fn get_video_resolution(path: &Path) -> (u32, u32) {
    // Try to parse IVF header for resolution
    if let Ok(file_data) = std::fs::read(path) {
        if file_data.len() >= 32 && &file_data[0..4] == b"DKIF" {
            // IVF format - width is at byte 12, height at byte 14
            let width = u16::from_le_bytes([file_data[12], file_data[13]]) as u32;
            let height = u16::from_le_bytes([file_data[14], file_data[15]]) as u32;
            if width > 0 && height > 0 {
                return (width, height);
            }
        }
    }
    // Default to 1080p
    (1920, 1080)
}

/// Create compare workspace from two files
///
/// Opens two video files and creates a compare workspace with:
/// - Automatic PTS-based alignment
/// - Fallback to display_idx alignment
/// - Resolution compatibility check
#[tauri::command]
pub async fn create_compare_workspace(
    state: tauri::State<'_, AppState>,
    path_a: String,
    path_b: String,
) -> Result<CompareWorkspaceData, String> {
    log::info!("create_compare_workspace: A={}, B={}", path_a, path_b);

    // Validate files exist
    let path_a_buf = Path::new(&path_a);
    let path_b_buf = Path::new(&path_b);

    if !path_a_buf.exists() {
        // SECURITY: Use generic error to avoid revealing file path
        return Err("Stream A file not found".to_string());
    }
    if !path_b_buf.exists() {
        // SECURITY: Use generic error to avoid revealing file path
        return Err("Stream B file not found".to_string());
    }

    // Parse frame metadata from files
    let frames_a = parse_video_metadata(path_a_buf)?;
    let frames_b = parse_video_metadata(path_b_buf)?;

    if frames_a.is_empty() {
        return Err("Failed to extract frames from stream A".to_string());
    }
    if frames_b.is_empty() {
        return Err("Failed to extract frames from stream B".to_string());
    }

    // Get resolutions
    let resolution_a = get_video_resolution(path_a_buf);
    let resolution_b = get_video_resolution(path_b_buf);

    log::info!("create_compare_workspace: A={} frames ({}x{}), B={} frames ({}x{})",
        frames_a.len(), resolution_a.0, resolution_a.1,
        frames_b.len(), resolution_b.0, resolution_b.1);

    // Create frame index maps
    let stream_a = FrameIndexMap::new(&frames_a);
    let stream_b = FrameIndexMap::new(&frames_b);

    // Create alignment engine
    let _alignment = AlignmentEngine::new(&stream_a, &stream_b);

    // Create compare workspace
    let workspace = CompareWorkspace::new(
        stream_a,
        stream_b,
        resolution_a,
        resolution_b,
    );

    // Store workspace in state
    {
        let mut workspace_guard = state.compare_workspace.lock()
            .map_err(|e| format!("Failed to lock workspace: {}", e))?;
        *workspace_guard = Some(workspace);
    }

    // Return workspace data
    let workspace_guard = state.compare_workspace.lock()
        .map_err(|e| format!("Failed to lock workspace: {}", e))?;
    let workspace = workspace_guard.as_ref()
        .ok_or("Workspace not initialized")?;

    Ok(workspace.clone().into())
}

/// Get aligned frame for stream A index
///
/// Returns the corresponding frame index in stream B with alignment.
#[tauri::command]
pub async fn get_aligned_frame(
    state: tauri::State<'_, AppState>,
    stream_a_idx: usize,
) -> Result<(usize, String), String> {
    log::info!("get_aligned_frame: stream_a_idx={}", stream_a_idx);

    let workspace_guard = state.compare_workspace.lock()
        .map_err(|e| format!("Failed to lock workspace: {}", e))?;
    let workspace = workspace_guard.as_ref()
        .ok_or("No compare workspace created")?;

    // SECURITY: Validate frame index bounds before accessing workspace data
    if stream_a_idx >= workspace.stream_a.frame_count {
        return Err(format!(
            "Frame index {} out of bounds (stream A has {} frames)",
            stream_a_idx, workspace.stream_a.frame_count
        ));
    }

    match workspace.get_aligned_frame(stream_a_idx) {
        Some((b_idx, quality)) => Ok((b_idx, format!("{:?}", quality))),
        None => Err(format!(
            "No aligned frame found for index {} (stream A has {} frames)",
            stream_a_idx, workspace.stream_a.frame_count
        )),
    }
}

/// Set sync mode for compare workspace
#[tauri::command]
pub async fn set_sync_mode(
    state: tauri::State<'_, AppState>,
    mode: String,
) -> Result<(), String> {
    log::info!("set_sync_mode: mode={}", mode);

    let sync_mode = match mode.as_str() {
        "Off" => SyncMode::Off,
        "Playhead" => SyncMode::Playhead,
        "Full" => SyncMode::Full,
        _ => return Err(format!("Invalid sync mode: {}", mode)),
    };

    let mut workspace_guard = state.compare_workspace.lock()
        .map_err(|e| format!("Failed to lock workspace: {}", e))?;

    if let Some(workspace) = workspace_guard.as_mut() {
        workspace.set_sync_mode(sync_mode);
        log::info!("set_sync_mode: Sync mode set to {:?}", sync_mode);
        Ok(())
    } else {
        Err("No compare workspace created".to_string())
    }
}

/// Set manual offset for compare workspace
///
/// Positive offset = B is ahead of A
/// Negative offset = B is behind A
#[tauri::command]
pub async fn set_manual_offset(
    state: tauri::State<'_, AppState>,
    offset: i32,
) -> Result<(), String> {
    log::info!("set_manual_offset: offset={}", offset);

    let mut workspace_guard = state.compare_workspace.lock()
        .map_err(|e| format!("Failed to lock workspace: {}", e))?;

    if let Some(workspace) = workspace_guard.as_mut() {
        workspace.set_manual_offset(offset);
        log::info!("set_manual_offset: Manual offset set to {}", offset);
        Ok(())
    } else {
        Err("No compare workspace created".to_string())
    }
}

/// Reset manual offset to 0
#[tauri::command]
pub async fn reset_offset(
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    log::info!("reset_offset");

    set_manual_offset(state, 0).await
}
