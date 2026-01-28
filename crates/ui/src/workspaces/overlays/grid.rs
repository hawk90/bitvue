//! Grid Overlay State - Extracted from PlayerWorkspace
//!
//! Contains grid overlay configuration (VQAnalyzer parity).

/// Grid overlay state
#[derive(Debug, Clone)]
pub struct GridOverlayState {
    /// Grid cell size in pixels (32, 64, 128)
    pub size: u32,
    /// Show CTB index labels inside cells
    pub show_ctb_labels: bool,
    /// Show row/column headers on grid edges
    pub show_headers: bool,
}

impl GridOverlayState {
    /// Create new grid overlay state with defaults
    pub fn new() -> Self {
        Self {
            size: 64,
            show_ctb_labels: true, // VQAnalyzer parity
            show_headers: true,    // VQAnalyzer parity
        }
    }

    /// Set grid size
    pub fn set_size(&mut self, size: u32) {
        self.size = size;
    }
}

impl Default for GridOverlayState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_values() {
        let state = GridOverlayState::new();
        assert_eq!(state.size, 64);
        assert!(state.show_ctb_labels);
        assert!(state.show_headers);
    }

    #[test]
    fn test_set_size() {
        let mut state = GridOverlayState::new();
        state.set_size(128);
        assert_eq!(state.size, 128);
    }
}
