//! ParseResult and parse_with_subcommands - Result of parsing command-line arguments

use super::args::{parse_flag_name, parse_flag_value};
use super::error::FlagsError;
use super::subcommand::Subcommand;

/// Environment variable fallback for flag values.
pub fn get_env_fallback(env_var: &str) -> Option<String> {
    #[cfg(feature = "std")]
    {
        std::env::var(env_var).ok()
    }
    #[cfg(not(feature = "std"))]
    {
        let _ = env_var;
        None
    }
}

/// A flag alias mapping.
#[derive(Clone, Debug, Default)]
pub struct FlagAliases {
    aliases: Vec<(String, String)>,
}

impl FlagAliases {
    /// Creates a new alias mapping.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds an alias (from -> to).
    pub fn add(mut self, from: &str, to: &str) -> Self {
        self.aliases.push((from.to_string(), to.to_string()));
        self
    }

    /// Resolves an alias to its canonical name.
    pub fn resolve(&self, name: &str) -> Option<&str> {
        for (from, to) in &self.aliases {
            if name == from {
                return Some(to.as_str());
            }
        }
        None
    }

    /// Checks if a name is an alias.
    pub fn is_alias(&self, name: &str) -> bool {
        self.aliases.iter().any(|(from, _)| from == name)
    }
}

/// Result of parsing command-line arguments.
#[derive(Clone, Debug, Default)]
pub struct ParseResult {
    pub flags: Vec<(String, String)>,
    pub positionals: Vec<String>,
    pub subcommand: Option<String>,
}

impl ParseResult {
    /// Creates a new parse result.
    pub fn new() -> Self {
        Self::default()
    }

    /// Gets a flag value by name.
    pub fn get_flag(&self, name: &str) -> Option<&str> {
        self.flags.iter().find(|(n, _)| n == name).map(|(_, v)| v.as_str())
    }

    /// Checks if a boolean flag is set.
    pub fn has_flag(&self, name: &str) -> bool {
        self.flags.iter().any(|(n, _)| n == name)
    }
}

/// Parse arguments with subcommand support.
pub fn parse_with_subcommands(
    args: &[String],
    subcommands: &[Subcommand],
) -> Result<ParseResult, FlagsError> {
    let mut result = ParseResult::new();
    let mut i = 1; // Skip program name

    while i < args.len() {
        let arg = &args[i];

        // Check for subcommand
        let subcommand = subcommands.iter().find(|s| s.name == arg.as_str());
        if let Some(sc) = subcommand {
            result.subcommand = Some(sc.name.clone());
            i += 1;
            continue;
        }

        // Check for flag
        if arg.starts_with("--") {
            if let Some(name) = parse_flag_name(arg) {
                if let Some(value) = parse_flag_value(arg) {
                    result.flags.push((name.to_string(), value.to_string()));
                } else {
                    // Boolean flag
                    result.flags.push((name.to_string(), "true".to_string()));
                }
            }
        } else if arg.starts_with('-') && !arg.starts_with("--") {
            if let Some(name) = parse_flag_name(arg) {
                if let Some(value) = parse_flag_value(arg) {
                    result.flags.push((name.to_string(), value.to_string()));
                } else if i + 1 < args.len() && !args[i + 1].starts_with('-') {
                    i += 1;
                    result.flags.push((name.to_string(), args[i].clone()));
                } else {
                    result.flags.push((name.to_string(), "true".to_string()));
                }
            }
        } else {
            result.positionals.push(arg.clone());
        }

        i += 1;
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_flag_aliases() {
        let aliases = FlagAliases::new()
            .add("v", "verbose")
            .add("h", "help");

        assert_eq!(aliases.resolve("v"), Some("verbose"));
        assert_eq!(aliases.resolve("h"), Some("help"));
        assert_eq!(aliases.resolve("unknown"), None);
        assert!(aliases.is_alias("v"));
        assert!(!aliases.is_alias("verbose"));
    }

    #[test]
    fn test_parse_result() {
        let mut result = ParseResult::new();
        result.flags.push(("verbose".to_string(), "true".to_string()));
        result.flags.push(("output".to_string(), "file.txt".to_string()));
        result.positionals.push("input.txt".to_string());
        result.subcommand = Some("build".to_string());

        assert!(result.has_flag("verbose"));
        assert_eq!(result.get_flag("output"), Some("file.txt"));
        assert_eq!(result.get_flag("verbose"), Some("true"));
        assert_eq!(result.subcommand, Some("build".to_string()));
        assert_eq!(result.positionals.len(), 1);
    }

    #[test]
    fn test_parse_with_subcommands() {
        let subcommands = vec![
            Subcommand::new("build").description("Build"),
            Subcommand::new("test").description("Test"),
        ];

        let args = vec![
            "prog".to_string(),
            "build".to_string(),
            "--verbose".to_string(),
            "input.txt".to_string(),
        ];

        let result = parse_with_subcommands(&args, &subcommands).unwrap();
        assert_eq!(result.subcommand, Some("build".to_string()));
        assert!(result.has_flag("verbose"));
        assert_eq!(result.positionals, vec!["input.txt"]);
    }

    #[test]
    fn test_parse_with_flag_value() {
        let subcommands = vec![];

        let args = vec![
            "prog".to_string(),
            "--output=file.txt".to_string(),
            "--verbose".to_string(),
        ];

        let result = parse_with_subcommands(&args, &subcommands).unwrap();
        assert_eq!(result.get_flag("output"), Some("file.txt"));
        assert!(result.has_flag("verbose"));
    }

    #[test]
    fn test_parse_with_short_flag() {
        let subcommands = vec![];

        let args = vec![
            "prog".to_string(),
            "-o".to_string(),
            "file.txt".to_string(),
        ];

        let result = parse_with_subcommands(&args, &subcommands).unwrap();
        assert_eq!(result.get_flag("o"), Some("file.txt"));
    }

    #[test]
    fn test_parse_with_short_flag_attached() {
        let subcommands = vec![];

        let args = vec![
            "prog".to_string(),
            "-ofile.txt".to_string(),
        ];

        let result = parse_with_subcommands(&args, &subcommands).unwrap();
        assert_eq!(result.get_flag("o"), Some("file.txt"));
    }
}
