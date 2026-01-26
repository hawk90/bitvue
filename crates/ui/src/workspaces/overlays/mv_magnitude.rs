//! MV Magnitude Heatmap Overlay - VQAnalyzer parity
//!
//! Shows motion vector magnitude as a heatmap.
//! Green (small) -> Yellow -> Red (large) color scale.

use egui::TextureHandle;

/// MV magnitude heatmap overlay state
pub struct MvMagnitudeOverlayState {
    /// Opacity (0.0..1.0)
    pub opacity: f32,
    /// Resolution mode
    pub resolution: bitvue_core::HeatmapResolution,
    /// Scale mode
    pub scale_mode: MvMagnitudeScale,
    /// Cached texture
    pub texture: Option<TextureHandle>,
    /// Show legend
    pub show_legend: bool,
    /// Which MV layer to visualize
    pub layer: bitvue_core::MVLayer,
}

/// Scale mode for MV magnitude heatmap
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MvMagnitudeScale {
    /// Automatically scale based on max magnitude in frame
    #[default]
    Auto,
    /// Fixed scale 0-16 pixels
    Fixed16,
    /// Fixed scale 0-64 pixels
    Fixed64,
    /// Fixed scale 0-128 pixels
    Fixed128,
}

impl MvMagnitudeScale {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Auto => "Auto",
            Self::Fixed16 => "0-16px",
            Self::Fixed64 => "0-64px",
            Self::Fixed128 => "0-128px",
        }
    }

    pub fn max_value(&self) -> Option<f32> {
        match self {
            Self::Auto => None,
            Self::Fixed16 => Some(16.0),
            Self::Fixed64 => Some(64.0),
            Self::Fixed128 => Some(128.0),
        }
    }
}

impl MvMagnitudeOverlayState {
    pub fn new() -> Self {
        Self {
            opacity: 0.5,
            resolution: bitvue_core::HeatmapResolution::Half,
            scale_mode: MvMagnitudeScale::Auto,
            texture: None,
            show_legend: true,
            layer: bitvue_core::MVLayer::Both,
        }
    }

    pub fn invalidate_texture(&mut self) {
        self.texture = None;
    }

    pub fn set_opacity(&mut self, opacity: f32) {
        self.opacity = opacity.clamp(0.0, 1.0);
        self.invalidate_texture();
    }

    /// Calculate MV magnitude from dx, dy (in quarter-pel units)
    pub fn magnitude(dx: i32, dy: i32) -> f32 {
        let dx_f = dx as f32 / 4.0; // Convert from quarter-pel
        let dy_f = dy as f32 / 4.0;
        (dx_f * dx_f + dy_f * dy_f).sqrt()
    }

    /// Get color for MV magnitude (green->yellow->red)
    pub fn get_color(magnitude: f32, max_magnitude: f32) -> (u8, u8, u8) {
        let normalized = if max_magnitude > 0.0 {
            (magnitude / max_magnitude).clamp(0.0, 1.0)
        } else {
            0.0
        };

        // Green -> Yellow -> Orange -> Red
        let (r, g, b) = if normalized < 0.33 {
            let t = normalized / 0.33;
            (t * 0.8, 0.8 + t * 0.2, 0.0) // Green to Yellow-green
        } else if normalized < 0.66 {
            let t = (normalized - 0.33) / 0.33;
            (0.8 + t * 0.2, 1.0 - t * 0.3, 0.0) // Yellow-green to Orange
        } else {
            let t = (normalized - 0.66) / 0.34;
            (1.0, 0.7 - t * 0.7, 0.0) // Orange to Red
        };

        ((r * 255.0) as u8, (g * 255.0) as u8, (b * 255.0) as u8)
    }
}

impl Default for MvMagnitudeOverlayState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_values() {
        let state = MvMagnitudeOverlayState::new();
        assert!((state.opacity - 0.5).abs() < f32::EPSILON);
        assert!(state.show_legend);
        assert!(state.texture.is_none());
    }

    #[test]
    fn test_magnitude_calculation() {
        // Zero MV
        assert!((MvMagnitudeOverlayState::magnitude(0, 0) - 0.0).abs() < f32::EPSILON);

        // 4 quarter-pels = 1 pixel
        let mag = MvMagnitudeOverlayState::magnitude(4, 0);
        assert!((mag - 1.0).abs() < f32::EPSILON);

        // Diagonal: sqrt(1^2 + 1^2) = sqrt(2) ≈ 1.414
        let mag = MvMagnitudeOverlayState::magnitude(4, 4);
        assert!((mag - 1.414).abs() < 0.01);
    }

    #[test]
    fn test_color_gradient() {
        // Small magnitude = green
        let (r, g, b) = MvMagnitudeOverlayState::get_color(0.0, 100.0);
        assert!(g > r);

        // Large magnitude = red
        let (r, g, b) = MvMagnitudeOverlayState::get_color(100.0, 100.0);
        assert!(r > g);
    }
}
