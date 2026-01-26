//! Event - Core → UI events
//!
//! Monster Pack v3: ARCHITECTURE.md §3.3

use crate::selection::StreamId;
use std::path::PathBuf;

/// Events published by Core to UI
#[derive(Debug, Clone)]
pub enum Event {
    // Model updates
    ModelUpdated {
        kind: ModelKind,
        stream: StreamId,
    },

    SelectionUpdated {
        stream: StreamId,
    },

    // Frame decoding (Phase 2)
    FrameDecoded {
        stream: StreamId,
        frame_index: usize,
    },

    // Worker events
    WorkerProgress {
        job_id: u64,
        progress: f32,
    },
    WorkerFinished {
        job_id: u64,
    },
    WorkerError {
        job_id: u64,
        error: String,
    },

    // Diagnostics
    DiagnosticAdded {
        diagnostic: Diagnostic,
    },
    DiagnosticsCleared {
        stream: StreamId,
    },

    // Export
    ExportFinished {
        path: PathBuf,
    },
    ExportFailed {
        error: String,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelKind {
    Container,
    Units,
    Syntax,
    Timeline,
    Stats,
    Metrics,
}

/// Diagnostic (ERROR_MODEL.md §2)
/// Extended with bitvue-specific fields for superior UX
#[derive(Debug, Clone)]
pub struct Diagnostic {
    pub id: u64,
    pub severity: Severity,
    pub stream_id: StreamId,
    pub message: String,
    pub category: Category,
    pub offset_bytes: u64, // MANDATORY
    pub timestamp_ms: u64,

    // Bitvue extensions (차별화 포인트)
    /// Frame number where error occurred (for tri-sync)
    pub frame_index: Option<usize>,
    /// Repetition count (for burst detection)
    pub count: u32,
    /// Impact score 0-100 (for priority sorting)
    pub impact_score: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    Info,
    Warn,
    Error,
    Fatal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Category {
    Container,
    Bitstream,
    Decode,
    Metric,
    IO,
    Worker,
}

/// Comprehensive test suite with Arrange-Act-Assert pattern
#[cfg(test)]
mod tests {
    include!("event_test.rs");
}
