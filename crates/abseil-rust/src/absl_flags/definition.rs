//! FlagDefinition - A flag definition with metadata

/// A flag definition with metadata.
#[derive(Clone)]
pub struct FlagDefinition {
    pub name: String,
    pub short: Option<char>,
    pub description: String,
    pub default_value: String,
    pub required: bool,
    pub category: Option<String>,
    pub env_var: Option<String>,
}

impl FlagDefinition {
    /// Creates a new flag definition.
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            short: None,
            description: String::new(),
            default_value: String::new(),
            required: false,
            category: None,
            env_var: None,
        }
    }

    /// Sets the short flag.
    pub fn short(mut self, c: char) -> Self {
        self.short = Some(c);
        self
    }

    /// Sets the description.
    pub fn description(mut self, desc: &str) -> Self {
        self.description = desc.to_string();
        self
    }

    /// Sets the default value.
    pub fn default(mut self, value: &str) -> Self {
        self.default_value = value.to_string();
        self
    }

    /// Marks the flag as required.
    pub fn required(mut self) -> Self {
        self.required = true;
        self
    }

    /// Sets the flag category.
    pub fn category(mut self, cat: &str) -> Self {
        self.category = Some(cat.to_string());
        self
    }

    /// Sets the environment variable name.
    pub fn env(mut self, var: &str) -> Self {
        self.env_var = Some(var.to_string());
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_flag_definition() {
        let flag = FlagDefinition::new("verbose")
            .short('v')
            .description("Enable verbose output")
            .default("false")
            .category("Logging");

        assert_eq!(flag.name, "verbose");
        assert_eq!(flag.short, Some('v'));
        assert_eq!(flag.description, "Enable verbose output");
        assert_eq!(flag.default_value, "false");
        assert_eq!(flag.category, Some("Logging".to_string()));
    }

    #[test]
    fn test_flag_definition_required() {
        let flag = FlagDefinition::new("output")
            .description("Output file")
            .default("out.txt")
            .required();

        assert!(flag.required);
    }

    #[test]
    fn test_flag_definition_env_var() {
        let flag = FlagDefinition::new("config")
            .description("Config file")
            .env("APP_CONFIG");

        assert_eq!(flag.env_var, Some("APP_CONFIG".to_string()));
    }
}
