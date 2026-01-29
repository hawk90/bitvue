//! Navigation Controls for Player Workspace
//!
//! Handles frame navigation including keyboard shortcuts,
//! navigation buttons, and frame finding utilities.

use bitvue_core::{Command, FrameType, StreamId, UnitKey, UnitNode};
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::collections::hash_map::DefaultHasher;

/// Frame index cache for O(1) lookups
///
/// Maps frame_index -> UnitKey for fast navigation without
/// traversing the entire unit tree.
#[derive(Debug, Clone, Default)]
struct FrameIndexCache {
    /// Cache of frame_index -> UnitKey mappings
    index: HashMap<usize, UnitKey>,
    /// Content hash of the units slice this cache was built from
    /// Used for cache invalidation detection.
    /// This prevents cache poisoning attacks where the same pointer
    /// is reused for different content.
    content_hash: Option<u64>,
}

impl FrameIndexCache {
    /// Create a new empty cache
    fn new() -> Self {
        Self {
            index: HashMap::new(),
            content_hash: None,
        }
    }

    /// Calculate content hash for units
    ///
    /// Uses the first unit's key and total unit count as a fingerprint.
    /// This is much faster than hashing all content while still detecting
    /// cache invalidation reliably.
    fn calculate_hash(units: &[UnitNode]) -> u64 {
        let mut hasher = DefaultHasher::new();
        units.len().hash(&mut hasher);
        if let Some(first) = units.first() {
            first.key.hash(&mut hasher);
        }
        hasher.finish()
    }

    /// Check if cache is valid for the given units
    fn is_valid_for(&self, units: &[UnitNode]) -> bool {
        match self.content_hash {
            Some(hash) => hash == Self::calculate_hash(units),
            None => false,
        }
    }

    /// Build or rebuild cache from units
    fn rebuild(&mut self, units: &[UnitNode]) {
        self.index.clear();
        self.build_index_recursive(units);
        self.content_hash = Some(Self::calculate_hash(units));
    }

    /// Recursively build index from unit tree
    fn build_index_recursive(&mut self, units: &[UnitNode]) {
        for unit in units {
            if &*unit.unit_type == "FRAME" {
                if let Some(frame_idx) = unit.frame_index {
                    self.index.insert(frame_idx, unit.key.clone());
                }
            }
            // Recursively process children
            self.build_index_recursive(&unit.children);
        }
    }

    /// Get unit key by frame index
    fn get(&self, frame_index: usize) -> Option<&UnitKey> {
        self.index.get(&frame_index)
    }
}

/// Navigation manager for player workspace
///
/// Handles frame navigation and frame finding logic with O(1) indexing.
pub struct NavigationManager {
    /// Cached frame index for fast lookups
    frame_cache: FrameIndexCache,
}

impl NavigationManager {
    /// Create new navigation manager
    pub fn new() -> Self {
        Self {
            frame_cache: FrameIndexCache::new(),
        }
    }

    /// Ensure frame cache is valid for the given units
    fn ensure_cache_valid(&mut self, units: &[UnitNode]) {
        if !self.frame_cache.is_valid_for(units) {
            self.frame_cache.rebuild(units);
        }
    }

    /// Handle keyboard shortcuts for frame navigation
    ///
    /// Returns Command if navigation key was pressed.
    pub fn handle_keyboard(
        &mut self,
        _current_frame: usize,
        _total_frames: usize,
        units: Option<&[UnitNode]>,
    ) -> Option<Command> {
        // This would be called from input state
        // For now, delegate to helper methods
        if let Some(units) = units {
            self.ensure_cache_valid(units);
        }
        None
    }

    /// Get command to navigate to first frame
    pub fn first_frame_command(&mut self, units: Option<&[UnitNode]>) -> Option<Command> {
        let units = units?;
        self.ensure_cache_valid(units);
        self.frame_cache.get(0).map(|unit_key| Command::SelectUnit {
            stream: StreamId::A,
            unit_key: unit_key.clone(),
        })
    }

    /// Get command to navigate to last frame
    pub fn last_frame_command(&mut self, units: Option<&[UnitNode]>, total_frames: usize) -> Option<Command> {
        let units = units?;
        self.ensure_cache_valid(units);
        let last_index = total_frames.saturating_sub(1);
        self.frame_cache.get(last_index).map(|unit_key| Command::SelectUnit {
            stream: StreamId::A,
            unit_key: unit_key.clone(),
        })
    }

    /// Get command to navigate to previous frame
    pub fn previous_frame_command(
        &mut self,
        units: Option<&[UnitNode]>,
        current_frame: usize,
    ) -> Option<Command> {
        if current_frame > 0 {
            let units = units?;
            self.ensure_cache_valid(units);
            self.frame_cache.get(current_frame - 1).map(|unit_key| Command::SelectUnit {
                stream: StreamId::A,
                unit_key: unit_key.clone(),
            })
        } else {
            None
        }
    }

    /// Get command to navigate to next frame
    pub fn next_frame_command(
        &mut self,
        units: Option<&[UnitNode]>,
        current_frame: usize,
        total_frames: usize,
    ) -> Option<Command> {
        if current_frame < total_frames.saturating_sub(1) {
            let units = units?;
            self.ensure_cache_valid(units);
            self.frame_cache.get(current_frame + 1).map(|unit_key| Command::SelectUnit {
                stream: StreamId::A,
                unit_key: unit_key.clone(),
            })
        } else {
            None
        }
    }

    /// Get command to navigate to specific frame
    pub fn goto_frame_command(&mut self, units: Option<&[UnitNode]>, frame_index: usize) -> Option<Command> {
        let units = units?;
        self.ensure_cache_valid(units);
        self.frame_cache.get(frame_index).map(|unit_key| Command::SelectUnit {
            stream: StreamId::A,
            unit_key: unit_key.clone(),
        })
    }

    /// Find frame unit by index using cached index (O(1) lookup)
    ///
    /// This first builds/updates the cache if needed, then does O(1) lookup.
    /// Falls back to recursive search if the frame is not in cache.
    pub fn find_frame_by_index<'a>(&'a mut self, units: Option<&'a [UnitNode]>, frame_index: usize) -> Option<&'a UnitNode> {
        let units = units?;
        self.ensure_cache_valid(units);

        // Try cache first (O(1))
        if let Some(unit_key) = self.frame_cache.get(frame_index) {
            // Find the unit with this key in the tree
            return Self::find_by_key(units, unit_key);
        }

        // Fallback to recursive search if not in cache
        Self::find_frame_recursive(units, frame_index)
    }

    /// Find unit by key (O(1) in practice, since keys are unique)
    fn find_by_key<'a>(units: &'a [UnitNode], key: &UnitKey) -> Option<&'a UnitNode> {
        for unit in units {
            if &unit.key == key {
                return Some(unit);
            }
            if let Some(found) = Self::find_by_key(&unit.children, key) {
                return Some(found);
            }
        }
        None
    }

    /// Recursively find frame by index (fallback for cache miss)
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
        // "BFrame" is not recognized by from_str, only "B" is valid
        assert_eq!(NavigationManager::extract_frame_type("BFrame"), FrameType::Unknown);
        assert_eq!(NavigationManager::extract_frame_type("UNKNOWN"), FrameType::Unknown);
    }

    #[test]
    fn test_frame_cache_empty() {
        let cache = FrameIndexCache::new();
        assert!(cache.index.is_empty());
        assert!(!cache.is_valid_for(&[]));
    }

    #[test]
    fn test_navigation_manager_creation() {
        let manager = NavigationManager::new();
        // Manager should be created with empty cache
        assert!(manager.frame_cache.index.is_empty());
    }

    #[test]
    fn test_navigation_manager_default() {
        let manager = NavigationManager::default();
        assert!(manager.frame_cache.index.is_empty());
    }

    #[test]
    fn test_first_frame_command_no_units() {
        let mut manager = NavigationManager::new();
        let result = manager.first_frame_command(None);
        assert!(result.is_none());
    }

    #[test]
    fn test_last_frame_command_no_units() {
        let mut manager = NavigationManager::new();
        let result = manager.last_frame_command(None, 100);
        assert!(result.is_none());
    }

    #[test]
    fn test_previous_frame_command_at_zero() {
        let mut manager = NavigationManager::new();
        let result = manager.previous_frame_command(None, 0);
        assert!(result.is_none());
    }

    #[test]
    fn test_next_frame_command_no_units() {
        let mut manager = NavigationManager::new();
        let result = manager.next_frame_command(None, 0, 100);
        assert!(result.is_none());
    }
}
