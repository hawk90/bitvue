//! Runtime configuration for the logging system.
//!
//! Provides global configuration for VLOG levels and module-specific overrides.
//! Inspired by Google Abseil's logging configuration.

use once_cell::sync::Lazy;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::atomic::{AtomicI32, Ordering};

/// Global logging configuration.
///
/// Thread-safe configuration for VLOG levels and module-specific overrides.
/// Uses atomic operations for the global level and RwLock for module map.
pub struct LogConfig {
    /// Global VLOG level (default: 0 = disabled)
    vlog_level: AtomicI32,
    /// Module-specific VLOG levels (module_path -> level)
    vmodule: RwLock<HashMap<String, i32>>,
}

impl LogConfig {
    /// Create a new LogConfig with default settings.
    pub fn new() -> Self {
        Self {
            vlog_level: AtomicI32::new(0),
            vmodule: RwLock::new(HashMap::new()),
        }
    }

    /// Get the global LogConfig instance.
    pub fn global() -> &'static LogConfig {
        static INSTANCE: Lazy<LogConfig> = Lazy::new(LogConfig::new);
        &INSTANCE
    }

    /// Set the global VLOG level.
    ///
    /// # Arguments
    /// * `level` - VLOG level (0 = disabled, higher = more verbose)
    pub fn set_vlog_level(&self, level: i32) {
        self.vlog_level.store(level, Ordering::SeqCst);
    }

    /// Get the global VLOG level.
    pub fn get_global_vlog_level(&self) -> i32 {
        self.vlog_level.load(Ordering::SeqCst)
    }

    /// Get the effective VLOG level for a module.
    ///
    /// Returns the module-specific level if set, otherwise the global level.
    ///
    /// # Arguments
    /// * `module` - Module path (e.g., "bitvue_core::decoder")
    pub fn get_vlog_level(&self, module: &str) -> i32 {
        // Check module-specific level first
        let vmodule = self.vmodule.read();

        // Try exact match first
        if let Some(&level) = vmodule.get(module) {
            return level;
        }

        // Try prefix match (e.g., "bitvue_core" matches "bitvue_core::decoder")
        for (pattern, &level) in vmodule.iter() {
            if module.starts_with(pattern) {
                return level;
            }
            // Also match end of module path (e.g., "decoder" matches "bitvue_core::decoder")
            if module.ends_with(pattern) {
                return level;
            }
            // Match module name without path
            if let Some(name) = module.rsplit("::").next() {
                if name == pattern {
                    return level;
                }
            }
        }

        // Fall back to global level
        self.vlog_level.load(Ordering::SeqCst)
    }

    /// Set a module-specific VLOG level.
    ///
    /// # Arguments
    /// * `module` - Module pattern (e.g., "decoder", "bitvue_core::parser")
    /// * `level` - VLOG level for this module
    pub fn set_module_vlog_level(&self, module: &str, level: i32) {
        let mut vmodule = self.vmodule.write();
        vmodule.insert(module.to_string(), level);
    }

    /// Parse a vmodule specification string.
    ///
    /// Format: "module1=level1,module2=level2"
    /// Example: "parser=3,decoder=2,timeline=1"
    ///
    /// # Arguments
    /// * `spec` - The vmodule specification string
    pub fn parse_vmodule(&self, spec: &str) {
        let mut vmodule = self.vmodule.write();
        for part in spec.split(',') {
            let part = part.trim();
            if part.is_empty() {
                continue;
            }
            if let Some((module, level_str)) = part.split_once('=') {
                if let Ok(level) = level_str.trim().parse::<i32>() {
                    vmodule.insert(module.trim().to_string(), level);
                }
            }
        }
    }

    /// Clear all module-specific VLOG levels.
    pub fn clear_vmodule(&self) {
        let mut vmodule = self.vmodule.write();
        vmodule.clear();
    }

    /// Check if verbose logging is enabled at the given level for a module.
    ///
    /// # Arguments
    /// * `level` - The VLOG level to check
    /// * `module` - The module path
    pub fn is_vlog_enabled(&self, level: i32, module: &str) -> bool {
        self.get_vlog_level(module) >= level
    }
}

impl Default for LogConfig {
    fn default() -> Self {
        Self::new()
    }
}

/// Initialize logging configuration from environment variables.
///
/// Reads:
/// - `VLOG_LEVEL`: Global VLOG level (integer)
/// - `VLOG_MODULE`: Module-specific levels (format: "module1=level1,module2=level2")
///
/// Call this early in your application's startup.
pub fn init_from_env() {
    let config = LogConfig::global();

    // Read global VLOG level
    if let Ok(level_str) = std::env::var("VLOG_LEVEL") {
        if let Ok(level) = level_str.parse::<i32>() {
            config.set_vlog_level(level);
        }
    }

    // Read module-specific levels
    if let Ok(vmodule) = std::env::var("VLOG_MODULE") {
        config.parse_vmodule(&vmodule);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_level() {
        let config = LogConfig::new();
        assert_eq!(config.get_global_vlog_level(), 0);
        assert_eq!(config.get_vlog_level("any::module"), 0);
    }

    #[test]
    fn test_set_global_level() {
        let config = LogConfig::new();
        config.set_vlog_level(3);
        assert_eq!(config.get_global_vlog_level(), 3);
        assert_eq!(config.get_vlog_level("any::module"), 3);
    }

    #[test]
    fn test_module_specific_level() {
        let config = LogConfig::new();
        config.set_vlog_level(1);
        config.set_module_vlog_level("decoder", 5);

        assert_eq!(config.get_vlog_level("bitvue_core::decoder"), 5);
        assert_eq!(config.get_vlog_level("other::module"), 1);
    }

    #[test]
    fn test_parse_vmodule() {
        let config = LogConfig::new();
        config.parse_vmodule("parser=3, decoder=2, timeline = 1");

        assert_eq!(config.get_vlog_level("parser"), 3);
        assert_eq!(config.get_vlog_level("decoder"), 2);
        assert_eq!(config.get_vlog_level("timeline"), 1);
    }

    #[test]
    fn test_is_vlog_enabled() {
        let config = LogConfig::new();
        config.set_vlog_level(2);

        assert!(config.is_vlog_enabled(1, "any::module"));
        assert!(config.is_vlog_enabled(2, "any::module"));
        assert!(!config.is_vlog_enabled(3, "any::module"));
    }

    #[test]
    fn test_clear_vmodule() {
        let config = LogConfig::new();
        config.set_vlog_level(1);
        config.set_module_vlog_level("decoder", 5);

        assert_eq!(config.get_vlog_level("decoder"), 5);

        config.clear_vmodule();
        assert_eq!(config.get_vlog_level("decoder"), 1);
    }

    #[test]
    fn test_prefix_match() {
        let config = LogConfig::new();
        config.set_module_vlog_level("bitvue_core", 3);

        // Should match modules starting with "bitvue_core"
        assert_eq!(config.get_vlog_level("bitvue_core::decoder"), 3);
        assert_eq!(config.get_vlog_level("bitvue_core::parser::obu"), 3);
    }
}
