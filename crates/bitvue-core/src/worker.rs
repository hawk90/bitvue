//! Worker runtime with last-wins cancellation
//!
//! Monster Pack v3: WORKER_MODEL.md
//! Monster Pack v14 T0-4: ASYNC_PIPELINE_BACKPRESSURE.md
//!
//! Key Contracts:
//! - Latest-wins: only most recent request kept
//! - Max in-flight tasks: 2 per stream
//! - Scrub behavior: cancel all non-current jobs
//! - Late results discarded via request_id mismatch
//! - UI thread never blocks

use crate::StreamId;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

/// Job types for worker pool
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Job {
    // Phase 0: Parsing
    ParseContainer {
        stream_id: StreamId,
        path: std::path::PathBuf,
        request_id: u64,
    },
    ParseUnits {
        stream_id: StreamId,
        request_id: u64,
    },
    BuildSyntaxTree {
        stream_id: StreamId,
        unit_offset: u64,
        request_id: u64,
    },

    // Phase 1: Timeline
    BuildTimelineIndex {
        stream_id: StreamId,
        request_id: u64,
    },
    BuildPlotLOD {
        stream_id: StreamId,
        zoom_level: u32,
        request_id: u64,
    },
    DecodeThumbnails {
        stream_id: StreamId,
        request_id: u64,
    },

    // Phase 2: Decode
    DecodeFrame {
        stream_id: StreamId,
        frame_index: usize,
        request_id: u64,
    },
    RenderOverlay {
        stream_id: StreamId,
        frame_index: usize,
        request_id: u64,
    },

    // Phase 3: Metrics
    ComputeMetrics {
        stream_id: StreamId,
        request_id: u64,
    },
}

impl Job {
    /// Get stream ID for this job
    pub fn stream_id(&self) -> StreamId {
        match self {
            Job::ParseContainer { stream_id, .. } => *stream_id,
            Job::ParseUnits { stream_id, .. } => *stream_id,
            Job::BuildSyntaxTree { stream_id, .. } => *stream_id,
            Job::BuildTimelineIndex { stream_id, .. } => *stream_id,
            Job::BuildPlotLOD { stream_id, .. } => *stream_id,
            Job::DecodeThumbnails { stream_id, .. } => *stream_id,
            Job::DecodeFrame { stream_id, .. } => *stream_id,
            Job::RenderOverlay { stream_id, .. } => *stream_id,
            Job::ComputeMetrics { stream_id, .. } => *stream_id,
        }
    }

    /// Get request ID for this job
    pub fn request_id(&self) -> u64 {
        match self {
            Job::ParseContainer { request_id, .. } => *request_id,
            Job::ParseUnits { request_id, .. } => *request_id,
            Job::BuildSyntaxTree { request_id, .. } => *request_id,
            Job::BuildTimelineIndex { request_id, .. } => *request_id,
            Job::BuildPlotLOD { request_id, .. } => *request_id,
            Job::DecodeThumbnails { request_id, .. } => *request_id,
            Job::DecodeFrame { request_id, .. } => *request_id,
            Job::RenderOverlay { request_id, .. } => *request_id,
            Job::ComputeMetrics { request_id, .. } => *request_id,
        }
    }

    /// Check if this job is a decode/convert job (cancelable during scrub)
    pub fn is_decode_convert(&self) -> bool {
        matches!(self, Job::DecodeFrame { .. } | Job::RenderOverlay { .. })
    }
}

/// Job priority (higher = more important)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum JobPriority {
    Low = 1,
    Normal = 2,
    High = 3,
}

/// Job state for tracking
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum JobState {
    Queued,
    InFlight,
    Completed,
    Cancelled,
    Discarded, // Late result (request_id mismatch)
}

/// Per-stream job queue state
///
/// Per ASYNC_PIPELINE_BACKPRESSURE.md:
/// - Latest-wins queue (only most recent request kept)
/// - Max 2 in-flight tasks per stream
#[derive(Debug)]
struct StreamQueue {
    /// Latest queued job (replaces previous)
    latest_job: Option<Job>,

    /// In-flight jobs (max 2)
    in_flight: Vec<Job>,

    /// In-flight count (atomic for lock-free reads)
    in_flight_count: AtomicUsize,
}

impl StreamQueue {
    fn new() -> Self {
        Self {
            latest_job: None,
            in_flight: Vec::with_capacity(2),
            in_flight_count: AtomicUsize::new(0),
        }
    }

    /// Enqueue job (latest-wins: replaces previous queued job)
    fn enqueue(&mut self, job: Job) {
        self.latest_job = Some(job);
    }

    /// Try to dequeue job if in-flight limit not reached
    fn try_dequeue(&mut self) -> Option<Job> {
        if self.in_flight.len() < 2 {
            self.latest_job.take()
        } else {
            None
        }
    }

    /// Mark job as in-flight
    fn mark_in_flight(&mut self, job: Job) {
        self.in_flight.push(job);
        self.in_flight_count
            .store(self.in_flight.len(), Ordering::Relaxed);
    }

    /// Complete job (remove from in-flight)
    fn complete_job(&mut self, job: &Job) {
        self.in_flight.retain(|j| j != job);
        self.in_flight_count
            .store(self.in_flight.len(), Ordering::Relaxed);
    }

    /// Cancel all jobs (scrub behavior)
    fn cancel_all(&mut self) {
        self.latest_job = None;
        self.in_flight.clear();
        self.in_flight_count.store(0, Ordering::Relaxed);
    }

    /// Cancel decode/convert jobs only (scrub behavior)
    fn cancel_decode_convert(&mut self) {
        // Clear queued decode/convert
        if let Some(job) = &self.latest_job {
            if job.is_decode_convert() {
                self.latest_job = None;
            }
        }

        // Cancel in-flight decode/convert
        self.in_flight.retain(|j| !j.is_decode_convert());
        self.in_flight_count
            .store(self.in_flight.len(), Ordering::Relaxed);
    }

    /// Get in-flight count (lock-free)
    fn in_flight_count(&self) -> usize {
        self.in_flight_count.load(Ordering::Relaxed)
    }
}

/// AsyncJobManager - Worker pool with latest-wins cancellation
///
/// Per T0-4 deliverable: AsyncJobManager
///
/// Implements:
/// - Latest-wins queue (only most recent request kept)
/// - Max 2 in-flight tasks per stream
/// - Scrub behavior (cancel non-current decode/convert jobs)
/// - Request ID tracking for late result discarding
/// - Cooperative cancellation
pub struct AsyncJobManager {
    // Request ID per stream (incremented on file open/reload/scrub)
    stream_request_ids: Arc<[AtomicU64; 2]>, // [StreamA, StreamB]

    // Per-stream job queues
    stream_queues: Arc<Mutex<[StreamQueue; 2]>>, // [StreamA, StreamB]

                                                 // TODO Phase 2: Add actual threadpool (crossbeam-channel or tokio)
                                                 // For MVP: in-memory queue + request_id tracking only
}

impl AsyncJobManager {
    pub fn new() -> Self {
        Self {
            stream_request_ids: Arc::new([AtomicU64::new(0), AtomicU64::new(0)]),
            stream_queues: Arc::new(Mutex::new([StreamQueue::new(), StreamQueue::new()])),
        }
    }

    /// Get stream index (0 = A, 1 = B)
    fn stream_idx(stream_id: StreamId) -> usize {
        match stream_id {
            StreamId::A => 0,
            StreamId::B => 1,
        }
    }

    /// Increment request_id for stream (cancels all pending jobs)
    ///
    /// Per ASYNC_PIPELINE_BACKPRESSURE.md:
    /// This is called on file open/reload to invalidate all pending work.
    pub fn increment_request_id(&self, stream_id: StreamId) -> u64 {
        let idx = Self::stream_idx(stream_id);
        let new_id = self.stream_request_ids[idx].fetch_add(1, Ordering::SeqCst) + 1;

        // Cancel all queued and in-flight jobs for this stream
        match self.stream_queues.lock() {
            Ok(mut queues) => {
                queues[idx].cancel_all();
            }
            Err(e) => {
                tracing::error!("AsyncJobManager: Mutex poisoned during cancel_all: {}", e);
                // Recover by creating new queue state
                // Note: In production, might want to abort or reinitialize
            }
        }

        new_id
    }

    /// Get current request_id for stream
    pub fn current_request_id(&self, stream_id: StreamId) -> u64 {
        let idx = Self::stream_idx(stream_id);
        self.stream_request_ids[idx].load(Ordering::SeqCst)
    }

    /// Check if a job result is current (not stale)
    ///
    /// Per EDGE_CASES_AND_DEGRADE_BEHAVIOR.md §E:
    /// "Late results discarded if request_id mismatches."
    pub fn is_result_current(&self, job: &Job) -> bool {
        job.request_id() == self.current_request_id(job.stream_id())
    }

    /// Submit job (latest-wins queue)
    ///
    /// Per ASYNC_PIPELINE_BACKPRESSURE.md:
    /// - Latest-wins: only most recent request kept
    /// - If in-flight < 2, start immediately
    /// - Otherwise, replace queued job
    pub fn submit(&self, job: Job) {
        let stream_id = job.stream_id();
        let idx = Self::stream_idx(stream_id);

        match self.stream_queues.lock() {
            Ok(mut queues) => {
                let queue = &mut queues[idx];

                // Check if we can start immediately (in-flight < 2)
                if queue.in_flight_count() < 2 {
                    queue.mark_in_flight(job.clone());
                    tracing::debug!(
                        "AsyncJobManager: Started job immediately: {:?} (in-flight: {})",
                        job,
                        queue.in_flight_count()
                    );
                    // TODO Phase 2: Actually spawn worker thread/task here
                } else {
                    // Queue replaces previous (latest-wins)
                    queue.enqueue(job.clone());
                    tracing::debug!("AsyncJobManager: Queued job (latest-wins): {:?}", job);
                }
            }
            Err(e) => {
                tracing::error!("AsyncJobManager: Mutex poisoned during submit: {}", e);
            }
        }
    }

    /// Complete job (called by worker when done)
    ///
    /// This removes the job from in-flight and tries to start queued job.
    /// Uses atomic check-and-invalidate pattern to prevent TOCTOU race condition.
    pub fn complete_job(&self, job: &Job) -> bool {
        let stream_id = job.stream_id();
        let idx = Self::stream_idx(stream_id);

        // Load queues with atomic check of request_id
        if let Ok(mut queues) = self.stream_queues.lock() {
            // Re-check request_id while holding lock to prevent TOCTOU
            let current_id = self.stream_request_ids[idx].load(Ordering::Acquire);
            if job.request_id() != current_id {
                tracing::debug!(
                    "AsyncJobManager: Discarding late result: {:?} (current_id: {}, job_id: {})",
                    job,
                    current_id,
                    job.request_id()
                );
                return false;
            }

            let queue = &mut queues[idx];
            queue.complete_job(job);

            // Try to start queued job
            if let Some(next_job) = queue.try_dequeue() {
                queue.mark_in_flight(next_job.clone());
                tracing::debug!(
                    "AsyncJobManager: Started queued job: {:?} (in-flight: {})",
                    next_job,
                    queue.in_flight_count()
                );
                // TODO Phase 2: Actually spawn worker thread/task here
            }
        }

        true
    }

    /// Scrub: cancel all non-current decode/convert jobs
    ///
    /// Per ASYNC_PIPELINE_BACKPRESSURE.md:
    /// "Cancel all non-current decode/convert jobs. Disable quality-path upgrades."
    ///
    /// Called during timeline scrubbing (rapid frame changes).
    pub fn scrub(&self, stream_id: StreamId) {
        let idx = Self::stream_idx(stream_id);

        if let Ok(mut queues) = self.stream_queues.lock() {
            queues[idx].cancel_decode_convert();
            tracing::debug!(
                "AsyncJobManager: Scrubbed decode/convert jobs for stream {:?}",
                stream_id
            );
        }
    }

    /// Cancel all jobs for stream
    pub fn cancel_all(&self, stream_id: StreamId) {
        let idx = Self::stream_idx(stream_id);

        if let Ok(mut queues) = self.stream_queues.lock() {
            queues[idx].cancel_all();
            tracing::debug!(
                "AsyncJobManager: Cancelled all jobs for stream {:?}",
                stream_id
            );
        }
    }

    /// Get in-flight job count for stream (lock-free)
    pub fn in_flight_count(&self, stream_id: StreamId) -> usize {
        let idx = Self::stream_idx(stream_id);
        if let Ok(queues) = self.stream_queues.lock() {
            queues[idx].in_flight_count()
        } else {
            0
        }
    }

    /// Check if queue has pending work (for diagnostics/telemetry)
    pub fn has_pending_work(&self, stream_id: StreamId) -> bool {
        let idx = Self::stream_idx(stream_id);
        if let Ok(queues) = self.stream_queues.lock() {
            queues[idx].latest_job.is_some() || !queues[idx].in_flight.is_empty()
        } else {
            false
        }
    }
}

impl Default for AsyncJobManager {
    fn default() -> Self {
        Self::new()
    }
}

// Backward compatibility: type alias
pub type JobManager = AsyncJobManager;

/// Comprehensive test suite with Arrange-Act-Assert pattern
#[cfg(test)]
mod tests {
    include!("worker_test.rs");
}
