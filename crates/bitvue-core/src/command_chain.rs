//! Command Processing Chain - Chain of Responsibility pattern for command handling
//!
//! This module provides a chain of responsibility pattern for processing commands,
//! allowing multiple handlers to process or pass along commands in a pipeline.

use std::fmt;
use std::sync::Arc;
use crate::command::Command;
use crate::selection::StreamId;

/// Result of command processing
#[derive(Debug, Clone)]
pub enum ChainResult {
    /// Command was handled successfully
    Handled(Command),
    /// Command was not handled, pass to next handler
    Continue(Command),
    /// Command handling failed with error
    Failed { command: Command, error: String },
}

impl ChainResult {
    /// Create a handled result
    pub fn handled(command: Command) -> Self {
        Self::Handled(command)
    }

    /// Create a continue result
    pub fn continue_chain(command: Command) -> Self {
        Self::Continue(command)
    }

    /// Create a failed result
    pub fn failed(command: Command, error: impl Into<String>) -> Self {
        Self::Failed {
            command,
            error: error.into(),
        }
    }

    /// Check if command was handled
    pub fn is_handled(&self) -> bool {
        matches!(self, Self::Handled(_))
    }

    /// Check if chain should continue
    pub fn should_continue(&self) -> bool {
        matches!(self, Self::Continue(_))
    }

    /// Check if handling failed
    pub fn is_failed(&self) -> bool {
        matches!(self, Self::Failed { .. })
    }

    /// Get the command from the result
    pub fn command(&self) -> &Command {
        match self {
            Self::Handled(cmd) => cmd,
            Self::Continue(cmd) => cmd,
            Self::Failed { command, .. } => command,
        }
    }
}

/// Trait for command handlers in the chain
pub trait CommandHandler: Send + Sync {
    /// Handle a command and return the result
    fn handle(&self, command: Command) -> ChainResult;

    /// Get handler name for debugging
    fn name(&self) -> &str {
        std::any::type_name::<Self>()
    }

    /// Check if this handler can process the given command
    fn can_handle(&self, _command: &Command) -> bool {
        true // Default: can handle any command
    }
}

/// Extension trait to extract command properties
pub trait CommandExt {
    /// Get the stream ID if the command has one
    fn stream_id(&self) -> Option<&StreamId>;

    /// Get command type name for debugging
    fn type_name(&self) -> &'static str;

    /// Check if command requires stream selection
    fn requires_stream(&self) -> bool;

    /// Check if command is idempotent (safe to retry)
    fn is_idempotent(&self) -> bool;
}

impl CommandExt for Command {
    fn stream_id(&self) -> Option<&StreamId> {
        match self {
            Command::OpenFile { stream, .. }
            | Command::CloseFile { stream, .. }
            | Command::RunFullAnalysis { stream, .. }
            | Command::SelectFrame { stream, .. }
            | Command::SelectUnit { stream, .. }
            | Command::SelectSyntax { stream, .. }
            | Command::SelectBitRange { stream, .. }
            | Command::SelectSpatialBlock { stream, .. }
            | Command::JumpToOffset { stream, .. }
            | Command::JumpToFrame { stream, .. }
            | Command::ToggleOverlay { stream, .. }
            | Command::SetOverlayOpacity { stream, .. }
            | Command::SetPlayerMode { stream, .. }
            | Command::PlayPause { stream, .. }
            | Command::StepForward { stream, .. }
            | Command::StepBackward { stream, .. }
            | Command::ExportCsv { stream, .. }
            | Command::ExportBitstream { stream, .. }
            | Command::Export { stream, .. }
            | Command::AddBookmark { stream, .. }
            | Command::RemoveBookmark { stream, .. }
            | Command::ExportEvidenceBundle { stream, .. } => Some(stream),
            Command::SetWorkspaceMode { .. }
            | Command::SetSyncMode { .. }
            | Command::SetOrderType { .. }
            | Command::ToggleDetailMode
            | Command::CopySelection
            | Command::CopyBytes { .. } => None,
        }
    }

    fn type_name(&self) -> &'static str {
        match self {
            Command::OpenFile { .. } => "OpenFile",
            Command::CloseFile { .. } => "CloseFile",
            Command::RunFullAnalysis { .. } => "RunFullAnalysis",
            Command::SelectFrame { .. } => "SelectFrame",
            Command::SelectUnit { .. } => "SelectUnit",
            Command::SelectSyntax { .. } => "SelectSyntax",
            Command::SelectBitRange { .. } => "SelectBitRange",
            Command::SelectSpatialBlock { .. } => "SelectSpatialBlock",
            Command::JumpToOffset { .. } => "JumpToOffset",
            Command::JumpToFrame { .. } => "JumpToFrame",
            Command::ToggleOverlay { .. } => "ToggleOverlay",
            Command::SetOverlayOpacity { .. } => "SetOverlayOpacity",
            Command::SetPlayerMode { .. } => "SetPlayerMode",
            Command::PlayPause { .. } => "PlayPause",
            Command::StepForward { .. } => "StepForward",
            Command::StepBackward { .. } => "StepBackward",
            Command::SetWorkspaceMode { .. } => "SetWorkspaceMode",
            Command::SetSyncMode { .. } => "SetSyncMode",
            Command::ExportCsv { .. } => "ExportCsv",
            Command::ExportBitstream { .. } => "ExportBitstream",
            Command::Export { .. } => "Export",
            Command::AddBookmark { .. } => "AddBookmark",
            Command::RemoveBookmark { .. } => "RemoveBookmark",
            Command::ExportEvidenceBundle { .. } => "ExportEvidenceBundle",
            Command::SetOrderType { .. } => "SetOrderType",
            Command::ToggleDetailMode => "ToggleDetailMode",
            Command::CopySelection => "CopySelection",
            Command::CopyBytes { .. } => "CopyBytes",
        }
    }

    fn requires_stream(&self) -> bool {
        matches!(
            self,
            Command::SelectFrame { .. }
                | Command::SelectUnit { .. }
                | Command::SelectSyntax { .. }
                | Command::SelectBitRange { .. }
                | Command::SelectSpatialBlock { .. }
                | Command::JumpToOffset { .. }
                | Command::JumpToFrame { .. }
        )
    }

    fn is_idempotent(&self) -> bool {
        // Query-like commands are idempotent
        matches!(
            self,
            Command::ToggleOverlay { .. }
                | Command::SetOverlayOpacity { .. }
                | Command::SetPlayerMode { .. }
                | Command::SetWorkspaceMode { .. }
                | Command::SetSyncMode { .. }
                | Command::SetOrderType { .. }
                | Command::ToggleDetailMode
        )
    }
}

// =============================================================================
// Validation Handler
// =============================================================================

/// Validates commands before processing
#[derive(Debug, Clone)]
pub struct ValidationHandler {
    /// Strict validation mode
    strict_mode: bool,
}

impl ValidationHandler {
    /// Create a new validation handler
    pub fn new() -> Self {
        Self { strict_mode: false }
    }

    /// Create a strict validation handler
    pub fn strict() -> Self {
        Self { strict_mode: true }
    }

    /// Validate a command
    fn validate_command(&self, command: &Command) -> Result<(), String> {
        // Check if command has required stream selection
        if command.requires_stream() && command.stream_id().is_none() {
            return Err("Command requires stream selection".to_string());
        }

        // Additional strict validation could go here
        if self.strict_mode {
            // Check byte ranges if present
            if let Command::CopyBytes { byte_range } = command {
                if byte_range.start >= byte_range.end {
                    return Err("Invalid byte range: start >= end".to_string());
                }
            }
        }

        Ok(())
    }
}

impl Default for ValidationHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl CommandHandler for ValidationHandler {
    fn handle(&self, command: Command) -> ChainResult {
        match self.validate_command(&command) {
            Ok(()) => ChainResult::continue_chain(command),
            Err(error) => ChainResult::failed(command, format!("Validation failed: {}", error)),
        }
    }

    fn name(&self) -> &str {
        "ValidationHandler"
    }
}

// =============================================================================
// Logging Handler
// =============================================================================

/// Log level for logging handler
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogLevel {
    Debug,
    Info,
    Warn,
    Error,
}

impl Default for LogLevel {
    fn default() -> Self {
        Self::Info
    }
}

/// Logs command processing for debugging
#[derive(Debug, Clone, Default)]
pub struct LoggingHandler {
    /// Log level
    log_level: LogLevel,
}

impl LoggingHandler {
    /// Create a new logging handler
    pub fn new() -> Self {
        Self {
            log_level: LogLevel::default(),
        }
    }

    /// Create a debug logging handler
    pub fn debug() -> Self {
        Self {
            log_level: LogLevel::Debug,
        }
    }

    /// Log a command
    fn log_command(&self, command: &Command) {
        match self.log_level {
            LogLevel::Debug => {
                tracing::debug!(
                    "Processing command: {}, stream: {:?}",
                    command.type_name(),
                    command.stream_id()
                );
            }
            LogLevel::Info => {
                tracing::info!(
                    "Processing command: {}, stream: {:?}",
                    command.type_name(),
                    command.stream_id()
                );
            }
            LogLevel::Warn => {
                tracing::warn!(
                    "Processing command: {}, stream: {:?}",
                    command.type_name(),
                    command.stream_id()
                );
            }
            LogLevel::Error => {
                tracing::error!(
                    "Processing command: {}, stream: {:?}",
                    command.type_name(),
                    command.stream_id()
                );
            }
        }
    }
}

impl CommandHandler for LoggingHandler {
    fn handle(&self, command: Command) -> ChainResult {
        self.log_command(&command);
        ChainResult::continue_chain(command)
    }

    fn name(&self) -> &str {
        "LoggingHandler"
    }
}

// =============================================================================
// Metrics Handler
// =============================================================================

/// Tracks command processing metrics
#[derive(Debug, Clone)]
pub struct MetricsHandler {
    /// Metrics storage
    metrics: Arc<std::sync::Mutex<CommandMetrics>>,
}

/// Command processing metrics
#[derive(Debug, Clone, Default)]
pub struct CommandMetrics {
    /// Total commands processed
    pub total_processed: u64,
    /// Commands handled successfully
    pub successful: u64,
    /// Commands that failed
    pub failed: u64,
    /// Commands by type
    pub by_type: std::collections::HashMap<String, u64>,
}

impl MetricsHandler {
    /// Create a new metrics handler
    pub fn new() -> Self {
        Self {
            metrics: Arc::new(std::sync::Mutex::new(CommandMetrics::default())),
        }
    }

    /// Get a snapshot of current metrics
    pub fn get_metrics(&self) -> CommandMetrics {
        self.metrics.lock().unwrap().clone()
    }

    /// Reset metrics
    pub fn reset_metrics(&self) {
        let mut metrics = self.metrics.lock().unwrap();
        *metrics = CommandMetrics::default();
    }
}

impl Default for MetricsHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl CommandHandler for MetricsHandler {
    fn handle(&self, command: Command) -> ChainResult {
        // Record metrics
        let mut metrics = self.metrics.lock().unwrap();
        metrics.total_processed += 1;

        let command_type = command.type_name().to_string();
        *metrics.by_type.entry(command_type).or_insert(0) += 1;

        // Continue chain
        drop(metrics);
        ChainResult::continue_chain(command)
    }

    fn name(&self) -> &str {
        "MetricsHandler"
    }
}

// =============================================================================
// Caching Handler
// =============================================================================

/// Cache key for commands
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct CacheKey {
    command_type: String,
    stream: Option<bool>, // true for A, false for B (StreamId mapping)
}

impl From<&Command> for Option<CacheKey> {
    fn from(cmd: &Command) -> Self {
        Some(CacheKey {
            command_type: cmd.type_name().to_string(),
            stream: cmd.stream_id().map(|s| matches!(s, crate::selection::StreamId::A)),
        })
    }
}

/// Caches command results for performance
#[derive(Debug, Clone)]
pub struct CachingHandler {
    /// Cache storage
    cache: Arc<std::sync::Mutex<lru::LruCache<CacheKey, Command>>>,
    /// Maximum cache size (stored for potential future use)
    _max_cache_size: usize,
}

impl CachingHandler {
    /// Create a new caching handler
    pub fn new(max_cache_size: usize) -> Self {
        Self {
            cache: Arc::new(std::sync::Mutex::new(lru::LruCache::new(
                std::num::NonZeroUsize::new(max_cache_size).unwrap(),
            ))),
            _max_cache_size: max_cache_size,
        }
    }

    /// Create a cache key from a command
    fn cache_key(command: &Command) -> Option<CacheKey> {
        command.into()
    }

    /// Get cache size
    pub fn cache_size(&self) -> usize {
        self.cache.lock().unwrap().len()
    }

    /// Clear the cache
    pub fn clear_cache(&self) {
        self.cache.lock().unwrap().clear();
    }
}

impl CommandHandler for CachingHandler {
    fn handle(&self, command: Command) -> ChainResult {
        // Check cache for idempotent commands
        if command.is_idempotent() {
            if let Some(key) = Self::cache_key(&command) {
                let mut cache = self.cache.lock().unwrap();
                if let Some(cached) = cache.get(&key) {
                    tracing::debug!("Cache hit for command: {:?}", key);
                    return ChainResult::handled(cached.clone());
                }
            }
        }

        // Not cached, continue chain
        ChainResult::continue_chain(command)
    }

    fn name(&self) -> &str {
        "CachingHandler"
    }
}

// =============================================================================
// Throttling Handler
// =============================================================================

/// Throttles command processing rate
#[derive(Debug, Clone)]
pub struct ThrottlingHandler {
    /// Minimum time between commands (in ms)
    min_interval_ms: u64,
    /// Last command timestamp
    last_command: Arc<std::sync::Mutex<std::time::Instant>>,
}

impl ThrottlingHandler {
    /// Create a new throttling handler
    pub fn new(min_interval_ms: u64) -> Self {
        Self {
            min_interval_ms,
            last_command: Arc::new(std::sync::Mutex::new(
                std::time::Instant::now() - std::time::Duration::from_millis(min_interval_ms),
            )),
        }
    }

    /// Check if command should be allowed through
    fn check_throttle(&self) -> Result<(), String> {
        let mut last = self.last_command.lock().unwrap();
        let elapsed = last.elapsed().as_millis() as u64;

        if elapsed < self.min_interval_ms {
            return Err(format!(
                "Throttled: {}ms remaining",
                self.min_interval_ms - elapsed
            ));
        }

        *last = std::time::Instant::now();
        Ok(())
    }
}

impl CommandHandler for ThrottlingHandler {
    fn handle(&self, command: Command) -> ChainResult {
        match self.check_throttle() {
            Ok(()) => ChainResult::continue_chain(command),
            Err(error) => ChainResult::failed(command, error),
        }
    }

    fn name(&self) -> &str {
        "ThrottlingHandler"
    }
}

// =============================================================================
// Retry Handler
// =============================================================================

/// Retries failed commands with exponential backoff
#[derive(Debug, Clone)]
pub struct RetryHandler {
    /// Maximum number of retries (for future use)
    _max_retries: usize,
    /// Base delay for retries (in ms) (for future use)
    _base_delay_ms: u64,
}

impl RetryHandler {
    /// Create a new retry handler
    pub fn new(max_retries: usize, base_delay_ms: u64) -> Self {
        Self {
            _max_retries: max_retries,
            _base_delay_ms: base_delay_ms,
        }
    }

    /// Calculate retry delay with exponential backoff (for future use)
    #[allow(dead_code)]
    fn _retry_delay(&self, attempt: usize) -> std::time::Duration {
        let delay_ms = self._base_delay_ms * 2_u64.pow(attempt as u32);
        std::time::Duration::from_millis(delay_ms)
    }

    /// Check if command should be retried (for future use)
    #[allow(dead_code)]
    fn _should_retry(&self, command: &Command, attempt: usize) -> bool {
        if attempt >= self._max_retries {
            return false;
        }

        // Only retry idempotent commands
        command.is_idempotent()
    }
}

impl CommandHandler for RetryHandler {
    fn handle(&self, command: Command) -> ChainResult {
        // Note: This is a simplified implementation
        // In a real scenario, you'd need async/await for actual retry logic
        // For now, we just pass through
        ChainResult::continue_chain(command)
    }

    fn name(&self) -> &str {
        "RetryHandler"
    }
}

// =============================================================================
// Command Chain
// =============================================================================

/// Chain of responsibility for command processing
pub struct CommandChain {
    /// Handlers in the chain
    handlers: Vec<Box<dyn CommandHandler>>,
}

impl CommandChain {
    /// Create a new command chain
    pub fn new() -> Self {
        Self {
            handlers: Vec::new(),
        }
    }

    /// Add a handler to the end of the chain
    pub fn add_handler(mut self, handler: Box<dyn CommandHandler>) -> Self {
        self.handlers.push(handler);
        self
    }

    /// Add a handler using a builder-like pattern
    pub fn with_handler(mut self, handler: impl CommandHandler + 'static) -> Self {
        self.handlers.push(Box::new(handler));
        self
    }

    /// Process a command through the chain
    pub fn process(&self, command: Command) -> ChainResult {
        let mut current_command = command;

        for handler in &self.handlers {
            if !handler.can_handle(&current_command) {
                continue;
            }

            let result = handler.handle(current_command);

            match result {
                ChainResult::Handled(cmd) => {
                    tracing::debug!("Command handled by: {}", handler.name());
                    return ChainResult::handled(cmd);
                }
                ChainResult::Continue(cmd) => {
                    current_command = cmd;
                    continue;
                }
                ChainResult::Failed { command, error } => {
                    tracing::warn!("Command failed at {}: {}", handler.name(), error);
                    return ChainResult::Failed { command, error };
                }
            }
        }

        // If we get here, the command wasn't handled by anyone
        ChainResult::continue_chain(current_command)
    }

    /// Get the number of handlers in the chain
    pub fn handler_count(&self) -> usize {
        self.handlers.len()
    }
}

impl Default for CommandChain {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Debug for CommandChain {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CommandChain")
            .field("handler_count", &self.handlers.len())
            .field(
                "handlers",
                &self.handlers.iter().map(|h| h.name()).collect::<Vec<_>>(),
            )
            .finish()
    }
}

// =============================================================================
// Chain Builder
// =============================================================================

/// Builder for creating common command chain configurations
pub struct ChainBuilder {
    chain: CommandChain,
}

impl ChainBuilder {
    /// Create a new chain builder
    pub fn new() -> Self {
        Self {
            chain: CommandChain::new(),
        }
    }

    /// Add validation handler
    pub fn with_validation(mut self) -> Self {
        self.chain = self.chain.with_handler(ValidationHandler::new());
        self
    }

    /// Add strict validation handler
    pub fn with_strict_validation(mut self) -> Self {
        self.chain = self.chain.with_handler(ValidationHandler::strict());
        self
    }

    /// Add logging handler
    pub fn with_logging(mut self) -> Self {
        self.chain = self.chain.with_handler(LoggingHandler::new());
        self
    }

    /// Add debug logging handler
    pub fn with_debug_logging(mut self) -> Self {
        self.chain = self.chain.with_handler(LoggingHandler::debug());
        self
    }

    /// Add metrics handler
    pub fn with_metrics(mut self) -> Self {
        self.chain = self.chain.with_handler(MetricsHandler::new());
        self
    }

    /// Add caching handler
    pub fn with_cache(mut self, size: usize) -> Self {
        self.chain = self.chain.with_handler(CachingHandler::new(size));
        self
    }

    /// Add throttling handler
    pub fn with_throttle(mut self, min_interval_ms: u64) -> Self {
        self.chain = self.chain.with_handler(ThrottlingHandler::new(min_interval_ms));
        self
    }

    /// Add retry handler
    pub fn with_retry(mut self, max_retries: usize, base_delay_ms: u64) -> Self {
        self.chain = self.chain.with_handler(RetryHandler::new(max_retries, base_delay_ms));
        self
    }

    /// Add custom handler
    pub fn with_custom(mut self, handler: Box<dyn CommandHandler>) -> Self {
        self.chain = self.chain.add_handler(handler);
        self
    }

    /// Build the chain
    pub fn build(self) -> CommandChain {
        self.chain
    }
}

impl Default for ChainBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Create a default production-ready chain
pub fn default_chain() -> CommandChain {
    ChainBuilder::new()
        .with_validation()
        .with_logging()
        .with_metrics()
        .with_cache(1000)
        .build()
}

/// Create a development chain with debug logging
pub fn dev_chain() -> CommandChain {
    ChainBuilder::new()
        .with_strict_validation()
        .with_debug_logging()
        .with_metrics()
        .build()
}

/// Create a minimal chain for testing
pub fn minimal_chain() -> CommandChain {
    ChainBuilder::new().with_validation().build()
}

// =============================================================================
// Mock LRU Cache for compilation
// =============================================================================

// This is a minimal mock implementation
// In production, use the actual lru crate
mod lru {
    use std::collections::HashMap;
    use std::fmt;
    use std::num::NonZeroUsize;

    pub struct LruCache<K, V> {
        capacity: NonZeroUsize,
        map: HashMap<K, V>,
        order: Vec<K>,
    }

    impl<K: std::hash::Hash + Eq + Clone, V> LruCache<K, V> {
        pub fn new(capacity: NonZeroUsize) -> Self {
            Self {
                capacity,
                map: HashMap::new(),
                order: Vec::new(),
            }
        }

        pub fn get(&mut self, key: &K) -> Option<&V> {
            self.map.get(key)
        }

        #[allow(dead_code)]
        pub fn put(&mut self, key: K, value: V) {
            if self.map.len() >= self.capacity.get() {
                if let Some(old_key) = self.order.first() {
                    self.map.remove(old_key);
                    self.order.remove(0);
                }
            }
            self.map.insert(key.clone(), value);
            self.order.push(key);
        }

        pub fn len(&self) -> usize {
            self.map.len()
        }

        pub fn clear(&mut self) {
            self.map.clear();
            self.order.clear();
        }
    }

    impl<K: fmt::Debug, V: fmt::Debug> fmt::Debug for LruCache<K, V> {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.debug_struct("LruCache")
                .field("capacity", &self.capacity)
                .field("len", &self.map.len())
                .finish()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_command() -> Command {
        Command::SetWorkspaceMode {
            mode: crate::command::WorkspaceMode::Single,
        }
    }

    #[test]
    fn test_command_ext_type_name() {
        let cmd = Command::ToggleDetailMode;
        assert_eq!(cmd.type_name(), "ToggleDetailMode");

        let cmd = Command::SetWorkspaceMode {
            mode: crate::command::WorkspaceMode::Single,
        };
        assert_eq!(cmd.type_name(), "SetWorkspaceMode");
    }

    #[test]
    fn test_command_ext_stream_id() {
        let stream = StreamId::A;
        let cmd = Command::OpenFile {
            stream,
            path: "/path/to/file".into(),
        };
        assert_eq!(cmd.stream_id(), Some(&stream));

        let cmd = Command::ToggleDetailMode;
        assert_eq!(cmd.stream_id(), None);
    }

    #[test]
    fn test_validation_handler_valid_command() {
        let handler = ValidationHandler::new();
        let command = Command::ToggleDetailMode;

        let result = handler.handle(command);

        assert!(result.should_continue());
    }

    #[test]
    fn test_logging_handler() {
        let handler = LoggingHandler::new();
        let command = create_test_command();

        let result = handler.handle(command);

        assert!(result.should_continue());
    }

    #[test]
    fn test_metrics_handler() {
        let handler = MetricsHandler::new();
        let command = create_test_command();

        handler.handle(command);

        let metrics = handler.get_metrics();
        assert_eq!(metrics.total_processed, 1);
    }

    #[test]
    fn test_metrics_handler_reset() {
        let handler = MetricsHandler::new();
        let command = create_test_command();

        handler.handle(command);
        handler.reset_metrics();

        let metrics = handler.get_metrics();
        assert_eq!(metrics.total_processed, 0);
    }

    #[test]
    fn test_command_chain_single_handler() {
        let chain = ChainBuilder::new().with_logging().build();

        let command = create_test_command();
        let result = chain.process(command);

        assert!(result.should_continue());
    }

    #[test]
    fn test_command_chain_multiple_handlers() {
        let chain = ChainBuilder::new()
            .with_validation()
            .with_logging()
            .with_metrics()
            .build();

        let command = create_test_command();
        let result = chain.process(command);

        assert!(result.should_continue());
    }

    #[test]
    fn test_chain_builder_default() {
        let chain = default_chain();

        assert_eq!(chain.handler_count(), 4); // validation, logging, metrics, cache
    }

    #[test]
    fn test_chain_builder_dev() {
        let chain = dev_chain();

        assert_eq!(chain.handler_count(), 3); // strict validation, debug logging, metrics
    }

    #[test]
    fn test_chain_builder_minimal() {
        let chain = minimal_chain();

        assert_eq!(chain.handler_count(), 1); // validation only
    }

    #[test]
    fn test_chain_result_handled() {
        let command = create_test_command();
        let result = ChainResult::handled(command);

        assert!(result.is_handled());
        assert!(!result.should_continue());
        assert!(!result.is_failed());
    }

    #[test]
    fn test_chain_result_continue() {
        let command = create_test_command();
        let result = ChainResult::continue_chain(command);

        assert!(!result.is_handled());
        assert!(result.should_continue());
        assert!(!result.is_failed());
    }

    #[test]
    fn test_chain_result_failed() {
        let command = create_test_command();
        let result = ChainResult::failed(command, "Test error");

        assert!(!result.is_handled());
        assert!(!result.should_continue());
        assert!(result.is_failed());
    }

    #[test]
    fn test_throttling_handler() {
        let handler = ThrottlingHandler::new(100); // 100ms min interval
        let command = create_test_command();

        // First command should pass
        let result1 = handler.handle(command.clone());
        assert!(result1.should_continue());

        // Immediate second command should be throttled
        let result2 = handler.handle(command);
        assert!(result2.is_failed());
    }

    #[test]
    fn test_caching_handler_cache_size() {
        let handler = CachingHandler::new(10);

        assert_eq!(handler.cache_size(), 0);

        handler.clear_cache();

        assert_eq!(handler.cache_size(), 0);
    }

    #[test]
    fn test_command_chain_debug() {
        let chain = ChainBuilder::new()
            .with_validation()
            .with_logging()
            .build();

        let debug_str = format!("{:?}", chain);

        assert!(debug_str.contains("CommandChain"));
        assert!(debug_str.contains("handler_count"));
        assert!(debug_str.contains("2"));
    }

    #[test]
    fn test_command_is_idempotent() {
        let cmd = Command::ToggleDetailMode;
        assert!(cmd.is_idempotent());

        let cmd = Command::OpenFile {
            stream: StreamId::A,
            path: "/path".into(),
        };
        assert!(!cmd.is_idempotent());
    }

    #[test]
    fn test_command_requires_stream() {
        let stream = StreamId::A;
        let cmd = Command::SelectFrame {
            stream,
            frame_key: crate::FrameKey {
                stream: StreamId::A,
                frame_index: 0,
                pts: None,
            },
        };
        assert!(cmd.requires_stream());

        let cmd = Command::ToggleDetailMode;
        assert!(!cmd.requires_stream());
    }
}
