//! PU Type Overlay - VQAnalyzer parity
//!
//! Shows prediction unit types as color-coded blocks.
//! Categorical coloring: Intra, Skip, Merge, AMVP, etc.

use egui::TextureHandle;

/// PU type overlay state
pub struct PuTypeOverlayState {
    /// Opacity (0.0..1.0)
    pub opacity: f32,
    /// Show intra blocks
    pub show_intra: bool,
    /// Show skip blocks
    pub show_skip: bool,
    /// Show merge blocks
    pub show_merge: bool,
    /// Show AMVP blocks
    pub show_amvp: bool,
    /// Cached texture
    pub texture: Option<TextureHandle>,
    /// Show legend
    pub show_legend: bool,
}

/// Prediction unit type categories
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PuType {
    /// Intra prediction (all modes)
    Intra,
    /// Skip mode (no residual, copy MV)
    Skip,
    /// Merge mode (inherit MV from neighbor)
    Merge,
    /// AMVP mode (explicit MV signaling)
    Amvp,
    /// Affine motion
    Affine,
    /// Other/unknown
    Other,
}

impl PuType {
    /// Get display label
    pub fn label(&self) -> &'static str {
        match self {
            Self::Intra => "Intra",
            Self::Skip => "Skip",
            Self::Merge => "Merge",
            Self::Amvp => "AMVP",
            Self::Affine => "Affine",
            Self::Other => "Other",
        }
    }

    /// Get categorical color for this PU type
    pub fn color(&self) -> (u8, u8, u8) {
        match self {
            Self::Intra => (255, 200, 50),   // Yellow/Orange
            Self::Skip => (180, 180, 180),   // Gray
            Self::Merge => (100, 200, 200),  // Cyan
            Self::Amvp => (100, 200, 100),   // Green
            Self::Affine => (200, 100, 200), // Purple
            Self::Other => (100, 100, 100),  // Dark gray
        }
    }

    /// Get all types for legend
    pub fn all() -> &'static [PuType] {
        &[
            Self::Intra,
            Self::Skip,
            Self::Merge,
            Self::Amvp,
            Self::Affine,
        ]
    }
}

impl PuTypeOverlayState {
    pub fn new() -> Self {
        Self {
            opacity: 0.6,
            show_intra: true,
            show_skip: true,
            show_merge: true,
            show_amvp: true,
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

    /// Check if a PU type should be shown based on current filter settings
    pub fn should_show(&self, pu_type: PuType) -> bool {
        match pu_type {
            PuType::Intra => self.show_intra,
            PuType::Skip => self.show_skip,
            PuType::Merge => self.show_merge,
            PuType::Amvp => self.show_amvp,
            PuType::Affine => self.show_amvp, // Group with AMVP
            PuType::Other => true,
        }
    }
}

impl Default for PuTypeOverlayState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_values() {
        let state = PuTypeOverlayState::new();
        assert!((state.opacity - 0.6).abs() < f32::EPSILON);
        assert!(state.show_intra);
        assert!(state.show_skip);
        assert!(state.show_merge);
        assert!(state.show_amvp);
        assert!(state.show_legend);
    }

    #[test]
    fn test_should_show_filter() {
        let mut state = PuTypeOverlayState::new();
        assert!(state.should_show(PuType::Intra));
        assert!(state.should_show(PuType::Skip));

        state.show_intra = false;
        assert!(!state.should_show(PuType::Intra));
        assert!(state.should_show(PuType::Skip));
    }

    #[test]
    fn test_pu_type_colors() {
        // Each type should have distinct colors
        let colors: Vec<_> = PuType::all().iter().map(|t| t.color()).collect();
        for i in 0..colors.len() {
            for j in (i + 1)..colors.len() {
                assert_ne!(colors[i], colors[j], "Colors should be distinct");
            }
        }
    }
}
