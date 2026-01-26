//! Parse Coordinator - Wraps ParseWorker with pending state tracking
//!
//! Coordinates async file parsing with pending state tracking.
//!
//! # Usage
//!
//! ```rust
//! let mut coordinator = ParseCoordinator::new();
//!
//! // Submit parse request
//! let request_id = coordinator.worker.next_request_id(StreamId::A);
//! let request = ParseRequest {
//!     stream_id: StreamId::A,
//!     path: PathBuf::from("video.ivf"),
//!     byte_cache: Arc::new(byte_cache),
//!     request_id,
//! };
//! coordinator.submit(request);
//!
//! // Poll for results
//! for result in coordinator.poll_results() {
//!     // Handle result
//! }
//! ```

use crate::parse_worker::{ParseProgress, ParseRequest, ParseResult, ParseWorker};
use bitvue_core::StreamId;
use std::path::PathBuf;

/// Coordinates async file parsing with pending state tracking
pub struct ParseCoordinator {
    /// The underlying parse worker
    worker: ParseWorker,
    /// Currently pending parse (stream, path)
    pending: Option<(StreamId, PathBuf)>,
}

impl ParseCoordinator {
    /// Create a new parse coordinator
    pub fn new() -> Self {
        Self {
            worker: ParseWorker::new(),
            pending: None,
        }
    }

    /// Submit a parse request (non-blocking)
    ///
    /// Returns the request ID if submitted successfully, None if queue is full.
    pub fn submit(&mut self, request: ParseRequest) -> Option<u64> {
        let request_id = request.request_id;
        let path = request.path.clone();
        let stream_id = request.stream_id;

        if self.worker.submit(request) {
            self.pending = Some((stream_id, path));
            Some(request_id)
        } else {
            None
        }
    }

    /// Poll for parse results (non-blocking)
    pub fn poll_results(&self) -> Vec<ParseResult> {
        self.worker.poll_results()
    }

    /// Poll for progress updates (non-blocking)
    pub fn poll_progress(&self) -> Vec<ParseProgress> {
        self.worker.poll_progress()
    }

    /// Clear pending state for a completed parse
    pub fn clear_pending_if_matches(&mut self, stream_id: StreamId, path: &PathBuf) {
        if let Some((pending_stream, pending_path)) = &self.pending {
            if *pending_stream == stream_id && pending_path == path {
                self.pending = None;
            }
        }
    }

    /// Get currently pending parse
    pub fn pending(&self) -> Option<(StreamId, PathBuf)> {
        self.pending.clone()
    }

    /// Check if there's a pending parse
    pub fn is_pending(&self) -> bool {
        self.pending.is_some()
    }

    /// Check if there's pending work (in queue or in pending state)
    pub fn has_pending_work(&self) -> bool {
        self.worker.has_pending_work() || self.pending.is_some()
    }

    /// Cancel all pending requests for a stream
    pub fn cancel_stream(&mut self, stream_id: StreamId) {
        self.worker.cancel_stream(stream_id);
        if let Some((pending_stream, _)) = &self.pending {
            if *pending_stream == stream_id {
                self.pending = None;
            }
        }
    }

    /// Get access to the underlying worker (for advanced operations)
    pub fn worker(&self) -> &ParseWorker {
        &self.worker
    }

    /// Get next request ID for a stream
    pub fn next_request_id(&self, stream_id: StreamId) -> u64 {
        self.worker.next_request_id(stream_id)
    }
}

impl Default for ParseCoordinator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_no_pending() {
        let coord = ParseCoordinator::new();
        assert!(!coord.is_pending());
        assert!(coord.pending().is_none());
    }

    #[test]
    fn test_pending_state() {
        let mut coord = ParseCoordinator::new();

        // Manually set pending for testing
        coord.pending = Some((StreamId::A, PathBuf::from("test.ivf")));

        assert!(coord.is_pending());
        assert_eq!(
            coord.pending(),
            Some((StreamId::A, PathBuf::from("test.ivf")))
        );
    }

    #[test]
    fn test_clear_pending_if_matches() {
        let mut coord = ParseCoordinator::new();
        coord.pending = Some((StreamId::A, PathBuf::from("test.ivf")));

        // Wrong stream - should not clear
        coord.clear_pending_if_matches(StreamId::B, &PathBuf::from("test.ivf"));
        assert!(coord.is_pending());

        // Wrong path - should not clear
        coord.clear_pending_if_matches(StreamId::A, &PathBuf::from("other.ivf"));
        assert!(coord.is_pending());

        // Correct match - should clear
        coord.clear_pending_if_matches(StreamId::A, &PathBuf::from("test.ivf"));
        assert!(!coord.is_pending());
    }

    #[test]
    fn test_cancel_stream_clears_pending() {
        let mut coord = ParseCoordinator::new();
        coord.pending = Some((StreamId::A, PathBuf::from("test.ivf")));

        // Cancel different stream - should not clear pending
        coord.cancel_stream(StreamId::B);
        assert!(coord.is_pending());

        // Cancel same stream - should clear pending
        coord.cancel_stream(StreamId::A);
        assert!(!coord.is_pending());
    }

    #[test]
    fn test_has_pending_work() {
        let coord = ParseCoordinator::new();
        // New coordinator has no pending work
        assert!(!coord.has_pending_work());
    }

    #[test]
    fn test_next_request_id() {
        let coord = ParseCoordinator::new();
        let id1 = coord.next_request_id(StreamId::A);
        let id2 = coord.next_request_id(StreamId::A);
        assert_eq!(id2, id1 + 1);
    }

    #[test]
    fn test_default() {
        let coord: ParseCoordinator = Default::default();
        assert!(!coord.is_pending());
    }
}
