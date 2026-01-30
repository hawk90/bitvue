//! File Operations Module
//!
//! Handles layout persistence, recent files tracking, and configuration directory management.

use bitvue_core::BitvueError;
use std::path::PathBuf;

/// Get the configuration directory (~/.bitvue)
pub fn get_config_dir() -> Result<PathBuf, BitvueError> {
    let home = dirs::home_dir()
        .ok_or_else(|| BitvueError::InvalidData("Could not determine home directory".to_string()))?;

    // Canonicalize to resolve symlinks and validate path
    let home = home
        .canonicalize()
        .map_err(|e| BitvueError::InvalidData(format!("Invalid home directory: {}", e)))?;

    let config_dir = home.join(".bitvue");

    // Verify the config directory is under home directory (prevent path traversal)
    let config_dir_canonical = config_dir
        .canonicalize()
        .unwrap_or_else(|_| config_dir.clone());

    if !config_dir_canonical.starts_with(&home) {
        return Err(BitvueError::InvalidData(
            "Config directory must be under home directory (potential path traversal)".to_string(),
        ));
    }

    // Create directory if it doesn't exist
    if !config_dir.exists() {
        std::fs::create_dir_all(&config_dir)?;
    }

    Ok(config_dir)
}

/// Save recent files to ~/.bitvue/recent.json
pub fn save_recent_files(recent_files: &[PathBuf]) -> Result<(), BitvueError> {
    let config_dir = get_config_dir()?;
    let recent_path = config_dir.join("recent.json");

    let json = serde_json::to_string_pretty(recent_files)?;

    std::fs::write(&recent_path, json)?;

    tracing::info!(
        "Saved {} recent files to {:?}",
        recent_files.len(),
        recent_path
    );
    Ok(())
}

/// Load recent files from ~/.bitvue/recent.json
pub fn load_recent_files() -> Result<Vec<PathBuf>, BitvueError> {
    let config_dir = get_config_dir()?;
    let recent_path = config_dir.join("recent.json");

    if !recent_path.exists() {
        return Ok(Vec::new());
    }

    let json = std::fs::read_to_string(&recent_path)?;

    let recent_files: Vec<PathBuf> = serde_json::from_str(&json)?;

    tracing::info!(
        "Loaded {} recent files from {:?}",
        recent_files.len(),
        recent_path
    );
    Ok(recent_files)
}
