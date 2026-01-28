//! Decode Coordinator - Extracted from BitvueApp
//!
//! Wraps DecodeWorker with pending state tracking for frame decoding.
//!
//! # Lazy Worker Spawning
//!
//! The decode worker thread is spawned lazily on the first decode request.
//! This saves 1 thread at startup if the user never decodes frames (e.g., just analyzing bitstream).

use crate::decode_worker::{DecodeRequest, DecodeResult, DecodeWorker};
use bitvue_core::StreamId;
use std::sync::Arc;

/// Coordinates async frame decoding with pending state tracking
pub struct DecodeCoordinator {
    /// The underlying decode worker (lazy-initialized on first decode)
    worker: Option<DecodeWorker>,
    /// Currently pending decodes (stream, frame_index) - tracks multiple frames
    pending: std::collections::HashSet<(StreamId, usize)>,
}

impl DecodeCoordinator {
    /// Create a new decode coordinator (no worker thread spawned yet)
    pub fn new() -> Self {
        Self {
            worker: None,
            pending: std::collections::HashSet::new(),
        }
    }

    /// Get or initialize the worker (lazy thread spawning)
    fn get_or_init_worker(&mut self) -> &mut DecodeWorker {
        if self.worker.is_none() {
            tracing::info!("Lazy spawning decode worker thread");
            self.worker = Some(DecodeWorker::new());
        }
        self.worker.as_mut().unwrap()
    }

    /// Submit a decode request (non-blocking)
    ///
    /// Returns the request ID if submitted successfully, None if queue is full.
    /// Spawns the worker thread on first call (lazy initialization).
    pub fn submit(
        &mut self,
        stream_id: StreamId,
        frame_index: usize,
        file_data: Arc<Vec<u8>>,
    ) -> Option<u64> {
        let worker = self.get_or_init_worker(); // Lazy spawn here!
        let request_id = worker.next_request_id(stream_id);

        let request = DecodeRequest {
            stream_id,
            frame_index,
            file_data,
            request_id,
        };

        if worker.submit(request) {
            self.pending.insert((stream_id, frame_index));
            tracing::info!(
                "Submitted async decode: stream {:?}, frame {}, request_id {} (pending: {})",
                stream_id,
                frame_index,
                request_id,
                self.pending.len()
            );
            Some(request_id)
        } else {
            None
        }
    }

    /// Poll for decode results (non-blocking)
    ///
    /// Returns empty vector if worker hasn't been initialized yet.
    pub fn poll_results(&self) -> Vec<DecodeResult> {
        self.worker
            .as_ref()
            .map(|w| w.poll_results())
            .unwrap_or_else(Vec::new)
    }

    /// Clear pending state for a completed decode
    pub fn clear_pending_if_matches(&mut self, stream_id: StreamId, frame_index: usize) {
        self.pending.remove(&(stream_id, frame_index));
    }

    /// Get count of pending decodes
    pub fn pending_count(&self) -> usize {
        self.pending.len()
    }

    /// Check if there's any pending decode
    pub fn has_pending(&self) -> bool {
        !self.pending.is_empty()
    }

    /// Check if a specific frame is pending decode
    pub fn is_pending(&self, stream_id: StreamId, frame_index: usize) -> bool {
        self.pending.contains(&(stream_id, frame_index))
    }

    /// Check if there's pending work (in queue or in pending state)
    ///
    /// Returns false if worker hasn't been initialized (no work can be pending).
    pub fn has_pending_work(&self) -> bool {
        self.worker
            .as_ref()
            .map(|w| w.has_pending_work())
            .unwrap_or(false)
            || !self.pending.is_empty()
    }

    /// Cancel all pending requests for a stream
    pub fn cancel_stream(&mut self, stream_id: StreamId) {
        if let Some(worker) = self.worker.as_mut() {
            worker.cancel_stream(stream_id);
        }
        // Remove all pending for this stream
        self.pending.retain(|(s, _)| *s != stream_id);
    }

    /// Get access to the underlying worker (for advanced operations)
    ///
    /// Returns None if worker hasn't been initialized yet.
    pub fn worker(&self) -> Option<&DecodeWorker> {
        self.worker.as_ref()
    }

    /// Check if the decode worker has been initialized
    pub fn is_worker_initialized(&self) -> bool {
        self.worker.is_some()
    }
}

impl Default for DecodeCoordinator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_no_pending() {
        let coord = DecodeCoordinator::new();
        assert!(!coord.is_pending(StreamId::A, 0));
        assert!(!coord.has_pending());
        assert_eq!(coord.pending_count(), 0);
    }

    #[test]
    fn test_pending_state() {
        let mut coord = DecodeCoordinator::new();

        // Manually set pending for testing (normally set by submit)
        coord.pending.insert((StreamId::A, 5));

        assert!(coord.is_pending(StreamId::A, 5));
        assert!(!coord.is_pending(StreamId::A, 0));
        assert_eq!(coord.pending_count(), 1);
        assert!(coord.has_pending());
    }

    #[test]
    fn test_clear_pending_if_matches() {
        let mut coord = DecodeCoordinator::new();
        coord.pending.insert((StreamId::A, 5));

        // Wrong stream - should not clear
        coord.clear_pending_if_matches(StreamId::B, 5);
        assert!(coord.is_pending(StreamId::A, 5));

        // Wrong frame - should not clear
        coord.clear_pending_if_matches(StreamId::A, 6);
        assert!(coord.is_pending(StreamId::A, 5));

        // Correct match - should clear
        coord.clear_pending_if_matches(StreamId::A, 5);
        assert!(!coord.is_pending(StreamId::A, 5));
        assert!(!coord.has_pending());
    }

    #[test]
    fn test_cancel_stream_clears_pending() {
        let mut coord = DecodeCoordinator::new();
        coord.pending.insert((StreamId::A, 5));

        // Cancel different stream - should not clear pending
        coord.cancel_stream(StreamId::B);
        assert!(coord.is_pending(StreamId::A, 5));

        // Cancel same stream - should clear pending
        coord.cancel_stream(StreamId::A);
        assert!(!coord.is_pending(StreamId::A, 5));
        assert!(!coord.has_pending());
    }

    #[test]
    fn test_has_pending_work() {
        let coord = DecodeCoordinator::new();
        // New coordinator has no pending work
        assert!(!coord.has_pending_work());
    }

    #[test]
    fn test_default() {
        let coord: DecodeCoordinator = Default::default();
        assert!(!coord.is_pending(StreamId::A, 0));
        assert!(!coord.has_pending());
    }

    #[test]
    fn test_lazy_worker_not_initialized_on_new() {
        let coord = DecodeCoordinator::new();
        assert!(!coord.is_worker_initialized());
        assert!(coord.worker().is_none());
    }

    #[test]
    fn test_poll_results_empty_before_init() {
        let coord = DecodeCoordinator::new();
        assert!(!coord.is_worker_initialized());
        let results = coord.poll_results();
        assert!(results.is_empty());
    }

    #[test]
    fn test_has_pending_work_false_before_init() {
        let coord = DecodeCoordinator::new();
        assert!(!coord.is_worker_initialized());
        assert!(!coord.has_pending_work());
    }

    #[test]
    fn test_cancel_stream_safe_before_init() {
        let mut coord = DecodeCoordinator::new();
        assert!(!coord.is_worker_initialized());
        // Should not panic
        coord.cancel_stream(StreamId::A);
        assert!(!coord.is_worker_initialized());
    }
}
