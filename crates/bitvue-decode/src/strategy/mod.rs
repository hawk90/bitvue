//! YUV to RGB conversion strategies using Strategy Pattern
//!
//! This module provides a pluggable architecture for YUV to RGB conversion
//! with automatic platform detection and optimal strategy selection.

mod registry;
mod scalar;

#[cfg(target_arch = "x86_64")]
mod avx2;

#[cfg(target_arch = "aarch64")]
mod neon;

#[cfg(target_os = "macos")]
mod metal;

pub use registry::{StrategyRegistry, best_strategy_type, current_strategy_type, set_strategy, available_strategies, StrategyType};
pub use scalar::ScalarStrategy;

#[cfg(target_arch = "x86_64")]
pub use avx2::Avx2Strategy;

#[cfg(target_arch = "aarch64")]
pub use neon::NeonStrategy;

// ============================================================================
// Core Trait Definitions
// ============================================================================

/// Capabilities advertised by a conversion strategy
#[derive(Debug, Clone, Copy)]
pub struct StrategyCapabilities {
    /// Expected speedup factor compared to scalar (1.0 = baseline)
    pub speedup_factor: f32,

    /// Whether this strategy supports 10-bit video
    pub supports_10bit: bool,

    /// Whether this strategy supports 12-bit video
    pub supports_12bit: bool,

    /// Maximum supported frame dimensions
    pub max_resolution: (u32, u32),

    /// Whether this strategy is hardware-accelerated (GPU, etc.)
    pub is_hardware_accelerated: bool,
}

impl StrategyCapabilities {
    /// Capabilities for scalar (baseline) implementation
    pub const fn scalar() -> Self {
        Self {
            speedup_factor: 1.0,
            supports_10bit: true,
            supports_12bit: true,
            max_resolution: (7680, 4320), // 8K
            is_hardware_accelerated: false,
        }
    }

    /// Capabilities for AVX2 SIMD implementation
    ///
    /// Supports 8, 10, and 12-bit video with SIMD acceleration.
    pub const fn avx2() -> Self {
        Self {
            speedup_factor: 4.5,
            supports_10bit: true,
            supports_12bit: true,
            max_resolution: (7680, 4320),
            is_hardware_accelerated: false,
        }
    }

    /// Capabilities for NEON SIMD implementation
    ///
    /// Supports 8, 10, and 12-bit video with SIMD acceleration.
    pub const fn neon() -> Self {
        Self {
            speedup_factor: 3.5,
            supports_10bit: true,
            supports_12bit: true,
            max_resolution: (7680, 4320),
            is_hardware_accelerated: false,
        }
    }

    /// Capabilities for Metal GPU implementation (future)
    pub const fn metal() -> Self {
        Self {
            speedup_factor: 9.0,
            supports_10bit: true,
            supports_12bit: true,
            max_resolution: (7680, 4320),
            is_hardware_accelerated: true,
        }
    }
}

/// Result type for conversion operations
pub type ConversionResult<T> = Result<T, ConversionError>;

/// Errors that can occur during YUV to RGB conversion
#[derive(Debug, thiserror::Error)]
pub enum ConversionError {
    #[error("Invalid frame dimensions: {width}x{height}")]
    InvalidDimensions { width: u32, height: u32 },

    #[error("Frame size exceeds maximum allowed: {0}x{0}")]
    FrameTooLarge(usize),

    #[error("Plane size mismatch: expected {expected}, got {actual}")]
    PlaneSizeMismatch { expected: usize, actual: usize },

    #[error("U plane missing for YUV420 frame")]
    MissingUPlane,

    #[error("V plane missing for YUV420 frame")]
    MissingVPlane,

    #[error("Bit depth {0} not supported by this strategy")]
    UnsupportedBitDepth(u8),
}

/// Core trait for YUV to RGB conversion strategies
///
/// Implementations of this trait provide platform-specific optimizations
/// for converting YUV video frames to RGB format.
pub trait YuvConversionStrategy: Send + Sync {
    /// Get the capabilities of this strategy
    fn capabilities(&self) -> StrategyCapabilities;

    /// Check if this strategy is available on the current platform
    fn is_available(&self) -> bool {
        true
    }

    /// Get a human-readable name for this strategy
    ///
    /// This must be implemented by each strategy to provide a simple
    /// string name like "Scalar", "AVX2", "NEON", or "Metal".
    fn name(&self) -> &'static str;

    /// Convert YUV420 format to RGB
    ///
    /// # Arguments
    /// * `y_plane` - Luminance plane
    /// * `u_plane` - Chroma blue plane
    /// * `v_plane` - Chroma red plane
    /// * `width` - Frame width in pixels
    /// * `height` - Frame height in pixels
    /// * `rgb` - Output buffer for RGB data (must be width * height * 3 bytes)
    /// * `bit_depth` - Video bit depth (8, 10, or 12)
    fn convert_yuv420_to_rgb(
        &self,
        y_plane: &[u8],
        u_plane: &[u8],
        v_plane: &[u8],
        width: usize,
        height: usize,
        rgb: &mut [u8],
        bit_depth: u8,
    ) -> ConversionResult<()>;

    /// Convert YUV422 format to RGB
    ///
    /// YUV422 has horizontal chroma subsampling (2:1).
    /// Each UV sample is shared by 2 horizontal Y samples.
    ///
    /// # Arguments
    /// * `y_plane` - Luminance plane
    /// * `u_plane` - Chroma blue plane
    /// * `v_plane` - Chroma red plane
    /// * `width` - Frame width in pixels
    /// * `height` - Frame height in pixels
    /// * `rgb` - Output buffer for RGB data (must be width * height * 3 bytes)
    /// * `bit_depth` - Video bit depth (8, 10, or 12)
    fn convert_yuv422_to_rgb(
        &self,
        y_plane: &[u8],
        u_plane: &[u8],
        v_plane: &[u8],
        width: usize,
        height: usize,
        rgb: &mut [u8],
        bit_depth: u8,
    ) -> ConversionResult<()>;

    /// Convert YUV444 format to RGB
    ///
    /// YUV444 has no chroma subsampling (4:4:4).
    /// Each pixel has its own Y, U, and V sample.
    ///
    /// # Arguments
    /// * `y_plane` - Luminance plane
    /// * `u_plane` - Chroma blue plane
    /// * `v_plane` - Chroma red plane
    /// * `width` - Frame width in pixels
    /// * `height` - Frame height in pixels
    /// * `rgb` - Output buffer for RGB data (must be width * height * 3 bytes)
    /// * `bit_depth` - Video bit depth (8, 10, or 12)
    fn convert_yuv444_to_rgb(
        &self,
        y_plane: &[u8],
        u_plane: &[u8],
        v_plane: &[u8],
        width: usize,
        height: usize,
        rgb: &mut [u8],
        bit_depth: u8,
    ) -> ConversionResult<()>;

    /// Validate input parameters before conversion
    fn validate_yuv420_params(
        &self,
        y_plane: &[u8],
        u_plane: &[u8],
        v_plane: &[u8],
        width: usize,
        height: usize,
        rgb: &mut [u8],
        bit_depth: u8,
    ) -> ConversionResult<()> {
        let y_expected = width * height;
        let uv_expected = (width / 2) * (height / 2);
        let rgb_expected = width * height * 3;

        if y_plane.len() < y_expected {
            return Err(ConversionError::PlaneSizeMismatch {
                expected: y_expected,
                actual: y_plane.len(),
            });
        }

        if u_plane.len() < uv_expected {
            return Err(ConversionError::PlaneSizeMismatch {
                expected: uv_expected,
                actual: u_plane.len(),
            });
        }

        if v_plane.len() < uv_expected {
            return Err(ConversionError::PlaneSizeMismatch {
                expected: uv_expected,
                actual: v_plane.len(),
            });
        }

        if rgb.len() < rgb_expected {
            return Err(ConversionError::PlaneSizeMismatch {
                expected: rgb_expected,
                actual: rgb.len(),
            });
        }

        if !matches!(bit_depth, 8 | 10 | 12) {
            return Err(ConversionError::UnsupportedBitDepth(bit_depth));
        }

        Ok(())
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_capabilities_display() {
        let scalar_caps = StrategyCapabilities::scalar();
        assert_eq!(scalar_caps.speedup_factor, 1.0);
        assert!(scalar_caps.supports_10bit);
        assert!(!scalar_caps.is_hardware_accelerated);
    }

    #[test]
    fn test_avx2_capabilities() {
        let avx2_caps = StrategyCapabilities::avx2();
        assert_eq!(avx2_caps.speedup_factor, 4.5);
        assert!(avx2_caps.supports_10bit);   // 10-bit support added
        assert!(avx2_caps.supports_12bit);   // 12-bit support added
        assert!(!avx2_caps.is_hardware_accelerated);
    }

    #[test]
    fn test_neon_capabilities() {
        let neon_caps = StrategyCapabilities::neon();
        assert_eq!(neon_caps.speedup_factor, 3.5);
        assert!(neon_caps.supports_10bit);   // 10-bit support added
        assert!(neon_caps.supports_12bit);   // 12-bit support added
    }

    #[test]
    fn test_metal_capabilities() {
        let metal_caps = StrategyCapabilities::metal();
        assert_eq!(metal_caps.speedup_factor, 9.0);
        assert!(metal_caps.is_hardware_accelerated);
    }

    #[test]
    fn test_conversion_error_display() {
        let err = ConversionError::InvalidDimensions {
            width: 1920,
            height: 1080,
        };
        assert!(err.to_string().contains("1920x1080"));
    }
}
