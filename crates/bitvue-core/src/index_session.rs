//! Index Session Management - T1-1 Session
//!
//! Deliverable: session_manager:Indexing:Session:AV1:viz_core
//!
//! Manages the two-phase indexing workflow:
//! - Phase 1: Quick index (keyframes only, fast startup)
//! - Phase 2: Full index (all frames, background task)
//!
//! Per INDEXING_STRATEGY_SPEC.md:
//! - Quick index enables first frame display ASAP
//! - Full index builds in background with progress
//! - Supports cancellation and error recovery

use crate::{
    BitvueError, FullIndex, IndexExtractor, IndexExtractorEvidenceManager, QuickIndex, ReadSeek,
};
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

/// Indexing state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum IndexingState {
    /// No indexing has started
    Idle,
    /// Quick index in progress
    QuickIndexing,
    /// Quick index complete, full index not started
    QuickComplete,
    /// Full index in progress
    FullIndexing,
    /// Full index complete
    FullComplete,
    /// Error occurred during indexing
    Error,
    /// Indexing was cancelled
    Cancelled,
}

/// Progress information for indexing operations
#[derive(Debug, Clone)]
pub struct IndexingProgress {
    /// Current phase (quick or full)
    pub phase: IndexingPhase,
    /// Progress fraction (0.0 to 1.0)
    pub progress: f64,
    /// Status message
    pub message: String,
    /// Frames indexed so far
    pub frames_indexed: usize,
}

/// Indexing phase
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum IndexingPhase {
    Quick,
    Full,
}

/// Index session manager
///
/// Coordinates two-phase indexing workflow with progress tracking and cancellation.
pub struct IndexSession {
    /// Current indexing state
    state: Arc<Mutex<IndexingState>>,

    /// Quick index result (None until quick index completes)
    quick_index: Arc<Mutex<Option<QuickIndex>>>,

    /// Full index result (None until full index completes)
    full_index: Arc<Mutex<Option<FullIndex>>>,

    /// Evidence manager for linking frames to bit offsets
    evidence_manager: Arc<Mutex<IndexExtractorEvidenceManager>>,

    /// Cancellation flag
    should_cancel: Arc<AtomicBool>,

    /// Progress counter for UI updates
    progress_counter: Arc<AtomicU64>,
}

impl IndexSession {
    /// Create a new index session
    pub fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(IndexingState::Idle)),
            quick_index: Arc::new(Mutex::new(None)),
            full_index: Arc::new(Mutex::new(None)),
            evidence_manager: Arc::new(Mutex::new(IndexExtractorEvidenceManager::new_empty())),
            should_cancel: Arc::new(AtomicBool::new(false)),
            progress_counter: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Get current indexing state
    pub fn state(&self) -> IndexingState {
        *self.state.lock().unwrap()
    }

    /// Check if quick index is complete
    pub fn is_quick_complete(&self) -> bool {
        matches!(
            self.state(),
            IndexingState::QuickComplete
                | IndexingState::FullIndexing
                | IndexingState::FullComplete
        )
    }

    /// Check if full index is complete
    pub fn is_full_complete(&self) -> bool {
        self.state() == IndexingState::FullComplete
    }

    /// Get quick index (if available)
    /// OPTIMIZATION: Avoid cloning by returning None and requiring caller to use get_quick_index_ref if needed
    pub fn quick_index(&self) -> Option<QuickIndex> {
        // Note: This clones for API compatibility. For performance-critical code,
        // consider using with_quick_index() pattern instead
        self.quick_index.lock().unwrap().as_ref().cloned()
    }

    /// Get full index (if available)
    /// OPTIMIZATION: Avoid cloning by returning None and requiring caller to use get_full_index_ref if needed
    pub fn full_index(&self) -> Option<FullIndex> {
        // Note: This clones for API compatibility. For performance-critical code,
        // consider using with_full_index() pattern instead
        self.full_index.lock().unwrap().as_ref().cloned()
    }

    /// Get evidence manager
    pub fn evidence_manager(&self) -> Arc<Mutex<IndexExtractorEvidenceManager>> {
        Arc::clone(&self.evidence_manager)
    }

    /// Request cancellation of current indexing operation
    pub fn cancel(&self) {
        self.should_cancel.store(true, Ordering::SeqCst);
    }

    /// Reset cancellation flag
    fn reset_cancellation(&self) {
        self.should_cancel.store(false, Ordering::SeqCst);
    }

    /// Check if cancellation was requested
    fn is_cancelled(&self) -> bool {
        self.should_cancel.load(Ordering::SeqCst)
    }

    /// Execute quick index phase
    ///
    /// Scans for keyframes only, enabling fast startup.
    /// Progress callback receives (phase, progress, message).
    pub fn execute_quick_index<F>(
        &self,
        extractor: &dyn IndexExtractor,
        reader: &mut dyn ReadSeek,
        progress_callback: Option<F>,
    ) -> Result<QuickIndex, BitvueError>
    where
        F: Fn(IndexingProgress),
    {
        // Update state
        {
            let mut state = self.state.lock().unwrap();
            if *state != IndexingState::Idle {
                return Err(BitvueError::InvalidData(format!(
                    "Cannot start quick index from state {:?}",
                    *state
                )));
            }
            *state = IndexingState::QuickIndexing;
        }

        self.reset_cancellation();

        // Report start
        if let Some(ref callback) = progress_callback {
            callback(IndexingProgress {
                phase: IndexingPhase::Quick,
                progress: 0.0,
                message: "Starting quick index...".to_string(),
                frames_indexed: 0,
            });
        }

        // Execute quick extraction
        let result = extractor.extract_quick_index(reader);

        match result {
            Ok(quick_idx) => {
                // OPTIMIZATION: Extract data we need before moving quick_idx into mutex
                let seek_points_count = quick_idx.seek_points.len();

                // Store result (move instead of clone)
                *self.quick_index.lock().unwrap() = Some(quick_idx);

                // Create evidence for seek points
                {
                    let mut evidence_mgr = self.evidence_manager.lock().unwrap();
                    // Get reference to the stored quick_idx to avoid clone
                    if let Some(ref idx) = *self.quick_index.lock().unwrap() {
                        for seek_point in &idx.seek_points {
                            evidence_mgr.create_seekpoint_evidence(seek_point);
                        }
                    }
                }

                // Update state
                *self.state.lock().unwrap() = IndexingState::QuickComplete;

                // Report completion
                if let Some(ref callback) = progress_callback {
                    callback(IndexingProgress {
                        phase: IndexingPhase::Quick,
                        progress: 1.0,
                        message: format!("Quick index complete: {} keyframes", seek_points_count),
                        frames_indexed: seek_points_count,
                    });
                }

                // Return by cloning since we moved it into the mutex
                // This is necessary for the API but less efficient
                self.quick_index
                    .lock()
                    .unwrap()
                    .as_ref()
                    .cloned()
                    .ok_or(BitvueError::InvalidData(
                        "Quick index not found".to_string(),
                    ))
            }
            Err(e) => {
                *self.state.lock().unwrap() = IndexingState::Error;
                Err(e)
            }
        }
    }

    /// Execute full index phase
    ///
    /// Scans all frames, building complete metadata.
    /// Can be cancelled via cancel() method.
    /// Progress callback receives (phase, progress, message).
    pub fn execute_full_index<F>(
        &self,
        extractor: &dyn IndexExtractor,
        reader: &mut dyn ReadSeek,
        progress_callback: Option<F>,
    ) -> Result<FullIndex, BitvueError>
    where
        F: Fn(IndexingProgress),
    {
        // Update state
        {
            let mut state = self.state.lock().unwrap();
            if *state != IndexingState::QuickComplete {
                return Err(BitvueError::InvalidData(format!(
                    "Cannot start full index from state {:?}. Run quick index first.",
                    *state
                )));
            }
            *state = IndexingState::FullIndexing;
        }

        self.reset_cancellation();

        // Report start
        if let Some(ref callback) = progress_callback {
            callback(IndexingProgress {
                phase: IndexingPhase::Full,
                progress: 0.0,
                message: "Starting full index...".to_string(),
                frames_indexed: 0,
            });
        }

        // Create cancellation checker
        let should_cancel = Arc::clone(&self.should_cancel);
        let cancel_fn = move || should_cancel.load(Ordering::SeqCst);

        // Execute full extraction (without progress callback for simplicity)
        let result = extractor.extract_full_index(
            reader,
            None, // Progress callbacks not yet supported due to lifetime constraints
            Some(&cancel_fn),
        );

        match result {
            Ok(frames) => {
                // Check if cancelled
                if self.is_cancelled() {
                    *self.state.lock().unwrap() = IndexingState::Cancelled;
                    return Err(BitvueError::InvalidData(
                        "Full index cancelled by user".to_string(),
                    ));
                }

                // OPTIMIZATION: Extract count before moving frames
                let frames_count = frames.len();

                // Build full index
                let file_size = {
                    let quick = self.quick_index.lock().unwrap();
                    quick.as_ref().map(|q| q.file_size).unwrap_or(0)
                };

                let full_idx = FullIndex::new(frames, file_size, true);

                // Store result (move instead of clone)
                *self.full_index.lock().unwrap() = Some(full_idx);

                // Create evidence for all frames
                {
                    let mut evidence_mgr = self.evidence_manager.lock().unwrap();

                    // Get reference to stored frames to avoid clone
                    if let Some(ref idx) = *self.full_index.lock().unwrap() {
                        for frame in &idx.frames {
                            evidence_mgr.create_frame_metadata_evidence(frame);
                        }

                        // Update seek point sizes with accurate data
                        if let Some(quick) = self.quick_index.lock().unwrap().as_ref() {
                            for seek_point in &quick.seek_points {
                                if let Some(frame) = idx
                                    .frames
                                    .iter()
                                    .find(|f| f.display_idx == seek_point.display_idx)
                                {
                                    evidence_mgr.update_seekpoint_size(
                                        frame.display_idx,
                                        frame.size as usize,
                                    );
                                }
                            }
                        }
                    }
                }

                // Update state
                *self.state.lock().unwrap() = IndexingState::FullComplete;

                // Report completion
                if let Some(ref callback) = progress_callback {
                    callback(IndexingProgress {
                        phase: IndexingPhase::Full,
                        progress: 1.0,
                        message: format!("Full index complete: {} frames", frames_count),
                        frames_indexed: frames_count,
                    });
                }

                // Return by cloning since we moved it into the mutex
                self.full_index
                    .lock()
                    .unwrap()
                    .as_ref()
                    .cloned()
                    .ok_or(BitvueError::InvalidData("Full index not found".to_string()))
            }
            Err(e) => {
                if self.is_cancelled() {
                    *self.state.lock().unwrap() = IndexingState::Cancelled;
                } else {
                    *self.state.lock().unwrap() = IndexingState::Error;
                }
                Err(e)
            }
        }
    }

    /// Execute both phases sequentially
    ///
    /// Convenience method that runs quick index followed by full index.
    pub fn execute_full_workflow<F>(
        &self,
        extractor: &dyn IndexExtractor,
        reader: &mut dyn ReadSeek,
        progress_callback: Option<F>,
    ) -> Result<(QuickIndex, FullIndex), BitvueError>
    where
        F: Fn(IndexingProgress) + Clone,
    {
        // Phase 1: Quick index
        let quick_callback = progress_callback.clone();
        let quick_idx = self.execute_quick_index(extractor, reader, quick_callback)?;

        // Phase 2: Full index
        let full_idx = self.execute_full_index(extractor, reader, progress_callback)?;

        Ok((quick_idx, full_idx))
    }

    /// Reset session to idle state
    ///
    /// Clears all indexing results and evidence.
    pub fn reset(&self) {
        *self.state.lock().unwrap() = IndexingState::Idle;
        *self.quick_index.lock().unwrap() = None;
        *self.full_index.lock().unwrap() = None;
        self.evidence_manager.lock().unwrap().clear();
        self.reset_cancellation();
        self.progress_counter.store(0, Ordering::SeqCst);
    }

    /// Get current progress (0.0 to 1.0)
    ///
    /// Estimates progress based on state and frame count.
    pub fn estimated_progress(&self) -> f64 {
        match self.state() {
            IndexingState::Idle => 0.0,
            IndexingState::QuickIndexing => 0.0, // Quick is too fast to track meaningfully
            IndexingState::QuickComplete => 0.0,
            IndexingState::FullIndexing => {
                // Estimate based on progress counter
                let frames_so_far = self.progress_counter.load(Ordering::SeqCst);
                if let Some(quick) = self.quick_index() {
                    if let Some(estimated_total) = quick.estimated_frame_count {
                        return (frames_so_far as f64 / estimated_total as f64).min(0.99);
                    }
                }
                0.5 // Unknown, assume halfway
            }
            IndexingState::FullComplete => 1.0,
            IndexingState::Error | IndexingState::Cancelled => 0.0,
        }
    }
}

impl Default for IndexSession {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    include!("index_session_test.rs");
}
