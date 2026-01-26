//! QP Heatmap Overlay State - Extracted from PlayerWorkspace
//!
//! Contains QP heatmap overlay configuration.
//! Per QP_HEATMAP_IMPLEMENTATION_SPEC.md

use egui::TextureHandle;

/// QP heatmap overlay state
pub struct QpOverlayState {
    /// Opacity (0.0..1.0)
    pub opacity: f32,
    /// Resolution mode
    pub resolution: bitvue_core::HeatmapResolution,
    /// Scale mode (Auto or Fixed 0-63)
    pub scale_mode: bitvue_core::QPScaleMode,
    /// Cached texture (invalidated on settings change)
    pub texture: Option<TextureHandle>,
}

impl QpOverlayState {
    /// Create new QP overlay state with defaults
    /// Per QP_HEATMAP_IMPLEMENTATION_SPEC.md
    pub fn new() -> Self {
        Self {
            opacity: 0.45, // Default per spec
            resolution: bitvue_core::HeatmapResolution::Half,
            scale_mode: bitvue_core::QPScaleMode::Auto,
            texture: None,
        }
    }

    /// Invalidate cached texture (call when settings change)
    pub fn invalidate_texture(&mut self) {
        self.texture = None;
    }

    /// Set opacity and invalidate texture
    pub fn set_opacity(&mut self, opacity: f32) {
        self.opacity = opacity.clamp(0.0, 1.0);
        self.invalidate_texture();
    }

    /// Set resolution and invalidate texture
    pub fn set_resolution(&mut self, resolution: bitvue_core::HeatmapResolution) {
        self.resolution = resolution;
        self.invalidate_texture();
    }

    /// Set scale mode and invalidate texture
    pub fn set_scale_mode(&mut self, scale_mode: bitvue_core::QPScaleMode) {
        self.scale_mode = scale_mode;
        self.invalidate_texture();
    }
}

impl Default for QpOverlayState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_values() {
        let state = QpOverlayState::new();
        assert!((state.opacity - 0.45).abs() < f32::EPSILON);
        assert!(matches!(state.resolution, bitvue_core::HeatmapResolution::Half));
        assert!(matches!(state.scale_mode, bitvue_core::QPScaleMode::Auto));
        assert!(state.texture.is_none());
    }

    #[test]
    fn test_set_opacity_clamps() {
        let mut state = QpOverlayState::new();
        state.set_opacity(1.5);
        assert!((state.opacity - 1.0).abs() < f32::EPSILON);

        state.set_opacity(-0.5);
        assert!((state.opacity - 0.0).abs() < f32::EPSILON);
    }
}
