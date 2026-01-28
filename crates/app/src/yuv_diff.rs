//! YUV Diff Module - Extracted from main.rs
//!
//! Handles YUV reference file loading, metrics calculation (PSNR/SSIM),
//! and multi-frame CSV export with ITU-T compliant weighting.

/// YUV format settings for diff/comparison (VQAnalyzer parity)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChromaSubsampling {
    Yuv420, // 4:2:0
    Yuv422, // 4:2:2
    Yuv444, // 4:4:4
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BitDepth {
    Bit8,
    Bit10,
    Bit12,
}

#[derive(Debug, Clone)]
pub struct YuvDiffSettings {
    pub subsampling: ChromaSubsampling,
    pub bit_depth: BitDepth,
    pub frame_offset: i32,
    pub show_psnr_map: bool,
    pub show_ssim_map: bool,
    pub show_delta: bool,
    pub reference_file: Option<std::path::PathBuf>,
    /// Loaded reference frame (cached for current comparison)
    pub reference_frame: Option<bitvue_decode::DecodedFrame>,
    /// Reference frame dimensions (for validation)
    pub reference_dimensions: Option<(u32, u32)>,
    /// Computed PSNR value (Y-plane, if available)
    pub psnr_value: Option<f32>,
    /// Computed SSIM value (Y-plane, if available)
    pub ssim_value: Option<f32>,
    /// Computed YUV-PSNR value (weighted average of Y/U/V)
    pub psnr_yuv_value: Option<f32>,
    /// Computed YUV-SSIM value (average of Y/U/V)
    pub ssim_yuv_value: Option<f32>,
    /// Export all frames (vs current frame only)
    pub export_all_frames: bool,
}

impl Default for YuvDiffSettings {
    fn default() -> Self {
        Self {
            subsampling: ChromaSubsampling::Yuv420,
            bit_depth: BitDepth::Bit8,
            frame_offset: 0,
            show_psnr_map: false,
            show_ssim_map: false,
            show_delta: false,
            reference_file: None,
            reference_frame: None,
            reference_dimensions: None,
            psnr_value: None,
            ssim_value: None,
            psnr_yuv_value: None,
            ssim_yuv_value: None,
            export_all_frames: false,
        }
    }
}
