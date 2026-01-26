//! Frame-related Tauri commands
//!
//! Commands for:
//! - Getting frame information
//! - Decoding frames
//! - Generating thumbnails
//! - YUV data and visualization
//! - QP and MV overlays

mod types;
mod get_frames;
mod decode;
mod thumbnails;
mod yuv;
mod overlay;

// Re-export types
pub use types::*;

// Tauri commands - wrappers to fix module path issues
#[tauri::command]
pub async fn get_frames(
    state: tauri::State<'_, crate::commands::AppState>,
) -> Result<Vec<FrameData>, String> {
    get_frames::get_frames_impl(state).await
}

#[tauri::command]
pub async fn get_frames_b(
    state: tauri::State<'_, crate::commands::AppState>,
) -> Result<Vec<FrameData>, String> {
    get_frames::get_frames_b_impl(state).await
}

#[tauri::command]
pub async fn decode_frame(
    frame_index: usize,
    state: tauri::State<'_, crate::commands::AppState>,
) -> Result<DecodedFrameResponse, String> {
    decode::decode_frame_impl(frame_index, state).await
}

#[tauri::command]
pub async fn decode_frames_batch(
    frame_indices: Vec<usize>,
    state: tauri::State<'_, crate::commands::AppState>,
) -> Result<Vec<DecodedFrameResponse>, String> {
    decode::decode_frames_batch_impl(frame_indices, state).await
}

#[tauri::command]
pub async fn get_thumbnails(
    start_frame: usize,
    end_frame: usize,
    target_width: u32,
    state: tauri::State<'_, crate::commands::AppState>,
) -> Result<Vec<ThumbnailResponse>, String> {
    thumbnails::get_thumbnails_impl(start_frame, end_frame, target_width, state).await
}

#[tauri::command]
pub async fn get_yuv_data(
    frame_index: usize,
    state: tauri::State<'_, crate::commands::AppState>,
) -> Result<YuvDataResponse, String> {
    yuv::get_yuv_data_impl(frame_index, state).await
}

#[tauri::command]
pub async fn get_yuv_histogram(
    frame_index: usize,
    state: tauri::State<'_, crate::commands::AppState>,
) -> Result<HistogramResponse, String> {
    yuv::get_yuv_histogram_impl(frame_index, state).await
}

#[tauri::command]
pub async fn get_overlay_data(
    frame_index: usize,
    state: tauri::State<'_, crate::commands::AppState>,
) -> Result<OverlayDataResponse, String> {
    overlay::get_overlay_data_impl(frame_index, state).await
}
