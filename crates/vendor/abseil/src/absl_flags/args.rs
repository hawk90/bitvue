//! Argument utilities - split_args, is_flag, is_long_flag, is_short_flag, parse_flag_name, parse_flag_value

/// Split command-line arguments into flags and positional arguments.
///
/// This function separates arguments that start with `-` (flags) from
/// regular positional arguments.
///
/// # Examples
///
/// ```rust
//! use abseil::absl_flags::split_args;
//!
//! let args = vec!["prog", "--verbose", "input.txt", "output.txt"];
//! let (flags, positionals) = split_args(&args);
//! assert_eq!(flags, vec!["--verbose"]);
//! assert_eq!(positionals, vec!["prog", "input.txt", "output.txt"]);
//! ```
pub fn split_args(args: &[String]) -> (Vec<&str>, Vec<&str>) {
    let mut flags = Vec::new();
    let mut positionals = Vec::new();

    for arg in args {
        if arg.starts_with('-') {
            flags.push(arg.as_str());
        } else {
            positionals.push(arg.as_str());
        }
    }

    (flags, positionals)
}

/// Check if an argument looks like a flag (starts with `-`).
///
/// # Examples
///
/// ```rust
/// use abseil::absl_flags::is_flag;
///
/// assert!(is_flag("--verbose"));
/// assert!(is_flag("-v"));
/// assert!(!is_flag("input.txt"));
/// ```
#[inline]
pub const fn is_flag(arg: &str) -> bool {
    arg.starts_with('-')
}

/// Check if an argument is a long flag (starts with `--`).
///
/// # Examples
///
/// ```rust
/// use abseil::absl_flags::is_long_flag;
///
/// assert!(is_long_flag("--verbose"));
/// assert!(!is_long_flag("-v"));
/// ```
#[inline]
pub const fn is_long_flag(arg: &str) -> bool {
    arg.starts_with("--")
}

/// Check if an argument is a short flag (starts with `-` but not `--`).
///
/// # Examples
///
/// ```rust
//! use abseil::absl_flags::is_short_flag;
//!
//! assert!(is_short_flag("-v"));
//! assert!(!is_short_flag("--verbose"));
//! ```
#[inline]
pub const fn is_short_flag(arg: &str) -> bool {
    arg.starts_with('-') && !arg.starts_with("--")
}

/// Parse a flag name from an argument.
///
/// Extracts the flag name from arguments like `--flag=value` or `-f`.
///
/// # Examples
///
/// ```rust
//! use abseil::absl_flags::parse_flag_name;
//!
//! assert_eq!(parse_flag_name("--verbose"), Some("verbose"));
//! assert_eq!(parse_flag_name("--output=file.txt"), Some("output"));
//! assert_eq!(parse_flag_name("-v"), Some("v"));
//! assert_eq!(parse_flag_name("input.txt"), None);
//! ```
pub fn parse_flag_name(arg: &str) -> Option<&str> {
    if let Some(rest) = arg.strip_prefix("--") {
        // Long flag: --flag or --flag=value
        rest.split('=').next()
    } else if let Some(rest) = arg.strip_prefix('-') {
        // Short flag: -f or -fvalue
        rest.split('=').next()
    } else {
        None
    }
}

/// Parse a flag value from an argument.
///
/// Extracts the value from arguments like `--flag=value`.
///
/// # Examples
///
/// ```rust
//! use abseil::absl_flags::parse_flag_value;
//!
//! assert_eq!(parse_flag_value("--output=file.txt"), Some("file.txt"));
//! assert_eq!(parse_flag_value("--verbose"), None);
//! ```
pub fn parse_flag_value(arg: &str) -> Option<&str> {
    arg.split('=').nth(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_flag() {
        assert!(is_flag("--verbose"));
        assert!(is_flag("-v"));
        assert!(is_flag("---flag")); // Even if unusual
        assert!(!is_flag("input.txt"));
        assert!(!is_flag(""));
    }

    #[test]
    fn test_is_long_flag() {
        assert!(is_long_flag("--verbose"));
        assert!(is_long_flag("--v"));
        assert!(!is_long_flag("-v"));
        assert!(!is_long_flag("input.txt"));
    }

    #[test]
    fn test_is_short_flag() {
        assert!(is_short_flag("-v"));
        assert!(is_short_flag("-verbose"));
        assert!(!is_short_flag("--verbose"));
        assert!(!is_short_flag("input.txt"));
    }

    #[test]
    fn test_parse_flag_name() {
        assert_eq!(parse_flag_name("--verbose"), Some("verbose"));
        assert_eq!(parse_flag_name("--output=file.txt"), Some("output"));
        assert_eq!(parse_flag_name("-v"), Some("v"));
        assert_eq!(parse_flag_name("-f=file.txt"), Some("f"));
        assert_eq!(parse_flag_name("input.txt"), None);
    }

    #[test]
    fn test_parse_flag_value() {
        assert_eq!(parse_flag_value("--output=file.txt"), Some("file.txt"));
        assert_eq!(parse_flag_value("--verbose"), None);
        assert_eq!(parse_flag_value("-v"), None);
        assert_eq!(parse_flag_value("-f=value"), Some("value"));
        assert_eq!(parse_flag_value("input.txt"), None);
    }

    #[test]
    fn test_split_args() {
        let args = vec![
            "prog".to_string(),
            "--verbose".to_string(),
            "input.txt".to_string(),
            "output.txt".to_string(),
        ];
        let (flags, positionals) = split_args(&args);
        assert_eq!(flags, vec!["--verbose"]);
        assert_eq!(positionals, vec!["prog", "input.txt", "output.txt"]);
    }

    #[test]
    fn test_split_args_no_flags() {
        let args = vec!["prog".to_string(), "input.txt".to_string()];
        let (flags, positionals) = split_args(&args);
        assert!(flags.is_empty());
        assert_eq!(positionals, vec!["prog", "input.txt"]);
    }
}
