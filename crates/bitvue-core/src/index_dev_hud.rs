//! Index Development HUD - T1-1 DevHUD
//!
//! Deliverable: dev_hud:Indexing:DevHUD:AV1:viz_core
//!
//! Provides diagnostic overlays and metrics for indexing system debugging.
//! Shows real-time state of sessions, windows, evidence chains, and caches.

use crate::{
    IndexExtractorEvidenceManager, IndexSession, IndexSessionEvidenceManager, IndexSessionWindow,
    IndexingState, WindowStats,
};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};

/// Development HUD for indexing system
///
/// Aggregates diagnostic data from all indexing components for debugging.
#[derive(Debug, Clone)]
pub struct IndexDevHUD {
    /// Session ID being monitored
    session_id: String,

    /// Snapshot of session state
    session_snapshot: SessionSnapshot,

    /// Snapshot of window state (if using out-of-core)
    window_snapshot: Option<WindowSnapshot>,

    /// Snapshot of evidence state
    evidence_snapshot: EvidenceSnapshot,

    /// Performance metrics
    performance_metrics: PerformanceMetrics,

    /// HUD update counter
    update_count: u64,
}

/// Session state snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionSnapshot {
    /// Current indexing state
    pub state: IndexingState,
    /// Quick index complete
    pub quick_complete: bool,
    /// Full index complete
    pub full_complete: bool,
    /// Total frames (from quick index estimate)
    pub total_frames_estimate: Option<usize>,
    /// Actual frames indexed (from full index)
    pub actual_frames_indexed: Option<usize>,
    /// Session progress (0.0 to 1.0)
    pub progress: f64,
}

/// Window state snapshot (out-of-core)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowSnapshot {
    /// Total frames in index
    pub total_frames: usize,
    /// Window start position
    pub window_start: usize,
    /// Window size
    pub window_size: usize,
    /// Current playback position
    pub current_position: usize,
    /// Materialized frame count
    pub materialized_count: usize,
    /// Window revision
    pub window_revision: u64,
    /// Window statistics
    pub stats: WindowStats,
    /// Cache hit rate
    pub hit_rate: f64,
}

/// Evidence chain snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvidenceSnapshot {
    /// Frame evidence count
    pub frame_evidence_count: usize,
    /// Session operation count
    pub session_operation_count: usize,
    /// Bit offset evidence count
    pub bit_offset_evidence_count: usize,
    /// Syntax evidence count
    pub syntax_evidence_count: usize,
}

/// Performance metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    /// Quick index duration (milliseconds)
    pub quick_index_duration_ms: Option<u64>,
    /// Full index duration (milliseconds)
    pub full_index_duration_ms: Option<u64>,
    /// Frames per second (full index)
    pub indexing_fps: Option<f64>,
    /// Memory estimate (bytes)
    pub estimated_memory_bytes: usize,
}

impl IndexDevHUD {
    /// Create a new development HUD
    pub fn new(session_id: String) -> Self {
        Self {
            session_id,
            session_snapshot: SessionSnapshot {
                state: IndexingState::Idle,
                quick_complete: false,
                full_complete: false,
                total_frames_estimate: None,
                actual_frames_indexed: None,
                progress: 0.0,
            },
            window_snapshot: None,
            evidence_snapshot: EvidenceSnapshot {
                frame_evidence_count: 0,
                session_operation_count: 0,
                bit_offset_evidence_count: 0,
                syntax_evidence_count: 0,
            },
            performance_metrics: PerformanceMetrics {
                quick_index_duration_ms: None,
                full_index_duration_ms: None,
                indexing_fps: None,
                estimated_memory_bytes: 0,
            },
            update_count: 0,
        }
    }

    /// Update HUD from index session
    pub fn update_from_session(&mut self, session: &IndexSession) {
        self.session_snapshot = SessionSnapshot {
            state: session.state(),
            quick_complete: session.is_quick_complete(),
            full_complete: session.is_full_complete(),
            total_frames_estimate: session.quick_index().and_then(|q| q.estimated_frame_count),
            actual_frames_indexed: session.full_index().map(|f| f.frames.len()),
            progress: session.estimated_progress(),
        };

        self.update_count += 1;
    }

    /// Update HUD from window state
    pub fn update_from_window(&mut self, window: &IndexSessionWindow) {
        self.window_snapshot = Some(WindowSnapshot {
            total_frames: window.total_frames,
            window_start: window.window_start,
            window_size: window.window_size,
            current_position: window.current_position,
            materialized_count: window.materialized_count(),
            window_revision: window.window_revision(),
            stats: window.stats().clone(),
            hit_rate: window.hit_rate(),
        });

        self.update_count += 1;
    }

    /// Update HUD from evidence managers
    pub fn update_from_evidence(
        &mut self,
        frame_evidence: Arc<Mutex<IndexExtractorEvidenceManager>>,
        session_evidence: &IndexSessionEvidenceManager,
    ) {
        let frame_mgr = frame_evidence.lock().unwrap();
        let chain = frame_mgr.evidence_chain();

        self.evidence_snapshot = EvidenceSnapshot {
            frame_evidence_count: frame_mgr.frame_count(),
            session_operation_count: session_evidence.operations().len(),
            bit_offset_evidence_count: chain.bit_offset_index.len(),
            syntax_evidence_count: chain.syntax_index.len(),
        };

        self.update_count += 1;
    }

    /// Update performance metrics
    pub fn update_performance(
        &mut self,
        quick_duration_ms: Option<u64>,
        full_duration_ms: Option<u64>,
    ) {
        self.performance_metrics.quick_index_duration_ms = quick_duration_ms;
        self.performance_metrics.full_index_duration_ms = full_duration_ms;

        // Calculate indexing FPS
        if let (Some(duration), Some(frame_count)) = (
            full_duration_ms,
            self.session_snapshot.actual_frames_indexed,
        ) {
            if duration > 0 {
                let duration_sec = duration as f64 / 1000.0;
                self.performance_metrics.indexing_fps = Some(frame_count as f64 / duration_sec);
            }
        }

        // Estimate memory usage
        self.performance_metrics.estimated_memory_bytes = self.estimate_memory_usage();

        self.update_count += 1;
    }

    /// Estimate current memory usage
    fn estimate_memory_usage(&self) -> usize {
        let mut total = 0;

        // Frame evidence (rough estimate: 200 bytes per frame)
        total += self.evidence_snapshot.frame_evidence_count * 200;

        // Window materialized frames (rough estimate: 300 bytes per frame)
        if let Some(ref window) = self.window_snapshot {
            total += window.materialized_count * 300;
        }

        // Evidence chain (rough estimate: 150 bytes per bit offset + syntax)
        total += self.evidence_snapshot.bit_offset_evidence_count * 150;
        total += self.evidence_snapshot.syntax_evidence_count * 150;

        total
    }

    /// Get session snapshot
    pub fn session_snapshot(&self) -> &SessionSnapshot {
        &self.session_snapshot
    }

    /// Get window snapshot
    pub fn window_snapshot(&self) -> Option<&WindowSnapshot> {
        self.window_snapshot.as_ref()
    }

    /// Get evidence snapshot
    pub fn evidence_snapshot(&self) -> &EvidenceSnapshot {
        &self.evidence_snapshot
    }

    /// Get performance metrics
    pub fn performance_metrics(&self) -> &PerformanceMetrics {
        &self.performance_metrics
    }

    /// Get update count
    pub fn update_count(&self) -> u64 {
        self.update_count
    }

    /// Format HUD as human-readable text
    pub fn format_text(&self) -> String {
        let mut output = String::new();

        output.push_str(&format!("=== Index DevHUD: {} ===\n", self.session_id));
        output.push_str(&format!("Updates: {}\n\n", self.update_count));

        // Session state
        output.push_str("--- Session ---\n");
        output.push_str(&format!("State: {:?}\n", self.session_snapshot.state));
        output.push_str(&format!(
            "Progress: {:.1}%\n",
            self.session_snapshot.progress * 100.0
        ));
        output.push_str(&format!(
            "Quick Complete: {}\n",
            self.session_snapshot.quick_complete
        ));
        output.push_str(&format!(
            "Full Complete: {}\n",
            self.session_snapshot.full_complete
        ));
        if let Some(est) = self.session_snapshot.total_frames_estimate {
            output.push_str(&format!("Estimated Frames: {}\n", est));
        }
        if let Some(actual) = self.session_snapshot.actual_frames_indexed {
            output.push_str(&format!("Indexed Frames: {}\n", actual));
        }
        output.push('\n');

        // Window state
        if let Some(ref window) = self.window_snapshot {
            output.push_str("--- Window (Out-of-Core) ---\n");
            output.push_str(&format!("Total Frames: {}\n", window.total_frames));
            output.push_str(&format!(
                "Window: {}-{}\n",
                window.window_start,
                window.window_start + window.window_size
            ));
            output.push_str(&format!("Position: {}\n", window.current_position));
            output.push_str(&format!(
                "Materialized: {}/{}\n",
                window.materialized_count, window.window_size
            ));
            output.push_str(&format!("Hit Rate: {:.1}%\n", window.hit_rate * 100.0));
            output.push_str(&format!("Window Moves: {}\n", window.stats.window_moves));
            output.push_str(&format!("Evictions: {}\n", window.stats.frames_evicted));
            output.push('\n');
        }

        // Evidence chain
        output.push_str("--- Evidence Chain ---\n");
        output.push_str(&format!(
            "Frame Evidence: {}\n",
            self.evidence_snapshot.frame_evidence_count
        ));
        output.push_str(&format!(
            "Session Ops: {}\n",
            self.evidence_snapshot.session_operation_count
        ));
        output.push_str(&format!(
            "Bit Offsets: {}\n",
            self.evidence_snapshot.bit_offset_evidence_count
        ));
        output.push_str(&format!(
            "Syntax Nodes: {}\n",
            self.evidence_snapshot.syntax_evidence_count
        ));
        output.push('\n');

        // Performance
        output.push_str("--- Performance ---\n");
        if let Some(duration) = self.performance_metrics.quick_index_duration_ms {
            output.push_str(&format!("Quick Index: {} ms\n", duration));
        }
        if let Some(duration) = self.performance_metrics.full_index_duration_ms {
            output.push_str(&format!("Full Index: {} ms\n", duration));
        }
        if let Some(fps) = self.performance_metrics.indexing_fps {
            output.push_str(&format!("Indexing FPS: {:.1}\n", fps));
        }
        output.push_str(&format!(
            "Est. Memory: {} KB\n",
            self.performance_metrics.estimated_memory_bytes / 1024
        ));

        output
    }

    /// Get session ID
    pub fn session_id(&self) -> &str {
        &self.session_id
    }
}

#[cfg(test)]
mod tests {
    include!("index_dev_hud_test.rs");
}
