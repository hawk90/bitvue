//! Core - Central coordinator for Command/Event flow
//!
//! Monster Pack v3: ARCHITECTURE.md §3

use crate::{ByteCache, Command, Event, JobManager, SelectionState, StreamId, StreamState};
use parking_lot::RwLock;
use std::path::Path;
use std::sync::Arc;

/// Core is the central coordinator between UI and state
///
/// Responsibilities:
/// - Process Commands from UI
/// - Emit Events to UI
/// - Manage StreamState for streams A and B
/// - Coordinate JobManager for background work
///
/// Usage:
/// ```no_run
/// use bitvue_core::{Core, Command, StreamId};
/// use std::path::Path;
///
/// let core = Core::new();
///
/// // UI sends command
/// let events = core.handle_command(Command::OpenFile {
///     stream: StreamId::A,
///     path: Path::new("video.ivf").to_path_buf(),
/// });
///
/// // UI processes events
/// for event in events {
///     println!("Event: {:?}", event);
/// }
/// ```
pub struct Core {
    /// Stream A state
    stream_a: Arc<RwLock<StreamState>>,

    /// Stream B state (for dual view)
    stream_b: Arc<RwLock<StreamState>>,

    /// Current selection state
    selection: Arc<RwLock<SelectionState>>,

    /// Job manager for background work
    job_manager: Arc<JobManager>,
}

impl Core {
    /// Create a new Core instance
    pub fn new() -> Self {
        Self {
            stream_a: Arc::new(RwLock::new(StreamState::new(StreamId::A))),
            stream_b: Arc::new(RwLock::new(StreamState::new(StreamId::B))),
            selection: Arc::new(RwLock::new(SelectionState::new(StreamId::A))),
            job_manager: Arc::new(JobManager::new()),
        }
    }

    /// Handle a command from the UI
    ///
    /// Commands are processed synchronously and return a list of events.
    /// For async operations, jobs are submitted to JobManager.
    pub fn handle_command(&self, command: Command) -> Vec<Event> {
        match command {
            Command::OpenFile { stream, path } => self.handle_open_file(stream, &path),

            Command::CloseFile { stream } => self.handle_close_file(stream),

            Command::SelectFrame { stream, frame_key } => {
                let mut selection = self.selection.write();
                selection.select_point(frame_key.frame_index);
                vec![Event::SelectionUpdated { stream }]
            }

            Command::SelectUnit { stream, unit_key } => {
                let mut selection = self.selection.write();
                selection.select_unit(unit_key);
                vec![Event::SelectionUpdated { stream }]
            }

            Command::SelectSyntax {
                stream,
                node_id,
                bit_range,
            } => {
                let mut selection = self.selection.write();
                selection.select_syntax(node_id, bit_range);
                vec![Event::SelectionUpdated { stream }]
            }

            Command::SelectBitRange { stream, bit_range } => {
                // Tri-sync: Find nearest syntax node for this bit range
                let stream_state = match stream {
                    StreamId::A => &self.stream_a,
                    StreamId::B => &self.stream_b,
                };

                let state = stream_state.read();
                let nearest_node = state.syntax.as_ref().and_then(|syntax| {
                    syntax
                        .find_nearest_node(&bit_range)
                        .map(|node| node.node_id.clone())
                });
                drop(state);

                let mut selection = self.selection.write();
                selection.select_bit_range(bit_range);

                // If we found a matching syntax node, update it
                if let Some(node_id) = nearest_node {
                    selection.syntax_node = Some(node_id);
                }

                vec![Event::SelectionUpdated { stream }]
            }

            Command::SelectSpatialBlock { stream, block } => {
                let mut selection = self.selection.write();
                // Get current frame index from cursor, or default to 0
                let frame_index = selection.current_frame().unwrap_or(0);
                selection.select_spatial_block(frame_index, block);
                vec![Event::SelectionUpdated { stream }]
            }

            _ => {
                // Placeholder for other commands
                tracing::debug!("Unhandled command: {:?}", command);
                vec![]
            }
        }
    }

    /// Handle OpenFile command
    fn handle_open_file(&self, stream: StreamId, path: &Path) -> Vec<Event> {
        tracing::info!("Opening file: {:?} for stream {:?}", path, stream);

        let stream_state = match stream {
            StreamId::A => &self.stream_a,
            StreamId::B => &self.stream_b,
        };

        // Increment request ID to cancel any pending jobs
        let request_id = self.job_manager.increment_request_id(stream);
        tracing::debug!("New request_id for {:?}: {}", stream, request_id);

        // Try to create ByteCache
        let byte_cache_result = ByteCache::new(
            path,
            ByteCache::DEFAULT_SEGMENT_SIZE,
            ByteCache::DEFAULT_MAX_MEMORY,
        );

        match byte_cache_result {
            Ok(byte_cache) => {
                let byte_cache = Arc::new(byte_cache);
                let mut state = stream_state.write();
                state.file_path = Some(path.to_path_buf());
                state.byte_cache = Some(byte_cache);
                state.file_invalidated = false;
                state.clear_diagnostics();

                vec![Event::ModelUpdated {
                    kind: crate::event::ModelKind::Container,
                    stream,
                }]
            }
            Err(e) => {
                let diagnostic = crate::event::Diagnostic {
                    id: 0, // TODO: proper ID generation
                    severity: crate::event::Severity::Error,
                    stream_id: stream,
                    message: format!("Failed to open file: {}", e),
                    category: crate::event::Category::IO,
                    offset_bytes: 0,
                    timestamp_ms: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map(|d| d.as_millis() as u64)
                        .unwrap_or(0),
                    // Bitvue extensions
                    frame_index: None,
                    count: 1,
                    impact_score: 100, // File open error is critical
                };

                let mut state = stream_state.write();
                state.add_diagnostic(diagnostic.clone());

                vec![Event::DiagnosticAdded { diagnostic }]
            }
        }
    }

    /// Handle CloseFile command
    fn handle_close_file(&self, stream: StreamId) -> Vec<Event> {
        tracing::info!("Closing file for stream {:?}", stream);

        let stream_state = match stream {
            StreamId::A => &self.stream_a,
            StreamId::B => &self.stream_b,
        };

        // Increment request ID to cancel pending jobs
        self.job_manager.increment_request_id(stream);

        // Clear state
        let mut state = stream_state.write();
        *state = StreamState::new(stream);

        vec![Event::ModelUpdated {
            kind: crate::event::ModelKind::Container,
            stream,
        }]
    }

    /// Get read access to stream state
    pub fn get_stream(&self, stream: StreamId) -> Arc<RwLock<StreamState>> {
        match stream {
            StreamId::A => self.stream_a.clone(),
            StreamId::B => self.stream_b.clone(),
        }
    }

    /// Get read access to selection state
    pub fn get_selection(&self) -> Arc<RwLock<SelectionState>> {
        self.selection.clone()
    }

    /// Get read access to job manager
    pub fn get_job_manager(&self) -> Arc<JobManager> {
        self.job_manager.clone()
    }
}

impl Default for Core {
    fn default() -> Self {
        Self::new()
    }
}

/// Per generate-tests skill: Comprehensive test suite with Arrange-Act-Assert pattern
#[cfg(test)]
mod tests {
    include!("core_test.rs");
}
