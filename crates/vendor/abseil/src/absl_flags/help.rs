//! Help formatting utilities - default_banner, format_flag_help

/// Get the default banner text for the program.
///
/// Returns a formatted banner with program name and version info.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_flags::default_banner;
///
/// let banner = default_banner("MyProgram", "1.0.0");
/// println!("{}", banner);
/// ```
pub fn default_banner(program_name: &str, version: &str) -> String {
    format!("{} version {}", program_name, version)
}

/// Format a flag description for help output.
///
/// # Examples
///
/// ```rust
//! use abseil::absl_flags::format_flag_help;
//!
//! let help = format_flag_help("verbose", "v", "Enable verbose output");
//! assert!(help.contains("verbose"));
//! assert!(help.contains("Enable verbose output"));
//! ```
pub fn format_flag_help(long_name: &str, short_name: &str, description: &str) -> String {
    let mut help = String::new();

    if !short_name.is_empty() {
        help.push_str(&format!("-{}, ", short_name));
    }

    help.push_str(&format!("--{}", long_name));

    if !description.is_empty() {
        help.push_str(&format!("\n    {}", description));
    }

    help
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_banner() {
        let banner = default_banner("MyProgram", "1.0.0");
        assert!(banner.contains("MyProgram"));
        assert!(banner.contains("1.0.0"));
    }

    #[test]
    fn test_format_flag_help() {
        let help = format_flag_help("verbose", "v", "Enable verbose output");
        assert!(help.contains("verbose"));
        assert!(help.contains("Enable verbose output"));
        assert!(help.contains("-v"));
    }

    #[test]
    fn test_format_flag_help_no_short() {
        let help = format_flag_help("output", "", "Output file path");
        assert!(help.contains("output"));
        assert!(help.contains("Output file path"));
        assert!(!help.contains("-"));
    }
}
