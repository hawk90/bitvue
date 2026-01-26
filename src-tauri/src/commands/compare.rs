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

/// Create frame metadata for a frame count
///
/// Creates placeholder frame metadata for alignment testing.
/// In production, this would be extracted from actual stream data.
fn create_placeholder_frame_metadata(frame_count: usize, pts_start: u64, pts_increment: u64) -> Vec<FrameMetadata> {
    (0..frame_count)
        .map(|i| FrameMetadata {
            pts: Some(pts_start + (i as u64) * pts_increment),
            dts: Some(pts_start + (i as u64) * pts_increment),
        })
        .collect()
}

/// Create compare workspace from two files
///
/// Opens two video files and creates a compare workspace with:
/// - Automatic PTS-based alignment
/// - Fallback to display_idx alignment
/// - Resolution compatibility check
#[tauri::command]
pub async fn create_compare_workspace(
    _state: tauri::State<'_, AppState>,
    path_a: String,
    path_b: String,
) -> Result<CompareWorkspaceData, String> {
    log::info!("create_compare_workspace: A={}, B={}", path_a, path_b);

    // Validate files exist
    if !Path::new(&path_a).exists() {
        return Err(format!("Stream A file not found: {}", path_a));
    }
    if !Path::new(&path_b).exists() {
        return Err(format!("Stream B file not found: {}", path_b));
    }

    // TODO: Load actual frame data from streams
    // For now, create placeholder frame index maps

    // Create placeholder frame metadata (1000 frames each, 1000 PTS per frame)
    let frame_count_a = 1000;
    let frame_count_b = 1000;

    let frames_a = create_placeholder_frame_metadata(frame_count_a, 0, 1000);
    let frames_b = create_placeholder_frame_metadata(frame_count_b, 0, 1000);

    // Create frame index maps
    let stream_a = FrameIndexMap::new(&frames_a);
    let stream_b = FrameIndexMap::new(&frames_b);

    // Create alignment engine
    let _alignment = AlignmentEngine::new(&stream_a, &stream_b);

    // Create resolution info (placeholder)
    let resolution_a = (1920, 1080);
    let resolution_b = (1920, 1080);
    let _resolution_info = ResolutionInfo::new(resolution_a, resolution_b);

    // Create compare workspace
    let workspace = CompareWorkspace::new(
        stream_a,
        stream_b,
        resolution_a,
        resolution_b,
    );

    Ok(workspace.into())
}

/// Get aligned frame for stream A index
///
/// Returns the corresponding frame index in stream B with alignment quality.
#[tauri::command]
pub async fn get_aligned_frame(
    _state: tauri::State<'_, AppState>,
    stream_a_idx: usize,
) -> Result<(usize, String), String> {
    log::info!("get_aligned_frame: stream_a_idx={}", stream_a_idx);

    // TODO: Implement proper alignment lookup using stored workspace state
    // For now, return the same index as placeholder
    Ok((stream_a_idx, "Exact".to_string()))
}

/// Set sync mode for compare workspace
#[tauri::command]
pub async fn set_sync_mode(
    _state: tauri::State<'_, AppState>,
    mode: String,
) -> Result<(), String> {
    log::info!("set_sync_mode: mode={}", mode);

    // TODO: Implement sync mode setting
    Ok(())
}

/// Set manual offset for compare workspace
///
/// Positive offset = B is ahead of A
/// Negative offset = B is behind A
#[tauri::command]
pub async fn set_manual_offset(
    _state: tauri::State<'_, AppState>,
    offset: i32,
) -> Result<(), String> {
    log::info!("set_manual_offset: offset={}", offset);

    // TODO: Implement manual offset setting
    Ok(())
}

/// Reset manual offset to 0
#[tauri::command]
pub async fn reset_offset(
    _state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    log::info!("reset_offset");

    // TODO: Implement offset reset
    Ok(())
}
