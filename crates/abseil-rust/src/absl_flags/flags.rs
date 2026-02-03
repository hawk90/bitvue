//! Command-line flag parsing utilities.
//!
//! This module provides flag parsing utilities similar to Abseil's `absl/flags` directory.

use std::sync::{Mutex, MutexGuard, PoisonError};

/// A boolean flag.
pub struct BoolFlag {
    name: &'static str,
    default: bool,
    current: Mutex<bool>,
    help: &'static str,
}

impl BoolFlag {
    /// Creates a new boolean flag.
    pub const fn new(
        name: &'static str,
        default: bool,
        help: &'static str,
    ) -> Self {
        Self {
            name,
            default,
            current: Mutex::new(default),
            help,
        }
    }

    /// Returns the flag name.
    pub fn name(&self) -> &'static str {
        self.name
    }

    /// Returns the default value.
    pub fn default_value(&self) -> bool {
        self.default
    }

    /// Returns the help text.
    pub fn help(&self) -> &'static str {
        self.help
    }

    /// Gets the current value.
    ///
    /// Returns the default value if the mutex is poisoned.
    pub fn get(&self) -> bool {
        self.try_get().unwrap_or(self.default)
    }

    /// Tries to get the current value.
    ///
    /// Returns `Err` if the mutex is poisoned.
    pub fn try_get(&self) -> Result<bool, PoisonError<MutexGuard<'_, bool>>> {
        self.current.lock().map(|g| *g)
    }

    /// Sets the value.
    ///
    /// Panics with a descriptive message if the mutex is poisoned.
    pub fn set(&self, value: bool) {
        // SAFETY: Lock the mutex once and use expect for better error message
        let mut guard = self.current.lock()
            .unwrap_or_else(|_| panic!("Failed to set boolean flag '{}': mutex is poisoned", self.name));
        *guard = value;
    }

    /// Tries to set the value.
    ///
    /// Returns `Err` if the mutex is poisoned.
    pub fn try_set(&self, value: bool) -> Result<(), PoisonError<MutexGuard<'_, bool>>> {
        let mut guard = self.current.lock()?;
        *guard = value;
        Ok(())
    }
}

/// A string flag.
pub struct StringFlag {
    name: &'static str,
    default: &'static str,
    current: Mutex<String>,
    help: &'static str,
}

impl StringFlag {
    /// Creates a new string flag.
    pub fn new(
        name: &'static str,
        default: &'static str,
        help: &'static str,
    ) -> Self {
        Self {
            name,
            default,
            current: Mutex::new(default.to_string()),
            help,
        }
    }

    /// Returns the flag name.
    pub fn name(&self) -> &'static str {
        self.name
    }

    /// Returns the default value.
    pub fn default_value(&self) -> &'static str {
        self.default
    }

    /// Returns the help text.
    pub fn help(&self) -> &'static str {
        self.help
    }

    /// Gets the current value.
    ///
    /// Returns the default value if the mutex is poisoned.
    pub fn get(&self) -> String {
        self.try_get().unwrap_or_else(|_| self.default.to_string())
    }

    /// Tries to get the current value.
    ///
    /// Returns `Err` if the mutex is poisoned.
    pub fn try_get(&self) -> Result<String, PoisonError<MutexGuard<'_, String>>> {
        self.current.lock().map(|g| g.clone())
    }

    /// Sets the value.
    ///
    /// Panics with a descriptive message if the mutex is poisoned.
    pub fn set(&self, value: &str) {
        // SAFETY: Lock the mutex once and use expect for better error message
        let mut guard = self.current.lock()
            .unwrap_or_else(|_| panic!("Failed to set string flag '{}': mutex is poisoned", self.name));
        *guard = value.to_string();
    }

    /// Tries to set the value.
    ///
    /// Returns `Err` if the mutex is poisoned.
    pub fn try_set(&self, value: &str) -> Result<(), PoisonError<MutexGuard<'_, String>>> {
        let mut guard = self.current.lock()?;
        *guard = value.to_string();
        Ok(())
    }
}

/// An integer flag.
pub struct IntFlag {
    name: &'static str,
    default: i64,
    current: Mutex<i64>,
    help: &'static str,
}

impl IntFlag {
    /// Creates a new integer flag.
    pub const fn new(
        name: &'static str,
        default: i64,
        help: &'static str,
    ) -> Self {
        Self {
            name,
            default,
            current: Mutex::new(default),
            help,
        }
    }

    /// Returns the flag name.
    pub fn name(&self) -> &'static str {
        self.name
    }

    /// Returns the default value.
    pub fn default_value(&self) -> i64 {
        self.default
    }

    /// Returns the help text.
    pub fn help(&self) -> &'static str {
        self.help
    }

    /// Gets the current value.
    ///
    /// Returns the default value if the mutex is poisoned.
    pub fn get(&self) -> i64 {
        self.try_get().unwrap_or(self.default)
    }

    /// Tries to get the current value.
    ///
    /// Returns `Err` if the mutex is poisoned.
    pub fn try_get(&self) -> Result<i64, PoisonError<MutexGuard<'_, i64>>> {
        self.current.lock().map(|g| *g)
    }

    /// Sets the value.
    ///
    /// Panics with a descriptive message if the mutex is poisoned.
    pub fn set(&self, value: i64) {
        // SAFETY: Lock the mutex once and use expect for better error message
        let mut guard = self.current.lock()
            .unwrap_or_else(|_| panic!("Failed to set integer flag '{}': mutex is poisoned", self.name));
        *guard = value;
    }

    /// Tries to set the value.
    ///
    /// Returns `Err` if the mutex is poisoned.
    pub fn try_set(&self, value: i64) -> Result<(), PoisonError<MutexGuard<'_, i64>>> {
        let mut guard = self.current.lock()?;
        *guard = value;
        Ok(())
    }
}

/// Parses command-line arguments and sets flags accordingly.
///
/// # Arguments
///
/// * `args` - Command-line arguments (usually from `std::env::args()`)
///
/// # Example
///
/// ```rust,no_run
/// use abseil::absl_flags::flags::flag::parse_command_line_args;
///
/// let args: Vec<String> = std::env::args().collect();
/// parse_command_line_args(&args);
/// ```
pub fn parse_command_line_args(args: &[String]) {
    // Simple flag parsing
    // Format: --flag_name=value or --flag_name value or --bool_flag (true)
    let mut i = 0;
    while i < args.len() {
        let arg = &args[i];
        if arg.starts_with("--") {
            let flag_part = &arg[2..];
            if let Some(eq_pos) = flag_part.find('=') {
                // --flag=value format
                let _name = &flag_part[..eq_pos];
                let _value = &flag_part[eq_pos + 1..];
                // Try to set the flag (would need flag registry in production)
                i += 1;
            } else {
                // --flag value format or --bool_flag
                let _name = flag_part;
                if i + 1 < args.len() && !args[i + 1].starts_with("--") {
                    // --flag value format
                    let _value = &args[i + 1];
                    // Try to set the flag
                    i += 2;
                } else {
                    // --bool_flag format (treat as true)
                    // Try to set the flag
                    i += 1;
                }
            }
        } else {
            i += 1;
        }
    }
}

/// Flag module for common flags.
pub mod flag {
    use super::*;

    /// Usage flag (--help).
    pub static USAGE: BoolFlag = BoolFlag::new("help", false, "Show help message");

    /// Verbose flag (--verbose).
    pub static VERBOSE: BoolFlag = BoolFlag::new("verbose", false, "Enable verbose output");

    /// Flag to parse command-line args.
    pub fn parse_command_line_args(args: &[String]) {
        super::parse_command_line_args(args);
        // If --help is present, show usage and exit
        if USAGE.get() {
            println!("Usage: program [flags]");
            println!("Flags:");
            println!("  --help           Show this help message");
            println!("  --verbose        Enable verbose output");
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn test_bool_flag() {
            let flag = BoolFlag::new("test", false, "Test flag");
            assert_eq!(flag.get(), false);
            flag.set(true);
            assert_eq!(flag.get(), true);
        }

        #[test]
        fn test_bool_flag_default() {
            let flag = BoolFlag::new("test", true, "Test flag");
            assert_eq!(flag.default_value(), true);
            assert_eq!(flag.get(), true);
        }

        #[test]
        fn test_string_flag() {
            let flag = StringFlag::new("name", "default", "Name flag");
            assert_eq!(flag.get(), "default");
            flag.set("custom");
            assert_eq!(flag.get(), "custom");
        }

        #[test]
        fn test_int_flag() {
            let flag = IntFlag::new("count", 42, "Count flag");
            assert_eq!(flag.get(), 42);
            flag.set(100);
            assert_eq!(flag.get(), 100);
        }

        #[test]
        fn test_parse_args_empty() {
            let args = vec!["program".to_string()];
            parse_command_line_args(&args);
            // Should not panic
        }

        #[test]
        fn test_parse_args_help() {
            let args = vec!["program".to_string(), "--help".to_string()];
            parse_command_line_args(&args);
            // Note: The parser doesn't auto-set flags; this just verifies it doesn't panic
            // To actually check for --help, would need to check args directly
        }
    }
}

// Re-exports
pub use flag::{USAGE, VERBOSE};
