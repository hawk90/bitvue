//! Config Worker - Background thread for configuration I/O
//!
//! Handles loading and saving of:
//! - Recent files list
//! - Layout state
//! - Application settings
//!
//! Prevents UI freezing during config operations by processing them in a background thread.

use bitvue_core::BitvueError;
use crate::retry_policy::RetryPolicy;
use crossbeam_channel::{bounded, Receiver, Sender};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::thread;

/// Request to load or save configuration
#[derive(Clone)]
pub enum ConfigRequest {
    /// Load recent files list
    LoadRecentFiles { request_id: u64 },
    /// Save recent files list
    SaveRecentFiles {
        files: Vec<PathBuf>,
        request_id: u64,
    },
    /// Load layout state
    LoadLayout { request_id: u64 },
    /// Save layout state
    SaveLayout {
        layout_json: String,
        request_id: u64,
    },
    /// Load application settings
    LoadSettings { request_id: u64 },
    /// Save application settings
    SaveSettings {
        settings_json: String,
        request_id: u64,
    },
}

/// Result of a config operation
pub struct ConfigResult {
    pub request_id: u64,
    pub result: ConfigResultData,
}

/// Data returned from config operations
pub enum ConfigResultData {
    RecentFilesLoaded(Result<Vec<PathBuf>, BitvueError>),
    RecentFilesSaved(Result<(), BitvueError>),
    LayoutLoaded(Result<String, BitvueError>),
    LayoutSaved(Result<(), BitvueError>),
    SettingsLoaded(Result<String, BitvueError>),
    SettingsSaved(Result<(), BitvueError>),
}

/// Background worker for configuration I/O
///
/// Uses a dedicated thread to perform file I/O without blocking the UI.
/// Config operations are generally fast, but can block on slow filesystems.
pub struct ConfigWorker {
    request_tx: Sender<ConfigRequest>,
    result_rx: Receiver<ConfigResult>,
    request_id: Arc<AtomicU64>,
    _thread: thread::JoinHandle<()>,
}

impl ConfigWorker {
    /// Create a new config worker with background thread
    pub fn new() -> Self {
        let (request_tx, request_rx) = bounded(4);
        let (result_tx, result_rx) = bounded(8);
        let request_id = Arc::new(AtomicU64::new(0));

        let worker_request_id = Arc::clone(&request_id);
        let thread = thread::Builder::new()
            .name("config-worker".to_string())
            .spawn(move || {
                Self::worker_loop(request_rx, result_tx, worker_request_id);
            })
            .expect("Failed to spawn config worker thread");

        Self {
            request_tx,
            result_rx,
            request_id,
            _thread: thread,
        }
    }

    /// Background worker loop - processes config requests
    fn worker_loop(
        request_rx: Receiver<ConfigRequest>,
        result_tx: Sender<ConfigResult>,
        _request_id: Arc<AtomicU64>,
    ) {
        while let Ok(request) = request_rx.recv() {
            let req_id = Self::extract_request_id(&request);

            // Note: No staleness checking for config operations - each operation is independent
            // and should complete regardless of subsequent operations

            // Process request
            let result_data = match request {
                ConfigRequest::LoadRecentFiles { .. } => {
                    ConfigResultData::RecentFilesLoaded(Self::load_recent_files())
                }
                ConfigRequest::SaveRecentFiles { files, .. } => {
                    ConfigResultData::RecentFilesSaved(Self::save_recent_files(&files))
                }
                ConfigRequest::LoadLayout { .. } => {
                    ConfigResultData::LayoutLoaded(Self::load_layout())
                }
                ConfigRequest::SaveLayout { layout_json, .. } => {
                    ConfigResultData::LayoutSaved(Self::save_layout(&layout_json))
                }
                ConfigRequest::LoadSettings { .. } => {
                    ConfigResultData::SettingsLoaded(Self::load_settings())
                }
                ConfigRequest::SaveSettings { settings_json, .. } => {
                    ConfigResultData::SettingsSaved(Self::save_settings(&settings_json))
                }
            };

            // Send result (ignore if channel closed)
            let _ = result_tx.send(ConfigResult {
                request_id: req_id,
                result: result_data,
            });
        }
    }

    /// Extract request ID from ConfigRequest
    fn extract_request_id(request: &ConfigRequest) -> u64 {
        match request {
            ConfigRequest::LoadRecentFiles { request_id }
            | ConfigRequest::SaveRecentFiles { request_id, .. }
            | ConfigRequest::LoadLayout { request_id }
            | ConfigRequest::SaveLayout { request_id, .. }
            | ConfigRequest::LoadSettings { request_id }
            | ConfigRequest::SaveSettings { request_id, .. } => *request_id,
        }
    }

    /// Get config directory (~/.bitvue/)
    fn config_dir() -> Result<PathBuf, BitvueError> {
        dirs::home_dir()
            .ok_or_else(|| BitvueError::InvalidData("Failed to find home directory".to_string()))
            .map(|home| home.join(".bitvue"))
    }

    /// Load recent files list
    fn load_recent_files() -> Result<Vec<PathBuf>, BitvueError> {
        let config_dir = Self::config_dir()?;
        let recent_path = config_dir.join("recent.json");

        if !recent_path.exists() {
            return Ok(Vec::new());
        }

        let content = std::fs::read_to_string(&recent_path)?;
        let recent_files: Vec<PathBuf> = serde_json::from_str(&content)
            .map_err(|e| BitvueError::Serialization(e))?;
        Ok(recent_files)
    }

    /// Save recent files list with retry logic
    fn save_recent_files(files: &[PathBuf]) -> Result<(), BitvueError> {
        let config_dir = Self::config_dir()?;
        std::fs::create_dir_all(&config_dir)?;

        let recent_path = config_dir.join("recent.json");
        let content = serde_json::to_string_pretty(files)?;

        let retry_policy = RetryPolicy::standard();
        let path_clone = recent_path.clone();
        let content_clone = content.clone();

        retry_policy.execute(|| {
            std::fs::write(&path_clone, &content_clone)?;
            Ok(())
        })
    }

    /// Load layout state
    fn load_layout() -> Result<String, BitvueError> {
        let config_dir = Self::config_dir()?;
        let layout_path = config_dir.join("layout.json");

        if !layout_path.exists() {
            return Err(BitvueError::NotFound("No saved layout".to_string()));
        }

        Ok(std::fs::read_to_string(&layout_path)?)
    }

    /// Save layout state with retry logic
    fn save_layout(layout_json: &str) -> Result<(), BitvueError> {
        let config_dir = Self::config_dir()?;
        std::fs::create_dir_all(&config_dir)?;

        let layout_path = config_dir.join("layout.json");
        let retry_policy = RetryPolicy::standard();
        let path_clone = layout_path.clone();
        let json_clone = layout_json.to_string();

        retry_policy.execute(|| {
            std::fs::write(&path_clone, &json_clone)?;
            Ok(())
        })
    }

    /// Load application settings
    fn load_settings() -> Result<String, BitvueError> {
        let config_dir = Self::config_dir()?;
        let settings_path = config_dir.join("settings.json");

        if !settings_path.exists() {
            return Err(BitvueError::NotFound("No saved settings".to_string()));
        }

        Ok(std::fs::read_to_string(&settings_path)?)
    }

    /// Save application settings with retry logic
    fn save_settings(settings_json: &str) -> Result<(), BitvueError> {
        let config_dir = Self::config_dir()?;
        std::fs::create_dir_all(&config_dir)?;

        let settings_path = config_dir.join("settings.json");
        let retry_policy = RetryPolicy::standard();
        let path_clone = settings_path.clone();
        let json_clone = settings_json.to_string();

        retry_policy.execute(|| {
            std::fs::write(&path_clone, &json_clone)?;
            Ok(())
        })
    }

    /// Submit a config request (non-blocking)
    pub fn submit(&self, request: ConfigRequest) -> bool {
        match self.request_tx.try_send(request) {
            Ok(()) => true,
            Err(e) => {
                tracing::warn!("Failed to submit config request: {}", e);
                false
            }
        }
    }

    /// Poll for config results (non-blocking)
    pub fn poll_results(&self) -> Vec<ConfigResult> {
        let mut results = Vec::new();
        while let Ok(result) = self.result_rx.try_recv() {
            results.push(result);
        }
        results
    }

    /// Get next request ID and increment counter
    pub fn next_request_id(&self) -> u64 {
        self.request_id.fetch_add(1, Ordering::SeqCst) + 1
    }

    /// Cancel pending config operations
    pub fn cancel(&self) {
        self.request_id.fetch_add(1, Ordering::SeqCst);
        tracing::debug!("Cancelled pending config operations");
    }

    /// Check if there are pending requests
    pub fn has_pending_work(&self) -> bool {
        !self.request_tx.is_empty()
    }
}

impl Default for ConfigWorker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;

    // NOTE: These tests share ~/.bitvue/ config directory and must run serially.
    // Run with: cargo test --package app --lib config_worker -- --test-threads=1
    // This is a known limitation of the ConfigWorker design for test isolation.

    #[test]
    fn test_new_worker() {
        let worker = ConfigWorker::new();
        assert!(!worker.has_pending_work());
    }

    #[test]
    fn test_next_request_id_increments() {
        let worker = ConfigWorker::new();
        let id1 = worker.next_request_id();
        let id2 = worker.next_request_id();
        assert_eq!(id2, id1 + 1);
    }

    #[test]
    fn test_save_and_load_recent_files() {
        // Clean up any existing config files from previous test runs
        if let Ok(config_dir) = ConfigWorker::config_dir() {
            let _ = std::fs::remove_file(config_dir.join("recent.json"));
            let _ = std::fs::remove_file(config_dir.join("layout.json"));
        }

        // Small delay to ensure files are fully deleted
        thread::sleep(Duration::from_millis(50));

        let worker = ConfigWorker::new();

        let test_files = vec![
            PathBuf::from("/test/file1.ivf"),
            PathBuf::from("/test/file2.ivf"),
        ];

        // Save
        let save_id = worker.next_request_id();
        let save_req = ConfigRequest::SaveRecentFiles {
            files: test_files.clone(),
            request_id: save_id,
        };
        assert!(worker.submit(save_req));

        // Wait for save to complete
        thread::sleep(Duration::from_millis(100));
        let save_results = worker.poll_results();
        assert_eq!(save_results.len(), 1);

        // Load
        let load_id = worker.next_request_id();
        let load_req = ConfigRequest::LoadRecentFiles {
            request_id: load_id,
        };
        assert!(worker.submit(load_req));

        // Wait for load to complete
        thread::sleep(Duration::from_millis(100));
        let load_results = worker.poll_results();
        assert_eq!(load_results.len(), 1);

        match &load_results[0].result {
            ConfigResultData::RecentFilesLoaded(Ok(files)) => {
                assert_eq!(files, &test_files);
            }
            _ => panic!("Expected RecentFilesLoaded"),
        }
    }

    #[test]
    fn test_save_and_load_layout() {
        // Clean up any existing config files from previous test runs
        if let Ok(config_dir) = ConfigWorker::config_dir() {
            let _ = std::fs::remove_file(config_dir.join("recent.json"));
            let _ = std::fs::remove_file(config_dir.join("layout.json"));
        }

        // Small delay to ensure files are fully deleted
        thread::sleep(Duration::from_millis(50));

        let worker = ConfigWorker::new();

        let test_layout = r#"{"version": 2, "layout": "test_unique"}"#;

        // Save
        let save_id = worker.next_request_id();
        let save_req = ConfigRequest::SaveLayout {
            layout_json: test_layout.to_string(),
            request_id: save_id,
        };
        assert!(worker.submit(save_req));

        // Wait for save to complete (retry logic may add delays)
        thread::sleep(Duration::from_millis(300));
        let save_results = worker.poll_results();
        assert_eq!(save_results.len(), 1);

        // Verify save succeeded
        match &save_results[0].result {
            ConfigResultData::LayoutSaved(Ok(())) => {}
            _ => panic!("Expected LayoutSaved success"),
        }

        // Add a small delay to ensure file is fully written
        thread::sleep(Duration::from_millis(100));

        // Load
        let load_id = worker.next_request_id();
        let load_req = ConfigRequest::LoadLayout {
            request_id: load_id,
        };
        assert!(worker.submit(load_req));

        // Wait for load (no retry on successful read, but wait to be safe)
        thread::sleep(Duration::from_millis(300));
        let load_results = worker.poll_results();
        assert_eq!(load_results.len(), 1);

        // Verify load succeeded with the layout we just saved
        match &load_results[0].result {
            ConfigResultData::LayoutLoaded(Ok(layout)) => {
                assert_eq!(layout, test_layout);
            }
            _ => panic!("Expected LayoutLoaded"),
        }
    }

    #[test]
    fn test_cancel_increments_request_id() {
        let worker = ConfigWorker::new();
        let id1 = worker.next_request_id();
        worker.cancel();
        let id2 = worker.next_request_id();
        assert!(id2 > id1 + 1);
    }

    #[test]
    fn test_load_nonexistent_recent_files() {
        let worker = ConfigWorker::new();

        // First delete any existing recent files
        if let Ok(config_dir) = ConfigWorker::config_dir() {
            let _ = std::fs::remove_file(config_dir.join("recent.json"));
        }

        let load_id = worker.next_request_id();
        let load_req = ConfigRequest::LoadRecentFiles {
            request_id: load_id,
        };
        assert!(worker.submit(load_req));

        thread::sleep(Duration::from_millis(100));
        let results = worker.poll_results();
        assert_eq!(results.len(), 1);

        match &results[0].result {
            ConfigResultData::RecentFilesLoaded(Ok(files)) => {
                assert!(files.is_empty());
            }
            _ => panic!("Expected empty RecentFilesLoaded"),
        }
    }

    #[test]
    fn test_cancel_does_not_discard_requests() {
        let worker = ConfigWorker::new();

        // Submit request with old ID
        let old_id = worker.next_request_id();
        let request = ConfigRequest::LoadRecentFiles { request_id: old_id };

        // Call cancel (increments request ID, but doesn't discard config operations)
        worker.cancel();

        assert!(worker.submit(request));

        // Wait and check - should still get result (config operations are independent)
        thread::sleep(Duration::from_millis(200));
        let results = worker.poll_results();

        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_multiple_config_operations() {
        // Clean up any existing config files from previous test runs
        if let Ok(config_dir) = ConfigWorker::config_dir() {
            let _ = std::fs::remove_file(config_dir.join("recent.json"));
            let _ = std::fs::remove_file(config_dir.join("layout.json"));
        }

        // Small delay to ensure files are fully deleted
        thread::sleep(Duration::from_millis(50));

        let worker = ConfigWorker::new();

        let files = vec![PathBuf::from("/test/file.ivf")];
        let layout = r#"{"test": "layout"}"#;

        let id1 = worker.next_request_id();
        let req1 = ConfigRequest::SaveRecentFiles {
            files,
            request_id: id1,
        };

        let id2 = worker.next_request_id();
        let req2 = ConfigRequest::SaveLayout {
            layout_json: layout.to_string(),
            request_id: id2,
        };

        assert!(worker.submit(req1));
        assert!(worker.submit(req2));

        // Poll multiple times to collect all results (worker processes sequentially)
        let mut all_results = Vec::new();
        for _ in 0..20 {
            thread::sleep(Duration::from_millis(100));
            let mut results = worker.poll_results();
            all_results.append(&mut results);
            if all_results.len() >= 2 {
                break;
            }
        }

        assert!(
            all_results.len() >= 2,
            "Expected 2 results, got {}",
            all_results.len()
        );
    }
}
