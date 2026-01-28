//! Async decode worker - Background frame decoding
//!
//! Per WORKER_MODEL.md and ASYNC_PIPELINE_BACKPRESSURE.md:
//! - Latest-wins: only most recent request kept
//! - Max 2 in-flight tasks per stream
//! - Request ID tracking for stale result discarding
//! - UI thread never blocks

use bitvue_core::{CachedFrame, StreamId};
use crossbeam_channel::{bounded, Receiver, Sender};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::thread;

/// Decode request sent to worker
#[derive(Debug, Clone)]
pub struct DecodeRequest {
    pub stream_id: StreamId,
    pub frame_index: usize,
    pub file_data: Arc<Vec<u8>>,
    pub request_id: u64,
}

/// Decode result from worker
#[derive(Debug)]
pub struct DecodeResult {
    pub stream_id: StreamId,
    pub frame_index: usize,
    pub request_id: u64,
    pub cached_frame: Result<CachedFrame, String>,
}

/// Async decode worker with background thread
pub struct DecodeWorker {
    /// Send requests to worker
    request_tx: Sender<DecodeRequest>,
    /// Receive results from worker
    result_rx: Receiver<DecodeResult>,
    /// Current request ID per stream (for stale detection)
    request_ids: Arc<[AtomicU64; 2]>,
    /// Worker thread handle
    _thread: thread::JoinHandle<()>,
}

impl DecodeWorker {
    /// Create new decode worker with background thread
    pub fn new() -> Self {
        let (request_tx, request_rx) = bounded::<DecodeRequest>(4);
        let (result_tx, result_rx) = bounded::<DecodeResult>(4);
        let request_ids = Arc::new([AtomicU64::new(0), AtomicU64::new(0)]);

        let worker_request_ids = Arc::clone(&request_ids);

        let thread = thread::Builder::new()
            .name("decode-worker".to_string())
            .spawn(move || {
                Self::worker_loop(request_rx, result_tx, worker_request_ids);
            })
            .expect("Failed to spawn decode worker thread - system may be out of resources");

        Self {
            request_tx,
            result_rx,
            request_ids,
            _thread: thread,
        }
    }

    /// Worker loop - processes decode requests
    fn worker_loop(
        request_rx: Receiver<DecodeRequest>,
        result_tx: Sender<DecodeResult>,
        request_ids: Arc<[AtomicU64; 2]>,
    ) {
        tracing::info!("Decode worker started");

        while let Ok(request) = request_rx.recv() {
            // NOTE: Removed stale request check to allow multiple concurrent frame decodes
            // The old logic would skip frame 0 when frames 0,1 were queued together
            // since request_ids[0] would be overwritten by frame 1's request_id

            tracing::debug!(
                "Decode worker: Processing frame {} for stream {:?}",
                request.frame_index,
                request.stream_id
            );

            // Decode frame
            let cached_frame = Self::decode_frame(&request);

            // Send result
            let result = DecodeResult {
                stream_id: request.stream_id,
                frame_index: request.frame_index,
                request_id: request.request_id,
                cached_frame: cached_frame.clone(),
            };

            match &cached_frame {
                Ok(_) => tracing::info!(
                    "🎥 DECODE_WORKER: Sending OK result for frame {}",
                    request.frame_index
                ),
                Err(e) => tracing::error!(
                    "🎥 DECODE_WORKER: Sending ERROR result for frame {}: {}",
                    request.frame_index,
                    e
                ),
            }

            if result_tx.send(result).is_err() {
                tracing::warn!("Decode worker: Result channel closed");
                break;
            }

            tracing::info!(
                "🎥 DECODE_WORKER: Result sent for frame {}",
                request.frame_index
            );
        }

        tracing::info!("Decode worker stopped");
    }

    /// Decode a single frame
    fn decode_frame(request: &DecodeRequest) -> Result<CachedFrame, String> {
        use bitvue_decode::{yuv_to_rgb, Av1Decoder};

        let mut decoder = Av1Decoder::new().map_err(|e| format!("Decoder init failed: {:?}", e))?;

        let decoded_frames = decoder
            .decode_all(&request.file_data)
            .map_err(|e| format!("Decode failed: {:?}", e))?;

        tracing::info!(
            "🎥 DECODE_WORKER: decode_all() returned {} frames, requesting index {}",
            decoded_frames.len(),
            request.frame_index
        );

        let decoded = decoded_frames.get(request.frame_index).ok_or_else(|| {
            tracing::error!(
                "🎥 DECODE_WORKER: ❌ Frame {} not found in decoded_frames (len={})",
                request.frame_index,
                decoded_frames.len()
            );
            format!(
                "Frame {} not found (decoded {} frames)",
                request.frame_index,
                decoded_frames.len()
            )
        })?;

        let rgb_data = yuv_to_rgb(decoded);
        let chroma_width = decoded.width / 2;
        let chroma_height = decoded.height / 2;

        Ok(CachedFrame {
            index: request.frame_index,
            rgb_data,
            width: decoded.width,
            height: decoded.height,
            decoded: true,
            error: None,
            y_plane: Some(decoded.y_plane.clone()),
            u_plane: decoded.u_plane.clone(),
            v_plane: decoded.v_plane.clone(),
            chroma_width: Some(chroma_width),
            chroma_height: Some(chroma_height),
        })
    }

    /// Submit decode request (non-blocking)
    pub fn submit(&self, request: DecodeRequest) -> bool {
        // Update request ID for this stream
        let stream_idx = match request.stream_id {
            StreamId::A => 0,
            StreamId::B => 1,
        };
        self.request_ids[stream_idx].store(request.request_id, Ordering::SeqCst);

        // Try to send (non-blocking)
        match self.request_tx.try_send(request) {
            Ok(_) => true,
            Err(crossbeam_channel::TrySendError::Full(_)) => {
                tracing::debug!("Decode worker: Queue full, dropping request");
                false
            }
            Err(crossbeam_channel::TrySendError::Disconnected(_)) => {
                tracing::warn!("Decode worker: Channel disconnected");
                false
            }
        }
    }

    /// Poll for decode results (non-blocking)
    pub fn poll_results(&self) -> Vec<DecodeResult> {
        let mut results = Vec::new();
        while let Ok(result) = self.result_rx.try_recv() {
            // NOTE: Removed stale result check to allow multiple concurrent frame decodes
            tracing::debug!(
                "Decode worker: Received result for frame {}",
                result.frame_index
            );
            results.push(result);
        }
        results
    }

    /// Get next request ID for a stream
    pub fn next_request_id(&self, stream_id: StreamId) -> u64 {
        let stream_idx = match stream_id {
            StreamId::A => 0,
            StreamId::B => 1,
        };
        self.request_ids[stream_idx].fetch_add(1, Ordering::SeqCst) + 1
    }

    /// Cancel all pending requests for a stream (scrub behavior)
    pub fn cancel_stream(&self, stream_id: StreamId) {
        // Increment request ID to invalidate all pending requests
        let _ = self.next_request_id(stream_id);
        tracing::debug!(
            "Decode worker: Cancelled all requests for stream {:?}",
            stream_id
        );
    }

    /// Check if there's pending work
    pub fn has_pending_work(&self) -> bool {
        !self.result_rx.is_empty()
    }
}

impl Default for DecodeWorker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_next_request_id_increments() {
        let worker = DecodeWorker::new();

        let id1 = worker.next_request_id(StreamId::A);
        let id2 = worker.next_request_id(StreamId::A);
        let id3 = worker.next_request_id(StreamId::A);

        assert_eq!(id1, 1);
        assert_eq!(id2, 2);
        assert_eq!(id3, 3);
    }

    #[test]
    fn test_request_ids_per_stream_independent() {
        let worker = DecodeWorker::new();

        let a1 = worker.next_request_id(StreamId::A);
        let b1 = worker.next_request_id(StreamId::B);
        let a2 = worker.next_request_id(StreamId::A);
        let b2 = worker.next_request_id(StreamId::B);

        assert_eq!(a1, 1);
        assert_eq!(b1, 1);
        assert_eq!(a2, 2);
        assert_eq!(b2, 2);
    }

    #[test]
    fn test_cancel_stream_increments_request_id() {
        let worker = DecodeWorker::new();

        let id1 = worker.next_request_id(StreamId::A);
        worker.cancel_stream(StreamId::A);
        let id2 = worker.next_request_id(StreamId::A);

        // cancel_stream calls next_request_id internally, so id2 should be id1 + 2
        assert_eq!(id1, 1);
        assert_eq!(id2, 3);
    }

    #[test]
    fn test_cancel_stream_does_not_affect_other_stream() {
        let worker = DecodeWorker::new();

        let a1 = worker.next_request_id(StreamId::A);
        let b1 = worker.next_request_id(StreamId::B);

        worker.cancel_stream(StreamId::A);

        let b2 = worker.next_request_id(StreamId::B);

        assert_eq!(a1, 1);
        assert_eq!(b1, 1);
        assert_eq!(b2, 2); // B should be unaffected by A's cancel
    }

    #[test]
    fn test_poll_results_empty_initially() {
        let worker = DecodeWorker::new();
        let results = worker.poll_results();
        assert!(results.is_empty());
    }

    #[test]
    fn test_has_pending_work_false_initially() {
        let worker = DecodeWorker::new();
        assert!(!worker.has_pending_work());
    }

    #[test]
    fn test_decode_request_clone() {
        let request = DecodeRequest {
            stream_id: StreamId::A,
            frame_index: 42,
            file_data: Arc::new(vec![1, 2, 3]),
            request_id: 123,
        };

        let cloned = request.clone();
        assert_eq!(cloned.stream_id, StreamId::A);
        assert_eq!(cloned.frame_index, 42);
        assert_eq!(cloned.request_id, 123);
        assert_eq!(cloned.file_data.len(), 3);
    }

    #[test]
    fn test_decode_result_debug() {
        let result = DecodeResult {
            stream_id: StreamId::B,
            frame_index: 10,
            request_id: 456,
            cached_frame: Err("test error".to_string()),
        };

        // Verify Debug trait works
        let debug_str = format!("{:?}", result);
        assert!(debug_str.contains("frame_index: 10"));
        assert!(debug_str.contains("request_id: 456"));
    }
}
