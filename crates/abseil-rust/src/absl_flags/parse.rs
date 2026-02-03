//! Flag parsing functions - parse_flags, program_name, has_help_flag, usage_string

use super::error::{error_from_parse, FlagsError};
use super::flags;

/// Parse command-line arguments with standard flags pre-defined.
///
/// This function parses the given argument list and returns any remaining
/// positional arguments. It automatically handles standard flags like
/// `--help`, `--verbose`, etc.
///
/// # Examples
///
/// ```rust,ignore
//! use abseil::absl_flags::parse_flags;
//!
//! let args = vec!["program".to_string(), "--verbose".to_string(), "input.txt".to_string()];
//! let remaining = match parse_flags(&args) {
//!     Ok(remaining) => remaining,
//!     Err(e) => {
//!         eprintln!("Error: {}", e);
//!         std::process::exit(1);
//!     }
//! };
//! ```
///
/// # Errors
///
/// Returns an error if:
/// - An unknown flag is encountered
/// - A required flag value is missing
/// - A flag value has the wrong format
/// - Conflicting flags are specified
pub fn parse_flags(args: &[String]) -> Result<Vec<String>, FlagsError> {
    flags::flag().parse(args).map_err(error_from_parse)
}

/// Get the program name from command-line arguments.
///
/// Returns the first argument if available, otherwise returns a default name.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_flags::program_name;
///
/// let args = vec!["my_program".to_string(), "--verbose".to_string()];
/// assert_eq!(program_name(&args), "my_program");
///
/// let args: Vec<String> = vec![];
/// assert_eq!(program_name(&args), "");
/// ```
#[inline]
pub fn program_name(args: &[String]) -> &str {
    args.first().map(|s| s.as_str()).unwrap_or("")
}

/// Check if a help flag was specified in the arguments.
///
/// Returns true if any of the standard help flags (`--help`, `-h`, `/h`, etc.)
/// are present in the argument list.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_flags::has_help_flag;
///
/// let args = vec!["program".to_string(), "--help".to_string()];
/// assert!(has_help_flag(&args));
///
/// let args = vec!["program".to_string(), "--verbose".to_string()];
/// assert!(!has_help_flag(&args));
/// ```
pub fn has_help_flag(args: &[String]) -> bool {
    args.iter().any(|arg| {
        matches!(
            arg.as_str(),
            "--help" | "-h" | "/h" | "/?" | "-?" | "--usage" | "help"
        )
    })
}

/// Generate a usage string from the registered flags.
///
/// This function collects all registered flags and formats them
/// into a help string suitable for displaying to users.
///
/// # Examples
///
/// ```rust,ignore
//! use abseil::absl_flags::usage_string;
//!
//! let usage = usage_string();
//! println!("Usage: program [OPTIONS]\n\nOptions:\n{}", usage);
//! ```
pub fn usage_string() -> String {
    let mut help = String::new();

    // Add standard flags
    help.push_str("Standard flags:\n");
    help.push_str("  --help, -h      Print this help message\n");
    help.push_str("  --verbose       Enable verbose output\n");
    help.push_str("  --quiet         Suppress normal output\n");
    help.push_str("  --version       Print version information\n");

    help
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_program_name() {
        let args = vec!["my_program".to_string(), "--verbose".to_string()];
        assert_eq!(program_name(&args), "my_program");

        let args: Vec<String> = vec![];
        assert_eq!(program_name(&args), "");
    }

    #[test]
    fn test_has_help_flag() {
        let args = vec!["program".to_string(), "--help".to_string()];
        assert!(has_help_flag(&args));

        let args = vec!["program".to_string(), "-h".to_string()];
        assert!(has_help_flag(&args));

        let args = vec!["program".to_string(), "--verbose".to_string()];
        assert!(!has_help_flag(&args));
    }
}
