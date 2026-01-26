//! Export Worker - Background thread for CSV/JSON export operations
//!
//! Prevents UI freezing during large exports by processing them in a background thread.
//! Follows the same pattern as ParseWorker and ByteCacheWorker.

use crate::retry_policy::RetryPolicy;
use crossbeam_channel::{bounded, Receiver, Sender};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::thread;

/// Request to export data in the background
#[derive(Clone)]
pub enum ExportRequest {
    /// Export units to CSV format
    Csv {
        data: String,
        path: PathBuf,
        request_id: u64,
    },
    /// Export data to JSON format
    Json {
        data: String,
        path: PathBuf,
        request_id: u64,
    },
}

/// Result of an export operation
pub struct ExportResult {
    pub path: PathBuf,
    pub request_id: u64,
    pub result: Result<(), String>,
}

/// Background worker for export operations
///
/// Uses a dedicated thread to perform file I/O without blocking the UI.
/// Implements latest-wins strategy - submitting a new request increments the request ID,
/// making older pending requests stale.
pub struct ExportWorker {
    request_tx: Sender<ExportRequest>,
    result_rx: Receiver<ExportResult>,
    request_id: Arc<AtomicU64>,
    _thread: thread::JoinHandle<()>,
}

impl ExportWorker {
    /// Create a new export worker with background thread
    pub fn new() -> Self {
        let (request_tx, request_rx) = bounded(2);
        let (result_tx, result_rx) = bounded(4);
        let request_id = Arc::new(AtomicU64::new(0));

        let worker_request_id = Arc::clone(&request_id);
        let thread = thread::Builder::new()
            .name("export-worker".to_string())
            .spawn(move || {
                Self::worker_loop(request_rx, result_tx, worker_request_id);
            })
            .expect("Failed to spawn export worker thread");

        Self {
            request_tx,
            result_rx,
            request_id,
            _thread: thread,
        }
    }

    /// Background worker loop - processes export requests
    fn worker_loop(
        request_rx: Receiver<ExportRequest>,
        result_tx: Sender<ExportResult>,
        _request_id: Arc<AtomicU64>,
    ) {
        while let Ok(request) = request_rx.recv() {
            let (path, req_id) = match &request {
                ExportRequest::Csv { path, request_id, .. } => (path.clone(), *request_id),
                ExportRequest::Json { path, request_id, .. } => (path.clone(), *request_id),
            };

            // Note: No staleness checking for exports - each export is independent
            // and should complete regardless of subsequent exports

            // Perform export (blocking I/O, but in background thread)
            let result = match request {
                ExportRequest::Csv { data, path, .. } => {
                    Self::export_csv(&data, &path)
                }
                ExportRequest::Json { data, path, .. } => {
                    Self::export_json(&data, &path)
                }
            };

            // Send result (ignore if channel closed)
            let _ = result_tx.send(ExportResult {
                path,
                request_id: req_id,
                result,
            });
        }
    }

    /// Export data to CSV file with retry logic
    fn export_csv(data: &str, path: &PathBuf) -> Result<(), String> {
        let retry_policy = RetryPolicy::standard();
        let data_clone = data.to_string();
        let path_clone = path.clone();

        retry_policy.execute(|| {
            std::fs::write(&path_clone, &data_clone)
                .map_err(|e| format!("Failed to write CSV: {}", e))
        })
    }

    /// Export data to JSON file with retry logic
    fn export_json(data: &str, path: &PathBuf) -> Result<(), String> {
        let retry_policy = RetryPolicy::standard();
        let data_clone = data.to_string();
        let path_clone = path.clone();

        retry_policy.execute(|| {
            std::fs::write(&path_clone, &data_clone)
                .map_err(|e| format!("Failed to write JSON: {}", e))
        })
    }

    /// Submit an export request (non-blocking)
    ///
    /// Returns true if request was submitted, false if queue is full.
    pub fn submit(&self, request: ExportRequest) -> bool {
        match self.request_tx.try_send(request) {
            Ok(()) => true,
            Err(e) => {
                tracing::warn!("Failed to submit export request: {}", e);
                false
            }
        }
    }

    /// Poll for completed export results (non-blocking)
    ///
    /// Returns all available results. Should be called every frame.
    pub fn poll_results(&self) -> Vec<ExportResult> {
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

    /// Cancel pending exports by incrementing request ID
    ///
    /// Any exports currently in flight will be discarded when they complete.
    pub fn cancel(&self) {
        self.request_id.fetch_add(1, Ordering::SeqCst);
        tracing::debug!("Cancelled pending exports");
    }

    /// Check if there are pending requests
    pub fn has_pending_work(&self) -> bool {
        !self.request_tx.is_empty()
    }
}

impl Default for ExportWorker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn test_new_worker() {
        let worker = ExportWorker::new();
        assert!(!worker.has_pending_work());
    }

    #[test]
    fn test_next_request_id_increments() {
        let worker = ExportWorker::new();
        let id1 = worker.next_request_id();
        let id2 = worker.next_request_id();
        assert_eq!(id2, id1 + 1);
    }

    #[test]
    fn test_submit_and_poll_csv() {
        let worker = ExportWorker::new();
        let temp_dir = std::env::temp_dir();
        let path = temp_dir.join("test_export.csv");

        let request_id = worker.next_request_id();
        let request = ExportRequest::Csv {
            data: "frame,type,size\n0,I,1024\n1,P,512\n".to_string(),
            path: path.clone(),
            request_id,
        };

        assert!(worker.submit(request));

        // Wait for result
        thread::sleep(Duration::from_millis(100));
        let results = worker.poll_results();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].path, path);
        assert!(results[0].result.is_ok());

        // Cleanup
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_submit_and_poll_json() {
        let worker = ExportWorker::new();
        let temp_dir = std::env::temp_dir();
        let path = temp_dir.join("test_export.json");

        let request_id = worker.next_request_id();
        let request = ExportRequest::Json {
            data: r#"{"frames": [{"index": 0, "type": "I"}]}"#.to_string(),
            path: path.clone(),
            request_id,
        };

        assert!(worker.submit(request));

        // Wait for result
        thread::sleep(Duration::from_millis(100));
        let results = worker.poll_results();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].path, path);
        assert!(results[0].result.is_ok());

        // Cleanup
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_cancel_increments_request_id() {
        let worker = ExportWorker::new();
        let id1 = worker.next_request_id();
        worker.cancel();
        let id2 = worker.next_request_id();
        assert!(id2 > id1 + 1);
    }

    #[test]
    fn test_cancel_does_not_discard_requests() {
        let worker = ExportWorker::new();
        let temp_dir = std::env::temp_dir();
        let path = temp_dir.join("test_cancel.csv");

        // Submit request with old ID
        let old_id = worker.next_request_id();
        let request = ExportRequest::Csv {
            data: "test".to_string(),
            path: path.clone(),
            request_id: old_id,
        };

        // Call cancel (increments request ID, but doesn't discard exports)
        worker.cancel();

        assert!(worker.submit(request));

        // Wait and check - should still get result (exports are independent)
        thread::sleep(Duration::from_millis(200));
        let results = worker.poll_results();

        assert_eq!(results.len(), 1);
        assert!(results[0].result.is_ok());

        // Cleanup
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_export_invalid_path() {
        let worker = ExportWorker::new();
        let path = PathBuf::from("/nonexistent/directory/file.csv");

        let request_id = worker.next_request_id();
        let request = ExportRequest::Csv {
            data: "test".to_string(),
            path: path.clone(),
            request_id,
        };

        assert!(worker.submit(request));

        // Wait for result (retry logic adds delays: 100ms + 200ms + 400ms = 700ms)
        thread::sleep(Duration::from_millis(1000));
        let results = worker.poll_results();

        assert_eq!(results.len(), 1);
        assert!(results[0].result.is_err());
    }

    #[test]
    fn test_multiple_exports() {
        let worker = ExportWorker::new();
        let temp_dir = std::env::temp_dir();

        let path1 = temp_dir.join("test_multi1.csv");
        let path2 = temp_dir.join("test_multi2.json");

        let id1 = worker.next_request_id();
        let req1 = ExportRequest::Csv {
            data: "test1".to_string(),
            path: path1.clone(),
            request_id: id1,
        };

        let id2 = worker.next_request_id();
        let req2 = ExportRequest::Json {
            data: "test2".to_string(),
            path: path2.clone(),
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

        // Verify we got both results
        assert!(all_results.len() >= 2, "Expected 2 results, got {}", all_results.len());
        for result in &all_results {
            assert!(result.result.is_ok());
        }

        // Cleanup
        let _ = std::fs::remove_file(&path1);
        let _ = std::fs::remove_file(&path2);
    }
}
