//! VLOG - Verbose Logging System.
//!
//! Runtime-controllable verbose logging inspired by Google Abseil.
//! VLOG levels can be set globally or per-module via environment variables.
//!
//! # Environment Variables
//!
//! - `VLOG_LEVEL=N`: Set global VLOG level (0 = disabled)
//! - `VLOG_MODULE=module1=N,module2=M`: Set per-module levels
//!
//! # Examples
//!
//! ```ignore
//! use bitvue_log::vlog;
//!
//! // Basic verbose logging
//! vlog!(1, "High-level operation");
//! vlog!(2, "More detailed info");
//! vlog!(3, "Very detailed trace: {:?}", some_data);
//!
//! // Conditional logging (only if VLOG_LEVEL >= level)
//! vlog!(1, "Processing frame {}", frame_index);
//! ```
//!
//! # Level Guidelines
//!
//! - Level 1: High-level operations (function entry/exit, major state changes)
//! - Level 2: Detailed operations (loop iterations, intermediate values)
//! - Level 3+: Very detailed trace (per-byte parsing, low-level state)

/// VLOG - Verbose logging macro.
///
/// Only logs if the VLOG level for the current module is >= the specified level.
///
/// # Arguments
///
/// * `level` - VLOG level (1, 2, 3, etc.)
/// * Format string and arguments (like `format!`)
///
/// # Examples
///
/// ```ignore
/// use bitvue_log::vlog;
///
/// vlog!(1, "Processing started");
/// vlog!(2, "Item count: {}", items.len());
/// vlog!(3, "Raw data: {:02x?}", &data[..16]);
/// ```
#[macro_export]
macro_rules! vlog {
    ($level:expr, $($arg:tt)+) => {{
        let level: i32 = $level;
        if $crate::config::LogConfig::global().is_vlog_enabled(level, module_path!()) {
            tracing::trace!(
                vlog_level = level,
                vlog_module = module_path!(),
                $($arg)+
            );
        }
    }};
}

/// VLOG_IS_ON - Check if VLOG is enabled at a given level.
///
/// Useful for conditionally executing expensive debug code.
///
/// # Examples
///
/// ```ignore
/// use bitvue_log::{vlog, vlog_is_on};
///
/// if vlog_is_on!(3) {
///     let expensive_debug_info = compute_expensive_debug_info();
///     vlog!(3, "Debug info: {:?}", expensive_debug_info);
/// }
/// ```
#[macro_export]
macro_rules! vlog_is_on {
    ($level:expr) => {{
        let level: i32 = $level;
        $crate::config::LogConfig::global().is_vlog_enabled(level, module_path!())
    }};
}

/// DVLOG - Debug-only VLOG (optimized out in release builds).
///
/// Same as `vlog!` but only compiled in debug builds.
#[macro_export]
macro_rules! dvlog {
    ($level:expr, $($arg:tt)+) => {
        #[cfg(debug_assertions)]
        $crate::vlog!($level, $($arg)+);
    };
}

#[cfg(test)]
mod tests {
    use crate::config::LogConfig;

    #[test]
    fn test_vlog_disabled_by_default() {
        // By default, VLOG level is 0, so vlog!(1, ...) should not log
        let config = LogConfig::new();
        assert!(!config.is_vlog_enabled(1, "test::module"));
    }

    #[test]
    fn test_vlog_enabled_when_level_set() {
        let config = LogConfig::new();
        config.set_vlog_level(2);

        assert!(config.is_vlog_enabled(1, "test::module"));
        assert!(config.is_vlog_enabled(2, "test::module"));
        assert!(!config.is_vlog_enabled(3, "test::module"));
    }

    #[test]
    fn test_vlog_module_specific() {
        let config = LogConfig::new();
        config.set_vlog_level(1);
        config.set_module_vlog_level("decoder", 5);

        // decoder module gets higher level
        assert!(config.is_vlog_enabled(5, "bitvue::decoder"));
        // other modules use global level
        assert!(!config.is_vlog_enabled(2, "bitvue::parser"));
    }

    #[test]
    fn test_vlog_is_on() {
        let config = LogConfig::global();
        let original_level = config.get_global_vlog_level();

        config.set_vlog_level(2);
        assert!(crate::vlog_is_on!(1));
        assert!(crate::vlog_is_on!(2));
        assert!(!crate::vlog_is_on!(3));

        // Restore original level
        config.set_vlog_level(original_level);
    }
}
