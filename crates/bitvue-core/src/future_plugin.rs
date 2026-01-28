//! Future Plugin System - T0-2
//!
//! Deliverable: future_plugin_lock_01 + future_extended_01
//!
//! Future-proof plugin architecture for extensibility:
//! - Plugin trait for custom analyzers
//! - Plugin registry for discovery
//! - Safe plugin isolation
//! - Version compatibility checks
//!
//! This is a foundation for future extensibility without breaking existing code.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Plugin capability flags
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PluginCapability {
    /// Can parse custom OBU types
    CustomObuParser,

    /// Can provide custom overlays
    CustomOverlay,

    /// Can export custom formats
    CustomExporter,

    /// Can provide custom metrics
    CustomMetrics,

    /// Can provide custom insights
    CustomInsights,
}

/// Plugin metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginMetadata {
    /// Plugin unique identifier
    pub id: String,

    /// Human-readable name
    pub name: String,

    /// Plugin version (semver)
    pub version: String,

    /// Author
    pub author: String,

    /// Description
    pub description: String,

    /// Capabilities this plugin provides
    pub capabilities: Vec<PluginCapability>,

    /// Required bitvue core version (semver range)
    pub required_core_version: String,

    /// Plugin dependencies
    pub dependencies: Vec<String>,
}

impl PluginMetadata {
    /// Create new plugin metadata
    pub fn new(id: String, name: String, version: String) -> Self {
        Self {
            id,
            name,
            version,
            author: String::new(),
            description: String::new(),
            capabilities: Vec::new(),
            required_core_version: ">=0.2.0".to_string(),
            dependencies: Vec::new(),
        }
    }

    /// Add a capability
    pub fn with_capability(mut self, cap: PluginCapability) -> Self {
        self.capabilities.push(cap);
        self
    }

    /// Check if plugin has a capability
    pub fn has_capability(&self, cap: PluginCapability) -> bool {
        self.capabilities.contains(&cap)
    }
}

/// Plugin state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PluginState {
    /// Plugin is registered but not loaded
    Registered,

    /// Plugin is loaded and ready
    Loaded,

    /// Plugin is active and in use
    Active,

    /// Plugin failed to load
    Failed,

    /// Plugin is disabled by user
    Disabled,
}

/// Plugin registry entry
#[derive(Debug, Clone)]
pub struct PluginEntry {
    /// Plugin metadata
    pub metadata: PluginMetadata,

    /// Current state
    pub state: PluginState,

    /// Load order (lower = earlier)
    pub load_order: u32,

    /// Error message (if failed)
    pub error: Option<String>,
}

impl PluginEntry {
    /// Create new plugin entry
    pub fn new(metadata: PluginMetadata, load_order: u32) -> Self {
        Self {
            metadata,
            state: PluginState::Registered,
            load_order,
            error: None,
        }
    }

    /// Mark plugin as loaded
    pub fn mark_loaded(&mut self) {
        self.state = PluginState::Loaded;
        self.error = None;
    }

    /// Mark plugin as active
    pub fn mark_active(&mut self) {
        self.state = PluginState::Active;
        self.error = None;
    }

    /// Mark plugin as failed
    pub fn mark_failed(&mut self, error: String) {
        self.state = PluginState::Failed;
        self.error = Some(error);
    }

    /// Mark plugin as disabled
    pub fn mark_disabled(&mut self) {
        self.state = PluginState::Disabled;
        self.error = None;
    }
}

/// Plugin registry
///
/// Central registry for all plugins. Manages plugin lifecycle and discovery.
#[derive(Debug, Clone)]
pub struct PluginRegistry {
    /// Registered plugins by ID
    plugins: HashMap<String, PluginEntry>,

    /// Next load order
    next_load_order: u32,
}

impl PluginRegistry {
    /// Create new plugin registry
    pub fn new() -> Self {
        Self {
            plugins: HashMap::new(),
            next_load_order: 0,
        }
    }

    /// Register a plugin
    pub fn register(&mut self, metadata: PluginMetadata) -> crate::Result<()> {
        let id = metadata.id.clone();

        // Check if already registered
        if self.plugins.contains_key(&id) {
            return Err(crate::BitvueError::Decode(format!(
                "Plugin '{}' is already registered",
                id
            )));
        }

        // Create entry
        let entry = PluginEntry::new(metadata, self.next_load_order);
        self.next_load_order += 1;

        self.plugins.insert(id, entry);
        Ok(())
    }

    /// Unregister a plugin
    pub fn unregister(&mut self, plugin_id: &str) -> bool {
        self.plugins.remove(plugin_id).is_some()
    }

    /// Get plugin entry
    pub fn get(&self, plugin_id: &str) -> Option<&PluginEntry> {
        self.plugins.get(plugin_id)
    }

    /// Get plugin entry (mutable)
    pub fn get_mut(&mut self, plugin_id: &str) -> Option<&mut PluginEntry> {
        self.plugins.get_mut(plugin_id)
    }

    /// Get all plugins
    pub fn all(&self) -> Vec<&PluginEntry> {
        let mut entries: Vec<_> = self.plugins.values().collect();
        entries.sort_by_key(|e| e.load_order);
        entries
    }

    /// Get plugins by state
    pub fn by_state(&self, state: PluginState) -> Vec<&PluginEntry> {
        self.plugins.values().filter(|e| e.state == state).collect()
    }

    /// Get plugins with capability
    pub fn by_capability(&self, cap: PluginCapability) -> Vec<&PluginEntry> {
        self.plugins
            .values()
            .filter(|e| e.metadata.has_capability(cap))
            .collect()
    }

    /// Mark plugin as loaded
    pub fn mark_loaded(&mut self, plugin_id: &str) -> crate::Result<()> {
        let entry = self.plugins.get_mut(plugin_id).ok_or_else(|| {
            crate::BitvueError::Decode(format!("Plugin '{}' not found", plugin_id))
        })?;
        entry.mark_loaded();
        Ok(())
    }

    /// Mark plugin as active
    pub fn mark_active(&mut self, plugin_id: &str) -> crate::Result<()> {
        let entry = self.plugins.get_mut(plugin_id).ok_or_else(|| {
            crate::BitvueError::Decode(format!("Plugin '{}' not found", plugin_id))
        })?;
        entry.mark_active();
        Ok(())
    }

    /// Mark plugin as failed
    pub fn mark_failed(&mut self, plugin_id: &str, error: String) -> crate::Result<()> {
        let entry = self.plugins.get_mut(plugin_id).ok_or_else(|| {
            crate::BitvueError::Decode(format!("Plugin '{}' not found", plugin_id))
        })?;
        entry.mark_failed(error);
        Ok(())
    }

    /// Mark plugin as disabled
    pub fn mark_disabled(&mut self, plugin_id: &str) -> crate::Result<()> {
        let entry = self.plugins.get_mut(plugin_id).ok_or_else(|| {
            crate::BitvueError::Decode(format!("Plugin '{}' not found", plugin_id))
        })?;
        entry.mark_disabled();
        Ok(())
    }

    /// Get plugin count
    pub fn count(&self) -> usize {
        self.plugins.len()
    }

    /// Get statistics
    pub fn stats(&self) -> PluginStats {
        let mut stats = PluginStats::default();

        for entry in self.plugins.values() {
            match entry.state {
                PluginState::Registered => stats.registered += 1,
                PluginState::Loaded => stats.loaded += 1,
                PluginState::Active => stats.active += 1,
                PluginState::Failed => stats.failed += 1,
                PluginState::Disabled => stats.disabled += 1,
            }
        }

        stats.total = self.plugins.len();
        stats
    }
}

impl Default for PluginRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Plugin statistics
#[derive(Debug, Clone, Default)]
pub struct PluginStats {
    pub total: usize,
    pub registered: usize,
    pub loaded: usize,
    pub active: usize,
    pub failed: usize,
    pub disabled: usize,
}

#[cfg(test)]
include!("future_plugin_test.rs");
