//! Async ByteCache worker - Background file loading
//!
//! Prevents UI freezing during file open by loading files in a background thread.
//!
//! # Architecture
//!
//! - Background thread processes file load requests
//! - Memory-maps files (or reads into memory for small files)
//! - Latest-wins strategy: newer requests invalidate old ones
//! - Request ID tracking for stale result discarding
//! - Non-blocking submit/poll operations
//!
//! # Usage
//!
//! ```rust
//! let worker = ByteCacheWorker::new();
//!
//! // Submit load request (non-blocking)
//! let request_id = worker.next_request_id(StreamId::A);
//! let request = ByteCacheRequest {
//!     stream_id: StreamId::A,
//!     path: PathBuf::from("video.ivf"),
//!     request_id,
//! };
//! worker.submit(request);
//!
//! // Poll for results (non-blocking)
//! for result in worker.poll_results() {
//!     match result.result {
//!         Ok(byte_cache) => { /* use byte cache */ }
//!         Err(e) => { /* show error */ }
//!     }
//! }
//! ```

use crate::retry_policy::RetryPolicy;
use bitvue_core::{ByteCache, StreamId};
use crossbeam_channel::{bounded, Receiver, Sender};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::thread;

/// ByteCache load request sent to worker
#[derive(Debug, Clone)]
pub struct ByteCacheRequest {
    /// Which stream (A or B) this load is for
    pub stream_id: StreamId,
    /// File path to load
    pub path: PathBuf,
    /// Request ID for staleness detection
    pub request_id: u64,
}

/// ByteCache load result from worker
pub struct ByteCacheResult {
    /// Which stream this result is for
    pub stream_id: StreamId,
    /// Original file path
    pub path: PathBuf,
    /// Request ID (for staleness checking)
    pub request_id: u64,
    /// Load result (ByteCache or error message)
    pub result: Result<Arc<ByteCache>, String>,
}

/// Async ByteCache worker with background thread
pub struct ByteCacheWorker {
    /// Send requests to worker
    request_tx: Sender<ByteCacheRequest>,
    /// Receive results from worker
    result_rx: Receiver<ByteCacheResult>,
    /// Current request ID per stream (for stale detection)
    request_ids: Arc<[AtomicU64; 2]>,
    /// Worker thread handle
    _thread: thread::JoinHandle<()>,
}

impl ByteCacheWorker {
    /// Create new ByteCache worker with background thread
    pub fn new() -> Self {
        let (request_tx, request_rx) = bounded::<ByteCacheRequest>(2);
        let (result_tx, result_rx) = bounded::<ByteCacheResult>(2);
        let request_ids = Arc::new([AtomicU64::new(0), AtomicU64::new(0)]);

        let worker_request_ids = Arc::clone(&request_ids);

        let thread = thread::Builder::new()
            .name("bytecache-worker".to_string())
            .spawn(move || {
                Self::worker_loop(request_rx, result_tx, worker_request_ids);
            })
            .expect("Failed to spawn ByteCache worker thread");

        Self {
            request_tx,
            result_rx,
            request_ids,
            _thread: thread,
        }
    }

    /// Worker loop - processes file load requests
    fn worker_loop(
        request_rx: Receiver<ByteCacheRequest>,
        result_tx: Sender<ByteCacheResult>,
        request_ids: Arc<[AtomicU64; 2]>,
    ) {
        tracing::info!("ByteCache worker started");

        while let Ok(request) = request_rx.recv() {
            let stream_idx = match request.stream_id {
                StreamId::A => 0,
                StreamId::B => 1,
            };

            // Check if request is stale before processing
            let current_id = request_ids[stream_idx].load(Ordering::SeqCst);
            if request.request_id != current_id {
                tracing::debug!(
                    "ByteCache worker: Skipping stale request (id: {}, current: {})",
                    request.request_id,
                    current_id
                );
                continue;
            }

            tracing::info!(
                "ByteCache worker: Loading {:?} for stream {:?}",
                request.path,
                request.stream_id
            );

            // Load file with retry logic (BLOCKING, but in background!)
            let retry_policy = RetryPolicy::standard();
            let path_clone = request.path.clone();
            let result = retry_policy
                .execute(|| {
                    ByteCache::new(
                        &path_clone,
                        ByteCache::DEFAULT_SEGMENT_SIZE,
                        ByteCache::DEFAULT_MAX_MEMORY,
                    )
                    .map_err(|e| format!("Failed to load file: {}", e))
                })
                .map(Arc::new);

            // Check staleness again before sending result
            let current_id = request_ids[stream_idx].load(Ordering::SeqCst);
            if request.request_id != current_id {
                tracing::debug!(
                    "ByteCache worker: Result became stale during processing (id: {}, current: {})",
                    request.request_id,
                    current_id
                );
                continue;
            }

            // Send result
            let cache_result = ByteCacheResult {
                stream_id: request.stream_id,
                path: request.path,
                request_id: request.request_id,
                result,
            };

            if result_tx.send(cache_result).is_err() {
                tracing::warn!("ByteCache worker: Failed to send result (channel disconnected)");
                break;
            }
        }

        tracing::info!("ByteCache worker shutting down");
    }

    /// Submit a load request (non-blocking)
    ///
    /// Returns `true` if submitted successfully, `false` if queue is full.
    pub fn submit(&self, request: ByteCacheRequest) -> bool {
        self.request_tx.try_send(request).is_ok()
    }

    /// Poll for load results (non-blocking)
    ///
    /// Returns all available results from the queue.
    pub fn poll_results(&self) -> Vec<ByteCacheResult> {
        let mut results = Vec::new();
        while let Ok(result) = self.result_rx.try_recv() {
            results.push(result);
        }
        results
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
            "ByteCache worker: Cancelled all requests for stream {:?}",
            stream_id
        );
    }

    /// Check if there's pending work (results in queue)
    ///
    /// Note: This is an approximation - it checks if the result channel is non-empty.
    pub fn has_pending_work(&self) -> bool {
        !self.result_rx.is_empty()
    }
}

impl Default for ByteCacheWorker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new() {
        let _worker = ByteCacheWorker::new();
    }

    #[test]
    fn test_next_request_id_increments() {
        let worker = ByteCacheWorker::new();
        let id1 = worker.next_request_id(StreamId::A);
        let id2 = worker.next_request_id(StreamId::A);
        assert_eq!(id2, id1 + 1);
    }

    #[test]
    fn test_next_request_id_per_stream() {
        let worker = ByteCacheWorker::new();
        let id_a = worker.next_request_id(StreamId::A);
        let id_b = worker.next_request_id(StreamId::B);
        // Each stream has independent counters
        assert_eq!(id_a, 1);
        assert_eq!(id_b, 1);
    }

    #[test]
    fn test_poll_results_empty_initially() {
        let worker = ByteCacheWorker::new();
        let results = worker.poll_results();
        assert!(results.is_empty());
    }

    #[test]
    fn test_cancel_stream_increments_request_id() {
        let worker = ByteCacheWorker::new();
        let id1 = worker.next_request_id(StreamId::A);
        worker.cancel_stream(StreamId::A);
        let id2 = worker.next_request_id(StreamId::A);
        // After cancel, ID should jump by 1000+
        assert!(id2 > id1 + 1000);
    }

    #[test]
    fn test_has_pending_work_false_initially() {
        let worker = ByteCacheWorker::new();
        // Initially no pending work
        assert!(!worker.has_pending_work());
    }

    #[test]
    fn test_default() {
        let _worker: ByteCacheWorker = Default::default();
    }
}
