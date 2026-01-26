//! Motion Vector Overlay State - Extracted from PlayerWorkspace
//!
//! Contains MV overlay configuration.
//! Per MV_VECTORS_IMPLEMENTATION_SPEC.md

/// Motion vector overlay state
#[derive(Debug, Clone)]
pub struct MvOverlayState {
    /// Layer selection (L0/L1/Both)
    pub layer: bitvue_core::MVLayer,
    /// User scale factor (0.1..3.0)
    pub user_scale: f32,
    /// Opacity (0.0..1.0)
    pub opacity: f32,
}

impl MvOverlayState {
    /// Create new MV overlay state with defaults
    /// Per MV_VECTORS_IMPLEMENTATION_SPEC.md
    pub fn new() -> Self {
        Self {
            layer: bitvue_core::MVLayer::Both,
            user_scale: bitvue_core::DEFAULT_USER_SCALE, // 1.0
            opacity: bitvue_core::mv_overlay::DEFAULT_OPACITY, // 0.55
        }
    }

    /// Set layer selection
    pub fn set_layer(&mut self, layer: bitvue_core::MVLayer) {
        self.layer = layer;
    }

    /// Set user scale (clamped to 0.1..3.0)
    pub fn set_user_scale(&mut self, scale: f32) {
        self.user_scale = scale.clamp(0.1, 3.0);
    }

    /// Set opacity (clamped to 0.0..1.0)
    pub fn set_opacity(&mut self, opacity: f32) {
        self.opacity = opacity.clamp(0.0, 1.0);
    }
}

impl Default for MvOverlayState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_values() {
        let state = MvOverlayState::new();
        assert!(matches!(state.layer, bitvue_core::MVLayer::Both));
        assert!((state.user_scale - 1.0).abs() < f32::EPSILON);
        assert!((state.opacity - 0.55).abs() < f32::EPSILON);
    }

    #[test]
    fn test_set_user_scale_clamps() {
        let mut state = MvOverlayState::new();
        state.set_user_scale(5.0);
        assert!((state.user_scale - 3.0).abs() < f32::EPSILON);

        state.set_user_scale(0.0);
        assert!((state.user_scale - 0.1).abs() < f32::EPSILON);
    }

    #[test]
    fn test_set_opacity_clamps() {
        let mut state = MvOverlayState::new();
        state.set_opacity(1.5);
        assert!((state.opacity - 1.0).abs() < f32::EPSILON);
    }
}
