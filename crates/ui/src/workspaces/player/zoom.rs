//! Zoom and Pan Management for Player Workspace
//!
//! Handles zoom level, pan offset, and mouse wheel interactions.

use egui::Vec2;

/// Zoom and pan manager for player workspace
///
/// Manages zoom level and pan offset for the player view.
pub struct ZoomManager {
    /// Zoom level (1.0 = 100%)
    zoom: f32,
    /// Pan offset
    offset: Vec2,
    /// Minimum zoom level
    min_zoom: f32,
    /// Maximum zoom level
    max_zoom: f32,
}

impl ZoomManager {
    /// Create new zoom manager with defaults
    pub fn new() -> Self {
        Self {
            zoom: 1.0,
            offset: Vec2::ZERO,
            min_zoom: 0.1,
            max_zoom: 10.0,
        }
    }

    /// Get current zoom level
    pub fn zoom(&self) -> f32 {
        self.zoom
    }

    /// Get pan offset
    pub fn offset(&self) -> Vec2 {
        self.offset
    }

    /// Set zoom level (clamped to min/max)
    pub fn set_zoom(&mut self, zoom: f32) {
        self.zoom = zoom.clamp(self.min_zoom, self.max_zoom);
    }

    /// Set pan offset
    pub fn set_offset(&mut self, offset: Vec2) {
        self.offset = offset;
    }

    /// Adjust zoom by delta factor
    ///
    /// Positive delta zooms in, negative zooms out.
    pub fn adjust_zoom(&mut self, delta: f32) {
        self.zoom = (self.zoom * delta).clamp(self.min_zoom, self.max_zoom);
    }

    /// Adjust pan offset by delta
    pub fn adjust_offset(&mut self, delta: Vec2) {
        self.offset += delta;
    }

    /// Reset zoom to 100% and offset to zero
    pub fn reset(&mut self) {
        self.zoom = 1.0;
        self.offset = Vec2::ZERO;
    }

    /// Get zoom level as percentage string
    pub fn zoom_percent(&self) -> String {
        format!("{:.0}%", self.zoom * 100.0)
    }

    /// Calculate scaled size for given dimensions
    pub fn scaled_size(&self, width: f32, height: f32) -> (f32, f32) {
        (width * self.zoom, height * self.zoom)
    }

    /// Handle mouse wheel zoom input
    ///
    /// Returns true if zoom changed.
    pub fn handle_mouse_wheel(&mut self, delta: f32) -> bool {
        let old_zoom = self.zoom;
        // Zoom factor: 1.1x per scroll step
        let factor = if delta > 0.0 { 1.1 } else { 0.9 };
        self.adjust_zoom(factor);
        self.zoom != old_zoom
    }

    /// Handle mouse drag for panning
    ///
    /// Returns true if offset changed.
    pub fn handle_mouse_drag(&mut self, delta: Vec2) -> bool {
        if delta.length_sq() > 0.0 {
            self.adjust_offset(delta);
            true
        } else {
            false
        }
    }
}

impl Default for ZoomManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_values() {
        let zoom = ZoomManager::new();
        assert_eq!(zoom.zoom(), 1.0);
        assert_eq!(zoom.offset(), Vec2::ZERO);
    }

    #[test]
    fn test_set_zoom_clamps() {
        let mut zoom = ZoomManager::new();
        zoom.set_zoom(20.0);
        assert_eq!(zoom.zoom(), zoom.max_zoom);

        zoom.set_zoom(0.05);
        assert_eq!(zoom.zoom(), zoom.min_zoom);
    }

    #[test]
    fn test_adjust_zoom() {
        let mut zoom = ZoomManager::new();
        zoom.adjust_zoom(2.0);
        assert_eq!(zoom.zoom(), 2.0);

        zoom.adjust_zoom(0.5);
        assert_eq!(zoom.zoom(), 1.0);
    }

    #[test]
    fn test_reset() {
        let mut zoom = ZoomManager::new();
        zoom.set_zoom(2.0);
        zoom.set_offset(Vec2::new(10.0, 20.0));
        zoom.reset();
        assert_eq!(zoom.zoom(), 1.0);
        assert_eq!(zoom.offset(), Vec2::ZERO);
    }

    #[test]
    fn test_zoom_percent() {
        let zoom = ZoomManager::new();
        assert_eq!(zoom.zoom_percent(), "100%");

        let mut zoom = ZoomManager::new();
        zoom.set_zoom(2.5);
        assert_eq!(zoom.zoom_percent(), "250%");
    }

    #[test]
    fn test_scaled_size() {
        let zoom = ZoomManager::new();
        let (w, h) = zoom.scaled_size(100.0, 200.0);
        assert_eq!(w, 100.0);
        assert_eq!(h, 200.0);

        let mut zoom = ZoomManager::new();
        zoom.set_zoom(2.0);
        let (w, h) = zoom.scaled_size(100.0, 200.0);
        assert_eq!(w, 200.0);
        assert_eq!(h, 400.0);
    }

    #[test]
    fn test_handle_mouse_wheel() {
        let mut zoom = ZoomManager::new();
        let old_zoom = zoom.zoom();

        // Zoom in
        let changed = zoom.handle_mouse_wheel(1.0);
        assert!(changed);
        assert!(zoom.zoom() > old_zoom);

        let old_zoom = zoom.zoom();
        // Zoom out
        let changed = zoom.handle_mouse_wheel(-1.0);
        assert!(changed);
        assert!(zoom.zoom() < old_zoom);
    }

    #[test]
    fn test_handle_mouse_drag() {
        let mut zoom = ZoomManager::new();
        let delta = Vec2::new(10.0, 20.0);

        let changed = zoom.handle_mouse_drag(delta);
        assert!(changed);
        assert_eq!(zoom.offset(), delta);
    }
}
