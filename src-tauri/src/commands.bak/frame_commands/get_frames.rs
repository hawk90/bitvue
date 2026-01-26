//! Get frames commands

use super::types::FrameData;
use crate::commands::AppState;
use crate::services::FrameService;
use bitvue_core::StreamId;

/// Get all frames for filmstrip display
pub async fn get_frames_impl(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<FrameData>, String> {
    let core = state.core.lock().map_err(|e| e.to_string())?;

    // Get stream state
    let stream_a_lock = core.get_stream(StreamId::A);
    let stream_a = stream_a_lock.read();

    // Get units from StreamState
    if let Some(unit_model) = &stream_a.units {
        let frames = FrameService::collect_frames(unit_model);

        tracing::info!("get_frames: Returning {} frames", frames.len());
        Ok(frames.into_iter().map(|f| FrameData {
            frame_index: f.frame_index,
            frame_type: f.frame_type,
            offset: f.offset,
            size: f.size,
            poc: f.poc,
            nal_type: f.nal_type,
            layer: f.layer,
            pts: f.pts,
            dts: f.dts,
            ref_list: f.ref_list,
        }).collect())
    } else {
        tracing::warn!("get_frames: No units found in stream");
        Ok(Vec::new())
    }
}

/// Get all frames for Stream B (dependent stream)
pub async fn get_frames_b_impl(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<FrameData>, String> {
    let core = state.core.lock().map_err(|e| e.to_string())?;

    // Get stream state B
    let stream_b_lock = core.get_stream(StreamId::B);
    let stream_b = stream_b_lock.read();

    // Get units from StreamState
    if let Some(unit_model) = &stream_b.units {
        let frames = FrameService::collect_frames(unit_model);

        tracing::info!("get_frames_b: Returning {} frames from Stream B", frames.len());
        Ok(frames.into_iter().map(|f| FrameData {
            frame_index: f.frame_index,
            frame_type: f.frame_type,
            offset: f.offset,
            size: f.size,
            poc: f.poc,
            nal_type: f.nal_type,
            layer: f.layer,
            pts: f.pts,
            dts: f.dts,
            ref_list: f.ref_list,
        }).collect())
    } else {
        tracing::warn!("get_frames_b: No units found in Stream B");
        Ok(Vec::new())
    }
}
