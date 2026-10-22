//! Reference frames and motion vectors.

use serde::{Deserialize, Serialize};

/// Reference frame type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum RefFrame {
    /// No reference (INTRA)
    Intra = 0,
    /// Last frame
    Last = 1,
    /// Last2 frame
    Last2 = 2,
    /// Last3 frame
    Last3 = 3,
    /// Golden frame
    Golden = 4,
    /// BWD reference frame
    BwdRef = 5,
    /// ALT2 reference frame
    AltRef2 = 6,
    /// ALT reference frame
    AltRef = 7,
}

impl RefFrame {
    /// Parse from value (0-7)
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(RefFrame::Intra),
            1 => Some(RefFrame::Last),
            2 => Some(RefFrame::Last2),
            3 => Some(RefFrame::Last3),
            4 => Some(RefFrame::Golden),
            5 => Some(RefFrame::BwdRef),
            6 => Some(RefFrame::AltRef2),
            7 => Some(RefFrame::AltRef),
            _ => None,
        }
    }
}

/// Motion vector in AV1's native 1/8-sample units (spec `Mv`, dav1d `mv.x`/`mv.y`). The overlay
/// grids use quarter-sample units; `overlay_extraction::mv_extractor` converts.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MotionVector {
    /// Horizontal component (1/8-sample units)
    pub x: i32,
    /// Vertical component (1/8-sample units)
    pub y: i32,
}

impl MotionVector {
    /// Create new motion vector
    pub fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }

    /// Zero motion vector
    pub fn zero() -> Self {
        Self { x: 0, y: 0 }
    }

    /// Create from QuarterPel components
    ///
    /// Provides type-safe construction from QuarterPel newtype.
    #[inline]
    pub fn from_quarter_pel(x: crate::QuarterPel, y: crate::QuarterPel) -> Self {
        Self {
            x: x.qpel(),
            y: y.qpel(),
        }
    }

    /// Get X component as QuarterPel
    #[inline]
    #[must_use]
    pub const fn x_quarter_pel(&self) -> crate::QuarterPel {
        crate::QuarterPel::from_qpel(self.x)
    }

    /// Get Y component as QuarterPel
    #[inline]
    #[must_use]
    pub const fn y_quarter_pel(&self) -> crate::QuarterPel {
        crate::QuarterPel::from_qpel(self.y)
    }

    /// Get X component in pixels (integer-pel)
    #[inline]
    #[must_use]
    pub const fn x_pel(&self) -> i32 {
        self.x / 4
    }

    /// Get Y component in pixels (integer-pel)
    #[inline]
    #[must_use]
    pub const fn y_pel(&self) -> i32 {
        self.y / 4
    }

    /// Add motion vectors (saturating to prevent overflow with large predictor values)
    #[inline]
    #[must_use]
    pub fn add(&self, other: MotionVector) -> MotionVector {
        MotionVector::new(
            self.x.saturating_add(other.x),
            self.y.saturating_add(other.y),
        )
    }

    /// Subtract motion vectors (saturating to prevent overflow with large predictor values)
    #[inline]
    #[must_use]
    pub fn sub(&self, other: MotionVector) -> MotionVector {
        MotionVector::new(
            self.x.saturating_sub(other.x),
            self.y.saturating_sub(other.y),
        )
    }

    /// Get magnitude in quarter-pel units
    #[inline]
    #[must_use]
    pub fn magnitude_qpel(&self) -> i32 {
        // Approximate magnitude (avoiding sqrt for performance)
        self.x.abs() + self.y.abs()
    }

    /// Get magnitude in pixels (integer-pel, rounded)
    #[inline]
    #[must_use]
    pub fn magnitude_pel(&self) -> i32 {
        (self.magnitude_qpel() / 4).max(0)
    }
}
