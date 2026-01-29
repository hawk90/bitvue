//! Async parse worker - Background file parsing
//!
//! Prevents UI freezing during file open by parsing bitstream files in a background thread.
//!
//! # Architecture
//!
//! - Background thread processes parse requests
//! - Latest-wins strategy: newer requests invalidate old ones
//! - Progress reporting during long parse operations
//! - Request ID tracking for stale result discarding
//! - Non-blocking submit/poll operations
//!
//! # Usage
//!
//! ```rust
//! let worker = ParseWorker::new();
//!
//! // Submit parse request (non-blocking)
//! let request_id = worker.next_request_id(StreamId::A);
//! let request = ParseRequest {
//!     stream_id: StreamId::A,
//!     path: PathBuf::from("video.ivf"),
//!     byte_cache: Arc::new(byte_cache),
//!     request_id,
//! };
//! worker.submit(request);
//!
//! // Poll for results and progress (non-blocking)
//! for result in worker.poll_results() {
//!     match result.result {
//!         Ok((container, units)) => { /* update UI */ }
//!         Err(e) => { /* show error */ }
//!     }
//! }
//!
//! for progress in worker.poll_progress() {
//!     // Update progress bar
//! }
//! ```

use bitvue_core::{ByteCache, ContainerModel, Result, StreamId, UnitModel};
use crossbeam_channel::{bounded, Receiver, Sender};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::thread;

/// Parse request sent to worker
#[derive(Clone)]
pub struct ParseRequest {
    /// Which stream (A or B) this parse is for
    pub stream_id: StreamId,
    /// File path to parse
    pub path: PathBuf,
    /// Byte cache for file data
    pub byte_cache: Arc<ByteCache>,
    /// Request ID for staleness detection
    pub request_id: u64,
}

/// Parse result from worker
#[derive(Debug)]
pub struct ParseResult {
    /// Which stream this result is for
    pub stream_id: StreamId,
    /// Original file path
    pub path: PathBuf,
    /// Request ID (for staleness checking)
    pub request_id: u64,
    /// Parsing result (container + units + diagnostics, or error)
    pub result: Result<(
        ContainerModel,
        UnitModel,
        Vec<bitvue_core::event::Diagnostic>,
    )>,
}

/// Parse progress update
#[derive(Debug, Clone)]
pub struct ParseProgress {
    /// Which stream this progress is for
    pub stream_id: StreamId,
    /// Request ID
    pub request_id: u64,
    /// Progress message (e.g., "Parsing IVF header...", "Parsing OBU 15/100...")
    pub message: String,
    /// Progress value 0.0 to 1.0 (if known, None otherwise)
    pub progress: Option<f32>,
}

/// Async parse worker with background thread
pub struct ParseWorker {
    /// Send requests to worker
    request_tx: Sender<ParseRequest>,
    /// Receive results from worker
    result_rx: Receiver<ParseResult>,
    /// Receive progress updates from worker
    progress_rx: Receiver<ParseProgress>,
    /// Current request ID per stream (for stale detection)
    request_ids: Arc<[AtomicU64; 2]>,
    /// Worker thread handle
    _thread: thread::JoinHandle<()>,
}

impl ParseWorker {
    /// Create new parse worker with background thread
    ///
    /// Returns error if thread spawning fails (e.g., system out of resources)
    pub fn new() -> std::result::Result<Self, std::io::Error> {
        let (request_tx, request_rx) = bounded::<ParseRequest>(2);
        let (result_tx, result_rx) = bounded::<ParseResult>(2);
        let (progress_tx, progress_rx) = bounded::<ParseProgress>(10);
        let request_ids = Arc::new([AtomicU64::new(0), AtomicU64::new(0)]);

        let worker_request_ids = Arc::clone(&request_ids);

        let thread = thread::Builder::new()
            .name("parse-worker".to_string())
            .spawn(move || {
                Self::worker_loop(request_rx, result_tx, progress_tx, worker_request_ids);
            })
            .map_err(|e| {
                tracing::error!("Failed to spawn parse worker thread: {}", e);
                e
            })?;

        Ok(Self {
            request_tx,
            result_rx,
            progress_rx,
            request_ids,
            _thread: thread,
        })
    }

    /// Worker loop - processes parse requests
    fn worker_loop(
        request_rx: Receiver<ParseRequest>,
        result_tx: Sender<ParseResult>,
        progress_tx: Sender<ParseProgress>,
        request_ids: Arc<[AtomicU64; 2]>,
    ) {
        tracing::info!("Parse worker started");

        while let Ok(request) = request_rx.recv() {
            let stream_idx = match request.stream_id {
                StreamId::A => 0,
                StreamId::B => 1,
            };

            // Check if request is stale before processing
            let current_id = request_ids[stream_idx].load(Ordering::SeqCst);
            if request.request_id != current_id {
                tracing::debug!(
                    "Parse worker: Skipping stale request (id: {}, current: {})",
                    request.request_id,
                    current_id
                );
                continue;
            }

            tracing::info!(
                "Parse worker: Processing {:?} for stream {:?}",
                request.path,
                request.stream_id
            );

            // Send initial progress
            let _ = progress_tx.try_send(ParseProgress {
                stream_id: request.stream_id,
                request_id: request.request_id,
                message: format!("Parsing {}...", request.path.display()),
                progress: None,
            });

            // Call existing parse_file function (BLOCKING, but in background!)
            let result = crate::parser_worker::parse_file(
                &request.path,
                request.stream_id,
                request.byte_cache.clone(),
            );

            // Check staleness again before sending result
            let current_id = request_ids[stream_idx].load(Ordering::SeqCst);
            if request.request_id != current_id {
                tracing::debug!(
                    "Parse worker: Result became stale during processing (id: {}, current: {})",
                    request.request_id,
                    current_id
                );
                continue;
            }

            // Send completion progress
            let success = result.is_ok();
            let _ = progress_tx.try_send(ParseProgress {
                stream_id: request.stream_id,
                request_id: request.request_id,
                message: if success {
                    "Parse complete".to_string()
                } else {
                    "Parse failed".to_string()
                },
                progress: Some(1.0),
            });

            // Send result
            let parse_result = ParseResult {
                stream_id: request.stream_id,
                path: request.path,
                request_id: request.request_id,
                result,
            };

            if result_tx.send(parse_result).is_err() {
                tracing::warn!("Parse worker: Failed to send result (channel disconnected)");
                break;
            }
        }

        tracing::info!("Parse worker shutting down");
    }

    /// Submit a parse request (non-blocking)
    ///
    /// Returns `true` if submitted successfully, `false` if queue is full.
    pub fn submit(&self, request: ParseRequest) -> bool {
        self.request_tx.try_send(request).is_ok()
    }

    /// Poll for parse results (non-blocking)
    ///
    /// Returns all available results from the queue.
    pub fn poll_results(&self) -> Vec<ParseResult> {
        let mut results = Vec::new();
        while let Ok(result) = self.result_rx.try_recv() {
            results.push(result);
        }
        results
    }

    /// Poll for progress updates (non-blocking)
    ///
    /// Returns all available progress updates from the queue.
    pub fn poll_progress(&self) -> Vec<ParseProgress> {
        let mut progress_updates = Vec::new();
        while let Ok(progress) = self.progress_rx.try_recv() {
            progress_updates.push(progress);
        }
        progress_updates
    }

    /// Get next request ID for a stream
    ///
    /// Increments the request ID counter and returns the new value.
    pub fn next_request_id(&self, stream_id: StreamId) -> u64 {
        let idx = match stream_id {
            StreamId::A => 0,
            StreamId::B => 1,
        };
        self.request_ids[idx].fetch_add(1, Ordering::SeqCst) + 1
    }

    /// Cancel all pending requests for a stream
    ///
    /// This works by incrementing the request ID, making all previous requests stale.
    pub fn cancel_stream(&self, stream_id: StreamId) {
        let idx = match stream_id {
            StreamId::A => 0,
            StreamId::B => 1,
        };
        // Increment by a large number to invalidate all pending requests
        self.request_ids[idx].fetch_add(1000, Ordering::SeqCst);
        tracing::info!(
            "Parse worker: Cancelled all requests for stream {:?}",
            stream_id
        );
    }

    /// Check if there's pending work (requests in queue)
    ///
    /// Note: This is an approximation - it checks if the result channel is non-empty.
    pub fn has_pending_work(&self) -> bool {
        !self.result_rx.is_empty() || !self.progress_rx.is_empty()
    }
}

impl Default for ParseWorker {
    fn default() -> Self {
        Self::new().expect("Failed to create default ParseWorker")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new() {
        let _worker = ParseWorker::new().expect("Failed to create worker");
    }

    #[test]
    fn test_next_request_id_increments() {
        let worker = ParseWorker::new().expect("Failed to create worker");
        let id1 = worker.next_request_id(StreamId::A);
        let id2 = worker.next_request_id(StreamId::A);
        assert_eq!(id2, id1 + 1);
    }

    #[test]
    fn test_next_request_id_per_stream() {
        let worker = ParseWorker::new().expect("Failed to create worker");
        let id_a = worker.next_request_id(StreamId::A);
        let id_b = worker.next_request_id(StreamId::B);
        // Each stream has independent counters
        assert_eq!(id_a, 1);
        assert_eq!(id_b, 1);
    }

    #[test]
    fn test_poll_results_empty_initially() {
        let worker = ParseWorker::new().expect("Failed to create worker");
        let results = worker.poll_results();
        assert!(results.is_empty());
    }

    #[test]
    fn test_poll_progress_empty_initially() {
        let worker = ParseWorker::new().expect("Failed to create worker");
        let progress = worker.poll_progress();
        assert!(progress.is_empty());
    }

    #[test]
    fn test_cancel_stream_increments_request_id() {
        let worker = ParseWorker::new().expect("Failed to create worker");
        let id1 = worker.next_request_id(StreamId::A);
        worker.cancel_stream(StreamId::A);
        let id2 = worker.next_request_id(StreamId::A);
        // After cancel, ID should jump by 1000+
        assert!(id2 > id1 + 1000);
    }

    #[test]
    fn test_has_pending_work_false_initially() {
        let worker = ParseWorker::new().expect("Failed to create worker");
        // Initially no pending work
        assert!(!worker.has_pending_work());
    }

    #[test]
    fn test_default() {
        let _worker: ParseWorker = Default::default();
    }
}
