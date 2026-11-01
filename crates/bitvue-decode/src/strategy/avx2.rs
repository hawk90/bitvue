//! AVX2 strategy for x86_64.
//!
//! **Currently the conversion itself is the scalar one.** The hand-written AVX2 kernels that used
//! to be here were wrong: they loaded 32 bytes per 8 pixels and OR-ed the second half into the
//! luma (an 8-bit black pixel next to other data came out white), duplicated chroma samples in the
//! wrong lanes or not at all for 10/12-bit, and read past the end of a plane after checking only
//! 8 bytes. A scalar-versus-strategy comparison on x86 CI found luma differences of up to 255
//! (`tests/yuv_test.rs`). They are removed until a correct, tested rewrite exists; the strategy
//! stays so that the registry and its name keep working.

use super::{ConversionResult, ScalarStrategy, StrategyCapabilities, YuvConversionStrategy};

/// AVX2 strategy - x86_64; converts with the scalar code for now (see the module doc)
#[derive(Debug, Clone, Copy)]
pub struct Avx2Strategy;

impl Avx2Strategy {
    pub const fn new() -> Self {
        Self
    }
}

impl Default for Avx2Strategy {
    fn default() -> Self {
        Self::new()
    }
}

impl YuvConversionStrategy for Avx2Strategy {
    fn capabilities(&self) -> StrategyCapabilities {
        StrategyCapabilities::avx2()
    }

    fn is_available(&self) -> bool {
        is_x86_feature_detected!("avx2")
    }

    fn name(&self) -> &'static str {
        "AVX2"
    }

    fn convert_yuv420_to_rgb(
        &self,
        y_plane: &[u8],
        u_plane: &[u8],
        v_plane: &[u8],
        width: usize,
        height: usize,
        rgb: &mut [u8],
        bit_depth: u8,
    ) -> ConversionResult<()> {
        ScalarStrategy::new()
            .convert_yuv420_to_rgb(y_plane, u_plane, v_plane, width, height, rgb, bit_depth)
    }

    fn convert_yuv422_to_rgb(
        &self,
        y_plane: &[u8],
        u_plane: &[u8],
        v_plane: &[u8],
        width: usize,
        height: usize,
        rgb: &mut [u8],
        bit_depth: u8,
    ) -> ConversionResult<()> {
        ScalarStrategy::new()
            .convert_yuv422_to_rgb(y_plane, u_plane, v_plane, width, height, rgb, bit_depth)
    }

    fn convert_yuv444_to_rgb(
        &self,
        y_plane: &[u8],
        u_plane: &[u8],
        v_plane: &[u8],
        width: usize,
        height: usize,
        rgb: &mut [u8],
        bit_depth: u8,
    ) -> ConversionResult<()> {
        ScalarStrategy::new()
            .convert_yuv444_to_rgb(y_plane, u_plane, v_plane, width, height, rgb, bit_depth)
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_avx2_strategy_creation() {
        let strategy = Avx2Strategy::new();
        assert_eq!(strategy.name(), "AVX2");
    }

    #[test]
    fn test_avx2_capabilities() {
        let strategy = Avx2Strategy::new();
        let caps = strategy.capabilities();
        assert_eq!(
            caps.speedup_factor, 1.0,
            "no faster than scalar until the kernels are rewritten"
        );
        assert!(caps.supports_10bit); // 10-bit support added
        assert!(caps.supports_12bit); // 12-bit support added
        assert!(!caps.is_hardware_accelerated);
    }

    #[test]
    fn test_avx2_default() {
        // This tests the public `Default` impl itself, so it must call `default()` rather than
        // name the unit struct (which is what clippy would suggest).
        #[allow(clippy::default_constructed_unit_structs)]
        let strategy = Avx2Strategy::default();
        assert_eq!(strategy.name(), "AVX2");
    }

    #[test]
    fn test_avx2_supported_bit_depths() {
        let strategy = Avx2Strategy::new();

        let y_plane = vec![0; 100];
        let u_plane = vec![128; 25];
        let v_plane = vec![128; 25];
        let mut rgb = vec![0u8; 300];

        // 8-bit is supported
        let result_8bit =
            strategy.convert_yuv420_to_rgb(&y_plane, &u_plane, &v_plane, 10, 10, &mut rgb, 8);
        assert!(result_8bit.is_ok());

        // 10-bit is now supported
        let result_10bit =
            strategy.convert_yuv420_to_rgb(&y_plane, &u_plane, &v_plane, 10, 10, &mut rgb, 10);
        assert!(result_10bit.is_ok());
    }

    #[test]
    #[cfg(target_arch = "x86_64")]
    fn test_avx2_is_available_when_supported() {
        let strategy = Avx2Strategy::new();
        // On x86_64 with AVX2 support, this should be true
        let available = strategy.is_available();
        // We can't assert true because the test might run on non-AVX2 hardware
        // But we can verify it returns a boolean without panicking
        let _ = available;
    }
}
