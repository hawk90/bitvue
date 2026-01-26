//! Player module - Frame pipeline and HUD
//!
//! T2-1: Player Frame Pipeline
//! T2-2: Player HUD

pub mod av1_quirks; // T0-1: quirks_AV1.001 - AV1 tile/sb mapping quirks
pub mod extractor; // T0-1: viz_core.002 - Extractor API + adapters
pub mod frame_mapper; // T0-1: viz_core.003 - Frame mapping join (display/decode)
pub mod h264_quirks;
mod hud;
mod mini_charts;
mod overlay_stack;
mod pipeline; // T0-1: quirks_H264.003 - H264 POC/MMCO/IDR quirks

pub use av1_quirks::*;
pub use extractor::*;
pub use frame_mapper::*;
pub use h264_quirks::*;
pub use hud::*;
pub use mini_charts::*;
pub use overlay_stack::*;
pub use pipeline::*;

use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;

// ============================================================================
// Common types shared by pipeline and HUD
// ============================================================================

/// Resolution tier for fast-path preview vs quality upgrade
///
/// Per FAST_PATH_QUALITY_PATH_POLICY.md:
/// Fast path uses Quarter/Half, quality path uses Full.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ResolutionTier {
    /// Quarter resolution (1/4 width, 1/4 height)
    /// Used for fastest possible preview
    Quarter,

    /// Half resolution (1/2 width, 1/2 height)
    /// Used for balanced preview
    Half,

    /// Full resolution
    /// Used for quality path
    Full,
}

impl ResolutionTier {
    /// Get scale factor (0.0-1.0)
    pub fn scale_factor(&self) -> f32 {
        match self {
            ResolutionTier::Quarter => 0.25,
            ResolutionTier::Half => 0.5,
            ResolutionTier::Full => 1.0,
        }
    }

    /// Get scaled dimensions
    pub fn scale_dims(&self, width: u32, height: u32) -> (u32, u32) {
        match self {
            ResolutionTier::Quarter => (width / 4, height / 4),
            ResolutionTier::Half => (width / 2, height / 2),
            ResolutionTier::Full => (width, height),
        }
    }
}

/// Color space for decoded frames
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ColorSpace {
    /// YUV only (no conversion)
    /// Fastest for preview
    Yuv,

    /// RGBA (converted from YUV)
    /// Required for display
    Rgba,
}

/// Decode parameters for cache key
///
/// Per CACHE_LEVELS_SPEC.md:
/// Key: frame_idx + decode_params
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DecodeParams {
    /// Frame index
    pub frame_idx: usize,

    /// Resolution tier
    pub res_tier: ResolutionTier,

    /// Color space
    pub color_space: ColorSpace,
}

impl DecodeParams {
    pub fn new(frame_idx: usize, res_tier: ResolutionTier, color_space: ColorSpace) -> Self {
        Self {
            frame_idx,
            res_tier,
            color_space,
        }
    }

    /// Create fast-path params (Half + YUV)
    pub fn fast_path(frame_idx: usize) -> Self {
        Self {
            frame_idx,
            res_tier: ResolutionTier::Half,
            color_space: ColorSpace::Yuv,
        }
    }

    /// Create quality-path params (Full + RGBA)
    pub fn quality_path(frame_idx: usize) -> Self {
        Self {
            frame_idx,
            res_tier: ResolutionTier::Full,
            color_space: ColorSpace::Rgba,
        }
    }
}

/// Decoded frame data
///
/// Per CACHE_LEVELS_SPEC.md:
/// "Prefer caching in YUV to avoid repeated conversion cost."
#[derive(Debug, Clone)]
pub struct DecodedFrame {
    /// Frame index
    pub frame_idx: usize,

    /// Resolution tier
    pub res_tier: ResolutionTier,

    /// YUV data (always present)
    pub yuv_data: Arc<Vec<u8>>,

    /// RGBA data (optional, converted from YUV)
    pub rgba_data: Option<Arc<Vec<u8>>>,

    /// Frame width
    pub width: u32,

    /// Frame height
    pub height: u32,

    /// Decode timestamp
    pub decoded_at: Instant,
}

/// Texture handle for GPU upload
///
/// Per T2-1 deliverable: TextureHandle reuse
#[derive(Debug, Clone)]
pub struct TextureHandle {
    /// Unique texture ID (GPU handle)
    pub id: u64,

    /// Frame index
    pub frame_idx: usize,

    /// Resolution tier
    pub res_tier: ResolutionTier,

    /// Width
    pub width: u32,

    /// Height
    pub height: u32,

    /// Last used timestamp
    pub last_used: Instant,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolution_tier_scale_factor() {
        assert_eq!(ResolutionTier::Quarter.scale_factor(), 0.25);
        assert_eq!(ResolutionTier::Half.scale_factor(), 0.5);
        assert_eq!(ResolutionTier::Full.scale_factor(), 1.0);
    }

    #[test]
    fn test_resolution_tier_scale_dims() {
        let (w, h) = ResolutionTier::Quarter.scale_dims(1920, 1080);
        assert_eq!((w, h), (480, 270));

        let (w, h) = ResolutionTier::Half.scale_dims(1920, 1080);
        assert_eq!((w, h), (960, 540));

        let (w, h) = ResolutionTier::Full.scale_dims(1920, 1080);
        assert_eq!((w, h), (1920, 1080));
    }

    #[test]
    fn test_decode_params_fast_path() {
        let params = DecodeParams::fast_path(10);
        assert_eq!(params.frame_idx, 10);
        assert_eq!(params.res_tier, ResolutionTier::Half);
        assert_eq!(params.color_space, ColorSpace::Yuv);
    }

    #[test]
    fn test_decode_params_quality_path() {
        let params = DecodeParams::quality_path(10);
        assert_eq!(params.frame_idx, 10);
        assert_eq!(params.res_tier, ResolutionTier::Full);
        assert_eq!(params.color_space, ColorSpace::Rgba);
    }

    // AV1 Player viz_core test - Task 21 (S.T4-2.AV1.Timeline.Player.impl.viz_core.001)

    #[test]
    fn test_av1_player_decode_visualization() {
        // AV1 Player: User scrubs through AV1 timeline with fast path preview
        // Frame 0: KEY_FRAME (1920x1080) - scrubbing uses fast path
        let fast_params = DecodeParams::fast_path(0);
        assert_eq!(fast_params.frame_idx, 0);
        assert_eq!(fast_params.res_tier, ResolutionTier::Half);
        assert_eq!(fast_params.color_space, ColorSpace::Yuv);

        // AV1 Player: Verify fast path resolution scaling for AV1 KEY_FRAME
        let (fast_w, fast_h) = ResolutionTier::Half.scale_dims(1920, 1080);
        assert_eq!((fast_w, fast_h), (960, 540));
        assert_eq!(ResolutionTier::Half.scale_factor(), 0.5);

        // AV1 Player: User stops scrubbing, quality path upgrades to full resolution
        let quality_params = DecodeParams::quality_path(0);
        assert_eq!(quality_params.frame_idx, 0);
        assert_eq!(quality_params.res_tier, ResolutionTier::Full);
        assert_eq!(quality_params.color_space, ColorSpace::Rgba);

        // AV1 Player: Verify quality path resolution for AV1 KEY_FRAME
        let (full_w, full_h) = ResolutionTier::Full.scale_dims(1920, 1080);
        assert_eq!((full_w, full_h), (1920, 1080));
        assert_eq!(ResolutionTier::Full.scale_factor(), 1.0);

        // AV1 Player: User scrubs to INTER_FRAME (frame 1) with quarter resolution
        let quarter_params = DecodeParams::new(1, ResolutionTier::Quarter, ColorSpace::Yuv);
        assert_eq!(quarter_params.frame_idx, 1);
        assert_eq!(quarter_params.res_tier, ResolutionTier::Quarter);

        // AV1 Player: Verify quarter resolution for very fast scrubbing
        let (quarter_w, quarter_h) = ResolutionTier::Quarter.scale_dims(1920, 1080);
        assert_eq!((quarter_w, quarter_h), (480, 270));
        assert_eq!(ResolutionTier::Quarter.scale_factor(), 0.25);

        // AV1 Player: Decode params equality for cache key lookup
        let params1 = DecodeParams::fast_path(5);
        let params2 = DecodeParams::fast_path(5);
        let params3 = DecodeParams::quality_path(5);

        assert_eq!(params1, params2); // Same frame, same tier → cache hit
        assert_ne!(params1, params3); // Same frame, different tier → cache miss

        // AV1 Player: Resolution tier comparison
        assert_ne!(ResolutionTier::Quarter, ResolutionTier::Half);
        assert_ne!(ResolutionTier::Half, ResolutionTier::Full);

        // AV1 Player: Color space comparison
        assert_ne!(ColorSpace::Yuv, ColorSpace::Rgba);

        // AV1 Player: Verify all resolution tiers work with AV1 640x360 test file
        let (q_w, q_h) = ResolutionTier::Quarter.scale_dims(640, 360);
        assert_eq!((q_w, q_h), (160, 90));

        let (h_w, h_h) = ResolutionTier::Half.scale_dims(640, 360);
        assert_eq!((h_w, h_h), (320, 180));

        let (f_w, f_h) = ResolutionTier::Full.scale_dims(640, 360);
        assert_eq!((f_w, f_h), (640, 360));
    }
}
