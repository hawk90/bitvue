//! Application Settings Module - Extracted from main.rs
//!
//! Handles color space, CPU optimization, theme, and layout persistence settings.

/// Color space for YUV-to-RGB conversion (VQAnalyzer parity - Options Menu)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorSpace {
    Bt601,  // SD video
    Bt709,  // HD video (default)
    Bt2020, // UHD/HDR video
}

impl ColorSpace {
    pub fn label(&self) -> &'static str {
        match self {
            ColorSpace::Bt601 => "BT.601 (SD)",
            ColorSpace::Bt709 => "BT.709 (HD)",
            ColorSpace::Bt2020 => "BT.2020 (UHD/HDR)",
        }
    }
}

/// CPU optimization mode (VQAnalyzer parity - Options Menu)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CpuOptimization {
    Auto,     // Auto-detect and use SIMD if available
    Disabled, // Force scalar code (for debugging)
}

impl CpuOptimization {
    pub fn label(&self) -> &'static str {
        match self {
            CpuOptimization::Auto => "Auto (SIMD enabled)",
            CpuOptimization::Disabled => "Disabled (scalar only)",
        }
    }
}

/// Application settings (VQAnalyzer parity - Options Menu, Phase 4+6)
#[derive(Debug, Clone)]
pub struct AppSettings {
    pub color_space: ColorSpace,
    pub cpu_optimization: CpuOptimization,
    pub theme: egui::Theme,
    pub auto_save_layout: bool,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            color_space: ColorSpace::Bt709, // HD default
            cpu_optimization: CpuOptimization::Auto,
            theme: egui::Theme::Dark,
            auto_save_layout: false, // User must opt-in
        }
    }
}
