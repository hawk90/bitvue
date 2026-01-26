//! Bit Allocation Heatmap Overlay - VQAnalyzer parity
//!
//! Shows bits per CTB/block as a heatmap visualization.
//! Blue (low) -> Red (high) color scale.

use egui::TextureHandle;

/// Bit allocation heatmap overlay state
pub struct BitAllocationOverlayState {
    /// Opacity (0.0..1.0)
    pub opacity: f32,
    /// Resolution mode
    pub resolution: bitvue_core::HeatmapResolution,
    /// Scale mode (Auto or Fixed range)
    pub scale_mode: BitAllocationScale,
    /// Cached texture (invalidated on settings change)
    pub texture: Option<TextureHandle>,
    /// Show legend
    pub show_legend: bool,
}

/// Scale mode for bit allocation heatmap
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BitAllocationScale {
    /// Automatically scale based on min/max in frame
    #[default]
    Auto,
    /// Fixed scale 0-1000 bits
    Fixed1K,
    /// Fixed scale 0-5000 bits
    Fixed5K,
    /// Fixed scale 0-10000 bits
    Fixed10K,
}

impl BitAllocationScale {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Auto => "Auto",
            Self::Fixed1K => "0-1K",
            Self::Fixed5K => "0-5K",
            Self::Fixed10K => "0-10K",
        }
    }

    pub fn max_value(&self) -> Option<u32> {
        match self {
            Self::Auto => None,
            Self::Fixed1K => Some(1000),
            Self::Fixed5K => Some(5000),
            Self::Fixed10K => Some(10000),
        }
    }
}

impl BitAllocationOverlayState {
    pub fn new() -> Self {
        Self {
            opacity: 0.5,
            resolution: bitvue_core::HeatmapResolution::Half,
            scale_mode: BitAllocationScale::Auto,
            texture: None,
            show_legend: true,
        }
    }

    pub fn invalidate_texture(&mut self) {
        self.texture = None;
    }

    pub fn set_opacity(&mut self, opacity: f32) {
        self.opacity = opacity.clamp(0.0, 1.0);
        self.invalidate_texture();
    }

    /// Get color for bit count (blue->cyan->green->yellow->red)
    pub fn get_color(bits: u32, max_bits: u32) -> (u8, u8, u8) {
        let normalized = if max_bits > 0 {
            (bits as f32 / max_bits as f32).clamp(0.0, 1.0)
        } else {
            0.0
        };

        // Blue -> Cyan -> Green -> Yellow -> Red
        let (r, g, b) = if normalized < 0.25 {
            let t = normalized / 0.25;
            (0.0, t, 1.0) // Blue to Cyan
        } else if normalized < 0.5 {
            let t = (normalized - 0.25) / 0.25;
            (0.0, 1.0, 1.0 - t) // Cyan to Green
        } else if normalized < 0.75 {
            let t = (normalized - 0.5) / 0.25;
            (t, 1.0, 0.0) // Green to Yellow
        } else {
            let t = (normalized - 0.75) / 0.25;
            (1.0, 1.0 - t, 0.0) // Yellow to Red
        };

        ((r * 255.0) as u8, (g * 255.0) as u8, (b * 255.0) as u8)
    }
}

impl Default for BitAllocationOverlayState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_values() {
        let state = BitAllocationOverlayState::new();
        assert!((state.opacity - 0.5).abs() < f32::EPSILON);
        assert!(state.show_legend);
        assert!(state.texture.is_none());
    }

    #[test]
    fn test_color_gradient() {
        // Low bits = blue
        let (r, g, b) = BitAllocationOverlayState::get_color(0, 1000);
        assert!(b > r && b > g);

        // High bits = red
        let (r, g, b) = BitAllocationOverlayState::get_color(1000, 1000);
        assert!(r > g && r > b);

        // Mid bits = green-ish
        let (r, g, b) = BitAllocationOverlayState::get_color(500, 1000);
        assert!(g >= r && g >= b);
    }
}
