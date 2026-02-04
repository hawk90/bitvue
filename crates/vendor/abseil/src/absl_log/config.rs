//! Logging configuration.
//!
//! This module provides configuration for the logging system, including
//! VLOG level settings, minimum log levels, and environment variable support.

use crate::absl_log::severity::LogSeverity;
use std::collections::HashMap;
use std::sync::atomic::{AtomicI32, AtomicU32, Ordering};
use std::sync::RwLock;

/// Global log configuration.
///
/// This struct contains the global logging settings for the application.
#[derive(Debug)]
pub struct LogConfig {
    /// Minimum log level (messages below this are not logged).
    min_log_level: AtomicU32,

    /// VLOG level per file pattern.
    vlog_levels: RwLock<HashMap<String, i32>>,

    /// Default VLOG level for files not matching any pattern.
    default_vlog_level: AtomicI32,
}

impl LogConfig {
    /// Creates a new `LogConfig` with default settings.
    #[inline]
    pub fn new() -> Self {
        Self {
            min_log_level: AtomicU32::new(LogSeverity::Info as u32),
            vlog_levels: RwLock::new(HashMap::new()),
            default_vlog_level: AtomicI32::new(0),
        }
    }

    /// Sets the minimum log level.
    ///
    /// Messages below this level will not be logged.
    ///
    /// # Example
    ///
    /// ```rust
    /// use abseil::absl_log::config::LogConfig;
    /// use abseil::absl_log::severity::LogSeverity;
    ///
    /// let config = LogConfig::new();
    /// config.set_min_log_level(LogSeverity::Warning);
    /// ```
    #[inline]
    pub fn set_min_log_level(&self, level: LogSeverity) {
        self.min_log_level.store(level as u32, Ordering::Release);
    }

    /// Gets the current minimum log level.
    #[inline]
    pub fn min_log_level(&self) -> LogSeverity {
        match self.min_log_level.load(Ordering::Acquire) {
            0 => LogSeverity::Info,
            1 => LogSeverity::Warning,
            2 => LogSeverity::Error,
            3 => LogSeverity::Fatal,
            _ => LogSeverity::Info,
        }
    }

    /// Sets the VLOG level for a specific file pattern.
    ///
    /// The pattern can be a full file path or a substring.
    /// Longer, more specific patterns take precedence.
    ///
    /// # Example
    ///
    /// ```rust
    /// use abseil::absl_log::config::LogConfig;
    ///
    /// let config = LogConfig::new();
    /// config.set_vlog_level("my_module", 2);
    /// ```
    pub fn set_vlog_level(&self, pattern: &str, level: i32) {
        let mut levels = self.vlog_levels.write().unwrap();
        levels.insert(pattern.to_string(), level.max(0));
    }

    /// Gets the VLOG level for a file.
    ///
    /// Returns the highest matching level, or the default level if
    /// no pattern matches.
    pub fn get_vlog_level(&self, file: &str) -> i32 {
        let levels = self.vlog_levels.read().unwrap();
        let mut best_level = self.default_vlog_level.load(Ordering::Acquire);

        for (pattern, level) in levels.iter() {
            if file.contains(pattern) && *level > best_level {
                best_level = *level;
            }
        }

        best_level.max(0)
    }

    /// Sets the default VLOG level for all files.
    #[inline]
    pub fn set_default_vlog_level(&self, level: i32) {
        self.default_vlog_level
            .store(level.max(0), Ordering::Release);
    }

    /// Gets the default VLOG level.
    #[inline]
    pub fn default_vlog_level(&self) -> i32 {
        self.default_vlog_level.load(Ordering::Acquire).max(0)
    }

    /// Clears all VLOG level settings.
    pub fn clear_vlog_levels(&self) {
        let mut levels = self.vlog_levels.write().unwrap();
        levels.clear();
    }

    /// Returns `true` if logging is enabled for the given severity.
    #[inline]
    pub fn is_logging_enabled(&self, severity: LogSeverity) -> bool {
        severity.as_i32() >= self.min_log_level.load(Ordering::Acquire) as i32
    }
}

impl Default for LogConfig {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

/// Global log configuration instance.
static GLOBAL_CONFIG: std::sync::OnceLock<LogConfig> = std::sync::OnceLock::new();

/// Gets the global log configuration.
///
/// # Example
///
/// ```rust
/// use abseil::absl_log::config::global_config;
/// use abseil::absl_log::severity::LogSeverity;
///
/// let config = global_config();
/// config.set_min_log_level(LogSeverity::Warning);
/// ```
#[inline]
pub fn global_config() -> &'static LogConfig {
    GLOBAL_CONFIG.get_or_init(|| LogConfig::new())
}

/// Initializes logging from environment variables.
///
/// Supported environment variables:
/// - `ABSL_MIN_LOG_LEVEL`: Minimum log level (0=INFO, 1=WARNING, 2=ERROR, 3=FATAL)
/// - `ABSL_VLOG_LEVEL`: Default VLOG level
/// - `ABSL_VMODULE`: Comma-separated list of file_pattern=level pairs
///
/// # Example
///
/// ```rust
/// use abseil::absl_log::config::init_from_env;
///
/// // Set environment variables before calling
/// init_from_env();
/// ```
pub fn init_from_env() {
    // Read minimum log level
    if let Ok(level_str) = std::env::var("ABSL_MIN_LOG_LEVEL") {
        if let Ok(level) = level_str.parse::<i32>() {
            if let Some(severity) = LogSeverity::from_i32(level) {
                global_config().set_min_log_level(severity);
            }
        }
    }

    // Read default VLOG level
    if let Ok(level_str) = std::env::var("ABSL_VLOG_LEVEL") {
        if let Ok(level) = level_str.parse::<i32>() {
            global_config().set_default_vlog_level(level);
        }
    }

    // Read VLOG module settings
    if let Ok(vmodule) = std::env::var("ABSL_VMODULE") {
        for pair in vmodule.split(',') {
            let pair = pair.trim();
            if let Some((pattern, level_str)) = pair.split_once('=') {
                if let Ok(level) = level_str.trim().parse::<i32>() {
                    global_config().set_vlog_level(pattern.trim(), level);
                }
            }
        }
    }
}

/// Returns `true` if VLOG is enabled for the given level and file.
///
/// # Example
///
/// ```rust
/// use abseil::absl_log::config::is_vlog_enabled;
///
/// if is_vlog_enabled(2, file!()) {
///     println!("Detailed debug info");
/// }
/// ```
#[inline]
pub fn is_vlog_enabled(level: i32, file: &str) -> bool {
    level <= global_config().get_vlog_level(file)
}

/// Returns `true` if logging is enabled for the given severity.
///
/// # Example
///
/// ```rust
/// use abseil::absl_log::config::is_logging_enabled;
/// use abseil::absl_log::severity::LogSeverity;
///
/// if is_logging_enabled(LogSeverity::Info) {
///     println!("Info message");
/// }
/// ```
#[inline]
pub fn is_logging_enabled(severity: LogSeverity) -> bool {
    global_config().is_logging_enabled(severity)
}

/// Sets the minimum log level globally.
///
/// # Example
///
/// ```rust
/// use abseil::absl_log::config::set_min_log_level;
/// use abseil::absl_log::severity::LogSeverity;
///
/// set_min_log_level(LogSeverity::Warning);
/// ```
#[inline]
pub fn set_min_log_level(level: LogSeverity) {
    global_config().set_min_log_level(level);
}

/// Gets the current minimum log level.
///
/// # Example
///
/// ```rust
/// use abseil::absl_log::config::min_log_level;
///
/// let level = min_log_level();
/// ```
#[inline]
pub fn min_log_level() -> LogSeverity {
    global_config().min_log_level()
}

/// Sets the VLOG level for a file pattern.
///
/// # Example
///
/// ```rust
/// use abseil::absl_log::config::set_vlog_level;
///
/// set_vlog_level("my_module", 2);
/// ```
#[inline]
pub fn set_vlog_level(pattern: &str, level: i32) {
    global_config().set_vlog_level(pattern, level);
}

/// Sets the default VLOG level.
///
/// # Example
///
/// ```rust
/// use abseil::absl_log::config::set_default_vlog_level;
///
/// set_default_vlog_level(1);
/// ```
#[inline]
pub fn set_default_vlog_level(level: i32) {
    global_config().set_default_vlog_level(level);
}

/// Gets the default VLOG level for all files.
///
/// # Example
///
/// ```rust
/// use abseil::absl_log::config::default_vlog_level;
///
/// let level = default_vlog_level();
/// ```
#[inline]
pub fn default_vlog_level() -> i32 {
    global_config().default_vlog_level()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_new() {
        let config = LogConfig::new();
        assert_eq!(config.min_log_level(), LogSeverity::Info);
        assert_eq!(config.default_vlog_level(), 0);
    }

    #[test]
    fn test_set_min_log_level() {
        let config = LogConfig::new();
        config.set_min_log_level(LogSeverity::Error);
        assert_eq!(config.min_log_level(), LogSeverity::Error);
    }

    #[test]
    fn test_is_logging_enabled() {
        let config = LogConfig::new();
        assert!(config.is_logging_enabled(LogSeverity::Info));
        assert!(config.is_logging_enabled(LogSeverity::Error));

        config.set_min_log_level(LogSeverity::Warning);
        assert!(!config.is_logging_enabled(LogSeverity::Info));
        assert!(config.is_logging_enabled(LogSeverity::Warning));
    }

    #[test]
    fn test_vlog_level() {
        let config = LogConfig::new();
        config.set_vlog_level("test_module", 2);
        assert_eq!(config.get_vlog_level("test_module.rs"), 2);
        assert_eq!(config.get_vlog_level("other_module.rs"), 0);
    }

    #[test]
    fn test_vlog_level_pattern() {
        let config = LogConfig::new();
        config.set_vlog_level("http", 2);
        config.set_vlog_level("http/server", 3);

        // More specific pattern wins
        assert_eq!(config.get_vlog_level("http/server.rs"), 3);
        assert_eq!(config.get_vlog_level("http/client.rs"), 2);
    }

    #[test]
    fn test_default_vlog_level() {
        let config = LogConfig::new();
        assert_eq!(config.default_vlog_level(), 0);

        config.set_default_vlog_level(1);
        assert_eq!(config.default_vlog_level(), 1);
        assert_eq!(config.get_vlog_level("unknown_file.rs"), 1);
    }

    #[test]
    fn test_clear_vlog_levels() {
        let config = LogConfig::new();
        config.set_vlog_level("module1", 1);
        config.set_vlog_level("module2", 2);

        config.clear_vlog_levels();
        assert_eq!(config.get_vlog_level("module1.rs"), 0);
        assert_eq!(config.get_vlog_level("module2.rs"), 0);
    }

    #[test]
    fn test_negative_vlog_level_clamped() {
        let config = LogConfig::new();
        config.set_vlog_level("test", -1);
        assert_eq!(config.get_vlog_level("test.rs"), 0);
    }

    #[test]
    fn test_is_vlog_enabled() {
        // Use global config for testing the global is_vlog_enabled function
        set_vlog_level("test_module", 2);

        assert!(is_vlog_enabled(2, "test_module.rs"));
        assert!(is_vlog_enabled(1, "test_module.rs"));
        assert!(!is_vlog_enabled(3, "test_module.rs"));

        // Clean up for other tests
        global_config().clear_vlog_levels();
    }

    #[test]
    fn test_global_config() {
        let config = global_config();
        config.set_min_log_level(LogSeverity::Warning);
        assert_eq!(min_log_level(), LogSeverity::Warning);

        // Reset for other tests
        config.set_min_log_level(LogSeverity::Info);
    }

    #[test]
    fn test_set_vlog_level_global() {
        set_vlog_level("my_module", 3);
        assert_eq!(global_config().get_vlog_level("my_module.rs"), 3);

        // Clean up
        global_config().clear_vlog_levels();
    }

    #[test]
    fn test_set_default_vlog_level_global() {
        set_default_vlog_level(2);
        assert_eq!(default_vlog_level(), 2);

        // Reset
        set_default_vlog_level(0);
    }
}
