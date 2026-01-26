//! Stream info command

use crate::commands::AppState;

/// Get stream statistics
pub async fn get_stream_info_impl(
    state: tauri::State<'_, AppState>,
) -> Result<String, String> {
    let _core = state.core.lock().map_err(|e| e.to_string())?;
    // TODO: Implement stream info retrieval from StreamState
    Ok("Stream info placeholder".to_string())
}
