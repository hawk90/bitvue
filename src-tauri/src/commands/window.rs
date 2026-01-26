//! Window commands
//!
//! Commands for window management.

/// Close the current window
#[tauri::command]
pub async fn close_window(window: tauri::Window) -> Result<(), String> {
    window.close().map_err(|e| e.to_string())
}
