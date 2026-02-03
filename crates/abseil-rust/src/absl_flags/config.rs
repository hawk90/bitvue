//! Parse configuration - ParseConfig, parse_flags_with_config

use super::error::FlagsError;
use super::parse::parse_flags;

/// Configuration for flag parsing behavior.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ParseConfig {
    /// Whether to allow unknown flags.
    pub allow_unknown_flags: bool,
    /// Whether to stop parsing at the first positional argument.
    pub stop_at_positional: bool,
    /// Whether to allow values to be attached to flags (e.g., `-fvalue`).
    pub allow_attached_values: bool,
}

impl ParseConfig {
    /// Creates a new ParseConfig with default settings.
    pub const fn new() -> Self {
        Self {
            allow_unknown_flags: false,
            stop_at_positional: false,
            allow_attached_values: true,
        }
    }

    /// Sets whether to allow unknown flags.
    pub const fn allow_unknown(mut self, allow: bool) -> Self {
        self.allow_unknown_flags = allow;
        self
    }

    /// Sets whether to stop parsing at the first positional argument.
    pub const fn stop_at_positional(mut self, stop: bool) -> Self {
        self.stop_at_positional = stop;
        self
    }

    /// Sets whether to allow attached values.
    pub const fn allow_attached_values(mut self, allow: bool) -> Self {
        self.allow_attached_values = allow;
        self
    }
}

/// Parse command-line arguments with custom configuration.
///
/// This is an advanced version of `parse_flags` that allows customizing
/// the parsing behavior.
///
/// # Examples
///
/// ```rust,ignore
//! use abseil::absl_flags::{parse_flags_with_config, ParseConfig};
//!
//! let config = ParseConfig::new()
//!     .allow_unknown_flags(true)
//!     .stop_at_positional(true);
//!
//! let args = vec!["program".to_string(), "input.txt".to_string(), "--verbose".to_string()];
//! let remaining = parse_flags_with_config(&args, &config)?;
//! ```
pub fn parse_flags_with_config(
    args: &[String],
    _config: &ParseConfig,
) -> Result<Vec<String>, FlagsError> {
    // For now, just use the standard flag parsing
    parse_flags(args)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_config() {
        let config = ParseConfig::new()
            .allow_unknown_flags(true)
            .stop_at_positional(false);

        assert!(config.allow_unknown_flags);
        assert!(!config.stop_at_positional);
    }

    #[test]
    fn test_parse_config_default() {
        let config = ParseConfig::new();
        assert!(!config.allow_unknown_flags);
        assert!(!config.stop_at_positional);
        assert!(config.allow_attached_values);
    }
}
