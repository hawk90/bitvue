//! Event Observer - Observer pattern for event handling
//!
//! This module provides an observer pattern for distributing events
//! to multiple listeners, decoupling event producers from consumers.
//!
//! # Example
//!
//! ```ignore
//! use bitvue_core::{EventBus, LoggingObserver, SelectionChangedEvent};
//!
//! let mut bus = EventBus::new();
//! bus.subscribe(Box::new(LoggingObserver::new()));
//!
//! let event = SelectionChangedEvent { /* ... */ };
//! bus.publish(&event);
//! ```

use std::fmt;
use std::sync::{Arc, Mutex};

/// Event types
#[derive(Debug, Clone, PartialEq)]
pub enum EventType {
    /// Selection changed event
    SelectionChanged,
    /// Frame decoded event
    FrameDecoded,
    /// Parse complete event
    ParseComplete,
    /// Error event
    Error,
    /// Cache invalidated event
    CacheInvalidated,
    /// File loaded event
    FileLoaded,
    /// File closed event
    FileClosed,
}

impl fmt::Display for EventType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EventType::SelectionChanged => write!(f, "SelectionChanged"),
            EventType::FrameDecoded => write!(f, "FrameDecoded"),
            EventType::ParseComplete => write!(f, "ParseComplete"),
            EventType::Error => write!(f, "Error"),
            EventType::CacheInvalidated => write!(f, "CacheInvalidated"),
            EventType::FileLoaded => write!(f, "FileLoaded"),
            EventType::FileClosed => write!(f, "FileClosed"),
        }
    }
}

/// Base event trait (renamed to avoid conflict with existing Event enum)
pub trait ObservableEvent: Send + Sync {
    /// Get the event type
    fn event_type(&self) -> EventType;

    /// Get the event timestamp
    fn timestamp(&self) -> i64;

    /// Convert to Any for downcasting
    fn as_any(&self) -> &dyn std::any::Any;
}

// =============================================================================
// Selection Changed Event
// =============================================================================

/// Selection changed event data
#[derive(Debug, Clone)]
pub struct SelectionChangedEvent {
    /// Event timestamp
    pub timestamp: i64,
    /// Stream ID
    pub stream_id: String,
    /// Previous selection (if any)
    pub previous_selection: Option<String>,
    /// New selection
    pub new_selection: String,
    /// Selection type (frame, unit, syntax, etc.)
    pub selection_type: String,
}

impl ObservableEvent for SelectionChangedEvent {
    fn event_type(&self) -> EventType {
        EventType::SelectionChanged
    }

    fn timestamp(&self) -> i64 {
        self.timestamp
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

// =============================================================================
// Frame Decoded Event
// =============================================================================

/// Frame decoded event data
#[derive(Debug, Clone)]
pub struct FrameDecodedEvent {
    /// Event timestamp
    pub timestamp: i64,
    /// Stream ID
    pub stream_id: String,
    /// Frame index
    pub frame_index: usize,
    /// Frame type (I, P, B, etc.)
    pub frame_type: String,
    /// Decoding time in milliseconds
    pub decode_time_ms: u64,
    /// Frame size in bytes
    pub frame_size: usize,
}

impl ObservableEvent for FrameDecodedEvent {
    fn event_type(&self) -> EventType {
        EventType::FrameDecoded
    }

    fn timestamp(&self) -> i64 {
        self.timestamp
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

// =============================================================================
// Parse Complete Event
// =============================================================================

/// Parse complete event data
#[derive(Debug, Clone)]
pub struct ParseCompleteEvent {
    /// Event timestamp
    pub timestamp: i64,
    /// Stream ID
    pub stream_id: String,
    /// Total frames parsed
    pub total_frames: usize,
    /// Parse duration in milliseconds
    pub parse_duration_ms: u64,
    /// Success flag
    pub success: bool,
    /// Error message (if failed)
    pub error_message: Option<String>,
}

impl ObservableEvent for ParseCompleteEvent {
    fn event_type(&self) -> EventType {
        EventType::ParseComplete
    }

    fn timestamp(&self) -> i64 {
        self.timestamp
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

// =============================================================================
// Error Event
// =============================================================================

/// Error event data
#[derive(Debug, Clone)]
pub struct ErrorEvent {
    /// Event timestamp
    pub timestamp: i64,
    /// Error severity
    pub severity: ErrorSeverity,
    /// Error code
    pub code: String,
    /// Error message
    pub message: String,
    /// Context information
    pub context: std::collections::HashMap<String, String>,
}

/// Error severity levels
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorSeverity {
    /// Info level (not critical)
    Info,
    /// Warning level (potential issue)
    Warning,
    /// Error level (operation failed)
    Error,
    /// Fatal level (application cannot continue)
    Fatal,
}

impl ObservableEvent for ErrorEvent {
    fn event_type(&self) -> EventType {
        EventType::Error
    }

    fn timestamp(&self) -> i64 {
        self.timestamp
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

// =============================================================================
// Cache Invalidated Event
// =============================================================================

/// Cache invalidated event data
#[derive(Debug, Clone)]
pub struct CacheInvalidatedEvent {
    /// Event timestamp
    pub timestamp: i64,
    /// Cache key that was invalidated
    pub cache_key: String,
    /// Invalidated entry count
    pub entry_count: usize,
    /// Reason for invalidation
    pub reason: String,
}

impl ObservableEvent for CacheInvalidatedEvent {
    fn event_type(&self) -> EventType {
        EventType::CacheInvalidated
    }

    fn timestamp(&self) -> i64 {
        self.timestamp
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

// =============================================================================
// File Loaded Event
// =============================================================================

/// File loaded event data
#[derive(Debug, Clone)]
pub struct FileLoadedEvent {
    /// Event timestamp
    pub timestamp: i64,
    /// File path
    pub file_path: std::path::PathBuf,
    /// File size in bytes
    pub file_size: u64,
    /// Stream ID
    pub stream_id: String,
    /// Codec type
    pub codec: String,
}

impl ObservableEvent for FileLoadedEvent {
    fn event_type(&self) -> EventType {
        EventType::FileLoaded
    }

    fn timestamp(&self) -> i64 {
        self.timestamp
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

// =============================================================================
// File Closed Event
// =============================================================================

/// File closed event data
#[derive(Debug, Clone)]
pub struct FileClosedEvent {
    /// Event timestamp
    pub timestamp: i64,
    /// Stream ID
    pub stream_id: String,
}

impl ObservableEvent for FileClosedEvent {
    fn event_type(&self) -> EventType {
        EventType::FileClosed
    }

    fn timestamp(&self) -> i64 {
        self.timestamp
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

// =============================================================================
// Event Observer Trait
// =============================================================================

/// Trait for event observers
///
/// Observers subscribe to events and implement custom handling logic.
pub trait EventObserver: Send + Sync {
    /// Called when selection changes
    fn on_selection_changed(&self, event: &SelectionChangedEvent);

    /// Called when a frame is decoded
    fn on_frame_decoded(&self, event: &FrameDecodedEvent);

    /// Called when parsing completes
    fn on_parse_complete(&self, event: &ParseCompleteEvent);

    /// Called when an error occurs
    fn on_error(&self, event: &ErrorEvent);

    /// Called when cache is invalidated
    fn on_cache_invalidated(&self, event: &CacheInvalidatedEvent);

    /// Called when a file is loaded
    fn on_file_loaded(&self, event: &FileLoadedEvent);

    /// Called when a file is closed
    fn on_file_closed(&self, event: &FileClosedEvent);

    /// Get observer name
    fn name(&self) -> &str {
        "EventObserver"
    }
}

// =============================================================================
// Logging Observer
// =============================================================================

/// Logging observer that logs all events
#[derive(Debug)]
pub struct LoggingObserver {
    name: String,
}

impl LoggingObserver {
    /// Create a new logging observer
    pub fn new() -> Self {
        Self {
            name: "LoggingObserver".to_string(),
        }
    }

    /// Create with custom name
    pub fn with_name(name: impl Into<String>) -> Self {
        Self { name: name.into() }
    }

    /// Log an event
    fn log_event(&self, event_type: &str, details: &str) {
        tracing::info!("[{}] {}", event_type, details);
    }
}

impl Default for LoggingObserver {
    fn default() -> Self {
        Self::new()
    }
}

impl EventObserver for LoggingObserver {
    fn on_selection_changed(&self, event: &SelectionChangedEvent) {
        self.log_event(
            "SelectionChanged",
            &format!(
                "stream={}, selection={}, type={}",
                event.stream_id, event.new_selection, event.selection_type
            ),
        );
    }

    fn on_frame_decoded(&self, event: &FrameDecodedEvent) {
        self.log_event(
            "FrameDecoded",
            &format!(
                "stream={}, frame={}, type={}, time={}ms",
                event.stream_id, event.frame_index, event.frame_type, event.decode_time_ms
            ),
        );
    }

    fn on_parse_complete(&self, event: &ParseCompleteEvent) {
        self.log_event(
            "ParseComplete",
            &format!(
                "stream={}, frames={}, time={}ms, success={}",
                event.stream_id, event.total_frames, event.parse_duration_ms, event.success
            ),
        );
    }

    fn on_error(&self, event: &ErrorEvent) {
        self.log_event(
            "Error",
            &format!(
                "severity={}, code={}, message={}",
                event.severity as u8, event.code, event.message
            ),
        );
    }

    fn on_cache_invalidated(&self, event: &CacheInvalidatedEvent) {
        self.log_event(
            "CacheInvalidated",
            &format!(
                "key={}, entries={}, reason={}",
                event.cache_key, event.entry_count, event.reason
            ),
        );
    }

    fn on_file_loaded(&self, event: &FileLoadedEvent) {
        self.log_event(
            "FileLoaded",
            &format!(
                "path={}, size={}KB, stream={}, codec={}",
                event.file_path.display(),
                event.file_size / 1024,
                event.stream_id,
                event.codec
            ),
        );
    }

    fn on_file_closed(&self, event: &FileClosedEvent) {
        self.log_event("FileClosed", &format!("stream={}", event.stream_id));
    }

    fn name(&self) -> &str {
        &self.name
    }
}

// =============================================================================
// History Observer
// =============================================================================

/// History observer that tracks event history
#[derive(Debug)]
pub struct HistoryObserver {
    name: String,
    history: Arc<Mutex<Vec<HistoryEntry>>>,
    max_entries: usize,
}

/// Entry in event history
#[derive(Debug, Clone)]
pub struct HistoryEntry {
    /// Event type
    pub event_type: EventType,
    /// Timestamp
    pub timestamp: i64,
    /// Event details
    pub details: String,
}

impl HistoryObserver {
    /// Create a new history observer
    pub fn new() -> Self {
        Self {
            name: "HistoryObserver".to_string(),
            history: Arc::new(Mutex::new(Vec::new())),
            max_entries: 1000,
        }
    }

    /// Create with custom max entries
    pub fn with_max_entries(mut self, max: usize) -> Self {
        self.max_entries = max;
        self
    }

    /// Add entry to history
    fn add_entry(&self, event_type: EventType, timestamp: i64, details: String) {
        let mut history = self.history.lock().unwrap();
        history.push(HistoryEntry {
            event_type,
            timestamp,
            details,
        });

        // Trim if necessary
        if history.len() > self.max_entries {
            history.remove(0);
        }
    }

    /// Get event history
    pub fn history(&self) -> Vec<HistoryEntry> {
        let history = self.history.lock().unwrap();
        history.clone()
    }

    /// Get last N entries
    pub fn last_entries(&self, count: usize) -> Vec<HistoryEntry> {
        let history = self.history.lock().unwrap();
        let len = history.len();
        let start = len.saturating_sub(count);
        history[start..].to_vec()
    }

    /// Clear history
    pub fn clear(&self) {
        let mut history = self.history.lock().unwrap();
        history.clear();
    }

    /// Get history length
    pub fn len(&self) -> usize {
        let history = self.history.lock().unwrap();
        history.len()
    }

    /// Check if history is empty
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl Default for HistoryObserver {
    fn default() -> Self {
        Self::new()
    }
}

impl EventObserver for HistoryObserver {
    fn on_selection_changed(&self, event: &SelectionChangedEvent) {
        self.add_entry(
            EventType::SelectionChanged,
            event.timestamp,
            format!(
                "Selected {} in stream {}",
                event.new_selection, event.stream_id
            ),
        );
    }

    fn on_frame_decoded(&self, event: &FrameDecodedEvent) {
        self.add_entry(
            EventType::FrameDecoded,
            event.timestamp,
            format!(
                "Decoded frame {} in stream {} ({} type)",
                event.frame_index, event.stream_id, event.frame_type
            ),
        );
    }

    fn on_parse_complete(&self, event: &ParseCompleteEvent) {
        self.add_entry(
            EventType::ParseComplete,
            event.timestamp,
            format!(
                "Parsed {} frames from stream {} in {}ms",
                event.total_frames, event.stream_id, event.parse_duration_ms
            ),
        );
    }

    fn on_error(&self, event: &ErrorEvent) {
        self.add_entry(
            EventType::Error,
            event.timestamp,
            format!("Error [{}]: {}", event.code, event.message),
        );
    }

    fn on_cache_invalidated(&self, event: &CacheInvalidatedEvent) {
        self.add_entry(
            EventType::CacheInvalidated,
            event.timestamp,
            format!("Invalidated cache key: {}", event.cache_key),
        );
    }

    fn on_file_loaded(&self, event: &FileLoadedEvent) {
        self.add_entry(
            EventType::FileLoaded,
            event.timestamp,
            format!("Loaded file: {}", event.file_path.display()),
        );
    }

    fn on_file_closed(&self, event: &FileClosedEvent) {
        self.add_entry(
            EventType::FileClosed,
            event.timestamp,
            format!("Closed stream: {}", event.stream_id),
        );
    }

    fn name(&self) -> &str {
        &self.name
    }
}

// =============================================================================
// Cache Invalidation Observer
// =============================================================================

/// Cache invalidation observer that automatically invalidates cache
pub struct CacheInvalidationObserver {
    name: String,
    /// Callback to execute on cache invalidation
    on_invalidate_callback: Arc<dyn Fn(&str) + Send + Sync>,
}

impl CacheInvalidationObserver {
    /// Create a new cache invalidation observer
    pub fn new(on_invalidate: Arc<dyn Fn(&str) + Send + Sync>) -> Self {
        Self {
            name: "CacheInvalidationObserver".to_string(),
            on_invalidate_callback: on_invalidate,
        }
    }
}

impl EventObserver for CacheInvalidationObserver {
    fn on_selection_changed(&self, _event: &SelectionChangedEvent) {
        // No-op
    }

    fn on_frame_decoded(&self, _event: &FrameDecodedEvent) {
        // No-op
    }

    fn on_parse_complete(&self, _event: &ParseCompleteEvent) {
        // No-op
    }

    fn on_error(&self, _event: &ErrorEvent) {
        // No-op
    }

    fn on_cache_invalidated(&self, event: &CacheInvalidatedEvent) {
        (self.on_invalidate_callback)(&event.cache_key);
    }

    fn on_file_loaded(&self, _event: &FileLoadedEvent) {
        // No-op
    }

    fn on_file_closed(&self, _event: &FileClosedEvent) {
        // No-op
    }

    fn name(&self) -> &str {
        &self.name
    }
}

// =============================================================================
// Analytics Observer
// =============================================================================

/// Analytics observer that tracks metrics
#[derive(Debug)]
pub struct AnalyticsObserver {
    name: String,
    metrics: Arc<Mutex<AnalyticsMetrics>>,
}

/// Metrics tracked by analytics observer
#[derive(Debug, Default, Clone)]
pub struct AnalyticsMetrics {
    /// Total frames decoded
    pub total_frames_decoded: u64,
    /// Total parse time in milliseconds
    pub total_parse_time_ms: u64,
    /// Total errors
    pub total_errors: u64,
    /// Total cache invalidations
    pub total_cache_invalidations: u64,
    /// Files loaded
    pub files_loaded: u64,
    /// Files closed
    pub files_closed: u64,
}

impl AnalyticsObserver {
    /// Create a new analytics observer
    pub fn new() -> Self {
        Self {
            name: "AnalyticsObserver".to_string(),
            metrics: Arc::new(Mutex::new(AnalyticsMetrics::default())),
        }
    }

    /// Get current metrics
    pub fn metrics(&self) -> AnalyticsMetrics {
        let metrics = self.metrics.lock().unwrap();
        metrics.clone()
    }

    /// Reset metrics
    pub fn reset(&self) {
        let mut metrics = self.metrics.lock().unwrap();
        *metrics = AnalyticsMetrics::default();
    }
}

impl Default for AnalyticsObserver {
    fn default() -> Self {
        Self::new()
    }
}

impl EventObserver for AnalyticsObserver {
    fn on_selection_changed(&self, _event: &SelectionChangedEvent) {
        // No-op (selections are not tracked in basic metrics)
    }

    fn on_frame_decoded(&self, _event: &FrameDecodedEvent) {
        let mut metrics = self.metrics.lock().unwrap();
        metrics.total_frames_decoded += 1;
    }

    fn on_parse_complete(&self, event: &ParseCompleteEvent) {
        let mut metrics = self.metrics.lock().unwrap();
        metrics.total_parse_time_ms += event.parse_duration_ms;
    }

    fn on_error(&self, _event: &ErrorEvent) {
        let mut metrics = self.metrics.lock().unwrap();
        metrics.total_errors += 1;
    }

    fn on_cache_invalidated(&self, event: &CacheInvalidatedEvent) {
        let mut metrics = self.metrics.lock().unwrap();
        metrics.total_cache_invalidations += 1;
        metrics.total_cache_invalidations += event.entry_count as u64;
    }

    fn on_file_loaded(&self, _event: &FileLoadedEvent) {
        let mut metrics = self.metrics.lock().unwrap();
        metrics.files_loaded += 1;
    }

    fn on_file_closed(&self, _event: &FileClosedEvent) {
        let mut metrics = self.metrics.lock().unwrap();
        metrics.files_closed += 1;
    }

    fn name(&self) -> &str {
        &self.name
    }
}

// =============================================================================
// Event Bus
// =============================================================================

/// Event bus for publishing events to observers
#[derive(Clone)] // Removed Debug due to trait object
pub struct EventBus {
    observers: Arc<Mutex<Vec<Box<dyn EventObserver>>>>,
}

impl fmt::Debug for EventBus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("EventBus")
            .field("observer_count", &self.observers.lock().unwrap().len())
            .finish()
    }
}

impl EventBus {
    /// Create a new event bus
    pub fn new() -> Self {
        Self {
            observers: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Subscribe an observer to events
    pub fn subscribe(&self, observer: Box<dyn EventObserver>) {
        let mut observers = self.observers.lock().unwrap();
        tracing::info!("EventBus: Subscribed observer '{}'", observer.name());
        observers.push(observer);
    }

    /// Publish an event to all observers
    pub fn publish(&self, event: &dyn ObservableEvent) {
        let observers = self.observers.lock().unwrap();

        // Convert to concrete event types
        if let Some(sel_event) = event.as_any().downcast_ref::<SelectionChangedEvent>() {
            for observer in observers.iter() {
                observer.on_selection_changed(sel_event);
            }
        } else if let Some(frame_event) = event.as_any().downcast_ref::<FrameDecodedEvent>() {
            for observer in observers.iter() {
                observer.on_frame_decoded(frame_event);
            }
        } else if let Some(parse_event) = event.as_any().downcast_ref::<ParseCompleteEvent>() {
            for observer in observers.iter() {
                observer.on_parse_complete(parse_event);
            }
        } else if let Some(err_event) = event.as_any().downcast_ref::<ErrorEvent>() {
            for observer in observers.iter() {
                observer.on_error(err_event);
            }
        } else if let Some(cache_event) = event.as_any().downcast_ref::<CacheInvalidatedEvent>() {
            for observer in observers.iter() {
                observer.on_cache_invalidated(cache_event);
            }
        } else if let Some(load_event) = event.as_any().downcast_ref::<FileLoadedEvent>() {
            for observer in observers.iter() {
                observer.on_file_loaded(load_event);
            }
        } else if let Some(close_event) = event.as_any().downcast_ref::<FileClosedEvent>() {
            for observer in observers.iter() {
                observer.on_file_closed(close_event);
            }
        }
    }

    /// Get observer count
    pub fn observer_count(&self) -> usize {
        let observers = self.observers.lock().unwrap();
        observers.len()
    }

    /// Remove all observers
    pub fn clear(&self) {
        let mut observers = self.observers.lock().unwrap();
        observers.clear();
    }

    /// Remove observers by name
    pub fn remove_by_name(&self, name: &str) {
        let mut observers = self.observers.lock().unwrap();
        observers.retain(|obs| obs.name() != name);
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// Global Event Bus
// =============================================================================

/// Get the global event bus instance
pub fn global_event_bus() -> &'static Mutex<EventBus> {
    use std::sync::OnceLock;
    static BUS: OnceLock<Mutex<EventBus>> = OnceLock::new();
    BUS.get_or_init(|| {
        let bus = EventBus::new();
        // Add default observers
        bus.subscribe(Box::new(LoggingObserver::new()));
        bus.subscribe(Box::new(HistoryObserver::new()));
        Mutex::new(bus)
    })
}

// =============================================================================
// Helper Functions
// =============================================================================

/// Get current timestamp in milliseconds
pub fn current_timestamp() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis()
        .try_into()
        .unwrap_or(0)
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_type_display() {
        assert_eq!(EventType::SelectionChanged.to_string(), "SelectionChanged");
        assert_eq!(EventType::FrameDecoded.to_string(), "FrameDecoded");
        assert_eq!(EventType::ParseComplete.to_string(), "ParseComplete");
    }

    #[test]
    fn test_event_bus_subscribe() {
        let bus = EventBus::new();
        assert_eq!(bus.observer_count(), 0);

        bus.subscribe(Box::new(LoggingObserver::new()));
        assert_eq!(bus.observer_count(), 1);
    }

    #[test]
    fn test_event_bus_publish() {
        let bus = EventBus::new();
        let _history = HistoryObserver::new();

        // Subscribe a second observer that we can inspect
        bus.subscribe(Box::new(LoggingObserver::new()));

        // Publish selection changed event
        let event = SelectionChangedEvent {
            timestamp: current_timestamp(),
            stream_id: "A".to_string(),
            previous_selection: None,
            new_selection: "frame_42".to_string(),
            selection_type: "frame".to_string(),
        };

        // The event is published to all observers
        // (we can't easily test this without capturing log output)
        bus.publish(&event);
    }

    #[test]
    fn test_history_observer() {
        let observer = HistoryObserver::new();

        assert_eq!(observer.len(), 0);

        let event = SelectionChangedEvent {
            timestamp: current_timestamp(),
            stream_id: "A".to_string(),
            previous_selection: None,
            new_selection: "frame_42".to_string(),
            selection_type: "frame".to_string(),
        };

        observer.on_selection_changed(&event);

        assert_eq!(observer.len(), 1);

        let history = observer.history();
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].event_type, EventType::SelectionChanged);
    }

    #[test]
    fn test_history_observer_max_entries() {
        let observer = HistoryObserver::new().with_max_entries(5);

        for i in 0..10 {
            let event = SelectionChangedEvent {
                timestamp: current_timestamp(),
                stream_id: "A".to_string(),
                previous_selection: None,
                new_selection: format!("frame_{}", i),
                selection_type: "frame".to_string(),
            };
            observer.on_selection_changed(&event);
        }

        assert_eq!(observer.len(), 5);
    }

    #[test]
    fn test_analytics_observer() {
        let observer = AnalyticsObserver::new();

        let frame_event = FrameDecodedEvent {
            timestamp: current_timestamp(),
            stream_id: "A".to_string(),
            frame_index: 0,
            frame_type: "I".to_string(),
            decode_time_ms: 10,
            frame_size: 1000,
        };

        observer.on_frame_decoded(&frame_event);
        observer.on_frame_decoded(&frame_event);

        let metrics = observer.metrics();
        assert_eq!(metrics.total_frames_decoded, 2);
    }

    #[test]
    fn test_remove_observer_by_name() {
        let bus = EventBus::new();

        bus.subscribe(Box::new(LoggingObserver::with_name("Logger1")));
        bus.subscribe(Box::new(LoggingObserver::with_name("Logger2")));

        assert_eq!(bus.observer_count(), 2);

        bus.remove_by_name("Logger1");

        assert_eq!(bus.observer_count(), 1);
    }

    #[test]
    fn test_clear_observers() {
        let bus = EventBus::new();

        bus.subscribe(Box::new(LoggingObserver::new()));
        bus.subscribe(Box::new(HistoryObserver::new()));

        assert_eq!(bus.observer_count(), 2);

        bus.clear();

        assert_eq!(bus.observer_count(), 0);
    }
}
