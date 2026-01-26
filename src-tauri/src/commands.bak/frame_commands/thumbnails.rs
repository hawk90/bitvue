//! Thumbnail generation commands

use super::types::ThumbnailResponse;
use crate::commands::AppState;

/// Generate thumbnails for a range of frames
pub async fn get_thumbnails_impl(
    start_frame: usize,
    end_frame: usize,
    target_width: u32,
    _state: tauri::State<'_, AppState>,
) -> Result<Vec<ThumbnailResponse>, String> {
    tracing::info!("get_thumbnails: Request for frames {}-{} (width: {})",
        start_frame, end_frame, target_width);

    // TODO: Implement thumbnail generation
    // For now, return empty vector
    Ok(Vec::new())
}
