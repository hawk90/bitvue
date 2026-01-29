//! Texture Management for Player Workspace
//!
//! Handles decoded frame texture and frame dimensions.

use egui::{ColorImage, TextureHandle, TextureOptions};

/// Texture manager for player workspace
///
/// Manages the current frame texture and dimensions.
pub struct TextureManager {
    /// Current decoded frame texture
    texture: Option<TextureHandle>,
    /// Frame dimensions (width, height)
    frame_size: Option<(u32, u32)>,
}

impl TextureManager {
    /// Create new texture manager
    pub fn new() -> Self {
        Self {
            texture: None,
            frame_size: None,
        }
    }

    /// Update the displayed frame
    ///
    /// Loads a new frame texture and stores frame dimensions.
    pub fn set_frame(&mut self, ctx: &egui::Context, image: ColorImage) {
        self.frame_size = Some((image.width() as u32, image.height() as u32));
        self.texture = Some(ctx.load_texture("player_frame", image, TextureOptions::LINEAR));
    }

    /// Get current texture handle
    pub fn texture(&self) -> Option<&TextureHandle> {
        self.texture.as_ref()
    }

    /// Get frame dimensions
    pub fn frame_size(&self) -> Option<(u32, u32)> {
        self.frame_size
    }

    /// Clear texture (e.g., when unloading a video)
    pub fn clear(&mut self) {
        self.texture = None;
        self.frame_size = None;
    }
}

impl Default for TextureManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_values() {
        let manager = TextureManager::new();
        assert!(manager.texture().is_none());
        assert!(manager.frame_size().is_none());
    }

    #[test]
    fn test_clear() {
        let mut manager = TextureManager::new();
        // Note: Can't test set_frame without egui context
        manager.clear();
        assert!(manager.texture().is_none());
        assert!(manager.frame_size().is_none());
    }
}
