//! Navigation Controls for Player Workspace
//!
//! Handles frame navigation including keyboard shortcuts,
//! navigation buttons, and frame finding utilities.

use bitvue_core::{Command, FrameType, StreamId, UnitNode};

/// Navigation manager for player workspace
///
/// Handles frame navigation and frame finding logic.
pub struct NavigationManager {
    // No state needed currently - navigation is handled via commands
}

impl NavigationManager {
    /// Create new navigation manager
    pub fn new() -> Self {
        Self {}
    }

    /// Handle keyboard shortcuts for frame navigation
    ///
    /// Returns Command if navigation key was pressed.
    pub fn handle_keyboard(
        &self,
        current_frame: usize,
        total_frames: usize,
        units: Option<&[UnitNode]>,
    ) -> Option<Command> {
        // This would be called from input state
        // For now, delegate to helper methods
        None
    }

    /// Get command to navigate to first frame
    pub fn first_frame_command(&self, units: Option<&[UnitNode]>) -> Option<Command> {
        self.find_frame_by_index(units, 0)
            .map(|unit| Command::SelectUnit {
                stream: StreamId::A,
                unit_key: unit.key.clone(),
            })
    }

    /// Get command to navigate to last frame
    pub fn last_frame_command(&self, units: Option<&[UnitNode]>, total_frames: usize) -> Option<Command> {
        let last_index = total_frames.saturating_sub(1);
        self.find_frame_by_index(units, last_index)
            .map(|unit| Command::SelectUnit {
                stream: StreamId::A,
                unit_key: unit.key.clone(),
            })
    }

    /// Get command to navigate to previous frame
    pub fn previous_frame_command(
        &self,
        units: Option<&[UnitNode]>,
        current_frame: usize,
    ) -> Option<Command> {
        if current_frame > 0 {
            self.find_frame_by_index(units, current_frame - 1)
                .map(|unit| Command::SelectUnit {
                    stream: StreamId::A,
                    unit_key: unit.key.clone(),
                })
        } else {
            None
        }
    }

    /// Get command to navigate to next frame
    pub fn next_frame_command(
        &self,
        units: Option<&[UnitNode]>,
        current_frame: usize,
        total_frames: usize,
    ) -> Option<Command> {
        if current_frame < total_frames.saturating_sub(1) {
            self.find_frame_by_index(units, current_frame + 1)
                .map(|unit| Command::SelectUnit {
                    stream: StreamId::A,
                    unit_key: unit.key.clone(),
                })
        } else {
            None
        }
    }

    /// Get command to navigate to specific frame
    pub fn goto_frame_command(&self, units: Option<&[UnitNode]>, frame_index: usize) -> Option<Command> {
        self.find_frame_by_index(units, frame_index)
            .map(|unit| Command::SelectUnit {
                stream: StreamId::A,
                unit_key: unit.key.clone(),
            })
    }

    /// Find frame unit by index
    pub fn find_frame_by_index<'a>(&self, units: Option<&'a [UnitNode]>, frame_index: usize) -> Option<&'a UnitNode> {
        units.and_then(|u| Self::find_frame_recursive(u, frame_index))
    }

    /// Recursively find frame by index
    fn find_frame_recursive(units: &[UnitNode], frame_index: usize) -> Option<&UnitNode> {
        for unit in units {
            if &*unit.unit_type == "FRAME" {
                if unit.frame_index == Some(frame_index) {
                    return Some(unit);
                }
            }

            // Search children
            if let Some(frame) = Self::find_frame_recursive(&unit.children, frame_index) {
                return Some(frame);
            }
        }

        None
    }

    /// Extract frame type from unit type string
    pub fn extract_frame_type(unit_type: &str) -> FrameType {
        FrameType::from_str(unit_type).unwrap_or(FrameType::Unknown)
    }
}

impl Default for NavigationManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bitvue_core::UnitKey;

    #[test]
    fn test_extract_frame_type() {
        assert_eq!(NavigationManager::extract_frame_type("I"), FrameType::Key);
        assert_eq!(NavigationManager::extract_frame_type("KEY"), FrameType::Key);
        assert_eq!(NavigationManager::extract_frame_type("P"), FrameType::Inter);
        assert_eq!(NavigationManager::extract_frame_type("INTER"), FrameType::Inter);
        assert_eq!(NavigationManager::extract_frame_type("B"), FrameType::BFrame);
        assert_eq!(NavigationManager::extract_frame_type("B_FRAME"), FrameType::BFrame);
        assert_eq!(NavigationManager::extract_frame_type("UNKNOWN"), FrameType::Unknown);
    }

    #[test]
    fn test_navigation_bounds() {
        let nav = NavigationManager::new();
        let units = Some(&[]);

        // Test bounds checking
        assert!(nav.previous_frame_command(units, 0).is_none());
        assert!(nav.next_frame_command(units, 0, 0).is_none());
    }

    #[test]
    fn test_first_frame_command() {
        let nav = NavigationManager::new();
        let units: Option<&[UnitNode]> = None;

        // No units should return None
        assert!(nav.first_frame_command(units).is_none());
    }

    #[test]
    fn test_last_frame_command() {
        let nav = NavigationManager::new();
        let units: Option<&[UnitNode]> = None;

        // No units should return None
        assert!(nav.last_frame_command(units, 10).is_none());
    }
}
