//! File Operations Module
//!
//! Handles layout persistence, recent files tracking, and configuration directory management.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Get the configuration directory (~/.bitvue)
pub fn get_config_dir() -> Result<PathBuf, String> {
    let home = dirs::home_dir().ok_or("Could not determine home directory")?;
    let config_dir = home.join(".bitvue");

    // Create directory if it doesn't exist
    if !config_dir.exists() {
        std::fs::create_dir_all(&config_dir)
            .map_err(|e| format!("Failed to create config directory: {}", e))?;
    }

    Ok(config_dir)
}

/// Save recent files to ~/.bitvue/recent.json
pub fn save_recent_files(recent_files: &[PathBuf]) -> Result<(), String> {
    let config_dir = get_config_dir()?;
    let recent_path = config_dir.join("recent.json");

    let json = serde_json::to_string_pretty(recent_files)
        .map_err(|e| format!("Failed to serialize recent files: {}", e))?;

    std::fs::write(&recent_path, json)
        .map_err(|e| format!("Failed to write recent files: {}", e))?;

    tracing::info!("Saved {} recent files to {:?}", recent_files.len(), recent_path);
    Ok(())
}

/// Load recent files from ~/.bitvue/recent.json
pub fn load_recent_files() -> Result<Vec<PathBuf>, String> {
    let config_dir = get_config_dir()?;
    let recent_path = config_dir.join("recent.json");

    if !recent_path.exists() {
        return Ok(Vec::new());
    }

    let json = std::fs::read_to_string(&recent_path)
        .map_err(|e| format!("Failed to read recent files: {}", e))?;

    let recent_files: Vec<PathBuf> = serde_json::from_str(&json)
        .map_err(|e| format!("Failed to deserialize recent files: {}", e))?;

    tracing::info!("Loaded {} recent files from {:?}", recent_files.len(), recent_path);
    Ok(recent_files)
}
