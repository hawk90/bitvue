//! Common types for AV1 parsing
//!
//! This module provides newtype wrappers for domain-specific values
//! to provide type safety and prevent mixing of incompatible values.

use bitvue_core::BitvueError;

/// Quantization Parameter (QP)
///
/// AV1 QP values range from 0 to 255 per the AV1 specification.
/// Lower values indicate higher quality, higher values indicate lower quality.
///
/// # Type Safety
///
/// This newtype prevents:
/// - Accidentally mixing QP with other i16 values
/// - Invalid QP values outside the valid range
/// - Confusion between different QP representations (base, delta, effective)
///
/// # Examples
///
/// ```
/// use bitvue_av1::types::Qp;
///
/// // Create from valid value
/// let qp = Qp::new(32)?;
/// assert_eq!(qp.value(), 32);
///
/// // Reject invalid values
/// assert!(Qp::new(256).is_err());
/// assert!(Qp::new(-1).is_err());
/// ```
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Qp(i16);

impl Qp {
    /// Minimum valid QP value per AV1 specification
    pub const MIN: i16 = 0;

    /// Maximum valid QP value per AV1 specification
    pub const MAX: i16 = 255;

    /// Default QP value for typical content
    pub const DEFAULT: i16 = 32;

    /// Create a new Qp value, validating the range
    ///
    /// # Errors
    ///
    /// Returns `BitvueError::InvalidData` if the value is outside [0, 255]
    #[inline]
    pub fn new(value: i16) -> Result<Self, BitvueError> {
        if (Self::MIN..=Self::MAX).contains(&value) {
            Ok(Self(value))
        } else {
            Err(BitvueError::InvalidData(format!(
                "QP out of range: {} (valid: {}-{})",
                value,
                Self::MIN,
                Self::MAX
            )))
        }
    }

    /// Create a new Qp value without validation
    ///
    /// # Safety
    ///
    /// Callers must ensure the value is in range [0, 255]
    ///
    /// # Use Cases
    ///
    /// This is safe to use when:
    /// - The value comes from a trusted source (e.g., parsed from valid bitstream)
    /// - The value has already been validated
    /// - Performance is critical and validation was done earlier
    #[inline]
    #[must_use]
    pub const unsafe fn new_unchecked(value: i16) -> Self {
        Self(value)
    }

    /// Get the underlying i16 value
    #[inline]
    #[must_use]
    pub const fn value(self) -> i16 {
        self.0
    }

    /// Convert to u8 for display/storage
    ///
    /// # Panics
    ///
    /// Panics in debug mode if value > 255 (should never happen with valid Qp)
    #[inline]
    #[must_use]
    pub fn as_u8(self) -> u8 {
        debug_assert!(self.0 <= 255, "QP value {} exceeds u8::MAX", self.0);
        self.0 as u8
    }

    /// Convert to f32 for calculations (e.g., quality metrics)
    #[inline]
    #[must_use]
    pub const fn as_f32(self) -> f32 {
        self.0 as f32
    }
}

impl From<Qp> for i16 {
    #[inline]
    fn from(qp: Qp) -> Self {
        qp.0
    }
}

impl From<Qp> for f32 {
    #[inline]
    fn from(qp: Qp) -> Self {
        qp.0 as f32
    }
}

impl From<Qp> for u8 {
    #[inline]
    fn from(qp: Qp) -> Self {
        debug_assert!(qp.0 <= 255, "QP value {} exceeds u8::MAX", qp.0);
        qp.0 as u8
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_qp_valid_values() {
        assert!(Qp::new(0).is_ok());
        assert!(Qp::new(32).is_ok());
        assert!(Qp::new(255).is_ok());
    }

    #[test]
    fn test_qp_invalid_values() {
        assert!(Qp::new(-1).is_err());
        assert!(Qp::new(256).is_err());
        assert!(Qp::new(i16::MIN).is_err());
        assert!(Qp::new(i16::MAX).is_err());
    }

    #[test]
    fn test_qp_value_accessors() {
        let qp = Qp::new(100).unwrap();
        assert_eq!(qp.value(), 100);
        assert_eq!(qp.as_u8(), 100);
        assert_eq!(qp.as_f32(), 100.0);
    }

    #[test]
    fn test_qp_conversions() {
        let qp = Qp::new(64).unwrap();
        assert_eq!(i16::from(qp), 64);
        assert_eq!(u8::from(qp), 64);
        assert_eq!(f32::from(qp), 64.0);
    }

    #[test]
    fn test_qp_ord() {
        let qp1 = Qp::new(20).unwrap();
        let qp2 = Qp::new(40).unwrap();
        assert!(qp1 < qp2);
        assert_eq!(qp1, Qp::new(20).unwrap());
    }

    #[test]
    fn test_qp_copy_clone() {
        let qp = Qp::new(50).unwrap();
        let qp_copy = qp;
        assert_eq!(qp, qp_copy);
        let qp_clone = qp_copy.clone();
        assert_eq!(qp, qp_clone);
    }

    #[test]
    fn test_qp_unchecked() {
        // Safe because 100 is in valid range
        let qp = unsafe { Qp::new_unchecked(100) };
        assert_eq!(qp.value(), 100);
    }

    #[test]
    fn test_qp_constants() {
        assert_eq!(Qp::MIN, 0);
        assert_eq!(Qp::MAX, 255);
        assert_eq!(Qp::DEFAULT, 32);
    }
}
