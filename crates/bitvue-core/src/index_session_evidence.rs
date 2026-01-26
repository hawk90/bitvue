//! Index Session Evidence Integration - T1-1 Session Evidence Chain
//!
//! Deliverable: evidence_chain_01_bit_offset:Indexing:Session:AV1:evidence_chain
//!
//! Integrates the evidence chain system with indexing sessions to enable:
//! - Tracing session operations (quick index, full index) back to indexed frames
//! - Linking session state transitions to evidence chain
//! - Bidirectional navigation: session events ↔ frames ↔ bit offsets

use crate::{
    EvidenceChain, EvidenceId, IndexExtractorEvidenceManager, IndexingPhase, IndexingState,
};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};

/// Session operation types that generate evidence
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SessionOperation {
    /// Quick index operation started
    QuickIndexStart,
    /// Quick index operation completed
    QuickIndexComplete { keyframe_count: usize },
    /// Full index operation started
    FullIndexStart,
    /// Full index operation completed
    FullIndexComplete { total_frames: usize },
    /// Session reset
    SessionReset,
    /// Operation cancelled
    OperationCancelled { phase: IndexingPhase },
    /// Operation error
    OperationError {
        phase: IndexingPhase,
        error_message: String,
    },
}

/// Evidence for a session operation
///
/// Links session-level events to the frame evidence they produced.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionOperationEvidence {
    /// Unique evidence ID for this operation
    pub operation_id: EvidenceId,

    /// Type of operation
    pub operation: SessionOperation,

    /// Session state before operation
    pub state_before: IndexingState,

    /// Session state after operation
    pub state_after: IndexingState,

    /// Timestamp (milliseconds since session start)
    pub timestamp_ms: u64,

    /// Frame evidence IDs produced by this operation
    pub frame_evidence_ids: Vec<EvidenceId>,
}

/// Session evidence manager
///
/// Tracks session operations and links them to frame evidence through the evidence chain.
#[derive(Debug, Clone)]
pub struct IndexSessionEvidenceManager {
    /// Session ID for this evidence manager
    session_id: String,

    /// Session start timestamp (for relative timestamps)
    session_start_ms: u64,

    /// All session operations (chronological order)
    operations: Vec<SessionOperationEvidence>,

    /// Frame extractor evidence manager (contains frame → bit offset mappings)
    frame_evidence_manager: Arc<Mutex<IndexExtractorEvidenceManager>>,

    /// Next operation ID counter
    next_operation_id: u64,
}

impl IndexSessionEvidenceManager {
    /// Create a new session evidence manager
    pub fn new(
        session_id: String,
        frame_evidence_manager: Arc<Mutex<IndexExtractorEvidenceManager>>,
    ) -> Self {
        Self {
            session_id,
            session_start_ms: Self::current_time_ms(),
            operations: Vec::new(),
            frame_evidence_manager,
            next_operation_id: 0,
        }
    }

    /// Get current timestamp in milliseconds (mock for testing)
    fn current_time_ms() -> u64 {
        // In production, this would use std::time::SystemTime
        // For testing, we use a simple counter
        0
    }

    /// Generate next operation ID
    fn next_operation_id(&mut self) -> EvidenceId {
        let id = format!("session_{}_op_{}", self.session_id, self.next_operation_id);
        self.next_operation_id += 1;
        id
    }

    /// Get relative timestamp from session start
    fn relative_timestamp_ms(&self) -> u64 {
        Self::current_time_ms().saturating_sub(self.session_start_ms)
    }

    /// Record a session operation
    ///
    /// Creates evidence for the operation and links to any frame evidence produced.
    pub fn record_operation(
        &mut self,
        operation: SessionOperation,
        state_before: IndexingState,
        state_after: IndexingState,
        frame_display_indices: Vec<usize>,
    ) -> EvidenceId {
        let operation_id = self.next_operation_id();

        // Collect frame evidence IDs for the indexed frames
        let frame_evidence_ids = {
            let frame_mgr = self.frame_evidence_manager.lock().unwrap();
            frame_display_indices
                .iter()
                .filter_map(|&display_idx| {
                    frame_mgr
                        .get_frame_evidence(display_idx)
                        .map(|ev| ev.bit_offset_id.clone())
                })
                .collect()
        };

        let evidence = SessionOperationEvidence {
            operation_id: operation_id.clone(),
            operation,
            state_before,
            state_after,
            timestamp_ms: self.relative_timestamp_ms(),
            frame_evidence_ids,
        };

        self.operations.push(evidence);
        operation_id
    }

    /// Get all session operations (chronological order)
    pub fn operations(&self) -> &[SessionOperationEvidence] {
        &self.operations
    }

    /// Get operation by ID
    pub fn get_operation(&self, operation_id: &str) -> Option<&SessionOperationEvidence> {
        self.operations
            .iter()
            .find(|op| op.operation_id == operation_id)
    }

    /// Get operations by type
    pub fn get_operations_by_type(
        &self,
        operation_type: &SessionOperation,
    ) -> Vec<&SessionOperationEvidence> {
        self.operations
            .iter()
            .filter(|op| {
                std::mem::discriminant(&op.operation) == std::mem::discriminant(operation_type)
            })
            .collect()
    }

    /// Get the last operation
    pub fn last_operation(&self) -> Option<&SessionOperationEvidence> {
        self.operations.last()
    }

    /// Trace from session operation to indexed frames
    ///
    /// Returns the display indices of frames indexed by this operation.
    pub fn trace_operation_to_frames(&self, operation_id: &str) -> Vec<usize> {
        if let Some(op) = self.get_operation(operation_id) {
            let frame_mgr = self.frame_evidence_manager.lock().unwrap();

            // For each frame evidence ID, find the display_idx
            op.frame_evidence_ids
                .iter()
                .filter_map(|frame_ev_id| {
                    // Search through all frame evidence to find matching bit_offset_id
                    frame_mgr
                        .all_frame_evidence()
                        .iter()
                        .find(|fe| &fe.bit_offset_id == frame_ev_id)
                        .map(|fe| fe.display_idx)
                })
                .collect()
        } else {
            Vec::new()
        }
    }

    /// Trace from display index to session operations
    ///
    /// Returns all operations that indexed this frame.
    pub fn trace_frame_to_operations(&self, display_idx: usize) -> Vec<&SessionOperationEvidence> {
        // Get frame evidence
        let frame_ev_id = {
            let frame_mgr = self.frame_evidence_manager.lock().unwrap();
            frame_mgr
                .get_frame_evidence(display_idx)
                .map(|ev| ev.bit_offset_id.clone())
        };

        if let Some(frame_id) = frame_ev_id {
            // Find all operations that reference this frame evidence
            self.operations
                .iter()
                .filter(|op| op.frame_evidence_ids.contains(&frame_id))
                .collect()
        } else {
            Vec::new()
        }
    }

    /// Get session statistics
    pub fn session_stats(&self) -> SessionStats {
        let mut stats = SessionStats::default();

        for op in &self.operations {
            match &op.operation {
                SessionOperation::QuickIndexComplete { keyframe_count } => {
                    stats.quick_index_count += 1;
                    stats.total_keyframes_indexed += keyframe_count;
                }
                SessionOperation::FullIndexComplete { total_frames } => {
                    stats.full_index_count += 1;
                    stats.total_frames_indexed += total_frames;
                }
                SessionOperation::OperationCancelled { .. } => {
                    stats.cancelled_operations += 1;
                }
                SessionOperation::OperationError { .. } => {
                    stats.error_operations += 1;
                }
                _ => {}
            }
        }

        stats.total_operations = self.operations.len();
        stats
    }

    /// Get evidence chain from frame evidence manager
    pub fn evidence_chain(&self) -> EvidenceChain {
        self.frame_evidence_manager
            .lock()
            .unwrap()
            .evidence_chain()
            .clone()
    }

    /// Get session ID
    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    /// Clear all session evidence
    pub fn clear(&mut self) {
        self.operations.clear();
        self.next_operation_id = 0;
        self.session_start_ms = Self::current_time_ms();
    }
}

/// Session statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SessionStats {
    /// Total number of operations
    pub total_operations: usize,
    /// Number of quick index operations
    pub quick_index_count: usize,
    /// Number of full index operations
    pub full_index_count: usize,
    /// Total keyframes indexed across all quick indices
    pub total_keyframes_indexed: usize,
    /// Total frames indexed across all full indices
    pub total_frames_indexed: usize,
    /// Number of cancelled operations
    pub cancelled_operations: usize,
    /// Number of error operations
    pub error_operations: usize,
}

#[cfg(test)]
mod tests {
    include!("index_session_evidence_test.rs");
}
