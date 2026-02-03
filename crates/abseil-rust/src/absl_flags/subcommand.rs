//! Subcommand and CommandParser - Command with subcommands support

use super::definition::FlagDefinition;

/// A subcommand definition.
#[derive(Clone)]
pub struct Subcommand {
    pub name: String,
    pub description: String,
    pub flags: Vec<FlagDefinition>,
}

impl Subcommand {
    /// Creates a new subcommand.
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            description: String::new(),
            flags: Vec::new(),
        }
    }

    /// Sets the description.
    pub fn description(mut self, desc: &str) -> Self {
        self.description = desc.to_string();
        self
    }

    /// Adds a flag to this subcommand.
    pub fn add_flag(mut self, flag: FlagDefinition) -> Self {
        self.flags.push(flag);
        self
    }
}

/// Parser for commands with subcommands.
pub struct CommandParser {
    program_name: String,
    program_version: String,
    program_description: String,
    global_flags: Vec<FlagDefinition>,
    subcommands: Vec<Subcommand>,
}

impl Default for CommandParser {
    fn default() -> Self {
        Self::new()
    }
}

impl CommandParser {
    /// Creates a new command parser.
    pub fn new() -> Self {
        Self {
            program_name: String::new(),
            program_version: String::new(),
            program_description: String::new(),
            global_flags: Vec::new(),
            subcommands: Vec::new(),
        }
    }

    /// Sets the program name.
    pub fn name(mut self, name: &str) -> Self {
        self.program_name = name.to_string();
        self
    }

    /// Sets the program version.
    pub fn version(mut self, version: &str) -> Self {
        self.program_version = version.to_string();
        self
    }

    /// Sets the program description.
    pub fn description(mut self, desc: &str) -> Self {
        self.program_description = desc.to_string();
        self
    }

    /// Adds a global flag.
    pub fn add_global_flag(mut self, flag: FlagDefinition) -> Self {
        self.global_flags.push(flag);
        self
    }

    /// Adds a subcommand.
    pub fn add_subcommand(mut self, cmd: Subcommand) -> Self {
        self.subcommands.push(cmd);
        self
    }

    /// Generates help text.
    pub fn help_text(&self) -> String {
        let mut help = String::new();

        // Program banner
        if !self.program_name.is_empty() {
            help.push_str(&format!("{} {}\n", self.program_name, self.program_version));
            if !self.program_description.is_empty() {
                help.push_str(&format!("{}\n", self.program_description));
            }
            help.push('\n');
        }

        // Usage
        help.push_str("USAGE:\n");
        if self.subcommands.is_empty() {
            help.push_str(&format!("    {} [OPTIONS]\n", self.program_name));
        } else {
            help.push_str(&format!("    {} [OPTIONS] <COMMAND>\n", self.program_name));
            help.push('\n');
            help.push_str("COMMANDS:\n");
            for cmd in &self.subcommands {
                help.push_str(&format!("    {:20} {}\n", cmd.name, cmd.description));
            }
        }

        // Global flags
        if !self.global_flags.is_empty() {
            help.push('\n');
            help.push_str("OPTIONS:\n");
            for flag in &self.global_flags {
                let mut flag_line = String::new();
                if let Some(short) = flag.short {
                    if short != '\0' {
                        flag_line.push_str(&format!("-{}, ", short));
                    }
                }
                flag_line.push_str(&format!("--{}", flag.name));
                help.push_str(&format!("    {:30} {}\n", flag_line, flag.description));
            }
        }

        help
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_subcommand() {
        let cmd = Subcommand::new("build")
            .description("Build the project")
            .add_flag(FlagDefinition::new("release").description("Build in release mode"));

        assert_eq!(cmd.name, "build");
        assert_eq!(cmd.flags.len(), 1);
    }

    #[test]
    fn test_command_parser_help_text() {
        let parser = CommandParser::new()
            .name("myapp")
            .version("1.0.0")
            .description("A test application")
            .add_global_flag(FlagDefinition::new("verbose").short('v'))
            .add_subcommand(
                Subcommand::new("build").description("Build the project"),
            )
            .add_subcommand(
                Subcommand::new("test").description("Run tests"),
            );

        let help = parser.help_text();
        assert!(help.contains("myapp"));
        assert!(help.contains("1.0.0"));
        assert!(help.contains("build"));
        assert!(help.contains("test"));
    }
}
