//! Shared server state and the path sandbox (only paths under the start directory are readable).

use anyhow::Result;
use bitvue_engine::Core;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

/// Shared application state
pub(crate) struct AppState {
    pub(crate) core: Arc<Mutex<Core>>,
    pub(crate) loaded_file: Arc<Mutex<Option<PathBuf>>>,
    /// Allowed base directories for file access (for security)
    pub(crate) allowed_paths: Vec<PathBuf>,
}

/// Validate that a path is within allowed directories
///
/// This prevents path traversal attacks by ensuring the canonical
/// path starts with one of the allowed base directories.
///
/// # Arguments
/// * `path` - User-provided path to validate
/// * `allowed_paths` - List of allowed base directories
///
/// # Returns
/// * `Ok(PathBuf)` - Canonicalized path if valid
/// * `Err(anyhow::Error)` - Error if path is outside allowed directories
pub(crate) fn validate_path(path: &str, allowed_paths: &[PathBuf]) -> Result<PathBuf> {
    let path_buf = PathBuf::from(path);

    // Resolve to absolute path, following symlinks
    let canonical = path_buf
        .canonicalize()
        .map_err(|e| anyhow::anyhow!("Invalid path: {}", e))?;

    // Check if path is within any allowed directory
    let is_allowed = allowed_paths
        .iter()
        .any(|allowed| canonical.starts_with(allowed));

    if !is_allowed {
        return Err(anyhow::anyhow!(
            "Path outside allowed directories: {}",
            path
        ));
    }

    Ok(canonical)
}

impl AppState {
    pub(crate) fn new() -> Self {
        // Initialize with current directory as the only allowed path
        // This prevents access to sensitive files outside the project
        // Canonicalize so it is comparable with `validate_path`'s canonical result: on Windows
        // `canonicalize` yields a `\\?\C:\...` verbatim path that never `starts_with` the plain
        // `current_dir()`, which rejected every file.
        let current_dir = std::env::current_dir()
            .and_then(|d| d.canonicalize())
            .unwrap_or_else(|_| PathBuf::from("."));

        Self {
            core: Arc::new(Mutex::new(Core::new())),
            loaded_file: Arc::new(Mutex::new(None)),
            allowed_paths: vec![current_dir],
        }
    }
}
