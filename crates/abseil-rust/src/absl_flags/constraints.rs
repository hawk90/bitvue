//! Flag constraints - FlagConstraint trait, RangeConstraint, ChoiceConstraint

/// A flag constraint that validates flag values.
pub trait FlagConstraint<T>: Send + Sync {
    /// Validates a flag value.
    fn validate(&self, value: &T) -> Result<(), String>;

    /// Gets the error message for this constraint.
    fn error_message(&self) -> &str {
        "constraint validation failed"
    }
}

/// A range constraint for numeric flags.
#[derive(Clone, Debug)]
pub struct RangeConstraint<T> {
    min: Option<T>,
    max: Option<T>,
    message: String,
}

impl<T> RangeConstraint<T> {
    /// Creates a new range constraint.
    pub fn new() -> Self {
        Self {
            min: None,
            max: None,
            message: String::new(),
        }
    }

    /// Sets the minimum value.
    pub fn min(mut self, value: T) -> Self {
        self.min = Some(value);
        self
    }

    /// Sets the maximum value.
    pub fn max(mut self, value: T) -> Self {
        self.max = Some(value);
        self
    }

    /// Sets the error message.
    pub fn message(mut self, msg: String) -> Self {
        self.message = msg;
        self
    }
}

impl<T: Default> Default for RangeConstraint<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl FlagConstraint<i64> for RangeConstraint<i64> {
    fn validate(&self, value: &i64) -> Result<(), String> {
        if let Some(min) = self.min {
            if *value < min {
                return Err(format!("value must be at least {}", min));
            }
        }
        if let Some(max) = self.max {
            if *value > max {
                return Err(format!("value must be at most {}", max));
            }
        }
        Ok(())
    }

    fn error_message(&self) -> &str {
        if !self.message.is_empty() {
            &self.message
        } else {
            "value out of range"
        }
    }
}

/// A choice constraint for enum-like flags.
#[derive(Clone, Debug)]
pub struct ChoiceConstraint {
    choices: Vec<String>,
    case_sensitive: bool,
}

impl ChoiceConstraint {
    /// Creates a new choice constraint.
    pub fn new(choices: &[&str]) -> Self {
        Self {
            choices: choices.iter().map(|s| s.to_string()).collect(),
            case_sensitive: true,
        }
    }

    /// Creates a case-insensitive choice constraint.
    pub fn case_insensitive(mut self) -> Self {
        self.case_sensitive = false;
        self
    }
}

impl FlagConstraint<String> for ChoiceConstraint {
    fn validate(&self, value: &String) -> Result<(), String> {
        if self.case_sensitive {
            if self.choices.contains(value) {
                Ok(())
            } else {
                Err(format!("value must be one of: {}", self.choices.join(", ")))
            }
        } else {
            let lower = value.to_lowercase();
            if self.choices.iter().any(|c| c.to_lowercase() == lower) {
                Ok(())
            } else {
                Err(format!("value must be one of: {}", self.choices.join(", ")))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_range_constraint() {
        let constraint = RangeConstraint::<i64>::new()
            .min(0)
            .max(100);

        assert!(constraint.validate(&50).is_ok());
        assert!(constraint.validate(&0).is_ok());
        assert!(constraint.validate(&100).is_ok());

        assert!(constraint.validate(&-1).is_err());
        assert!(constraint.validate(&101).is_err());
    }

    #[test]
    fn test_choice_constraint() {
        let constraint = ChoiceConstraint::new(&["small", "medium", "large"]);

        assert!(constraint.validate(&"medium".to_string()).is_ok());
        assert!(constraint.validate(&"small".to_string()).is_ok());
        assert!(constraint.validate(&"extra".to_string()).is_err());
    }

    #[test]
    fn test_choice_constraint_case_insensitive() {
        let constraint = ChoiceConstraint::new(&["small", "medium", "large"]).case_insensitive();

        assert!(constraint.validate(&"MEDIUM".to_string()).is_ok());
        assert!(constraint.validate(&"Small".to_string()).is_ok());
    }
}
