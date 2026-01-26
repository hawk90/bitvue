//! Recent files commands
//!
//! Commands for managing recent file history.

/// Get list of recent files
#[tauri::command]
pub async fn get_recent_files() -> Result<Vec<String>, String> {
    // TODO: Implement recent files functionality
    Ok(vec![])
}

/// Add a file to recent history
#[tauri::command]
pub async fn add_recent_file(_path: String) -> Result<(), String> {
    // TODO: Implement recent files functionality
    Ok(())
}

/// Clear recent file history
#[tauri::command]
pub async fn clear_recent_files() -> Result<(), String> {
    // TODO: Implement recent files functionality
    Ok(())
}
