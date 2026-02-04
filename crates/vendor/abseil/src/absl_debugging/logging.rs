//! Debug logging utilities.

use alloc::string::String;
use alloc::vec::Vec;

/// Log level.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum LogLevel {
    /// Trace level (most verbose).
    Trace = 0,
    /// Debug level.
    Debug = 1,
    /// Info level.
    Info = 2,
    /// Warning level.
    Warn = 3,
    /// Error level.
    Error = 4,
    /// Fatal level (least verbose).
    Fatal = 5,
}

impl LogLevel {
    /// Returns the name of this log level.
    pub fn name(&self) -> &'static str {
        match self {
            LogLevel::Trace => "TRACE",
            LogLevel::Debug => "DEBUG",
            LogLevel::Info => "INFO",
            LogLevel::Warn => "WARN",
            LogLevel::Error => "ERROR",
            LogLevel::Fatal => "FATAL",
        }
    }
}

impl core::fmt::Display for LogLevel {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.name())
    }
}

/// A log entry.
#[derive(Clone, Debug)]
pub struct LogEntry {
    /// The log level.
    pub level: LogLevel,
    /// The log message.
    pub message: String,
    /// The module or source.
    pub module: Option<String>,
    /// The file path.
    pub file: Option<String>,
    /// The line number.
    pub line: Option<u32>,
    /// The timestamp (if available).
    pub timestamp: Option<u64>,
}

impl LogEntry {
    /// Creates a new log entry.
    pub fn new(level: LogLevel, message: String) -> Self {
        Self {
            level,
            message,
            module: None,
            file: None,
            line: None,
            timestamp: None,
        }
    }

    /// Sets the module.
    pub fn with_module(mut self, module: String) -> Self {
        self.module = Some(module);
        self
    }

    /// Sets the file location.
    pub fn with_location(mut self, file: String, line: u32) -> Self {
        self.file = Some(file);
        self.line = Some(line);
        self
    }

    /// Sets the timestamp.
    pub fn with_timestamp(mut self, timestamp: u64) -> Self {
        self.timestamp = Some(timestamp);
        self
    }
}

/// A debug logger.
#[derive(Clone, Debug)]
pub struct DebugLogger {
    /// Minimum log level.
    min_level: LogLevel,
    /// Log entries (for in-memory logging).
    entries: Vec<LogEntry>,
    /// Maximum number of entries to keep.
    max_entries: usize,
}

impl DebugLogger {
    /// Creates a new logger.
    pub fn new() -> Self {
        Self {
            min_level: LogLevel::Info,
            entries: Vec::new(),
            max_entries: 1000,
        }
    }

    /// Sets the minimum log level.
    pub fn set_min_level(&mut self, level: LogLevel) {
        self.min_level = level;
    }

    /// Returns the minimum log level.
    pub fn min_level(&self) -> LogLevel {
        self.min_level
    }

    /// Logs a message.
    pub fn log(&mut self, entry: LogEntry) {
        if entry.level < self.min_level {
            return;
        }

        // In a real implementation, this would output to stderr
        self.entries.push(entry);
        if self.entries.len() > self.max_entries {
            self.entries.remove(0);
        }
    }

    /// Returns all log entries.
    pub fn entries(&self) -> &[LogEntry] {
        &self.entries
    }

    /// Clears all log entries.
    pub fn clear(&mut self) {
        self.entries.clear();
    }

    /// Returns entries filtered by level.
    pub fn entries_by_level(&self, level: LogLevel) -> Vec<&LogEntry> {
        self.entries.iter().filter(|e| e.level == level).collect()
    }
}

impl Default for DebugLogger {
    fn default() -> Self {
        Self::new()
    }
}

/// Debug assertion options.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AssertionOptions {
    /// Whether to panic on assertion failure.
    pub panic_on_failure: bool,
    /// Whether to log assertion failures.
    pub log_failures: bool,
    /// Custom failure message.
    pub message: Option<String>,
}

impl AssertionOptions {
    /// Creates new assertion options.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets panic on failure.
    pub fn with_panic(mut self, panic: bool) -> Self {
        self.panic_on_failure = panic;
        self
    }

    /// Sets log failures.
    pub fn with_logging(mut self, log: bool) -> Self {
        self.log_failures = log;
        self
    }

    /// Sets custom message.
    pub fn with_message(mut self, message: String) -> Self {
        self.message = Some(message);
        self
    }
}

/// Assertion result.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AssertionError {
    /// Assertion passed.
    Passed,
    /// Assertion failed with message.
    Failed(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    // Tests for LogLevel
    #[test]
    fn test_log_level_name() {
        assert_eq!(LogLevel::Trace.name(), "TRACE");
        assert_eq!(LogLevel::Debug.name(), "DEBUG");
        assert_eq!(LogLevel::Info.name(), "INFO");
        assert_eq!(LogLevel::Warn.name(), "WARN");
        assert_eq!(LogLevel::Error.name(), "ERROR");
        assert_eq!(LogLevel::Fatal.name(), "FATAL");
    }

    #[test]
    fn test_log_level_display() {
        let s = format!("{}", LogLevel::Info);
        assert_eq!(s, "INFO");
    }

    #[test]
    fn test_log_level_ordering() {
        assert!(LogLevel::Debug < LogLevel::Info);
        assert!(LogLevel::Error > LogLevel::Warn);
    }

    // Tests for LogEntry
    #[test]
    fn test_log_entry_new() {
        let entry = LogEntry::new(LogLevel::Info, "test message".to_string());
        assert_eq!(entry.level, LogLevel::Info);
        assert_eq!(entry.message, "test message");
    }

    #[test]
    fn test_log_entry_with_module() {
        let entry = LogEntry::new(LogLevel::Info, "test".to_string())
            .with_module("test_module".to_string());
        assert_eq!(entry.module, Some("test_module".to_string()));
    }

    #[test]
    fn test_log_entry_with_location() {
        let entry = LogEntry::new(LogLevel::Info, "test".to_string())
            .with_location("file.rs".to_string(), 42);
        assert_eq!(entry.file, Some("file.rs".to_string()));
        assert_eq!(entry.line, Some(42));
    }

    #[test]
    fn test_log_entry_with_timestamp() {
        let entry = LogEntry::new(LogLevel::Info, "test".to_string())
            .with_timestamp(12345);
        assert_eq!(entry.timestamp, Some(12345));
    }

    // Tests for DebugLogger
    #[test]
    fn test_debug_logger_new() {
        let logger = DebugLogger::new();
        assert_eq!(logger.min_level(), LogLevel::Info);
    }

    #[test]
    fn test_debug_logger_set_min_level() {
        let mut logger = DebugLogger::new();
        logger.set_min_level(LogLevel::Debug);
        assert_eq!(logger.min_level(), LogLevel::Debug);
    }

    #[test]
    fn test_debug_logger_log() {
        let mut logger = DebugLogger::new();
        logger.log(LogEntry::new(LogLevel::Error, "error".to_string()));
        assert_eq!(logger.entries().len(), 1);
    }

    #[test]
    fn test_debug_logger_log_filtered() {
        let mut logger = DebugLogger::new();
        logger.log(LogEntry::new(LogLevel::Debug, "debug".to_string()));
        // Debug < Info, so should be filtered
        assert_eq!(logger.entries().len(), 0);
    }

    #[test]
    fn test_debug_logger_clear() {
        let mut logger = DebugLogger::new();
        logger.log(LogEntry::new(LogLevel::Error, "error".to_string()));
        logger.clear();
        assert_eq!(logger.entries().len(), 0);
    }

    #[test]
    fn test_debug_logger_entries_by_level() {
        let mut logger = DebugLogger::new();
        logger.log(LogEntry::new(LogLevel::Error, "error1".to_string()));
        logger.log(LogEntry::new(LogLevel::Error, "error2".to_string()));
        logger.log(LogEntry::new(LogLevel::Info, "info".to_string()));
        let errors = logger.entries_by_level(LogLevel::Error);
        assert_eq!(errors.len(), 2);
    }

    #[test]
    fn test_debug_logger_max_entries() {
        let mut logger = DebugLogger::new();
        logger.max_entries = 5;
        for i in 0..10 {
            logger.log(LogEntry::new(LogLevel::Info, format!("msg{}", i)));
        }
        // Should only keep the last 5 entries
        assert_eq!(logger.entries().len(), 5);
    }

    // Tests for AssertionOptions
    #[test]
    fn test_assertion_options_new() {
        let options = AssertionOptions::new();
        assert!(options.panic_on_failure);
        assert!(options.log_failures);
    }

    #[test]
    fn test_assertion_options_with_panic() {
        let options = AssertionOptions::new().with_panic(false);
        assert!(!options.panic_on_failure);
    }

    #[test]
    fn test_assertion_options_with_logging() {
        let options = AssertionOptions::new().with_logging(false);
        assert!(!options.log_failures);
    }

    #[test]
    fn test_assertion_options_with_message() {
        let options = AssertionOptions::new()
            .with_message("custom message".to_string());
        assert_eq!(options.message, Some("custom message".to_string()));
    }
}
