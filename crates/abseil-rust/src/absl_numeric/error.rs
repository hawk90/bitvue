//! Numeric error types.

use core::fmt;

/// Errors that can occur during numeric operations.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NumericError {
    /// Overflow during arithmetic operation.
    Overflow,
    /// Underflow during arithmetic operation.
    Underflow,
    /// Division by zero.
    DivisionByZero,
    /// Invalid numeric conversion.
    InvalidConversion,
}

impl fmt::Display for NumericError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NumericError::Overflow => write!(f, "Numeric overflow"),
            NumericError::Underflow => write!(f, "Numeric underflow"),
            NumericError::DivisionByZero => write!(f, "Division by zero"),
            NumericError::InvalidConversion => write!(f, "Invalid numeric conversion"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for NumericError {}
