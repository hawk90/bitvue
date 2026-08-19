//! Validation Strategy - Strategy pattern for validation logic
//!
//! This module provides a strategy pattern for validating different types of data,
//! unifying validation logic across the codebase.

/// Validation result
#[derive(Debug, Clone, PartialEq)]
pub struct ValidationResult {
    /// Is valid
    pub is_valid: bool,
    /// Error message (if invalid)
    pub error_message: Option<String>,
    /// Warnings (non-critical issues)
    pub warnings: Vec<String>,
}

impl ValidationResult {
    /// Create a valid result
    pub fn valid() -> Self {
        Self {
            is_valid: true,
            error_message: None,
            warnings: Vec::new(),
        }
    }

    /// Create an invalid result with error message
    pub fn invalid(message: impl Into<String>) -> Self {
        Self {
            is_valid: false,
            error_message: Some(message.into()),
            warnings: Vec::new(),
        }
    }

    /// Add a warning to the result
    pub fn with_warning(mut self, warning: impl Into<String>) -> Self {
        self.warnings.push(warning.into());
        self
    }

    /// Combine two results
    pub fn and(mut self, other: ValidationResult) -> ValidationResult {
        self.is_valid = self.is_valid && other.is_valid;
        if other.error_message.is_some() {
            self.error_message = other.error_message;
        }
        self.warnings.extend(other.warnings);
        self
    }
}

/// Trait for validation strategies
pub trait ValidationStrategy: Send + Sync {
    /// Validate data and return result
    fn validate(&self, data: &ValidationData) -> ValidationResult;

    /// Get error message for last validation
    fn error_message(&self) -> String {
        "Validation failed".to_string()
    }
}

/// Data for validation operations
#[derive(Debug, Clone)]
pub enum ValidationData {
    /// String data
    String(String),
    /// Byte array
    Bytes(Vec<u8>),
    /// Integer value
    Integer(i64),
    /// Unsigned integer
    Unsigned(u64),
    /// Float value
    Float(f64),
    /// Frame key
    FrameKey { stream_id: u8, frame_index: u64 },
    /// Byte range
    ByteRange { start: u64, end: u64 },
    /// Custom data with type name
    Custom { type_name: String, data: String },
}

// =============================================================================
// Strict Validation Strategy
// =============================================================================

/// Strict validation - all rules must pass
#[derive(Debug, Clone, Default)]
pub struct StrictValidationStrategy;

impl ValidationStrategy for StrictValidationStrategy {
    fn validate(&self, data: &ValidationData) -> ValidationResult {
        match data {
            ValidationData::String(s) => {
                if s.is_empty() {
                    return ValidationResult::invalid("String cannot be empty");
                }
                ValidationResult::valid()
            }
            ValidationData::Bytes(b) => {
                if b.is_empty() {
                    return ValidationResult::invalid("Bytes cannot be empty");
                }
                ValidationResult::valid()
            }
            ValidationData::Integer(i) => {
                if *i < 0 {
                    return ValidationResult::invalid("Integer cannot be negative");
                }
                ValidationResult::valid()
            }
            ValidationData::Unsigned(u) => {
                if *u == 0 {
                    return ValidationResult::invalid("Unsigned integer cannot be zero");
                }
                ValidationResult::valid()
            }
            ValidationData::Float(f) => {
                if f.is_nan() || f.is_infinite() {
                    return ValidationResult::invalid("Float must be finite");
                }
                ValidationResult::valid()
            }
            ValidationData::FrameKey {
                stream_id,
                frame_index: _,
            } => {
                if *stream_id > 1 {
                    return ValidationResult::invalid("Stream ID must be 0 or 1");
                }
                ValidationResult::valid()
            }
            ValidationData::ByteRange { start, end } => {
                if start >= end {
                    return ValidationResult::invalid("Byte range start must be less than end");
                }
                ValidationResult::valid()
            }
            ValidationData::Custom { type_name, .. } => {
                ValidationResult::invalid(format!("Unknown type: {}", type_name))
            }
        }
    }

    fn error_message(&self) -> String {
        "Strict validation failed".to_string()
    }
}

// =============================================================================
// Lenient Validation Strategy
// =============================================================================

/// Lenient validation - some rules can pass
#[derive(Debug, Clone, Default)]
pub struct LenientValidationStrategy;

impl ValidationStrategy for LenientValidationStrategy {
    fn validate(&self, data: &ValidationData) -> ValidationResult {
        match data {
            ValidationData::String(s) => {
                if s.len() > 1000000 {
                    return ValidationResult::invalid("String too long")
                        .with_warning("String length exceeds 1MB");
                }
                ValidationResult::valid()
            }
            ValidationData::Bytes(b) => {
                if b.len() > 10_000_000 {
                    return ValidationResult::invalid("Bytes too large")
                        .with_warning("Byte size exceeds 10MB");
                }
                ValidationResult::valid()
            }
            ValidationData::Integer(_) => ValidationResult::valid(),
            ValidationData::Unsigned(_) => ValidationResult::valid(),
            ValidationData::Float(f) => {
                if f.is_nan() || f.is_infinite() {
                    return ValidationResult::invalid("Float must be finite");
                }
                ValidationResult::valid()
            }
            ValidationData::FrameKey { .. } => ValidationResult::valid(),
            ValidationData::ByteRange { start, end } => {
                if start >= end {
                    return ValidationResult::invalid("Byte range start must be less than end")
                        .with_warning("Consider swapping start and end values");
                }
                ValidationResult::valid()
            }
            ValidationData::Custom { type_name, .. } => {
                ValidationResult::invalid(format!("Unknown type: {}", type_name))
            }
        }
    }

    fn error_message(&self) -> String {
        "Lenient validation failed".to_string()
    }
}

// =============================================================================
// Permissive Validation Strategy
// =============================================================================

/// Permissive validation - minimal validation
#[derive(Debug, Clone, Default)]
pub struct PermissiveValidationStrategy;

impl ValidationStrategy for PermissiveValidationStrategy {
    fn validate(&self, data: &ValidationData) -> ValidationResult {
        match data {
            ValidationData::Custom { type_name, .. } => {
                ValidationResult::invalid(format!("Unknown type: {}", type_name))
            }
            _ => ValidationResult::valid(),
        }
    }

    fn error_message(&self) -> String {
        "Permissive validation failed".to_string()
    }
}

/// Factory for creating validation strategies
pub struct ValidationStrategyFactory;

impl ValidationStrategyFactory {
    /// Create a strategy by strictness level
    pub fn create(strictness: ValidationStrictness) -> Box<dyn ValidationStrategy> {
        match strictness {
            ValidationStrictness::Strict => Box::new(StrictValidationStrategy),
            ValidationStrictness::Lenient => Box::new(LenientValidationStrategy),
            ValidationStrictness::Permissive => Box::new(PermissiveValidationStrategy),
        }
    }
}

/// Validation strictness level
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValidationStrictness {
    Strict,
    Lenient,
    Permissive,
}

// =============================================================================
// Specialized Validators
// =============================================================================

/// Frame validator
#[derive(Debug, Clone, Default)]
pub struct FrameValidator {
    max_frame_index: u64,
}

impl FrameValidator {
    pub fn new(max_frame_index: u64) -> Self {
        Self { max_frame_index }
    }

    pub fn validate_frame(&self, frame_index: u64) -> ValidationResult {
        if frame_index > self.max_frame_index {
            return ValidationResult::invalid(format!(
                "Frame index {} exceeds maximum {}",
                frame_index, self.max_frame_index
            ));
        }
        ValidationResult::valid()
    }
}

/// Byte range validator
#[derive(Debug, Clone)]
pub struct ByteRangeValidator {
    max_size: u64,
}

impl ByteRangeValidator {
    pub fn new(max_size: u64) -> Self {
        Self { max_size }
    }

    pub fn validate_range(&self, start: u64, end: u64) -> ValidationResult {
        if start >= end {
            return ValidationResult::invalid("Range start must be less than end");
        }
        let size = end - start;
        if size > self.max_size {
            return ValidationResult::invalid(format!(
                "Range size {} exceeds maximum {}",
                size, self.max_size
            ));
        }
        ValidationResult::valid()
    }
}

/// Stream data validator
pub struct StreamDataValidator {
    strategy: Box<dyn ValidationStrategy>,
}

impl StreamDataValidator {
    pub fn new(strategy: Box<dyn ValidationStrategy>) -> Self {
        Self { strategy }
    }

    pub fn validate_frame_key(&self, stream_id: u8, frame_index: u64) -> ValidationResult {
        let data = ValidationData::FrameKey {
            stream_id,
            frame_index,
        };
        self.strategy.validate(&data)
    }

    pub fn validate_byte_range(&self, start: u64, end: u64) -> ValidationResult {
        let data = ValidationData::ByteRange { start, end };
        self.strategy.validate(&data)
    }
}

// =============================================================================
// Validation Chain
// =============================================================================

/// Chain of validators
pub struct ValidationChain {
    validators: Vec<Box<dyn ValidationStrategy>>,
}

impl ValidationChain {
    pub fn new() -> Self {
        Self {
            validators: Vec::new(),
        }
    }

    #[allow(clippy::should_implement_trait)]
    pub fn add(mut self, validator: Box<dyn ValidationStrategy>) -> Self {
        self.validators.push(validator);
        self
    }

    pub fn validate(&self, data: &ValidationData) -> ValidationResult {
        let mut result = ValidationResult::valid();

        for validator in &self.validators {
            let validator_result = validator.validate(data);
            result = result.and(validator_result);
            if !result.is_valid {
                return result;
            }
        }

        result
    }
}

impl Default for ValidationChain {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strict_validation_empty_string() {
        let strategy = StrictValidationStrategy;
        let data = ValidationData::String("".to_string());
        let result = strategy.validate(&data);
        assert!(!result.is_valid);
    }

    #[test]
    fn test_strict_validation_valid_string() {
        let strategy = StrictValidationStrategy;
        let data = ValidationData::String("valid".to_string());
        let result = strategy.validate(&data);
        assert!(result.is_valid);
    }

    #[test]
    fn test_lenient_validation_long_string() {
        let strategy = LenientValidationStrategy;
        let data = ValidationData::String("a".repeat(1_000_001));
        let result = strategy.validate(&data);
        assert!(!result.is_valid);
        assert!(!result.warnings.is_empty());
    }

    #[test]
    fn test_permissive_validation() {
        let strategy = PermissiveValidationStrategy;
        let data = ValidationData::String("".to_string());
        let result = strategy.validate(&data);
        assert!(result.is_valid);
    }

    #[test]
    fn test_validation_result_with_warning() {
        let result = ValidationResult::valid().with_warning("Test warning");
        assert!(result.is_valid);
        assert_eq!(result.warnings.len(), 1);
    }

    #[test]
    fn test_validation_result_and() {
        let result1 = ValidationResult::valid();
        let result2 = ValidationResult::invalid("Error");
        let combined = result1.and(result2);
        assert!(!combined.is_valid);
        assert_eq!(combined.error_message, Some("Error".to_string()));
    }

    #[test]
    fn test_frame_key_validation() {
        let strategy = StrictValidationStrategy;
        let data = ValidationData::FrameKey {
            stream_id: 0,
            frame_index: 100,
        };
        let result = strategy.validate(&data);
        assert!(result.is_valid);
    }

    #[test]
    fn test_frame_key_invalid_stream() {
        let strategy = StrictValidationStrategy;
        let data = ValidationData::FrameKey {
            stream_id: 2,
            frame_index: 100,
        };
        let result = strategy.validate(&data);
        assert!(!result.is_valid);
    }

    #[test]
    fn test_byte_range_validation() {
        let strategy = StrictValidationStrategy;
        let data = ValidationData::ByteRange { start: 0, end: 100 };
        let result = strategy.validate(&data);
        assert!(result.is_valid);
    }

    #[test]
    fn test_byte_range_invalid() {
        let strategy = StrictValidationStrategy;
        let data = ValidationData::ByteRange {
            start: 100,
            end: 50,
        };
        let result = strategy.validate(&data);
        assert!(!result.is_valid);
    }

    #[test]
    fn test_factory_strict() {
        let strategy = ValidationStrategyFactory::create(ValidationStrictness::Strict);
        let data = ValidationData::String("test".to_string());
        assert!(strategy.validate(&data).is_valid);
    }

    #[test]
    fn test_factory_lenient() {
        let strategy = ValidationStrategyFactory::create(ValidationStrictness::Lenient);
        let data = ValidationData::String("test".to_string());
        assert!(strategy.validate(&data).is_valid);
    }

    #[test]
    fn test_factory_permissive() {
        let strategy = ValidationStrategyFactory::create(ValidationStrictness::Permissive);
        let data = ValidationData::String("".to_string());
        assert!(strategy.validate(&data).is_valid);
    }

    #[test]
    fn test_frame_validator() {
        let validator = FrameValidator::new(1000);
        assert!(validator.validate_frame(500).is_valid);
        assert!(!validator.validate_frame(2000).is_valid);
    }

    #[test]
    fn test_byte_range_validator() {
        let validator = ByteRangeValidator::new(10000);
        assert!(validator.validate_range(0, 1000).is_valid);
        assert!(!validator.validate_range(1000, 0).is_valid);
        assert!(!validator.validate_range(0, 20000).is_valid);
    }

    #[test]
    fn test_stream_data_validator() {
        let strategy = Box::new(StrictValidationStrategy);
        let validator = StreamDataValidator::new(strategy);
        assert!(validator.validate_frame_key(0, 100).is_valid);
        assert!(!validator.validate_frame_key(2, 100).is_valid);
    }

    #[test]
    fn test_validation_chain() {
        let chain = ValidationChain::new()
            .add(Box::new(StrictValidationStrategy))
            .add(Box::new(LenientValidationStrategy));

        let data = ValidationData::String("valid".to_string());
        assert!(chain.validate(&data).is_valid);
    }

    #[test]
    fn test_float_validation_strict() {
        let strategy = StrictValidationStrategy;
        assert!(strategy.validate(&ValidationData::Float(42.0)).is_valid);
        assert!(!strategy.validate(&ValidationData::Float(f64::NAN)).is_valid);
        assert!(
            !strategy
                .validate(&ValidationData::Float(f64::INFINITY))
                .is_valid
        );
    }
}
