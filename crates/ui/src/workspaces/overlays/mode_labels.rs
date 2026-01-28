//! Mode Labels Overlay State - Block prediction mode visualization
//!
//! Shows prediction mode labels on blocks (AMVP, Merge, Skip, Intra modes).
//! VQAnalyzer parity: matches VQAnalyzer's mode label visualization.

/// Block prediction mode label types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockModeLabel {
    // Intra modes
    IntraDC,
    IntraPlanar,
    IntraAngular(u8), // Angular mode 2-66 for HEVC/VVC

    // Inter modes
    Skip,
    Merge,
    AMVP,

    // Advanced modes (VVC/AV1)
    GPM,    // Geometric Partition Mode
    SbTMVP, // Subblock Temporal MV Prediction
    DMVR,   // Decoder-side Motion Vector Refinement
    Affine, // Affine motion

    // AV1 specific
    Intra,     // Generic intra (DC/V/H/etc in AV1)
    Inter,     // Generic inter
    NewMV,     // New motion vector
    NearMV,    // Near motion vector
    NearestMV, // Nearest motion vector
    GlobalMV,  // Global motion vector

    // Unknown/unsupported
    Unknown,
}

impl BlockModeLabel {
    /// Get short label text for overlay
    pub fn short_label(&self) -> &'static str {
        match self {
            Self::IntraDC => "DC",
            Self::IntraPlanar => "P",
            Self::IntraAngular(n) => match n {
                2..=17 => "A",  // Angular left/top-left
                18..=49 => "A", // Angular vertical-ish
                50..=66 => "A", // Angular right/top-right
                _ => "A",
            },
            Self::Skip => "S",
            Self::Merge => "M",
            Self::AMVP => "A",
            Self::GPM => "G",
            Self::SbTMVP => "T",
            Self::DMVR => "D",
            Self::Affine => "Af",
            Self::Intra => "I",
            Self::Inter => "P",
            Self::NewMV => "N",
            Self::NearMV => "Nr",
            Self::NearestMV => "Nt",
            Self::GlobalMV => "G",
            Self::Unknown => "?",
        }
    }

    /// Get full label text for tooltip
    pub fn full_label(&self) -> &'static str {
        match self {
            Self::IntraDC => "Intra DC",
            Self::IntraPlanar => "Intra Planar",
            Self::IntraAngular(_) => "Intra Angular",
            Self::Skip => "Skip",
            Self::Merge => "Merge",
            Self::AMVP => "AMVP",
            Self::GPM => "Geometric Partition",
            Self::SbTMVP => "Subblock TMVP",
            Self::DMVR => "DMVR",
            Self::Affine => "Affine",
            Self::Intra => "Intra",
            Self::Inter => "Inter",
            Self::NewMV => "New MV",
            Self::NearMV => "Near MV",
            Self::NearestMV => "Nearest MV",
            Self::GlobalMV => "Global MV",
            Self::Unknown => "Unknown",
        }
    }

    /// Get label color (RGBA)
    pub fn color(&self) -> (u8, u8, u8, u8) {
        match self {
            // Intra modes - yellow/orange family
            Self::IntraDC | Self::IntraPlanar | Self::IntraAngular(_) | Self::Intra => {
                (255, 200, 50, 230) // Yellow
            }
            // Skip - gray (no residual)
            Self::Skip => (180, 180, 180, 230),
            // Merge - cyan
            Self::Merge => (100, 220, 220, 230),
            // AMVP - green
            Self::AMVP => (100, 220, 100, 230),
            // Advanced inter modes - purple/magenta family
            Self::GPM | Self::SbTMVP | Self::DMVR | Self::Affine => (200, 100, 220, 230),
            // AV1 inter modes - blue/green family
            Self::Inter | Self::NewMV | Self::NearMV | Self::NearestMV | Self::GlobalMV => {
                (100, 180, 255, 230) // Blue
            }
            Self::Unknown => (128, 128, 128, 200),
        }
    }

    /// Check if this is an intra mode
    pub fn is_intra(&self) -> bool {
        matches!(
            self,
            Self::IntraDC | Self::IntraPlanar | Self::IntraAngular(_) | Self::Intra
        )
    }

    /// Check if this is an inter mode
    pub fn is_inter(&self) -> bool {
        !self.is_intra() && !matches!(self, Self::Unknown)
    }
}

/// Mode labels overlay state
#[derive(Debug, Clone)]
pub struct ModeLabelOverlayState {
    /// Show intra mode labels (DC, Planar, Angular)
    pub show_intra_modes: bool,
    /// Show inter mode labels (Skip, Merge, AMVP)
    pub show_inter_modes: bool,
    /// Font scale factor (0.5..2.0)
    pub font_scale: f32,
    /// Label opacity (0.0..1.0)
    pub opacity: f32,
    /// Minimum block size to show labels (in screen pixels)
    pub min_block_size: f32,
    /// Show background behind labels for readability
    pub show_background: bool,
}

impl ModeLabelOverlayState {
    /// Create new mode labels overlay state with defaults
    pub fn new() -> Self {
        Self {
            show_intra_modes: true,
            show_inter_modes: true,
            font_scale: 1.0,
            opacity: 0.9,
            min_block_size: 16.0, // Only show labels if block is >= 16 screen pixels
            show_background: true,
        }
    }

    /// Set font scale (clamped to 0.5..2.0)
    pub fn set_font_scale(&mut self, scale: f32) {
        self.font_scale = scale.clamp(0.5, 2.0);
    }

    /// Set opacity (clamped to 0.0..1.0)
    pub fn set_opacity(&mut self, opacity: f32) {
        self.opacity = opacity.clamp(0.0, 1.0);
    }

    /// Check if a mode should be displayed based on current settings
    pub fn should_show(&self, mode: &BlockModeLabel) -> bool {
        if mode.is_intra() {
            self.show_intra_modes
        } else if mode.is_inter() {
            self.show_inter_modes
        } else {
            false
        }
    }
}

impl Default for ModeLabelOverlayState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_values() {
        let state = ModeLabelOverlayState::new();
        assert!(state.show_intra_modes);
        assert!(state.show_inter_modes);
        assert!((state.font_scale - 1.0).abs() < f32::EPSILON);
        assert!((state.opacity - 0.9).abs() < f32::EPSILON);
        assert!((state.min_block_size - 16.0).abs() < f32::EPSILON);
        assert!(state.show_background);
    }

    #[test]
    fn test_set_font_scale_clamps() {
        let mut state = ModeLabelOverlayState::new();

        state.set_font_scale(3.0);
        assert!((state.font_scale - 2.0).abs() < f32::EPSILON);

        state.set_font_scale(0.1);
        assert!((state.font_scale - 0.5).abs() < f32::EPSILON);
    }

    #[test]
    fn test_set_opacity_clamps() {
        let mut state = ModeLabelOverlayState::new();

        state.set_opacity(1.5);
        assert!((state.opacity - 1.0).abs() < f32::EPSILON);

        state.set_opacity(-0.5);
        assert!((state.opacity - 0.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_mode_label_colors() {
        let (r, g, b, a) = BlockModeLabel::Skip.color();
        assert!(r > 100 && g > 100 && b > 100); // Gray
        assert!(a > 200); // High alpha

        let (r, g, b, _) = BlockModeLabel::IntraDC.color();
        assert!(r > 200); // Yellow has high red
        assert!(g > 150); // Yellow has medium-high green
    }

    #[test]
    fn test_mode_label_short_labels() {
        assert_eq!(BlockModeLabel::Skip.short_label(), "S");
        assert_eq!(BlockModeLabel::Merge.short_label(), "M");
        assert_eq!(BlockModeLabel::AMVP.short_label(), "A");
        assert_eq!(BlockModeLabel::IntraDC.short_label(), "DC");
    }

    #[test]
    fn test_should_show_filter() {
        let mut state = ModeLabelOverlayState::new();

        // Both enabled
        assert!(state.should_show(&BlockModeLabel::IntraDC));
        assert!(state.should_show(&BlockModeLabel::Skip));

        // Disable intra
        state.show_intra_modes = false;
        assert!(!state.should_show(&BlockModeLabel::IntraDC));
        assert!(state.should_show(&BlockModeLabel::Skip));

        // Disable inter
        state.show_intra_modes = true;
        state.show_inter_modes = false;
        assert!(state.should_show(&BlockModeLabel::IntraDC));
        assert!(!state.should_show(&BlockModeLabel::Skip));
    }

    #[test]
    fn test_is_intra_inter() {
        assert!(BlockModeLabel::IntraDC.is_intra());
        assert!(BlockModeLabel::IntraPlanar.is_intra());
        assert!(BlockModeLabel::IntraAngular(26).is_intra());
        assert!(BlockModeLabel::Intra.is_intra());

        assert!(!BlockModeLabel::IntraDC.is_inter());
        assert!(BlockModeLabel::Skip.is_inter());
        assert!(BlockModeLabel::Merge.is_inter());
        assert!(BlockModeLabel::AMVP.is_inter());
    }
}
