//! FlagBuilder - Builder for defining command-line flags

use super::definition::FlagDefinition;

/// Builder for defining command-line flags.
pub struct FlagBuilder {
    flags: Vec<FlagDefinition>,
}

impl Default for FlagBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl FlagBuilder {
    /// Creates a new flag builder.
    pub fn new() -> Self {
        Self { flags: Vec::new() }
    }

    /// Adds a flag definition.
    pub fn add_flag(mut self, flag: FlagDefinition) -> Self {
        self.flags.push(flag);
        self
    }

    /// Adds a boolean flag.
    pub fn bool_flag(
        mut self,
        name: &str,
        short: Option<char>,
        description: &str,
        default_value: bool,
    ) -> Self {
        let flag = FlagDefinition::new(name)
            .short(short.unwrap_or('\0'))
            .description(description)
            .default(if default_value { "true" } else { "false" });
        self.flags.push(flag);
        self
    }

    /// Adds a string flag.
    pub fn string_flag(
        mut self,
        name: &str,
        short: Option<char>,
        description: &str,
        default_value: &str,
    ) -> Self {
        let flag = FlagDefinition::new(name)
            .short(short.unwrap_or('\0'))
            .description(description)
            .default(default_value);
        self.flags.push(flag);
        self
    }

    /// Adds an integer flag.
    pub fn int_flag(
        mut self,
        name: &str,
        short: Option<char>,
        description: &str,
        default_value: i64,
    ) -> Self {
        let flag = FlagDefinition::new(name)
            .short(short.unwrap_or('\0'))
            .description(description)
            .default(&default_value.to_string());
        self.flags.push(flag);
        self
    }

    /// Builds the flag definitions.
    pub fn build(self) -> Vec<FlagDefinition> {
        self.flags
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_flag_builder() {
        let flags = FlagBuilder::new()
            .bool_flag("verbose", Some('v'), "Verbose output", false)
            .string_flag("output", Some('o'), "Output file", "out.txt")
            .int_flag("count", None, "Iteration count", 10)
            .build();

        assert_eq!(flags.len(), 3);
        assert_eq!(flags[0].name, "verbose");
        assert_eq!(flags[1].name, "output");
        assert_eq!(flags[2].name, "count");
    }
}
