//! Recent files commands
//!
//! Commands for managing recent file history using Tauri's persistent store.

use serde::{Deserialize, Serialize};
use tauri_plugin_store::StoreExt;
use std::path::Path;

/// Maximum number of recent files to track
const MAX_RECENT_FILES: usize = 10;

/// Maximum path length to prevent DoS through extremely long paths
const MAX_PATH_LENGTH: usize = 4096;

/// Characters to sanitize from paths
const SANITIZE_CHARS: &[char] = &['\r', '\n', '\t', '\x00'];

/// Sanitize a file path by removing dangerous characters
fn sanitize_path(path: &str) -> String {
    // Limit length first
    let truncated = if path.len() > MAX_PATH_LENGTH {
        &path[..MAX_PATH_LENGTH]
    } else {
        path
    };

    // Remove dangerous characters
    truncated
        .chars()
        .map(|ch| if SANITIZE_CHARS.contains(&ch) { ' ' } else { ch })
        .collect()
}

/// Store key for recent files
const RECENT_FILES_KEY: &str = "recent_files";

/// Recent file entry with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
struct RecentFileEntry {
    path: String,
    timestamp: i64,
}

impl RecentFileEntry {
    fn new(path: String) -> Self {
        Self {
            path,
            timestamp: chrono::Utc::now().timestamp(),
        }
    }
}

/// Get list of recent files
///
/// Returns a list of recently opened file paths in most-recently-used order.
/// Files that no longer exist are filtered out.
#[tauri::command]
pub async fn get_recent_files(
    app: tauri::AppHandle,
) -> Result<Vec<String>, String> {
    let store = app.store("recent_files.json").map_err(|e| e.to_string())?;

    // Load recent files from store
    let entries: Vec<RecentFileEntry> = if let Some(value) = store.get(RECENT_FILES_KEY) {
        // Convert JsonValue to serde_json::Value then deserialize
        let json_str = serde_json::to_string(&value).map_err(|e| e.to_string())?;
        serde_json::from_str(&json_str).map_err(|e| e.to_string())?
    } else {
        vec![]
    };

    // Filter to only files that still exist
    let mut valid_files = Vec::new();
    for entry in entries {
        if Path::new(&entry.path).exists() {
            // SECURITY: Sanitize paths before returning them
            valid_files.push(sanitize_path(&entry.path));
        }
    }

    log::info!("get_recent_files: returning {} valid recent files", valid_files.len());
    Ok(valid_files)
}

/// Add a file to recent history
///
/// Tracks a file as recently opened. If the file is already in the list,
/// it's moved to the front. The list is limited to MAX_RECENT_FILES entries.
#[tauri::command]
pub async fn add_recent_file(
    app: tauri::AppHandle,
    path: String,
) -> Result<(), String> {
    // SECURITY: Sanitize path before storing to prevent injection attacks
    let sanitized_path = sanitize_path(&path);

    let store = app.store("recent_files.json").map_err(|e| e.to_string())?;

    // Load existing entries
    let mut entries: Vec<RecentFileEntry> = if let Some(value) = store.get(RECENT_FILES_KEY) {
        let json_str = serde_json::to_string(&value).map_err(|e| e.to_string())?;
        serde_json::from_str(&json_str).map_err(|e| e.to_string())?
    } else {
        vec![]
    };

    // Remove if already exists (to move to front)
    entries.retain(|e| e.path != sanitized_path);

    // Add new entry at front
    entries.insert(0, RecentFileEntry::new(sanitized_path));

    // Limit to max entries
    entries.truncate(MAX_RECENT_FILES);

    // Save to store
    let json_value = serde_json::to_value(&entries).map_err(|e| e.to_string())?;
    store.set(RECENT_FILES_KEY, json_value);
    store.save().map_err(|e| e.to_string())?;

    log::info!("add_recent_file: added file, total recent files: {}", entries.len());
    Ok(())
}

/// Clear recent file history
///
/// Removes all entries from the recent files list.
#[tauri::command]
pub async fn clear_recent_files(
    app: tauri::AppHandle,
) -> Result<(), String> {
    let store = app.store("recent_files.json").map_err(|e| e.to_string())?;

    // Clear the store
    store.set(RECENT_FILES_KEY, serde_json::json!([]));
    store.save().map_err(|e| e.to_string())?;

    log::info!("clear_recent_files: cleared all recent files");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_max_recent_files_constant() {
        assert_eq!(MAX_RECENT_FILES, 10);
    }

    #[test]
    fn test_recent_file_entry_creation() {
        let entry = RecentFileEntry::new("/path/to/file.ivf".to_string());
        assert_eq!(entry.path, "/path/to/file.ivf");
        assert!(entry.timestamp > 0);
    }

    #[test]
    fn test_recent_file_entry_serialization() {
        let entry = RecentFileEntry::new("/path/to/file.ivf".to_string());
        let json = serde_json::to_string(&entry).unwrap();
        assert!(json.contains("/path/to/file.ivf"));
    }
}
