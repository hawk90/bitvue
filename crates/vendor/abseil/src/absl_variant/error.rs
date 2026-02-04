//! VariantError - Errors that can occur during variant operations.

use alloc::string::String;
use core::fmt;

/// Errors that can occur during variant operations.
#[derive(Clone, Debug, PartialEq)]
pub enum VariantError {
    /// Type mismatch error.
    TypeMismatch {
        expected: String,
        found: String,
    },
    /// Invalid float value (e.g., NaN).
    InvalidFloat(String),
    /// Conversion error.
    ConversionError(String),
    /// Empty variant error.
    EmptyVariant,
    /// Index out of bounds.
    IndexError {
        index: usize,
        max: usize,
    },
}

impl fmt::Display for VariantError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            VariantError::TypeMismatch { expected, found } => {
                write!(f, "Type mismatch: expected {}, found {}", expected, found)
            }
            VariantError::InvalidFloat(msg) => write!(f, "Invalid float value: {}", msg),
            VariantError::ConversionError(msg) => write!(f, "Conversion error: {}", msg),
            VariantError::EmptyVariant => write!(f, "Empty variant"),
            VariantError::IndexError { index, max } => {
                write!(f, "Index {} out of bounds (max {})", index, max)
            }
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for VariantError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_variant_error_display() {
        let err = VariantError::TypeMismatch {
            expected: "i32".to_string(),
            found: "string".to_string(),
        };
        assert_eq!(format!("{}", err), "Type mismatch: expected i32, found string");

        let err = VariantError::InvalidFloat("NaN".to_string());
        assert_eq!(format!("{}", err), "Invalid float value: NaN");

        let err = VariantError::EmptyVariant;
        assert_eq!(format!("{}", err), "Empty variant");

        let err = VariantError::IndexError { index: 5, max: 3 };
        assert_eq!(format!("{}", err), "Index 5 out of bounds (max 3)");
    }
}
