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
/// use bitvue_av1_codec::types::Qp;
///
/// // Create from valid value
/// let qp = Qp::new(32).unwrap();
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

/// Quarter-Pel Motion Vector Component
///
/// AV1 motion vectors are specified in quarter-pel (quarter-pixel) precision.
/// Each unit represents 1/4 of a pixel sample.
///
/// # Type Safety
///
/// This newtype prevents:
/// - Accidentally mixing motion vectors with other i32 values
/// - Using integer-pel values where quarter-pel is expected
/// - Confusion about motion vector precision
///
/// # Examples
///
/// ```
/// use bitvue_av1_codec::types::QuarterPel;
///
/// // Create from quarter-pel value
/// let mv = QuarterPel::from_qpel(8);  // 2 pixels
/// assert_eq!(mv.qpel(), 8);
/// assert_eq!(mv.pel(), 2);
///
/// // Create from pixel value
/// let mv = QuarterPel::from_pel(3);  // 12 quarter-pels
/// assert_eq!(mv.qpel(), 12);
/// ```
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct QuarterPel(i32);

impl QuarterPel {
    /// Zero motion vector (no motion)
    pub const ZERO: QuarterPel = QuarterPel(0);

    /// Create from quarter-pel value
    ///
    /// # Arguments
    ///
    /// * `qpel` - Motion in quarter-pel units (1/4 pixel)
    #[inline]
    #[must_use]
    pub const fn from_qpel(qpel: i32) -> Self {
        Self(qpel)
    }

    /// Create from pixel (integer-pel) value
    ///
    /// # Arguments
    ///
    /// * `pel` - Motion in pixel units
    #[inline]
    #[must_use]
    pub const fn from_pel(pel: i32) -> Self {
        Self(pel * 4)
    }

    /// Get value in quarter-pel units
    #[inline]
    #[must_use]
    pub const fn qpel(self) -> i32 {
        self.0
    }

    /// Get value in pixel (integer-pel) units
    ///
    /// # Note
    ///
    /// This truncates towards zero. For exact pixel values,
    /// ensure the quarter-pel value is a multiple of 4.
    #[inline]
    #[must_use]
    pub const fn pel(self) -> i32 {
        self.0 / 4
    }

    /// Get value in half-pel units
    #[inline]
    #[must_use]
    pub const fn hpel(self) -> i32 {
        self.0 / 2
    }

    /// Add two quarter-pel values
    #[inline]
    #[must_use]
    pub const fn add(self, other: QuarterPel) -> QuarterPel {
        QuarterPel(self.0 + other.0)
    }

    /// Subtract two quarter-pel values
    #[inline]
    #[must_use]
    pub const fn sub(self, other: QuarterPel) -> QuarterPel {
        QuarterPel(self.0 - other.0)
    }

    /// Absolute value of motion vector
    #[inline]
    #[must_use]
    pub fn abs(self) -> QuarterPel {
        QuarterPel(self.0.abs())
    }
}

impl From<i32> for QuarterPel {
    #[inline]
    fn from(value: i32) -> Self {
        // Assume i32 values in context are quarter-pel
        Self::from_qpel(value)
    }
}

impl From<QuarterPel> for i32 {
    #[inline]
    fn from(qp: QuarterPel) -> Self {
        qp.0
    }
}

impl std::ops::Add for QuarterPel {
    type Output = QuarterPel;

    #[inline]
    fn add(self, other: Self) -> Self::Output {
        self.add(other)
    }
}

impl std::ops::Sub for QuarterPel {
    type Output = QuarterPel;

    #[inline]
    fn sub(self, other: Self) -> Self::Output {
        self.sub(other)
    }
}

impl std::ops::Neg for QuarterPel {
    type Output = QuarterPel;

    #[inline]
    fn neg(self) -> Self::Output {
        QuarterPel(-self.0)
    }
}

/// Presentation Timestamp (PTS)
///
/// Represents the presentation timestamp of a frame in media.
/// PTS values must be non-negative as they represent time offsets.
///
/// # Type Safety
///
/// This newtype prevents:
/// - Accidentally mixing timestamps with other i64 values
/// - Negative timestamp values (invalid per media specifications)
/// - Confusion between PTS and DTS (Decode Timestamp)
///
/// # Examples
///
/// ```
/// use bitvue_av1_codec::types::TimestampPts;
///
/// // Create from valid value
/// let pts = TimestampPts::new(1000).unwrap();
/// assert_eq!(pts.value(), 1000);
///
/// // Reject negative values
/// assert!(TimestampPts::new(-1).is_err());
/// ```
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TimestampPts(i64);

impl TimestampPts {
    /// Zero timestamp (first frame)
    pub const ZERO: TimestampPts = TimestampPts(0);

    /// Create a new timestamp, validating it's non-negative
    ///
    /// # Errors
    ///
    /// Returns `BitvueError::InvalidData` if the value is negative
    #[inline]
    pub fn new(value: i64) -> Result<Self, BitvueError> {
        if value >= 0 {
            Ok(Self(value))
        } else {
            Err(BitvueError::InvalidData(format!(
                "Invalid PTS: {} (must be >= 0)",
                value
            )))
        }
    }

    /// Create a new timestamp without validation
    ///
    /// # Safety
    ///
    /// Callers must ensure the value is non-negative
    #[inline]
    #[must_use]
    pub const unsafe fn new_unchecked(value: i64) -> Self {
        Self(value)
    }

    /// Get the underlying i64 value
    #[inline]
    #[must_use]
    pub const fn value(self) -> i64 {
        self.0
    }

    /// Check if this is the first frame (zero timestamp)
    #[inline]
    #[must_use]
    pub const fn is_zero(self) -> bool {
        self.0 == 0
    }
}

impl From<TimestampPts> for i64 {
    #[inline]
    fn from(pts: TimestampPts) -> Self {
        pts.0
    }
}

#[cfg(test)]
mod quarter_pel_tests {
    use super::*;

    #[test]
    fn test_quarter_pel_from_qpel() {
        let mv = QuarterPel::from_qpel(8);
        assert_eq!(mv.qpel(), 8);
        assert_eq!(mv.pel(), 2);
        assert_eq!(mv.hpel(), 4);
    }

    #[test]
    fn test_quarter_pel_from_pel() {
        let mv = QuarterPel::from_pel(3);
        assert_eq!(mv.pel(), 3);
        assert_eq!(mv.qpel(), 12);
        assert_eq!(mv.hpel(), 6);
    }

    #[test]
    fn test_quarter_pel_zero() {
        assert_eq!(QuarterPel::ZERO.qpel(), 0);
        assert_eq!(QuarterPel::ZERO.pel(), 0);
    }

    #[test]
    fn test_quarter_pel_arithmetic() {
        let mv1 = QuarterPel::from_qpel(8);
        let mv2 = QuarterPel::from_qpel(4);

        let sum = mv1.add(mv2);
        assert_eq!(sum.qpel(), 12);

        let diff = mv1.sub(mv2);
        assert_eq!(diff.qpel(), 4);
    }

    #[test]
    fn test_quarter_pel_abs() {
        let mv = QuarterPel::from_qpel(-8);
        assert_eq!(mv.abs().qpel(), 8);
    }

    #[test]
    fn test_quarter_pel_neg() {
        let mv = QuarterPel::from_qpel(8);
        let neg = -mv;
        assert_eq!(neg.qpel(), -8);
    }

    #[test]
    fn test_quarter_pel_add_operator() {
        let mv1 = QuarterPel::from_qpel(5);
        let mv2 = QuarterPel::from_qpel(3);
        assert_eq!((mv1 + mv2).qpel(), 8);
    }

    #[test]
    fn test_quarter_pel_sub_operator() {
        let mv1 = QuarterPel::from_qpel(10);
        let mv2 = QuarterPel::from_qpel(3);
        assert_eq!((mv1 - mv2).qpel(), 7);
    }

    #[test]
    fn test_quarter_pel_conversions() {
        let mv = QuarterPel::from_qpel(16);
        assert_eq!(i32::from(mv), 16);
    }

    #[test]
    fn test_quarter_pel_default() {
        let mv: QuarterPel = QuarterPel::default();
        assert_eq!(mv.qpel(), 0);
    }

    #[test]
    fn test_quarter_pel_copy_clone() {
        let mv = QuarterPel::from_qpel(20);
        let mv_copy = mv;
        assert_eq!(mv, mv_copy);
        let mv_clone = mv_copy.clone();
        assert_eq!(mv, mv_clone);
    }
}

#[cfg(test)]
mod timestamp_pts_tests {
    use super::*;

    #[test]
    fn test_timestamp_pts_valid() {
        assert!(TimestampPts::new(0).is_ok());
        assert!(TimestampPts::new(1000).is_ok());
        assert!(TimestampPts::new(i64::MAX).is_ok());
    }

    #[test]
    fn test_timestamp_pts_invalid() {
        assert!(TimestampPts::new(-1).is_err());
        assert!(TimestampPts::new(-1000).is_err());
    }

    #[test]
    fn test_timestamp_pts_value() {
        let pts = TimestampPts::new(5000).unwrap();
        assert_eq!(pts.value(), 5000);
        assert!(!pts.is_zero());
    }

    #[test]
    fn test_timestamp_pts_zero() {
        assert!(TimestampPts::ZERO.is_zero());
        assert_eq!(TimestampPts::ZERO.value(), 0);
    }

    #[test]
    fn test_timestamp_pts_unchecked() {
        // Safe because 100 is non-negative
        let pts = unsafe { TimestampPts::new_unchecked(100) };
        assert_eq!(pts.value(), 100);
    }

    #[test]
    fn test_timestamp_pts_conversion() {
        let pts = TimestampPts::new(2500).unwrap();
        assert_eq!(i64::from(pts), 2500);
    }

    #[test]
    fn test_timestamp_pts_copy_clone() {
        let pts = TimestampPts::new(1000).unwrap();
        let pts_copy = pts;
        assert_eq!(pts, pts_copy);
        let pts_clone = pts_copy.clone();
        assert_eq!(pts, pts_clone);
    }

    #[test]
    fn test_timestamp_pts_ord() {
        let pts1 = TimestampPts::new(100).unwrap();
        let pts2 = TimestampPts::new(200).unwrap();
        assert!(pts1 < pts2);
    }
}
